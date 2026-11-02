//! Lossless exact Full Round through the shared native/WASM operation router.
use crate::{field, input, Result, Value};
use brep_core::{
    feature_ops::{full_round_curved::admitted::transport, full_round_exact},
    profile_certificate::original_restriction::wire::material::Limits,
    Model,
};
fn allowed(v: &Value, keys: &[&str]) -> Result<()> {
    let object = v
        .as_object()
        .ok_or_else(|| input("Expected exact Full Round request object"))?;
    if object.keys().any(|k| !keys.contains(&k.as_str())) {
        return Err(input("Unexpected exact Full Round request field"));
    }
    Ok(())
}
fn optional<T: for<'a> value_codec::Deserialize<'a>>(v: &Value, key: &str) -> Result<Option<T>> {
    v.get(key).map(|_| field(v, key)).transpose()
}
fn max_bytes(v: &Value) -> Result<usize> {
    let limit = optional(v, "maxBytes")?.unwrap_or(4 << 20);
    if limit == 0 || limit > 4 << 20 {
        return Err(input("Invalid exact Full Round transport byte allowance"));
    }
    Ok(limit)
}
fn limits(v: &Value) -> Result<Limits> {
    let Some(l) = v.get("limits") else {
        return Ok(Limits::default());
    };
    allowed(
        l,
        &[
            "maxPairProjections",
            "maxAtoms",
            "maxRayCells",
            "refinements",
            "radiusHalvings",
        ],
    )?;
    let d = Limits::default();
    Ok(Limits {
        max_pair_projections: optional(l, "maxPairProjections")?.unwrap_or(d.max_pair_projections),
        max_atoms: optional(l, "maxAtoms")?.unwrap_or(d.max_atoms),
        max_ray_cells: optional(l, "maxRayCells")?.unwrap_or(d.max_ray_cells),
        refinements: optional(l, "refinements")?.unwrap_or(d.refinements),
        radius_halvings: optional(l, "radiusHalvings")?.unwrap_or(d.radius_halvings),
    })
}
pub(crate) fn construct(v: Value) -> Result<Value> {
    allowed(&v, &["op", "source", "faces", "limits", "maxBytes"])?;
    let bytes = max_bytes(&v)?;
    let source: Model = field(&v, "source")?;
    let body = full_round_exact(&source, field(&v, "faces")?, limits(&v)?)?;
    transport::export(&body, bytes)
}
pub(crate) fn restore(v: Value) -> Result<Value> {
    allowed(&v, &["op", "artifact", "maxBytes"])?;
    let bytes = max_bytes(&v)?;
    let artifact = v
        .get("artifact")
        .ok_or_else(|| input("Missing exact Full Round artifact"))?;
    let body = transport::restore(artifact, bytes)?;
    transport::export(&body, bytes)
}
