use super::{Result, Value, field};
pub fn measure(v: Value) -> Result<Value> {
    let a: brep_core::Model = field(&v, "a")?;
    let b: brep_core::Model = field(&v, "b")?;
    let tolerance_uv: f64 = field(&v, "toleranceUv")?;
    let mut result = brep_core::face_domain::distance_between_faces(
        &a,
        field(&v, "faceA")?,
        &b,
        field(&v, "faceB")?,
        field(&v, "toleranceMm")?,
        tolerance_uv,
        field(&v, "maxCells")?,
        field(&v, "maxDomainCells")?,
    )?
    .to_value();
    if let Value::Object(ref mut object) = result {
        object.insert("toleranceUv".into(), value_codec::json!(tolerance_uv));
    }
    Ok(result)
}
