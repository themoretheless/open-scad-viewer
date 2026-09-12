//! JSON transport for planar path / effects / editor helpers.
use crate::{Result, encode, field, input};
use planar_geometry::edit;
use planar_geometry::effects::{self, ArcMode, StippleKind};
use planar_geometry::path::{BezierPath, PathSegment};
use planar_geometry::scissors;
use value_codec::{Value, json};

fn encode_path(path: &BezierPath) -> Value {
    let segments: Vec<Value> = path
        .segments
        .iter()
        .map(|seg| match *seg {
            PathSegment::Line { to } => json!({"type":"line","to":to}),
            PathSegment::Cubic { c1, c2, to } => {
                json!({"type":"cubic","c1":c1,"c2":c2,"to":to})
            }
        })
        .collect();
    json!({"start":path.start,"segments":segments,"closed":path.closed})
}

fn decode_path(v: &Value) -> Result<BezierPath> {
    let start: [f64; 2] = field(v, "start")?;
    let closed: bool = field(v, "closed")?;
    let raw = v["segments"]
        .as_array()
        .ok_or_else(|| input("path.segments must be an array"))?;
    let mut segments = Vec::with_capacity(raw.len());
    for seg in raw {
        match seg["type"].as_str().unwrap_or("") {
            "line" => segments.push(PathSegment::Line {
                to: field(seg, "to")?,
            }),
            "cubic" => segments.push(PathSegment::Cubic {
                c1: field(seg, "c1")?,
                c2: field(seg, "c2")?,
                to: field(seg, "to")?,
            }),
            other => return Err(input(format!("Unknown path segment type '{other}'"))),
        }
    }
    if closed {
        Ok(BezierPath::closed(start, segments)?)
    } else {
        Ok(BezierPath::open(start, segments)?)
    }
}

fn decode_snap_geometry(v: &Value) -> Result<edit::SnapGeometry> {
    use edit::SnapGeometry;
    let outline = || {
        if v["outline"].is_null() {
            Ok(None)
        } else {
            decode_path(&v["outline"]).map(Some)
        }
    };
    Ok(match v["type"].as_str().unwrap_or("") {
        "line" => SnapGeometry::Line {
            start: field(v, "start")?,
            end: field(v, "end")?,
        },
        "rectangle" => SnapGeometry::Rectangle {
            min: field(v, "min")?,
            max: field(v, "max")?,
            outline: outline()?,
        },
        "ellipse" => SnapGeometry::Ellipse {
            center: field(v, "center")?,
            radii: field(v, "radii")?,
        },
        "polygon" => SnapGeometry::Polygon {
            points: field(v, "points")?,
            closed: field(v, "closed")?,
            outline: outline()?,
        },
        "polyline" => SnapGeometry::Polyline(field(v, "points")?),
        "path" => SnapGeometry::Path(decode_path(&v["path"])?),
        "compound" => SnapGeometry::Compound(decode_paths(&v["paths"])?),
        _ => return Err(input("Unknown snap geometry type")),
    })
}

fn encode_cut_hit(hit: scissors::CutHit) -> Value {
    json!({"segmentIndex":hit.segment_index,"t":hit.t,"distance":hit.distance,"point":hit.point})
}

fn decode_cut_hit(v: &Value) -> Result<scissors::CutHit> {
    Ok(scissors::CutHit {
        segment_index: field(v, "segmentIndex")?,
        t: field(v, "t")?,
        distance: v["distance"].as_f64().unwrap_or(0.),
        point: if v["point"].is_null() {
            [0., 0.]
        } else {
            field(v, "point")?
        },
    })
}

fn encode_path_pieces(pieces: &[Vec<BezierPath>]) -> Value {
    Value::Array(pieces.iter().map(|paths| encode_paths(paths)).collect())
}

fn encode_paths(paths: &[BezierPath]) -> Value {
    Value::Array(paths.iter().map(encode_path).collect())
}

fn decode_paths(value: &Value) -> Result<Vec<BezierPath>> {
    value
        .as_array()
        .ok_or_else(|| input("paths must be an array"))?
        .iter()
        .map(decode_path)
        .collect()
}

fn optional_holes(value: &Value) -> Result<Vec<BezierPath>> {
    match value.get("holes") {
        Some(holes) if !holes.is_null() => decode_paths(holes),
        _ => Ok(Vec::new()),
    }
}

fn fill_rule(value: &Value) -> Result<planar_geometry::tessellation::FillRule> {
    use planar_geometry::tessellation::FillRule;
    match value["fillRule"].as_str().unwrap_or("nonzero") {
        "nonzero" | "nonZero" | "NonZero" => Ok(FillRule::NonZero),
        "evenodd" | "evenOdd" | "EvenOdd" => Ok(FillRule::EvenOdd),
        other => Err(input(format!("Unknown fill rule '{other}'"))),
    }
}

fn offset_options(
    value: &Value,
    distance: f64,
) -> Result<planar_geometry::path_offset::OffsetOptions> {
    Ok(planar_geometry::path_offset::OffsetOptions {
        distance,
        join: planar_geometry::path_offset::parse_join(value["join"].as_str().unwrap_or("Miter")),
        miter_limit: value["miterLimit"].as_f64().unwrap_or(4.),
        fill_rule: fill_rule(value)?,
        tolerance: value["tolerance"].as_f64().unwrap_or(0.25),
        segments: value["segments"].as_u64().unwrap_or(8) as usize,
    })
}

fn decode_region(value: &Value) -> Result<planar_geometry::rings::Rings> {
    if value.get("rings").is_some_and(|r| !r.is_null()) {
        return decode_rings(&value["rings"]);
    }
    let tolerance = value["tolerance"].as_f64().unwrap_or(0.25);
    let mut rings = vec![decode_path(&value["path"])?.to_ring(tolerance)?];
    for hole in optional_holes(value)? {
        rings.push(hole.to_ring(tolerance)?);
    }
    Ok(rings)
}

