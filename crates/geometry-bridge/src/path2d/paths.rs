//! Paths operations at the JSON boundary. Algorithms live in the geometry crates.
use super::codec::{
    decode_gradient, decode_path, decode_region, decode_rings, encode_path, encode_paths,
    fill_rule, offset_options, optional_holes, stroke_options,
};
use crate::Result;
use crate::encode;
use crate::field;
use crate::input;
use planar_geometry::path::BezierPath;
use value_codec::Value;
use value_codec::json;

pub(super) fn dispatch(action: &str, v: &Value) -> Result<Option<Value>> {
    let output = match action {
        "from_rect" => encode_path(&BezierPath::from_rect(field(v, "min")?, field(v, "max")?)?),
        "from_circle" => encode_path(&BezierPath::from_circle(
            field(v, "center")?,
            field(v, "radius")?,
        )?),
        "from_ellipse" => encode_path(&BezierPath::from_ellipse(
            field(v, "center")?,
            field(v, "radiusX")?,
            field(v, "radiusY")?,
        )?),
        "from_polygon" => encode_path(&BezierPath::from_polygon(
            &field::<Vec<[f64; 2]>>(v, "points")?,
            field(v, "closed")?,
        )?),
        "flatten" => {
            encode(decode_path(&v["path"])?.flatten_tol(v["tolerance"].as_f64().unwrap_or(0.25))?)?
        }
        "to_ring" => {
            encode(decode_path(&v["path"])?.to_ring(v["tolerance"].as_f64().unwrap_or(0.25))?)?
        }
        "insert_anchor" => encode_path(
            &decode_path(&v["path"])?.insert_anchor(field(v, "index")?, field(v, "t")?)?,
        ),
        "delete_anchor" => encode_path(&decode_path(&v["path"])?.delete_anchor(field(v, "node")?)?),
        "split_at_anchor" => {
            encode_paths(&decode_path(&v["path"])?.split_at_anchor(field(v, "node")?)?)
        }
        "smooth_anchor" => {
            encode_path(&decode_path(&v["path"])?.make_anchor_smooth(field(v, "node")?)?)
        }
        "corner_anchor" => {
            encode_path(&decode_path(&v["path"])?.make_anchor_corner(field(v, "node")?)?)
        }
        "add_anchors" => encode_path(&decode_path(&v["path"])?.add_anchors()?),
        "subdivide" => encode_path(&decode_path(&v["path"])?.subdivide(field(v, "levels")?)?),
        "set_anchor_position" => encode_path(
            &decode_path(&v["path"])?
                .set_anchor_position(field(v, "node")?, field(v, "position")?)?,
        ),
        "anchor_handles" => {
            let (incoming, outgoing) =
                decode_path(&v["path"])?.anchor_handles(field(v, "node")?)?;
            json!({"incoming": incoming, "outgoing": outgoing})
        }
        "reverse" => encode_path(&decode_path(&v["path"])?.reverse()),
        "simplify" => encode_path(&decode_path(&v["path"])?.simplify(field(v, "tolerance")?)?),
        "outline_stroke" => {
            encode_path(&decode_path(&v["path"])?.outline_stroke(field(v, "width")?)?)
        }
        "outline_stroke_styled" => {
            let opts = stroke_options(v)?;
            encode_paths(&planar_geometry::stroke::outline_stroke_tol(
                &decode_path(&v["path"])?,
                &opts,
                v["tolerance"].as_f64().unwrap_or(0.25),
            )?)
        }
        "dash_spans" => {
            use planar_geometry::stroke::path_dash_spans;
            let pattern = field::<Vec<f64>>(v, "dash")?;
            let spans = path_dash_spans(
                &decode_path(&v["path"])?,
                &pattern,
                v["dashOffset"].as_f64().unwrap_or(0.0),
                v["tolerance"].as_f64().unwrap_or(0.25),
            )?;
            encode(spans)?
        }
        "average_anchors" => {
            use planar_geometry::path::AverageAxis;
            let axis = match v["axis"].as_str().unwrap_or("both") {
                "horizontal" => AverageAxis::Horizontal,
                "vertical" => AverageAxis::Vertical,
                _ => AverageAxis::Both,
            };
            encode_path(
                &decode_path(&v["path"])?
                    .average_anchors(&field::<Vec<usize>>(v, "nodes")?, axis)?,
            )
        }
        "set_anchor_handle" => {
            use planar_geometry::path::HandleSide;
            let side = match v["side"].as_str().unwrap_or("out") {
                "in" => HandleSide::In,
                _ => HandleSide::Out,
            };
            encode_path(&decode_path(&v["path"])?.set_anchor_handle(
                field(v, "node")?,
                side,
                field(v, "pos")?,
            )?)
        }
        "apply_handle_link" => {
            use planar_geometry::path::{HandleLink, HandleSide};
            let moved = match v["moved"].as_str().unwrap_or("out") {
                "in" => HandleSide::In,
                _ => HandleSide::Out,
            };
            let mode = match v["mode"].as_str().unwrap_or("free") {
                "mirrored" => HandleLink::Mirrored,
                "symmetric" => HandleLink::Symmetric,
                _ => HandleLink::Free,
            };
            encode_path(&decode_path(&v["path"])?.apply_handle_link(
                field(v, "node")?,
                moved,
                mode,
            )?)
        }
        "round_corners" => encode_path(&planar_geometry::corners::round_corners(
            &decode_path(&v["path"])?,
            field(v, "radius")?,
        )?),
        "rounded_rect" => {
            use planar_geometry::corners::CornerStyle;
            let styles: Vec<CornerStyle> = v["styles"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|s| match s.as_str().unwrap_or("round") {
                            "chamfer" => CornerStyle::Chamfer,
                            "inverted" => CornerStyle::Inverted,
                            "notch" => CornerStyle::Notch,
                            _ => CornerStyle::Round,
                        })
                        .collect()
                })
                .unwrap_or_default();
            encode_path(&planar_geometry::corners::rounded_rect(
                field(v, "min")?,
                field(v, "max")?,
                &field::<Vec<f64>>(v, "radii")?,
                &styles,
            )?)
        }
        "rounded_polygon" | "rounded_open_polyline" => {
            use planar_geometry::corners::CornerStyle;
            let styles: Vec<CornerStyle> = v["styles"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|s| match s.as_str().unwrap_or("round") {
                            "chamfer" => CornerStyle::Chamfer,
                            "inverted" => CornerStyle::Inverted,
                            "notch" => CornerStyle::Notch,
                            _ => CornerStyle::Round,
                        })
                        .collect()
                })
                .unwrap_or_default();
            let rounded = if action == "rounded_open_polyline" {
                planar_geometry::corners::rounded_open_polyline
            } else {
                planar_geometry::corners::rounded_polygon
            };
            encode_path(&rounded(
                &field::<Vec<[f64; 2]>>(v, "points")?,
                &field::<Vec<f64>>(v, "radii")?,
                &styles,
            )?)
        }
        "join" => {
            let bridge = if v["bridge"].is_null() {
                Vec::new()
            } else {
                decode_path(
                    &json!({"start":[0.,0.],"closed":false,"segments":v["bridge"].clone()}),
                )?
                .segments
            };
            encode_path(&planar_geometry::path::join_paths_with_bridge(
                &decode_path(&v["head"])?,
                field(v, "reverseHead")?,
                &decode_path(&v["tail"])?,
                field(v, "reverseTail")?,
                &bridge,
                v["weld"].as_f64().unwrap_or(1e-6),
            )?)
        }
        "join_tangents" => encode_path(&planar_geometry::path::join_paths_at_tangents(
            &decode_path(&v["head"])?,
            v["reverseHead"].as_bool().unwrap_or(false),
            &decode_path(&v["tail"])?,
            v["reverseTail"].as_bool().unwrap_or(false),
            v["weld"].as_f64().unwrap_or(1e-6),
        )?),
        "close_tangents" => encode_path(&planar_geometry::path::close_path_at_tangents(
            &decode_path(&v["path"])?,
            v["weld"].as_f64().unwrap_or(1e-6),
        )?),
        "tessellate" => {
            let mesh =
                planar_geometry::tessellation::tessellate_rings(&decode_region(v)?, fill_rule(v)?)?;
            json!({
                "positions": mesh.positions,
                "indices": mesh.indices,
                "triangleCount": mesh.triangle_count()
            })
        }
        "offset" => {
            use planar_geometry::path_offset::offset_closed_path;
            let distance = field(v, "distance")?;
            let join_name = v["join"].as_str().unwrap_or("Miter");
            let opts = offset_options(v, distance)?;
            if v.get("rings").is_some_and(|r| !r.is_null()) {
                let rings = planar_geometry::path_offset::offset_closed_rings(
                    &decode_rings(&v["rings"])?,
                    &opts,
                )?;
                let mut paths = Vec::with_capacity(rings.len());
                for r in rings {
                    paths.push(BezierPath::from_polyline(&r, true)?);
                }
                encode_paths(&paths)
            } else {
                let path = decode_path(&v["path"])?;
                if path.closed
                    || v.get("holes").is_some_and(|h| !h.is_null())
                    || v["region"].as_bool() == Some(true)
                {
                    encode_paths(&offset_closed_path(&path, &optional_holes(&v)?, &opts)?)
                } else {
                    encode_paths(&path.offset(distance, join_name, opts.segments)?)
                }
            }
        }
        "smooth" => encode_path(&decode_path(&v["path"])?.smooth()?),
        "open_path" => encode_path(&decode_path(&v["path"])?.open_path()?),
        "delete_segments" => {
            encode_paths(
                &decode_path(&v["path"])?.delete_segments(&field::<Vec<usize>>(v, "nodes")?)?,
            )
        }
        "merge_by_color" => {
            let raw = v["shapes"]
                .as_array()
                .ok_or_else(|| input("shapes must be an array"))?;
            let mut shapes = Vec::with_capacity(raw.len());
            for s in raw {
                shapes.push(planar_geometry::pathfinder::ColoredRegion {
                    rings: field(s, "rings")?,
                    color: field(s, "color")?,
                });
            }
            let out = planar_geometry::pathfinder::merge_by_color(&shapes)?;
            encode(
                out.into_iter()
                    .map(|s| json!({"rings":s.rings,"color":s.color}))
                    .collect::<Vec<_>>(),
            )?
        }
        "arrow_markers" => {
            use planar_geometry::stroke::{ArrowMarker, path_arrow_markers};
            let parse = |s: &str| match s {
                "arrow" => ArrowMarker::Arrow,
                "chevron" => ArrowMarker::Chevron,
                "dot" => ArrowMarker::Dot,
                "bar" => ArrowMarker::Bar,
                _ => ArrowMarker::None,
            };
            encode_paths(&path_arrow_markers(
                &decode_path(&v["path"])?,
                parse(v["start"].as_str().unwrap_or("none")),
                parse(v["end"].as_str().unwrap_or("none")),
                field(v, "width")?,
            )?)
        }
        "sample_gradient" => encode(decode_gradient(v)?.sample_at(field(v, "point")?).to_rgba())?,
        _ => return Ok(None),
    };
    Ok(Some(output))
}
