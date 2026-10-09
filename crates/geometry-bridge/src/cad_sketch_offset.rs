//! Bounded planar sketch validation and miter offset, retaining analytic arcs.
use super::{Result, Value, cad_sketch, encode, field, input};
use planar_geometry::sketch_offset::validate;
fn finite(values: impl IntoIterator<Item = f64>) -> Result<()> {
    if values.into_iter().all(f64::is_finite) {
        Ok(())
    } else {
        Err(input("Contour arithmetic exceeds finite numeric range."))
    }
}
type P = [f64; 2];
pub fn validate_request(v: Value) -> Result<Value> {
    validate(&field::<Vec<P>>(&v, "points")?, field(&v, "closed")?)?;
    Ok(Value::Null)
}
pub fn offset(v: Value) -> Result<Value> {
    let mut sketch: Value = field(&v, "sketch")?;
    let distance: f64 = field(&v, "distance")?;
    if !distance.is_finite() || distance.abs() < 0.001 {
        return Err(input("Enter a nonzero offset."));
    }
    if let Some(curve) = sketch.get_mut("analytic") {
        let radius = field::<f64>(curve, "radius")? + distance;
        finite([radius])?;
        curve["radius"] = encode(radius)?;
        let points = cad_sketch::sample(curve.clone())?;
        sketch["points"] = points;
        return Ok(sketch);
    }
    if !field::<bool>(&sketch, "closed")? {
        return Err(input(
            "Offset requires a closed contour or an analytic arc.",
        ));
    }
    let p: Vec<P> = field(&sketch, "points")?;
    sketch["points"] = encode(planar_geometry::sketch_offset::offset(&p, distance)?)?;
    Ok(sketch)
}
