//! JSON transport for planar path / effects / editor helpers.
use crate::{Result, encode, field, input};
use planar_geometry::edit;
use planar_geometry::effects::{self, ArcMode, StippleKind};
use planar_geometry::path::{BezierPath, PathSegment};
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

fn encode_paths(paths: &[BezierPath]) -> Value {
    Value::Array(paths.iter().map(encode_path).collect())
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
            let dash = v["dash"]
                .as_array()
                .map(|a| a.iter().filter_map(|x| x.as_f64()).collect::<Vec<_>>());
            let opts = StrokeOptions {
                width: field(&v, "width")?,
                cap,
                join,
                miter_limit: v["miterLimit"].as_f64().unwrap_or(4.0),
                dash,
                dash_offset: v["dashOffset"].as_f64().unwrap_or(0.0),
            };
            encode_paths(&decode_path(&v["path"])?.outline_stroke_with(&opts)?)
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
        "rounded_polygon" => {
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
            encode_path(&planar_geometry::corners::rounded_polygon(
                &field::<Vec<[f64; 2]>>(&v, "points")?,
                &field::<Vec<f64>>(&v, "radii")?,
                &styles,
            )?)
        }
        "concentric_offset" => encode(effects::concentric_offset(
            &field(&v, "rings")?,
            field(&v, "count")?,
            field(&v, "step")?,
            v["join"].as_str().unwrap_or("Miter"),
            v["segments"].as_u64().unwrap_or(8) as usize,
        )?)?,
        "join" => encode_path(&planar_geometry::path::join_paths(
            &decode_path(&v["head"])?,
            field(&v, "reverseHead")?,
            &decode_path(&v["tail"])?,
            field(&v, "reverseTail")?,
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
        "hatch" => encode_paths(&effects::hatch(
            &field::<Vec<[f64; 2]>>(&v, "ring")?,
            field(&v, "spacing")?,
            v["angle"].as_f64().unwrap_or(0.0),
            v["cross"].as_bool().unwrap_or(false),
        )?),
        "stipple" => {
            let kind = match v["kind"].as_str().unwrap_or("dot") {
                "ring" => StippleKind::Ring,
                "cross" => StippleKind::Cross,
                "square" => StippleKind::Square,
                _ => StippleKind::Dot,
            };
            encode_paths(&effects::stipple(
                &field::<Vec<[f64; 2]>>(&v, "ring")?,
                field(&v, "spacing")?,
                field(&v, "size")?,
                kind,
            )?)
        }
        "zig_zag" => encode_path(&effects::zig_zag(
            &decode_path(&v["path"])?,
            field(&v, "amplitude")?,
            field(&v, "wavelength")?,
        )?),
        "pucker_bloat" => encode_path(&effects::pucker_bloat(
            &decode_path(&v["path"])?,
            field(&v, "amount")?,
        )?),
        "roughen" => encode_path(&effects::roughen(
            &decode_path(&v["path"])?,
            field(&v, "amount")?,
            v["seed"].as_u64().unwrap_or(1),
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
        "measure" => {
            let m = edit::measure(field(&v, "a")?, field(&v, "b")?)?;
            json!({"a":m.a,"b":m.b,"distance":m.distance,"angleDeg":m.angle_deg,"delta":m.delta})
        }
        "offset" => encode_paths(&decode_path(&v["path"])?.offset(
            field(&v, "distance")?,
            v["join"].as_str().unwrap_or("Miter"),
            v["segments"].as_u64().unwrap_or(8) as usize,
        )?),
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
        "sample_gradient" => {
            use polygon_core::appearance::{
                Color, Gradient, GradientKind, GradientSpread, GradientStop,
            };
            let kind = match v["kind"].as_str().unwrap_or("linear") {
                "radial" => GradientKind::Radial {
                    center: field(&v, "center")?,
                    radius: field(&v, "radius")?,
                },
                "conic" => GradientKind::Conic {
                    center: field(&v, "center")?,
                    angle: v["angle"].as_f64().unwrap_or(0.0),
                },
                _ => GradientKind::Linear {
                    p1: field(&v, "p1")?,
                    p2: field(&v, "p2")?,
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
            let g = Gradient {
                kind,
                stops,
                spread,
            };
            encode(g.sample_at(field(&v, "point")?).to_rgba())?
        }
        _ => return Err(input(format!("Unknown path2d action '{action}'"))),
    })
}