fn decode_point(v: &Value) -> Result<[f64; 2]> {
    if let Some(arr) = v.as_array() {
        if arr.len() == 2 {
            let x = arr[0]
                .as_f64()
                .ok_or_else(|| input("point[0] must be a number"))?;
            let y = arr[1]
                .as_f64()
                .ok_or_else(|| input("point[1] must be a number"))?;
            return Ok([x, y]);
        }
    }
    field(v, "xy").or_else(|_| Ok([field(v, "x")?, field(v, "y")?]))
}

fn decode_rings(v: &Value) -> Result<planar_geometry::rings::Rings> {
    let raw = v
        .as_array()
        .ok_or_else(|| input("rings must be an array"))?;
    let mut rings = Vec::with_capacity(raw.len());
    for r in raw {
        let pts = r
            .as_array()
            .ok_or_else(|| input("ring must be an array of points"))?;
        let mut ring = Vec::with_capacity(pts.len());
        for p in pts {
            ring.push(decode_point(p)?);
        }
        rings.push(ring);
    }
    Ok(rings)
}

fn decode_boxes(v: &Value) -> Result<Vec<edit::BBox>> {
    let raw = v
        .as_array()
        .ok_or_else(|| input("boxes must be an array"))?;
    let mut boxes = Vec::with_capacity(raw.len());
    for b in raw {
        boxes.push(edit::BBox {
            min: field(b, "min")?,
            max: field(b, "max")?,
        });
    }
    Ok(boxes)
}

fn decode_gradient(v: &Value) -> Result<polygon_core::appearance::Gradient> {
    use polygon_core::appearance::{Color, Gradient, GradientKind, GradientSpread, GradientStop};
    let kind = match v["kind"].as_str().unwrap_or("linear") {
        "radial" => GradientKind::Radial {
            center: field(v, "center")?,
            radius: field(v, "radius")?,
        },
        "conic" => GradientKind::Conic {
            center: field(v, "center")?,
            angle: v["angle"].as_f64().unwrap_or(0.0),
        },
        _ => GradientKind::Linear {
            p1: field(v, "p1")?,
            p2: field(v, "p2")?,
        },
    };
    let spread = match v["spread"].as_str().unwrap_or("pad") {
        "repeat" => GradientSpread::Repeat,
        "reflect" => GradientSpread::Reflect,
        _ => GradientSpread::Pad,
    };
    let raw = v["stops"]
        .as_array()
        .ok_or_else(|| input("stops must be an array"))?;
    let mut stops = Vec::with_capacity(raw.len());
    for s in raw {
        stops.push(GradientStop {
            offset: field(s, "offset")?,
            color: Color::from_rgba(field(s, "color")?),
        });
    }
    Ok(Gradient {
        kind,
        stops,
        spread,
    })
}

fn encode_colored_mesh(mesh: &polygon_core::gradient_mesh::ColoredMesh) -> Value {
    json!({
        "positions": mesh.positions,
        "colors": mesh.colors,
        "indices": mesh.indices,
        "triangleCount": mesh.triangle_count()
    })
}

