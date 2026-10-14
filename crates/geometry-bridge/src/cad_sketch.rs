//! Native planar sketch placement and analytic circle/arc display sampling.
use super::{Result, Value, encode, field, input};
pub(crate) use planar_geometry::sketch_shapes::arc_direction;
fn curve(v: &Value) -> Result<planar_geometry::sketch_transform::AnalyticCurve> {
    use planar_geometry::{sketch_shapes::CurveKind, sketch_transform::AnalyticCurve};
    let kind = match v["kind"].as_str() {
        Some("circle") => CurveKind::Circle,
        Some("arc") => CurveKind::Arc,
        _ => return Err(input("Invalid circle or arc parameters.")),
    };
    Ok(AnalyticCurve {
        kind,
        center: field(v, "center")?,
        radius: field(v, "radius")?,
        start: field(v, "start")?,
        sweep: field(v, "sweep")?,
    })
}
pub fn sample(v: Value) -> Result<Value> {
    encode(curve(&v)?.sample()?)
}
pub fn transform(v: Value) -> Result<Value> {
    use planar_geometry::sketch_transform::{self, Transform};
    let mut sketch: Value = field(&v, "sketch")?;
    let analytic = sketch.get("analytic").map(curve).transpose()?;
    let pivot = v
        .get("pivot")
        .filter(|p| !p.is_null())
        .map(|_| field(&v, "pivot"))
        .transpose()?;
    let result = sketch_transform::transform(
        &field::<Vec<[f64; 2]>>(&sketch, "points")?,
        analytic,
        Transform {
            delta: field(&v, "delta")?,
            angle: field(&v, "angle")?,
            scale: field(&v, "scale")?,
            pivot,
        },
    )?;
    sketch["points"] = encode(result.points)?;
    if let Some(curve) = result.analytic {
        let record = sketch
            .get_mut("analytic")
            .ok_or_else(|| input("Missing analytic record"))?;
        record["center"] = encode(curve.center)?;
        record["radius"] = encode(curve.radius)?;
        record["start"] = encode(curve.start)?;
    }
    Ok(sketch)
}

/// Place local 2D/3D points in an authored basis, retaining basis scale.
pub fn world_points(v: Value) -> Result<Value> {
    let points: Vec<Vec<f64>> = field(&v, "points")?;
    if points.len() > 300000 {
        return Err(input("Sketch placement point budget exceeded."));
    }
    let plane: Value = field(&v, "plane")?;
    let origin: [f64; 3] = field(&plane, "origin")?;
    let u: [f64; 3] = field(&plane, "u")?;
    let w: [f64; 3] = field(&plane, "v")?;
    let plane = polygon_core::profile_tools::SketchPlane { origin, u, v: w };
    let local = points
        .into_iter()
        .map(|p| {
            if !(2..=3).contains(&p.len()) {
                return Err(input("Sketch placement requires 2D or 3D points."));
            }
            Ok([p[0], p[1], p.get(2).copied().unwrap_or(0.)])
        })
        .collect::<Result<Vec<_>>>()?;
    let result = plane.place(&local).map_err(|e| input(e.message))?;
    encode(result)
}

/// Legacy direct point transform around the arithmetic centroid, in native code.
pub fn transform_points(v: Value) -> Result<Value> {
    encode(geometry_ops::point_transform::transform_points(
        &field::<Vec<Vec<f64>>>(&v, "points")?,
        &field::<Vec<f64>>(&v, "delta")?,
        field(&v, "angle")?,
        field(&v, "scale")?,
    )?)
}
pub fn slot(v: Value) -> Result<Value> {
    encode(planar_geometry::sketch_shapes::slot(
        field(&v, "a")?,
        field(&v, "b")?,
        field(&v, "width")?,
    )?)
}

#[cfg(test)]
mod slot_tests {
    use super::*;
    #[test]
    fn capsule_bounds_and_invalid_centers() {
        let result = slot(super::super::json!({"a":[0.,0.],"b":[10.,0.],"width":4.})).unwrap();
        let points: Vec<[f64; 2]> = result
            .as_array()
            .unwrap()
            .iter()
            .map(|p| [p[0].as_f64().unwrap(), p[1].as_f64().unwrap()])
            .collect();
        assert_eq!(points.len(), 66);
        assert!((points.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min) + 2.).abs() < 1e-9);
        assert!(
            (points
                .iter()
                .map(|p| p[0])
                .fold(f64::NEG_INFINITY, f64::max)
                - 12.)
                .abs()
                < 1e-9
        );
        assert!(slot(super::super::json!({"a":[0.,0.],"b":[0.,0.],"width":4.})).is_err());
        assert!(slot(super::super::json!({"a":[0.,0.],"b":[10.,0.],"width":0.})).is_err());
    }
}
