//! Portable frozen-source reconstruction, not deserialization of a success bit.
use super::*;
use value_codec::{json, Value};
pub const FORMAT: &str = "admitted-full-round/1";
const MAX_BYTES: usize = 4 << 20;
fn field<T: for<'a> value_codec::Deserialize<'a>>(v: &Value, key: &str) -> Result<T> {
    let v = v
        .get(key)
        .ok_or_else(|| error(format!("Missing exact transport field {key}")))?;
    value_codec::from_value(v.clone()).map_err(|e| error(e.to_string()))
}
fn bound(v: &Value, max_bytes: usize) -> Result<()> {
    if max_bytes == 0 || max_bytes > MAX_BYTES {
        return Err(error("Invalid exact transport byte allowance"));
    }
    let bytes = value_codec::encode_binary(v).map_err(|e| error(e.to_string()))?;
    if bytes.len() > max_bytes {
        return Err(error("Exact Full Round transport exceeds byte allowance"));
    }
    Ok(())
}
pub fn export(body: &AdmittedFullRound, max_bytes: usize) -> Result<Value> {
    let r = body.recipe();
    let limits = body.material().certificate().relation().limits();
    let radius = body.radius();
    let value = json!({"format":FORMAT,"source":body.source(),"faces":body.selected_faces(),
        "recipe":{"contactRanges":r.contact_ranges,"bridges":r.bridges,"center":r.center,"radius":r.radius},
        "limits":{"maxPairProjections":limits.max_pair_projections,"maxAtoms":limits.max_atoms,
            "maxRayCells":limits.max_ray_cells,"refinements":limits.refinements,"radiusHalvings":limits.radius_halvings},
        "geometry":super::geometry_value::encode(body)?,
        "evidence":{"noAddedMaterial":true,"exactG1":true,"canonicalSourceUV":true,
            "radius":{"center":radius.center(),"target":radius.radius(),"tolerance":radius.tolerance(),
                "spans":radius.spans().iter().map(|s|json!({"span":s.span,"error":s.radial_error})).collect::<Vec<_>>()}}});
    bound(&value, max_bytes)?;
    Ok(value)
}
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(_), Value::Number(_)) => a.as_f64() == b.as_f64(),
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len() && a.iter().all(|(k, v)| b.get(k).is_some_and(|b| same(v, b)))
        }
        _ => a == b,
    }
}
pub fn restore(value: &Value, max_bytes: usize) -> Result<AdmittedFullRound> {
    bound(value, max_bytes)?;
    let object = value
        .as_object()
        .ok_or_else(|| error("Expected exact Full Round transport object"))?;
    let keys = [
        "format", "source", "faces", "recipe", "limits", "geometry", "evidence",
    ];
    if object.len() != keys.len()
        || keys.iter().any(|k| !object.contains_key(*k))
        || value["format"].as_str() != Some(FORMAT)
    {
        return Err(error(
            "Unknown exact Full Round transport fields or version",
        ));
    }
    let source: Model = field(value, "source")?;
    let faces = field(value, "faces")?;
    let r = &value["recipe"];
    let recipe = recipe::Recipe {
        contact_ranges: field(r, "contactRanges")?,
        bridges: field(r, "bridges")?,
        center: field(r, "center")?,
        radius: field(r, "radius")?,
    };
    let l = &value["limits"];
    let limits = material::Limits {
        max_pair_projections: field(l, "maxPairProjections")?,
        max_atoms: field(l, "maxAtoms")?,
        max_ray_cells: field(l, "maxRayCells")?,
        refinements: field(l, "refinements")?,
        radius_halvings: field(l, "radiusHalvings")?,
    };
    // ALL actual geometry and admission are rebuilt from frozen untrusted input.
    // Supplied evidence and output coefficients cannot authorize this operation.
    let body = recipe::restore(&source, faces, &recipe, limits)?;
    let mut expected = export(&body, max_bytes)?;
    // Older v1 artifacts omit optional derived naming metadata. Every actual
    // geometry field still receives exact reconstruction before this upgrade.
    if value["geometry"].get("topologyIds").is_none()
        && value["geometry"].get("faceOrigins").is_none()
    {
        let geometry = expected["geometry"]
            .as_object_mut()
            .ok_or_else(|| error("Missing rebuilt exact geometry"))?;
        geometry.remove("topologyIds");
        geometry.remove("faceOrigins");
    }
    if !same(value, &expected) {
        return Err(error(
            "Exact output/evidence differs from frozen-source reconstruction",
        ));
    }
    Ok(body)
}
