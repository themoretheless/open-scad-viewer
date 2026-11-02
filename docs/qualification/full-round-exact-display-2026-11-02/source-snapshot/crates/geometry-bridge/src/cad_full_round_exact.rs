//! Lossless exact Full Round through the shared native/WASM operation router.
use crate::{Result, Value, field, input};
use brep_core::{
    Model,
    feature_ops::{full_round_curved::admitted::transport, full_round_exact},
    profile_certificate::original_restriction::wire::material::Limits,
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

/// The artifact is restored and proved before sampling; no f64 BRep is created.
pub(crate) fn display(v: Value) -> Result<Value> {
    allowed(
        &v,
        &["op", "artifact", "segments", "maxTriangles", "maxBytes"],
    )?;
    let bytes = max_bytes(&v)?;
    let body = transport::restore(
        v.get("artifact")
            .ok_or_else(|| input("Missing exact display artifact"))?,
        bytes,
    )?;
    let sampled =
        brep_core::profile_certificate::original_restriction::wire::prism::display::tessellate(
            body.material().result(),
            field(&v, "segments")?,
            optional(&v, "maxTriangles")?.unwrap_or(20_000),
        )?;
    let mesh = crate::Mesh {
        positions: sampled.positions,
        indices: sampled.indices,
        uv: None,
    };
    let report = mesh.inspect()?;
    if !report.closed || report.orientation_conflicts != 0 || report.degenerate_triangles != 0 {
        return Err(input(
            "Exact display sampling failed closed oriented mesh admission",
        ));
    }
    let (vertices, indices, area) = crate::brep_display::prepare(&mesh)?;
    let ids = body
        .lineage()?
        .identities
        .faces
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    Ok(
        value_codec::json!({"representation":"display-mesh", "sourceRepresentation":"rational-brep",
        "displayVertices":vertices,"displayIndices":indices,"faceIds":sampled.face_ids,"topologyFaceIds":ids,
        "report":report,"surfaceArea":area,"geometricErrorCertified":false}),
    )
}