pub fn dispatch(v: Value) -> Result<Value> {
    let action = v["action"].as_str().unwrap_or("");
    Ok(match action {
        "from_rect" => encode_path(&BezierPath::from_rect(
            field(&v, "min")?,
            field(&v, "max")?,
        )?),
        "from_circle" => encode_path(&BezierPath::from_circle(
            field(&v, "center")?,
            field(&v, "radius")?,
        )?),
        "from_ellipse" => encode_path(&BezierPath::from_ellipse(
            field(&v, "center")?,
            field(&v, "radiusX")?,
            field(&v, "radiusY")?,
        )?),
        "from_polygon" => encode_path(&BezierPath::from_polygon(
            &field::<Vec<[f64; 2]>>(&v, "points")?,
            field(&v, "closed")?,
        )?),
        "flatten" => {
            encode(decode_path(&v["path"])?.flatten_tol(v["tolerance"].as_f64().unwrap_or(0.25))?)?
        }
        "to_ring" => {
            encode(decode_path(&v["path"])?.to_ring(v["tolerance"].as_f64().unwrap_or(0.25))?)?
        }
        "insert_anchor" => encode_path(
            &decode_path(&v["path"])?.insert_anchor(field(&v, "index")?, field(&v, "t")?)?,
        ),
        "delete_anchor" => {
            encode_path(&decode_path(&v["path"])?.delete_anchor(field(&v, "node")?)?)
        }
        "split_at_anchor" => {
            encode_paths(&decode_path(&v["path"])?.split_at_anchor(field(&v, "node")?)?)
        }
        "smooth_anchor" => {
            encode_path(&decode_path(&v["path"])?.make_anchor_smooth(field(&v, "node")?)?)
        }
        "corner_anchor" => {
            encode_path(&decode_path(&v["path"])?.make_anchor_corner(field(&v, "node")?)?)
        }
        "add_anchors" => encode_path(&decode_path(&v["path"])?.add_anchors()?),
        "subdivide" => encode_path(&decode_path(&v["path"])?.subdivide(field(&v, "levels")?)?),
        "set_anchor_position" => encode_path(
            &decode_path(&v["path"])?
                .set_anchor_position(field(&v, "node")?, field(&v, "position")?)?,
        ),
        "anchor_handles" => {
            let (incoming, outgoing) =
                decode_path(&v["path"])?.anchor_handles(field(&v, "node")?)?;
            json!({"incoming": incoming, "outgoing": outgoing})
        }
        "reverse" => encode_path(&decode_path(&v["path"])?.reverse()),
        "simplify" => encode_path(&decode_path(&v["path"])?.simplify(field(&v, "tolerance")?)?),
        "outline_stroke" => {
            encode_path(&decode_path(&v["path"])?.outline_stroke(field(&v, "width")?)?)
        }
        "outline_stroke_styled" => {
            use planar_geometry::stroke::{LineCap, LineJoin, StrokeOptions};
            let cap = match v["cap"].as_str().unwrap_or("butt") {
                "round" => LineCap::Round,
                "square" => LineCap::Square,
                _ => LineCap::Butt,
            };
            let join = match v["join"].as_str().unwrap_or("miter") {
                "round" => LineJoin::Round,
                "bevel" => LineJoin::Bevel,
                _ => LineJoin::Miter,
            };
            let dash = if v.get("dash").is_some_and(|d| !d.is_null()) {
                Some(field::<Vec<f64>>(&v, "dash")?)
            } else {
                None
            };
            let opts = StrokeOptions {
                width: field(&v, "width")?,
                cap,
                join,
                miter_limit: v["miterLimit"].as_f64().unwrap_or(4.0),
                dash,
                dash_offset: v["dashOffset"].as_f64().unwrap_or(0.0),
            };
            encode_paths(&planar_geometry::stroke::outline_stroke_tol(
                &decode_path(&v["path"])?,
                &opts,
                v["tolerance"].as_f64().unwrap_or(0.25),
            )?)
        }
        "dash_spans" => {
            use planar_geometry::stroke::path_dash_spans;
            let pattern = field::<Vec<f64>>(&v, "dash")?;
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
                    .average_anchors(&field::<Vec<usize>>(&v, "nodes")?, axis)?,
            )
        }
        "set_anchor_handle" => {
            use planar_geometry::path::HandleSide;
            let side = match v["side"].as_str().unwrap_or("out") {
                "in" => HandleSide::In,
                _ => HandleSide::Out,
            };
            encode_path(&decode_path(&v["path"])?.set_anchor_handle(
                field(&v, "node")?,
                side,
                field(&v, "pos")?,
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
                field(&v, "node")?,
                moved,
                mode,
            )?)
        }
        "round_corners" => encode_path(&planar_geometry::corners::round_corners(
            &decode_path(&v["path"])?,
            field(&v, "radius")?,
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
                field(&v, "min")?,
                field(&v, "max")?,
                &field::<Vec<f64>>(&v, "radii")?,
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
                &field::<Vec<[f64; 2]>>(&v, "points")?,
                &field::<Vec<f64>>(&v, "radii")?,
                &styles,
            )?)
        }
        "concentric_offset" => encode(effects::concentric_offset_with_options(
            &field(&v, "rings")?,
            field(&v, "count")?,
            &offset_options(&v, field(&v, "step")?)?,
        )?)?,
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
                field(&v, "reverseHead")?,
                &decode_path(&v["tail"])?,
                field(&v, "reverseTail")?,
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
        "spiral" => encode_path(&effects::spiral(
            field(&v, "min")?,
            field(&v, "max")?,
            field(&v, "turns")?,
            v["innerRatio"].as_f64().unwrap_or(0.0),
            v["clockwise"].as_bool().unwrap_or(true),
        )?),
        "polar_grid" => encode_paths(&effects::polar_grid(
            field(&v, "min")?,
            field(&v, "max")?,
            field(&v, "circles")?,
            field(&v, "spokes")?,
            v["innerRatio"].as_f64().unwrap_or(0.0),
            v["fullSpokes"].as_bool().unwrap_or(true),
        )?),
        "step_and_repeat" => encode_paths(&effects::step_and_repeat(
            &decode_path(&v["path"])?,
            field(&v, "count")?,
            field(&v, "delta")?,
        )?),
        "radial_repeat" => encode_paths(&effects::radial_repeat(
            &decode_path(&v["path"])?,
            field(&v, "count")?,
            field(&v, "center")?,
        )?),
        "grid_array" => encode_paths(&effects::grid_array(
            &decode_path(&v["path"])?,
            field(&v, "rows")?,
            field(&v, "cols")?,
            field(&v, "spacing")?,
        )?),
        "step_and_repeat_paths" => encode_paths(&effects::step_and_repeat_paths(
            &decode_paths(&v["paths"])?,
            field(&v, "count")?,
            field(&v, "delta")?,
        )?),
        "radial_repeat_paths" => encode_paths(&effects::radial_repeat_paths(
            &decode_paths(&v["paths"])?,
            field(&v, "count")?,
            field(&v, "center")?,
            field(&v, "angleStep")?,
            v["rotateCopies"].as_bool().unwrap_or(true),
        )?),
        "grid_array_paths" => encode_paths(&effects::grid_array_paths(
            &decode_paths(&v["paths"])?,
            field(&v, "rows")?,
            field(&v, "cols")?,
            field(&v, "spacing")?,
        )?),
        "hatch" => {
            let rule = fill_rule(&v)?;
            let spacing = field(&v, "spacing")?;
            let angle = v["angle"].as_f64().unwrap_or(0.0);
            let cross = v["cross"].as_bool().unwrap_or(false);
            if v.get("path").is_some_and(|p| !p.is_null()) {
                let holes = optional_holes(&v)?;
                encode_paths(&effects::hatch_path(
                    &decode_path(&v["path"])?,
                    &holes,
                    rule,
                    spacing,
                    angle,
                    cross,
                    v["tolerance"].as_f64().unwrap_or(0.25),
                )?)
            } else if v.get("rings").is_some_and(|r| !r.is_null()) {
                encode_paths(&effects::hatch_rings(
                    &decode_rings(&v["rings"])?,
                    rule,
                    spacing,
                    angle,
                    cross,
                )?)
            } else {
                encode_paths(&effects::hatch_rings(
                    &[field::<Vec<[f64; 2]>>(&v, "ring")?],
                    rule,
                    spacing,
                    angle,
                    cross,
                )?)
            }
        }
        "stipple" => {
            let kind = match v["kind"].as_str().unwrap_or("dot") {
                "ring" => StippleKind::Ring,
                "cross" => StippleKind::Cross,
                "square" => StippleKind::Square,
                _ => StippleKind::Dot,
            };
            let opts = effects::StippleOptions {
                spacing: field(&v, "spacing")?,
                size: field(&v, "size")?,
                kind,
                jitter: v["jitter"].as_f64().unwrap_or(0.0),
                seed: v["seed"].as_u64().unwrap_or(1),
                fill_rule: fill_rule(&v)?,
            };
            if v.get("path").is_some_and(|p| !p.is_null()) {
                let holes = optional_holes(&v)?;
                encode_paths(&effects::stipple_path(
                    &decode_path(&v["path"])?,
                    &holes,
                    &opts,
                    v["tolerance"].as_f64().unwrap_or(0.25),
                )?)
            } else if v.get("rings").is_some_and(|r| !r.is_null()) {
                encode_paths(&effects::stipple_rings(&decode_rings(&v["rings"])?, &opts)?)
            } else {
                encode_paths(&effects::stipple_rings(
                    &[field::<Vec<[f64; 2]>>(&v, "ring")?],
                    &opts,
                )?)
            }
        }
        "zig_zag" => {
            let path = decode_path(&v["path"])?;
            encode_path(&if v.get("ridges").is_some_and(|r| !r.is_null()) {
                effects::zig_zag_with_options(
                    &path,
                    &effects::ZigZagOptions {
                        amplitude: field(&v, "amplitude")?,
                        ridges: field(&v, "ridges")?,
                        smooth: v["smooth"].as_bool().unwrap_or(false),
                        tolerance: v["tolerance"].as_f64().unwrap_or(0.25),
                    },
                )?
            } else {
                effects::zig_zag(&path, field(&v, "amplitude")?, field(&v, "wavelength")?)?
            })
        }
        "pucker_bloat" => encode_path(&effects::pucker_bloat(
            &decode_path(&v["path"])?,
            field(&v, "amount")?,
        )?),
        "roughen" => encode_path(&effects::roughen_with_options(
            &decode_path(&v["path"])?,
            &effects::RoughenOptions {
                amplitude: field(&v, "amount")?,
                detail: v["detail"].as_u64().unwrap_or(4) as usize,
                smooth: v["smooth"].as_bool().unwrap_or(false),
                seed: v["seed"].as_u64().unwrap_or(1),
            },
        )?),
        "twist" => encode_path(&effects::twist(
            &decode_path(&v["path"])?,
            field(&v, "radians")?,
        )?),
        "scatter" => encode_paths(&effects::scatter(
            &decode_path(&v["path"])?,
            field(&v, "count")?,
            field(&v, "radius")?,
            v["seed"].as_u64().unwrap_or(1),
        )?),
        "scatter_paths" => encode_paths(&effects::scatter_paths(
            &decode_paths(&v["paths"])?,
            &effects::ScatterOptions {
                count: field(&v, "count")?,
                position: v["position"].as_f64().unwrap_or(0.),
                rotation_radians: v["rotationRadians"].as_f64().unwrap_or(0.),
                scale_min: v["scaleMin"].as_f64().unwrap_or(1.),
                scale_max: v["scaleMax"].as_f64().unwrap_or(1.),
                seed: v["seed"].as_u64().unwrap_or(0),
            },
        )?),
        "blend" => encode_paths(&effects::blend(
            &decode_path(&v["a"])?,
            &decode_path(&v["b"])?,
            field(&v, "steps")?,
        )?),
        "free_distort" => encode_path(&effects::free_distort(
            &decode_path(&v["path"])?,
            field(&v, "quad")?,
        )?),
        "arc" => {
            let mode = match v["mode"].as_str().unwrap_or("arc") {
                "pie" => ArcMode::Pie,
                "segment" => ArcMode::Segment,
                _ => ArcMode::Arc,
            };
            encode_path(&effects::arc_path(
                field(&v, "min")?,
                field(&v, "max")?,
                field(&v, "start")?,
                field(&v, "sweep")?,
                mode,
            )?)
        }
        "star" => encode_path(&effects::star(
            field(&v, "min")?,
            field(&v, "max")?,
            field(&v, "points")?,
            v["innerRatio"].as_f64().unwrap_or(0.4),
        )?),
        "snap_geometry" => {
            let shapes = v["geometry"]
                .as_array()
                .ok_or_else(|| input("geometry must be an array"))?
                .iter()
                .map(decode_snap_geometry)
                .collect::<Result<Vec<_>>>()?;
            match edit::find_geometry_snap(
                field(&v, "cursor")?,
                field(&v, "threshold")?,
                v["grid"].as_f64(),
                &shapes,
            )? {
                Some(h) => {
                    json!({"point":h.point,"kind":format!("{:?}",h.kind),"distance":h.distance})
                }
                None => Value::Null,
            }
        }
        "drag_snap" => {
            let moving = edit::BBox {
                min: field(&v["moving"], "min")?,
                max: field(&v["moving"], "max")?,
            };
            let result = edit::find_drag_snap(
                moving,
                &decode_boxes(&v["targets"])?,
                field(&v, "threshold")?,
            )?;
            let guides: Vec<_> = result
                .guides
                .iter()
                .map(|g| {
                    json!({
                        "vertical":g.vertical,"position":g.position,
                        "moving":{"min":g.moving.min,"max":g.moving.max},
                        "target":{"min":g.target.min,"max":g.target.max},
                    })
                })
                .collect();
            json!({"delta":result.delta,"guides":guides})
        }
        "distance_marks" => {
            let moving = edit::BBox {
                min: field(&v["moving"], "min")?,
                max: field(&v["moving"], "max")?,
            };
            Value::Array(
                edit::compute_distance_marks(moving, &decode_boxes(&v["targets"])?)
                    .iter()
                    .map(|m| json!({"from":m.from,"to":m.to,"distance":m.distance}))
                    .collect(),
            )
        }
        "intersecting_paths" => Value::Array(
            edit::intersecting_paths(
                &decode_paths(&v["paths"])?,
                field(&v, "a")?,
                field(&v, "b")?,
            )?
            .into_iter()
            .map(|(angle, point)| json!({"angle":angle,"point":point}))
            .collect(),
        ),
        "intersecting_directions" => encode(edit::intersecting_path_directions(
            &decode_paths(&v["paths"])?,
            field(&v, "a")?,
            field(&v, "b")?,
        )?)?,
        "snap" => {
            let paths_v = v["paths"]
                .as_array()
                .ok_or_else(|| input("paths must be an array"))?;
            let paths: Vec<BezierPath> = paths_v.iter().map(decode_path).collect::<Result<_>>()?;
            match edit::find_snap(
                field(&v, "cursor")?,
                field(&v, "threshold")?,
                v["grid"].as_f64(),
                &paths,
            )? {
                Some(h) => {
                    json!({"point":h.point,"kind":format!("{:?}",h.kind),"distance":h.distance})
                }
                None => Value::Null,
            }
        }
        "align" => {
            let boxes = decode_boxes(&v["boxes"])?;
            let reference = edit::BBox {
                min: field(&v["reference"], "min")?,
                max: field(&v["reference"], "max")?,
            };
            let h = match v["h"].as_str() {
                Some("left") => Some(edit::HAlign::Left),
                Some("center") => Some(edit::HAlign::Center),
                Some("right") => Some(edit::HAlign::Right),
                _ => None,
            };
            let vert = match v["v"].as_str() {
                Some("top") => Some(edit::VAlign::Top),
                Some("center") => Some(edit::VAlign::Center),
                Some("bottom") => Some(edit::VAlign::Bottom),
                _ => None,
            };
            encode(edit::align_boxes(&boxes, reference, h, vert))?
        }
        "distribute" => encode(edit::distribute_centers(
            &decode_boxes(&v["boxes"])?,
            v["horizontal"].as_bool().unwrap_or(true),
        )?)?,
        "path_hit_test" => match scissors::hit_test(
            &decode_path(&v["path"])?,
            field(&v, "click")?,
            field(&v, "maxDist")?,
        )? {
            Some(hit) => encode_cut_hit(hit),
            None => Value::Null,
        },
        "path_cut_at" => encode_paths(&scissors::cut_at(
            &decode_path(&v["path"])?,
            decode_cut_hit(&v["hit"])?,
        )?),
        "path_cut_many" => {
            let hits = v["hits"]
                .as_array()
                .ok_or_else(|| input("hits must be an array"))?
                .iter()
                .map(decode_cut_hit)
                .collect::<Result<Vec<_>>>()?;
            encode_paths(&scissors::cut_at_many(&decode_path(&v["path"])?, &hits)?)
        }
        "compound_hit_test" => match scissors::hit_test_compound(
            &decode_paths(&v["paths"])?,
            field(&v, "click")?,
            field(&v, "maxDist")?,
        )? {
            Some(cut) => {
                let mut hit = encode_cut_hit(cut.hit);
                hit["ring"] = json!(cut.ring);
                hit
            }
            None => Value::Null,
        },
        "compound_knife_hits" => {
            let hits = scissors::knife_hits_compound(
                &decode_paths(&v["paths"])?,
                field(&v, "a")?,
                field(&v, "b")?,
            )?;
            Value::Array(
                hits.into_iter()
                    .map(|cut| {
                        let mut hit = encode_cut_hit(cut.hit);
                        hit["ring"] = json!(cut.ring);
                        hit
                    })
                    .collect(),
            )
        }
        "compound_cut_at" => encode_path_pieces(&scissors::cut_compound_at(
            &decode_paths(&v["paths"])?,
            scissors::CompoundCutHit {
                ring: field(&v["hit"], "ring")?,
                hit: decode_cut_hit(&v["hit"])?,
            },
        )?),
        "compound_cut_many" => {
            let hits = v["hits"]
                .as_array()
                .ok_or_else(|| input("hits must be an array"))?
                .iter()
                .map(|hit| {
                    Ok(scissors::CompoundCutHit {
                        ring: field(hit, "ring")?,
                        hit: decode_cut_hit(hit)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            encode_path_pieces(&scissors::cut_compound_at_many(
                &decode_paths(&v["paths"])?,
                &hits,
            )?)
        }
        "compound_scissors" => encode_path_pieces(&scissors::scissors_cut_compound(
            &decode_paths(&v["paths"])?,
            field(&v, "click")?,
            field(&v, "maxDist")?,
        )?),
        "compound_knife" => encode_path_pieces(&scissors::knife_cut_compound(
            &decode_paths(&v["paths"])?,
            field(&v, "a")?,
            field(&v, "b")?,
        )?),
        "compound_knife_split" => encode_path_pieces(&scissors::knife_split_compound(
            &decode_paths(&v["paths"])?,
            field(&v, "a")?,
            field(&v, "b")?,
        )?),
        "scissors" => encode_paths(&edit::scissors_cut(
            &decode_path(&v["path"])?,
            field(&v, "click")?,
            field(&v, "maxDist")?,
        )?),
        "knife" => encode_paths(&edit::knife_cut(
            &decode_path(&v["path"])?,
            field(&v, "a")?,
            field(&v, "b")?,
        )?),
        "knife_split" => encode_paths(&edit::knife_split(
            &decode_path(&v["path"])?,
            field(&v, "a")?,
            field(&v, "b")?,
        )?),
        "tessellate" => {
            let mesh = planar_geometry::tessellation::tessellate_rings(
                &decode_region(&v)?,
                fill_rule(&v)?,
            )?;
            json!({
                "positions": mesh.positions,
                "indices": mesh.indices,
                "triangleCount": mesh.triangle_count()
            })
        }
        "measure" => {
            let m = edit::measure(field(&v, "a")?, field(&v, "b")?)?;
            json!({"a":m.a,"b":m.b,"distance":m.distance,"angleDeg":m.angle_deg,"delta":m.delta})
        }
        "offset" => {
            use planar_geometry::path_offset::offset_closed_path;
            let distance = field(&v, "distance")?;
            let join_name = v["join"].as_str().unwrap_or("Miter");
            let opts = offset_options(&v, distance)?;
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
                &decode_path(&v["path"])?.delete_segments(&field::<Vec<usize>>(&v, "nodes")?)?,
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
        "snap_rays" => match edit::snap_to_rays(
            field(&v, "start")?,
            field(&v, "cursor")?,
            v["base"].as_f64().unwrap_or(0.),
            v["count"].as_u64().unwrap_or(8) as usize,
            v["toleranceDegrees"].as_f64().unwrap_or(3.),
        )? {
            Some((point, angle)) => json!({"point":point,"angle":angle}),
            None => Value::Null,
        },
        "snap_angle" => encode(edit::snap_to_angle(
            field(&v, "start")?,
            field(&v, "end")?,
            v["stepDeg"].as_f64().unwrap_or(45.0),
        )?)?,
        "drag_guides" => {
            let moving = edit::BBox {
                min: field(&v["moving"], "min")?,
                max: field(&v["moving"], "max")?,
            };
            let targets = decode_boxes(&v["targets"])?;
            let g = edit::drag_align_guides(moving, &targets, field(&v, "threshold")?)?;
            json!({
                "delta": g.delta,
                "guides": g.guides.iter().map(|l| json!({"vertical":l.vertical,"position":l.position})).collect::<Vec<_>>()
            })
        }
        "align_relative" => {
            let boxes = decode_boxes(&v["boxes"])?;
            let relative = match v["relative"].as_str().unwrap_or("selection") {
                "key" | "keyObject" => edit::RelativeTo::KeyObject,
                "page" => edit::RelativeTo::Page,
                "last" | "lastSelected" => edit::RelativeTo::LastSelected,
                _ => edit::RelativeTo::Selection,
            };
            let page = if v["page"].is_object() {
                Some(edit::BBox {
                    min: field(&v["page"], "min")?,
                    max: field(&v["page"], "max")?,
                })
            } else {
                None
            };
            let h = match v["h"].as_str() {
                Some("left") => Some(edit::HAlign::Left),
                Some("center") => Some(edit::HAlign::Center),
                Some("right") => Some(edit::HAlign::Right),
                _ => None,
            };
            let vert = match v["v"].as_str() {
                Some("top") => Some(edit::VAlign::Top),
                Some("center") => Some(edit::VAlign::Center),
                Some("bottom") => Some(edit::VAlign::Bottom),
                _ => None,
            };
            encode(edit::align_boxes_relative(
                &boxes,
                relative,
                v["key"].as_u64().map(|n| n as usize),
                page,
                v["last"].as_u64().map(|n| n as usize),
                h,
                vert,
            )?)?
        }
        "distribute_objects" => {
            let anchor = match v["anchor"].as_str().unwrap_or("center") {
                "start" | "leftOrTop" => edit::DistributeAnchor::LeftOrTop,
                "end" | "rightOrBottom" => edit::DistributeAnchor::RightOrBottom,
                _ => edit::DistributeAnchor::Center,
            };
            encode(edit::distribute_objects(
                &decode_boxes(&v["boxes"])?,
                v["horizontal"].as_bool().unwrap_or(true),
                anchor,
            )?)?
        }
        "distribute_spacing" => encode(edit::distribute_spacing(
            &decode_boxes(&v["boxes"])?,
            v["horizontal"].as_bool().unwrap_or(true),
            field(&v, "gap")?,
        )?)?,
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
                field(&v, "width")?,
            )?)
        }
        "recolor" => {
            use polygon_core::appearance::{Color, shift_color};
            let c = field::<[u8; 4]>(&v, "color")?;
            let out = shift_color(
                Color::from_rgba(c),
                field(&v, "hue")?,
                field(&v, "sat")?,
                field(&v, "light")?,
            );
            encode(out.to_rgba())?
        }
        "sample_gradient" => encode(
            decode_gradient(&v)?
                .sample_at(field(&v, "point")?)
                .to_rgba(),
        )?,
        "gradient_fill_mesh" => {
            let mesh = polygon_core::gradient_mesh::gradient_fill_rings(
                &decode_region(&v)?,
                &decode_gradient(&v)?,
                fill_rule(&v)? == planar_geometry::tessellation::FillRule::EvenOdd,
            )?;
            encode_colored_mesh(&mesh)
        }
        "gradient_stroke_mesh" => {
            use planar_geometry::stroke::{LineCap, LineJoin, StrokeOptions};
            use polygon_core::gradient_mesh::{GradientSampleMode, gradient_stroke_mesh};
            let cap = match v["cap"].as_str().unwrap_or("butt") {
                "round" => LineCap::Round,
                "square" => LineCap::Square,
                _ => LineCap::Butt,
            };
            let join = match v["join"].as_str().unwrap_or("miter") {
                "round" => LineJoin::Round,
                "bevel" => LineJoin::Bevel,
                _ => LineJoin::Miter,
            };
            let dash = if v.get("dash").is_some_and(|d| !d.is_null()) {
                Some(field::<Vec<f64>>(&v, "dash")?)
            } else {
                None
            };
            let opts = StrokeOptions {
                width: field(&v, "width")?,
                cap,
                join,
                miter_limit: v["miterLimit"].as_f64().unwrap_or(4.0),
                dash,
                dash_offset: v["dashOffset"].as_f64().unwrap_or(0.0),
            };
            let mode = match v["mode"].as_str().unwrap_or("spatial") {
                "along" | "alongPath" | "AlongPath" => GradientSampleMode::AlongPath,
                _ => GradientSampleMode::Spatial,
            };
            encode_colored_mesh(&gradient_stroke_mesh(
                &decode_path(&v["path"])?,
                &opts,
                &decode_gradient(&v)?,
                mode,
            )?)
        }
        _ => return Err(input(format!("Unknown path2d action '{action}'"))),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(min: [f64; 2], max: [f64; 2]) -> Value {
        encode_path(&BezierPath::from_rect(min, max).unwrap())
    }

    fn mesh_area(mesh: &Value) -> f64 {
        let points: Vec<[f64; 2]> = field(mesh, "positions").unwrap();
        let indices: Vec<usize> = field(mesh, "indices").unwrap();
        indices
            .chunks_exact(3)
            .map(|t| {
                let [a, b, c] = [points[t[0]], points[t[1]], points[t[2]]];
                ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() * 0.5
            })
            .sum()
    }

    #[test]
    fn dispatches_ridge_smoothing_and_keeps_legacy_repeat_defaults() {
        let line = encode_path(&BezierPath::from_polyline(&[[0., 0.], [8., 0.]], false).unwrap());
        let wave = decode_path(
            &dispatch(
                json!({"action":"zig_zag","path":line,"amplitude":1.,"ridges":3,"smooth":true}),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            wave.anchors(),
            vec![[0., 0.], [2., 1.], [4., -1.], [6., 1.], [8., 0.]]
        );
        assert!(
            wave.segments
                .iter()
                .all(|s| matches!(s, PathSegment::Cubic { .. }))
        );
        let path = rect([0., 0.], [2., 1.]);
        let legacy =
            dispatch(json!({"action":"step_and_repeat","path":path,"count":2,"delta":[10.,0.]}))
                .unwrap();
        let copies = dispatch(
            json!({"action":"step_and_repeat_paths","paths":[path],"count":2,"delta":[10.,0.]}),
        )
        .unwrap();
        assert_eq!(decode_paths(&legacy).unwrap()[0].start, [0., 0.]);
        assert_eq!(decode_paths(&copies).unwrap()[0].start, [10., 0.]);
    }

    #[test]
    fn dispatches_single_ring_stipple_jitter_and_seed_without_dropping_options() {
        let ring = json!([[0., 0.], [12., 0.], [12., 12.], [0., 12.]]);
        let single = dispatch(json!({"action":"stipple","ring":ring,"spacing":4.,"size":0.2,"jitter":0.8,"seed":42,"fillRule":"evenodd"})).unwrap();
        let multi = dispatch(json!({"action":"stipple","rings":[ring],"spacing":4.,"size":0.2,"jitter":0.8,"seed":42,"fillRule":"evenodd"})).unwrap();
        assert_eq!(single, multi);
        let other_seed = dispatch(json!({"action":"stipple","ring":ring,"spacing":4.,"size":0.2,"jitter":0.8,"seed":43,"fillRule":"evenodd"})).unwrap();
        assert_ne!(single, other_seed);
        assert!(!single.as_array().unwrap().is_empty());
    }

    #[test]
    fn dispatches_fill_rule_and_holes_for_mesh_offset_and_gradient() {
        let outer = rect([0., 0.], [10., 10.]);
        let hole = rect([4., 4.], [6., 6.]);
        let evenodd = dispatch(
            json!({"action":"tessellate","path":outer,"holes":[hole],"fillRule":"evenodd"}),
        )
        .unwrap();
        let nonzero = dispatch(
            json!({"action":"tessellate","path":outer,"holes":[hole],"fillRule":"nonzero"}),
        )
        .unwrap();
        assert!((mesh_area(&evenodd) - 96.).abs() < 1e-8);
        assert!((mesh_area(&nonzero) - 100.).abs() < 1e-8);
        let offset = dispatch(json!({"action":"offset","path":outer,"holes":[hole],"distance":0.5,"fillRule":"evenodd","miterLimit":2.,"tolerance":0.01})).unwrap();
        let offset_area: f64 = decode_paths(&offset)
            .unwrap()
            .iter()
            .map(|p| planar_geometry::rings::area(&p.to_ring(0.01).unwrap()))
            .sum();
        assert!((offset_area - 120.).abs() < 1e-8);
        let gradient = dispatch(json!({"action":"gradient_fill_mesh","path":outer,"holes":[hole],"fillRule":"evenodd","kind":"linear","p1":[0.,0.],"p2":[10.,0.],"stops":[{"offset":0.,"color":[255,0,0,255]},{"offset":1.,"color":[0,0,255,255]}]})).unwrap();
        assert!((mesh_area(&gradient) - 96.).abs() < 1e-8);
        assert_eq!(
            gradient["positions"].as_array().unwrap().len(),
            gradient["colors"].as_array().unwrap().len()
        );
    }

    #[test]
    fn dispatches_miter_limit_without_requiring_region_flag_and_preserves_concentric_nesting() {
        let triangle = encode_path(
            &BezierPath::from_polyline(&[[0., 0.], [10., 0.], [0.2, 0.2]], true).unwrap(),
        );
        let low =
            dispatch(json!({"action":"offset","path":triangle,"distance":1.,"miterLimit":1.}))
                .unwrap();
        let high =
            dispatch(json!({"action":"offset","path":triangle,"distance":1.,"miterLimit":100.}))
                .unwrap();
        assert_ne!(low, high);
        let result = dispatch(json!({"action":"concentric_offset","rings":[[[0.,0.],[20.,0.],[20.,20.],[0.,20.]],[[6.,6.],[14.,6.],[14.,14.],[6.,14.]]],"count":2,"step":1.,"fillRule":"evenodd"})).unwrap();
        let levels = result.as_array().unwrap();
        assert_eq!(levels.len(), 2);
        assert_eq!(levels[0].as_array().unwrap().len(), 2);
        assert_eq!(levels[1].as_array().unwrap().len(), 2);
    }

    #[test]
    fn node_editing_and_missing_mesh_tools_are_reachable() {
        let line = encode_path(&BezierPath::from_polyline(&[[0., 0.], [10., 0.]], false).unwrap());
        let subdivided = dispatch(json!({"action":"subdivide","path":line,"levels":2})).unwrap();
        assert_eq!(decode_path(&subdivided).unwrap().segments.len(), 4);
        let split =
            dispatch(json!({"action":"split_at_anchor","path":subdivided,"node":2})).unwrap();
        let paths = decode_paths(&split).unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].segments.last().unwrap().end(), paths[1].start);
        let dashes =
            dispatch(json!({"action":"dash_spans","path":line,"dash":[2.,2.],"tolerance":0.01}))
                .unwrap();
        assert_eq!(dashes.as_array().unwrap().len(), 3);
        let knife = dispatch(json!({"action":"knife_split","path":rect([0.,0.],[10.,10.]),"a":[5.,-1.],"b":[5.,11.]})).unwrap();
        assert_eq!(decode_paths(&knife).unwrap().len(), 2);
        let stroke = dispatch(json!({"action":"gradient_stroke_mesh","path":line,"width":1.,"cap":"square","kind":"linear","p1":[0.,0.],"p2":[10.,0.],"stops":[{"offset":0.,"color":[255,0,0,255]},{"offset":1.,"color":[0,0,255,255]}]})).unwrap();
        assert!((mesh_area(&stroke) - 11.).abs() < 1e-7);
    }

    #[test]
    fn rejects_invalid_holes_and_dash_values_instead_of_discarding_them() {
        let path = rect([0., 0.], [10., 10.]);
        assert!(dispatch(json!({"action":"hatch","path":path,"holes":42,"spacing":1.})).is_err());
        assert!(dispatch(json!({"action":"outline_stroke_styled","path":path,"width":1.,"dash":[2.,"bad",2.]})).is_err());
        assert!(dispatch(json!({"action":"tessellate","path":path,"fillRule":"bad"})).is_err());
    }

    #[test]
    fn semantic_editor_options_survive_transport() {
        let hit = dispatch(
            json!({"action":"snap_geometry","cursor":[4.8,0.],"threshold":0.3,"geometry":[
                {"type":"line","start":[0.,0.],"end":[10.,0.]},
                {"type":"line","start":[4.8,0.],"end":[4.8,10.]}
            ]}),
        )
        .unwrap();
        assert_eq!(hit["kind"].as_str(), Some("Midpoint"));
        assert_eq!(field::<[f64; 2]>(&hit, "point").unwrap(), [5., 0.]);
        let rounded = dispatch(json!({"action":"rounded_open_polyline","points":[[0.,0.],[10.,0.],[10.,10.]],"radii":[0.,-2.,0.]})).unwrap();
        assert_eq!(
            decode_path(&rounded).unwrap().flatten().unwrap(),
            vec![[0., 0.], [8., 0.], [10., 2.], [10., 10.]]
        );
        let ray =
            dispatch(json!({"action":"snap_rays","start":[0.,0.],"cursor":[10.,0.1]})).unwrap();
        assert_eq!(field::<[f64; 2]>(&ray, "point").unwrap(), [10., 0.]);
        let head = encode_path(&BezierPath::from_polyline(&[[0., 0.], [2., 0.]], false).unwrap());
        let tail = encode_path(&BezierPath::from_polyline(&[[4., 2.], [4., 4.]], false).unwrap());
        let joined = dispatch(json!({"action":"join_tangents","head":head,"tail":tail})).unwrap();
        assert_eq!(decode_path(&joined).unwrap().segments.len(), 4);
        let snap = dispatch(json!({"action":"drag_snap","moving":{"min":[0.,0.],"max":[10.,10.]},"targets":[{"min":[1.,20.],"max":[11.,30.]}],"threshold":1.1})).unwrap();
        assert_eq!(field::<[f64; 2]>(&snap, "delta").unwrap(), [1., 0.]);
        let marks = dispatch(json!({"action":"distance_marks","moving":{"min":[0.,0.],"max":[10.,10.]},"targets":[{"min":[-3.,-4.],"max":[15.,16.]}]})).unwrap();
        assert_eq!(marks.as_array().unwrap().len(), 4);
    }

    #[test]
    fn compound_knife_and_scissors_transport_preserves_ring_grouping() {
        let outer = rect([0., 0.], [20., 20.]);
        let hole = rect([2., 2.], [8., 8.]);
        let paths = json!([outer, hole]);
        let hit = dispatch(
            json!({"action":"compound_hit_test","paths":paths,"click":[2.,5.],"maxDist":0.1}),
        )
        .unwrap();
        assert_eq!(hit["ring"].as_u64(), Some(1));
        let pieces = dispatch(json!({"action":"compound_cut_at","paths":paths,"hit":hit})).unwrap();
        let pieces = pieces.as_array().unwrap();
        assert_eq!(pieces.len(), 2);
        assert_eq!(
            decode_paths(&pieces[0]).unwrap()[0],
            decode_path(&outer).unwrap()
        );
        assert!(!decode_paths(&pieces[1]).unwrap()[0].closed);
        let pieces = dispatch(
            json!({"action":"compound_knife_split","paths":paths,"a":[1.,5.],"b":[9.,5.]}),
        )
        .unwrap();
        let pieces = pieces.as_array().unwrap();
        assert_eq!(pieces.len(), 1);
        assert_eq!(decode_paths(&pieces[0]).unwrap().len(), 3);
    }
}
