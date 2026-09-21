//! JSON codecs and option defaults shared by all 2D operation handlers.
use crate::Result;
use crate::field;
use crate::input;
use planar_geometry::edit;
use planar_geometry::path::BezierPath;
use planar_geometry::path::PathSegment;
use planar_geometry::scissors;
use value_codec::Value;
use value_codec::json;

pub(super) fn encode_path(path: &BezierPath) -> Value {
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

pub(super) fn decode_path(v: &Value) -> Result<BezierPath> {
    let start: [f64; 2] = field(v, "start")?;
    let closed: bool = field(v, "closed")?;
    let raw = v["segments"]
        .as_array()
        .ok_or_else(|| input("path.segments must be an array"))?;
    if raw.len() > planar_geometry::limits::PATH_SEGMENTS {
        return Err(input("Path exceeds segment budget"));
    }
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

pub(super) fn decode_snap_geometry(v: &Value) -> Result<edit::SnapGeometry> {
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

pub(super) fn encode_cut_hit(hit: scissors::CutHit) -> Value {
    json!({"segmentIndex":hit.segment_index,"t":hit.t,"distance":hit.distance,"point":hit.point})
}

pub(super) fn decode_cut_hit(v: &Value) -> Result<scissors::CutHit> {
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

pub(super) fn encode_path_pieces(pieces: &[Vec<BezierPath>]) -> Value {
    Value::Array(pieces.iter().map(|paths| encode_paths(paths)).collect())
}

pub(super) fn encode_paths(paths: &[BezierPath]) -> Value {
    Value::Array(paths.iter().map(encode_path).collect())
}

pub(super) fn decode_paths(value: &Value) -> Result<Vec<BezierPath>> {
    value
        .as_array()
        .ok_or_else(|| input("paths must be an array"))?
        .iter()
        .map(decode_path)
        .collect()
}

pub(super) fn optional_holes(value: &Value) -> Result<Vec<BezierPath>> {
    match value.get("holes") {
        Some(holes) if !holes.is_null() => decode_paths(holes),
        _ => Ok(Vec::new()),
    }
}

pub(super) fn fill_rule(value: &Value) -> Result<planar_geometry::tessellation::FillRule> {
    use planar_geometry::tessellation::FillRule;
    match value["fillRule"].as_str().unwrap_or("nonzero") {
        "nonzero" | "nonZero" | "NonZero" => Ok(FillRule::NonZero),
        "evenodd" | "evenOdd" | "EvenOdd" => Ok(FillRule::EvenOdd),
        other => Err(input(format!("Unknown fill rule '{other}'"))),
    }
}

pub(super) fn offset_options(
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

pub(super) fn decode_region(value: &Value) -> Result<planar_geometry::rings::Rings> {
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

pub(super) fn decode_point(v: &Value) -> Result<[f64; 2]> {
    if let Some(arr) = v.as_array()
        && arr.len() == 2 {
            let x = arr[0]
                .as_f64()
                .ok_or_else(|| input("point[0] must be a number"))?;
            let y = arr[1]
                .as_f64()
                .ok_or_else(|| input("point[1] must be a number"))?;
            return Ok([x, y]);
        }
    field(v, "xy").or_else(|_| Ok([field(v, "x")?, field(v, "y")?]))
}

pub(super) fn decode_rings(v: &Value) -> Result<planar_geometry::rings::Rings> {
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

pub(super) fn decode_boxes(v: &Value) -> Result<Vec<edit::BBox>> {
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

pub(super) fn decode_gradient(v: &Value) -> Result<polygon_core::appearance::Gradient> {
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

pub(super) fn encode_colored_mesh(mesh: &polygon_core::gradient_mesh::ColoredMesh) -> Value {
    json!({
        "positions": mesh.positions,
        "colors": mesh.colors,
        "indices": mesh.indices,
        "triangleCount": mesh.triangle_count()
    })
}

pub(super) fn stroke_options(v: &Value) -> Result<planar_geometry::stroke::StrokeOptions> {
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
        Some(field::<Vec<f64>>(v, "dash")?)
    } else {
        None
    };
    Ok(StrokeOptions {
        width: field(v, "width")?,
        cap,
        join,
        miter_limit: v["miterLimit"].as_f64().unwrap_or(4.0),
        dash,
        dash_offset: v["dashOffset"].as_f64().unwrap_or(0.0),
    })
}
