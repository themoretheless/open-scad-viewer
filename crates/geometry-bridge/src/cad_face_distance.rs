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
        brep_core::face_domain::DistanceLimits {
            tolerance_mm: field(&v, "toleranceMm")?,
            tolerance_uv,
            max_cells: field(&v, "maxCells")?,
            max_domain_cells: field(&v, "maxDomainCells")?,
        },
    )?
    .to_value();
    if let Value::Object(ref mut object) = result {
        object.insert("toleranceUv".into(), value_codec::json!(tolerance_uv));
    }
    Ok(result)
}
