//! AP242 baked-tessellation to viewer-mesh adapter.
//!
//! This is the cache boundary: no existing content-addressed STEP mesh cache is
//! available in geometry-bridge.  The returned neutral `positions`/`indices`
//! buffers are ready for a parent cache/viewer integration and never trigger
//! exact-B-rep tessellation.  Exact entity IDs remain authoritative geometry.

use crate::{Result, Value, field, input};
use cad_step::step_interchange::{
    StepDiagnosticSeverity, StepDriftAssessment, StepDriftNotAssessedReason, StepSchemaDiagnostic,
    StepSchemaDiagnosticCode, StepTessellatedDiagnostic, StepTessellatedDiagnosticCode,
    StepTessellatedLod0, StepTessellatedSeverity, import_ap242_tessellated_lod0,
};
use polygon_core::{MAX_MESH_TRIANGLES, MAX_VERTICES, Mesh};
use sha2::{Digest, Sha256};
use value_codec::json;

const SOURCE: &str = "step-ap242-tessellated";
const CACHE_DOMAIN: &str = "geometry-bridge/step-ap242-tessellated/v1\n";

fn hex(digest: &[u8]) -> String {
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn document_cache_key(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(CACHE_DOMAIN.as_bytes());
    hasher.update(text.as_bytes());
    hex(&hasher.finalize())
}

fn shell_cache_key(document_key: &str, shell_entity_id: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(CACHE_DOMAIN.as_bytes());
    hasher.update(document_key.as_bytes());
    hasher.update(format!("\nshell={shell_entity_id}").as_bytes());
    hex(&hasher.finalize())
}

#[derive(Debug)]
struct MeshPayload {
    mesh: Mesh,
    triangle_count: usize,
    vertex_count: usize,
    bounds: Value,
}

fn mesh_from_lod0(lod0: &StepTessellatedLod0) -> Result<MeshPayload> {
    let (vertex_count, triangle_count) =
        lod0.faces
            .iter()
            .try_fold((0usize, 0usize), |(vertices, triangles), face| {
                let vertices = vertices
                    .checked_add(face.positions.len())
                    .ok_or_else(|| input("AP242 LOD-0 vertex count overflow"))?;
                let triangles = triangles
                    .checked_add(face.triangles.len())
                    .ok_or_else(|| input("AP242 LOD-0 triangle count overflow"))?;
                if vertices > MAX_VERTICES || triangles > MAX_MESH_TRIANGLES {
                    return Err(input(format!(
                        "AP242 LOD-0 shell #{} exceeds the polygon mesh resource budget",
                        lod0.shell_entity_id
                    )));
                }
                Ok((vertices, triangles))
            })?;
    let position_capacity = vertex_count
        .checked_mul(3)
        .ok_or_else(|| input("AP242 LOD-0 position buffer size overflow"))?;
    let index_capacity = triangle_count
        .checked_mul(3)
        .ok_or_else(|| input("AP242 LOD-0 index buffer size overflow"))?;
    let mut positions = Vec::with_capacity(position_capacity);
    let mut indices = Vec::with_capacity(index_capacity);
    let mut minimum = [f64::INFINITY; 3];
    let mut maximum = [f64::NEG_INFINITY; 3];

    for face in &lod0.faces {
        let base = positions.len() / 3;
        for point in &face.positions {
            if point.iter().any(|coordinate| !coordinate.is_finite()) {
                return Err(input(format!(
                    "AP242 LOD-0 face #{} contains a non-finite coordinate",
                    face.entity_id
                )));
            }
            positions.extend_from_slice(point);
            for axis in 0..3 {
                minimum[axis] = minimum[axis].min(point[axis]);
                maximum[axis] = maximum[axis].max(point[axis]);
            }
        }
        for triangle in &face.triangles {
            for index in triangle {
                let local = *index as usize;
                if local >= face.positions.len() {
                    return Err(input(format!(
                        "AP242 LOD-0 face #{} contains an out-of-range index",
                        face.entity_id
                    )));
                }
                indices.push(base + local);
            }
        }
    }

    let mesh = Mesh {
        positions,
        indices,
        uv: None,
    };
    // Domain validation is intentionally repeated at the real Mesh boundary:
    // finite coordinates, index bounds, tuple layout and resource budgets.
    mesh.validate()?;
    let bounds = if vertex_count == 0 {
        Value::Null
    } else {
        json!({"min":minimum,"max":maximum})
    };
    Ok(MeshPayload {
        mesh,
        triangle_count,
        vertex_count,
        bounds,
    })
}

fn schema_diagnostic_code(code: StepSchemaDiagnosticCode) -> &'static str {
    match code {
        StepSchemaDiagnosticCode::MissingFileSchema => "missing-file-schema",
        StepSchemaDiagnosticCode::MalformedFileSchema => "malformed-file-schema",
        StepSchemaDiagnosticCode::MultipleSchemas => "multiple-schemas",
        StepSchemaDiagnosticCode::UnknownSchema => "unknown-schema",
        StepSchemaDiagnosticCode::PossibleTruncatedSchema => "possible-truncated-schema",
    }
}

fn schema_diagnostic(diagnostic: &StepSchemaDiagnostic) -> Value {
    json!({
        "code":schema_diagnostic_code(diagnostic.code),
        "severity":match diagnostic.severity {
            StepDiagnosticSeverity::Warning => "warning",
        },
        "schema":diagnostic.schema,
        "message":diagnostic.message,
    })
}

fn tessellated_diagnostic_code(code: StepTessellatedDiagnosticCode) -> &'static str {
    match code {
        StepTessellatedDiagnosticCode::MissingExactBrepLink => "missing-exact-brep-link",
        StepTessellatedDiagnosticCode::AmbiguousExactBrepLink => "ambiguous-exact-brep-link",
        StepTessellatedDiagnosticCode::UnresolvedGeometricModelRelationship => {
            "unresolved-geometric-model-relationship"
        }
        StepTessellatedDiagnosticCode::OrphanTessellatedFace => "orphan-tessellated-face",
        StepTessellatedDiagnosticCode::UnsupportedTessellatedEntity => {
            "unsupported-tessellated-entity"
        }
        StepTessellatedDiagnosticCode::DriftNotAssessed => "drift-not-assessed",
        StepTessellatedDiagnosticCode::DriftExceedsDeclaredTolerance => {
            "drift-exceeds-declared-tolerance"
        }
    }
}

fn tessellated_diagnostic(diagnostic: &StepTessellatedDiagnostic) -> Value {
    json!({
        "code":tessellated_diagnostic_code(diagnostic.code),
        "severity":match diagnostic.severity {
            StepTessellatedSeverity::Info => "info",
            StepTessellatedSeverity::Warning => "warning",
        },
        "entityId":diagnostic.entity_id,
        "message":diagnostic.message,
    })
}

fn drift_json(lod0: &StepTessellatedLod0) -> Value {
    match &lod0.drift {
        StepDriftAssessment::NotAssessed { reason } => json!({
            "status":"not-assessed",
            "reason":match reason {
                StepDriftNotAssessedReason::NoExactSamples => "no-exact-samples",
                StepDriftNotAssessedReason::EmptyTessellation => "empty-tessellation",
                StepDriftNotAssessedReason::NonFiniteExactSample => "non-finite-exact-sample",
            },
            "hausdorffCertified":false,
        }),
        StepDriftAssessment::VertexToExactSamples {
            max_distance,
            declared_tolerance,
            within_declared_tolerance,
            exact_sample_count,
        } => json!({
            "status":"vertex-to-exact-samples",
            "maxDistance":max_distance,
            "declaredTolerance":declared_tolerance,
            "withinDeclaredTolerance":within_declared_tolerance,
            "exactSampleCount":exact_sample_count,
            "hausdorffCertified":false,
        }),
    }
}

/// Convert `{text}` into neutral, cache-ready LOD-0 mesh payloads.
///
/// The parent integration owns route registration and actual cache storage.
/// No exact B-rep is constructed, replaced or retessellated by this adapter.
pub(crate) fn import(v: Value) -> Result<Value> {
    let object = v
        .as_object()
        .ok_or_else(|| input("AP242 LOD-0 import expects an object containing only text"))?;
    if !object.contains_key("text")
        || object
            .keys()
            .any(|key| !matches!(key.as_str(), "op" | "text"))
    {
        return Err(input("AP242 LOD-0 import expects op and text only"));
    }
    let text: String = field(&v, "text")?;
    let imported = import_ap242_tessellated_lod0(&text)?;
    let document_key = document_cache_key(&text);
    let diagnostics: Vec<Value> = imported
        .diagnostics
        .iter()
        .map(tessellated_diagnostic)
        .collect();
    let schema_diagnostics: Vec<Value> = imported
        .schema
        .diagnostics
        .iter()
        .map(schema_diagnostic)
        .collect();

    let mut shells = Vec::with_capacity(imported.lod0_shells.len());
    for lod0 in &imported.lod0_shells {
        let mesh = mesh_from_lod0(lod0)?;
        let shell_diagnostics: Vec<Value> = imported
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.entity_id.is_none() || diagnostic.entity_id == Some(lod0.shell_entity_id)
            })
            .map(tessellated_diagnostic)
            .collect();
        shells.push(json!({
            "shellEntityId":lod0.shell_entity_id,
            "name":lod0.name,
            "source":SOURCE,
            "cacheKey":shell_cache_key(&document_key,lod0.shell_entity_id),
            "mesh":{
                "positions":mesh.mesh.positions,
                "indices":mesh.mesh.indices,
            },
            "triangleCount":mesh.triangle_count,
            "vertexCount":mesh.vertex_count,
            "bounds":mesh.bounds,
            "exactBrepEntityIds":lod0.exact_brep_entity_ids,
            "declaredTolerance":lod0.declared_tolerance,
            "drift":drift_json(lod0),
            "diagnostics":shell_diagnostics,
        }));
    }

    Ok(json!({
        "source":SOURCE,
        "cacheKey":document_key,
        "cacheReady":true,
        "requiresRetessellation":false,
        "exactGeometryAuthoritative":true,
        "lod0Shells":shells,
        "exactBrepEntityIds":imported.exact_brep_entity_ids,
        "diagnostics":diagnostics,
        "schemaDiagnostics":schema_diagnostics,
    }))
}

/// Two-sided sampled surface estimate using the existing binary64 distance BVH.
/// Vertices, edge midpoints and centroids are observations, not a Hausdorff bound.
pub(crate) fn hausdorff_estimate(v: Value) -> Result<Value> {
    let object = v
        .as_object()
        .ok_or_else(|| input("LOD-0 Hausdorff estimate expects exactMesh and lod0Mesh"))?;
    if object
        .keys()
        .any(|key| !matches!(key.as_str(), "op" | "exactMesh" | "lod0Mesh"))
    {
        return Err(input(
            "LOD-0 Hausdorff estimate expects exactMesh and lod0Mesh only",
        ));
    }
    let decode = |name: &str| -> Result<(Vec<f64>, Vec<usize>)> {
        let mesh = v
            .get(name)
            .ok_or_else(|| input(format!("Missing {name}")))?;
        let positions: Vec<f64> = field(mesh, "positions")?;
        let indices: Vec<u32> = field(mesh, "indices")?;
        let indices = indices
            .into_iter()
            .map(|index| {
                usize::try_from(index).map_err(|_| input("LOD-0 mesh index does not fit usize"))
            })
            .collect::<Result<Vec<usize>>>()?;
        if positions.len() > 300_000 || indices.len() > 300_000 {
            return Err(input(
                "LOD-0 Hausdorff estimate exceeds the mesh component budget",
            ));
        }
        Ok((positions, indices))
    };
    let (exact_positions, exact_indices) = decode("exactMesh")?;
    let (lod_positions, lod_indices) = decode("lod0Mesh")?;
    let exact = mesh_query::MeshView::new(&exact_positions, &exact_indices);
    let lod = mesh_query::MeshView::new(&lod_positions, &lod_indices);
    let report = mesh_query::hausdorff_distance_approx(&exact, &lod)?;
    Ok(json!({
        "method":"two-sided-bvh-sampled-hausdorff-estimate",
        "samplePattern":"vertices-edge-midpoints-centroids",
        "min":report.min_mm,
        "max":report.max_mm,
        "mean":report.mean_mm,
        "rms":report.rms_mm,
        "sampleCount":report.sample_count,
        "certified":false,
        "limitation":"Finite samples can miss surface deviation; this is neither a directed exact Hausdorff distance nor a certified two-sided bound. Distances use the input model unit."
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('x'),'2;1');\nFILE_NAME('x','1970-01-01T00:00:00',(''),(''),'','','');\nFILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'));\nENDSEC;\nDATA;\n";
    const FOOTER: &str = "ENDSEC;\nEND-ISO-10303-21;\n";

    fn document(extra: &str) -> String {
        format!(
            "{HEADER}#1=COORDINATES_LIST('',((0.,0.,0.),(2.,0.,0.),(0.,3.,0.)));\n#2=TRIANGULATED_FACE('lod',#1,((1,2,3)));\n#3=TESSELLATED_SHELL('shell',(#2));\n{extra}{FOOTER}"
        )
    }

    #[test]
    fn emits_direct_mesh_payload_and_preserves_exact_authority() {
        let text = document(
            "#4=MANIFOLD_SOLID_BREP('exact',#5);\n#5=CLOSED_SHELL('',());\n#6=GEOMETRIC_MODEL_RELATIONSHIP('','',#3,#4);\n",
        );
        let output = import(json!({"text":text})).unwrap();
        assert_eq!(output["source"].as_str(), Some(SOURCE));
        assert_eq!(output["cacheReady"].as_bool(), Some(true));
        assert_eq!(output["requiresRetessellation"].as_bool(), Some(false));
        assert_eq!(output["exactGeometryAuthoritative"].as_bool(), Some(true));
        assert_eq!(output["exactBrepEntityIds"], json!([4]));
        assert_eq!(output["cacheKey"].as_str().unwrap().len(), 64);

        let shell = &output["lod0Shells"][0];
        assert_eq!(shell["triangleCount"].as_u64(), Some(1));
        assert_eq!(shell["vertexCount"].as_u64(), Some(3));
        assert_eq!(shell["exactBrepEntityIds"], json!([4]));
        assert_eq!(shell["mesh"]["indices"], json!([0, 1, 2]));
        assert_eq!(
            shell["mesh"]["positions"],
            json!([0., 0., 0., 2., 0., 0., 0., 3., 0.])
        );
        assert_eq!(shell["bounds"]["min"], json!([0., 0., 0.]));
        assert_eq!(shell["bounds"]["max"], json!([2., 3., 0.]));
        assert_eq!(shell["drift"]["status"].as_str(), Some("not-assessed"));
        assert_eq!(shell["drift"]["hausdorffCertified"].as_bool(), Some(false));
        assert!(
            shell["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| { entry["code"].as_str() == Some("drift-not-assessed") })
        );
    }

    #[test]
    fn cache_keys_are_deterministic_and_content_sensitive() {
        let text = document("");
        let first = import(json!({"text":text.clone()})).unwrap();
        let second = import(json!({"text":text.clone()})).unwrap();
        assert_eq!(first["cacheKey"], second["cacheKey"]);
        assert_eq!(
            first["lod0Shells"][0]["cacheKey"],
            second["lod0Shells"][0]["cacheKey"]
        );
        assert_ne!(first["cacheKey"], first["lod0Shells"][0]["cacheKey"]);

        let changed = text.replace("(2.,0.,0.)", "(4.,0.,0.)");
        let changed = import(json!({"text":changed})).unwrap();
        assert_ne!(first["cacheKey"], changed["cacheKey"]);
    }

    #[test]
    fn rejects_invalid_requests_and_malformed_mesh_indices() {
        assert!(import(json!({})).is_err());
        assert!(import(json!({"text":document(""),"extra":true})).is_err());
        let bad = document("").replace("(1,2,3)", "(1,2,4)");
        assert!(import(json!({"text":bad})).is_err());
        let ap203 = document("").replace(
            "AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF",
            "CONFIG_CONTROL_DESIGN",
        );
        assert!(import(json!({"text":ap203})).is_err());
    }

    #[test]
    fn enforces_budget_before_allocating_mesh_buffers() {
        let imported = import_ap242_tessellated_lod0(&document("")).unwrap();
        let mut lod0 = imported.lod0_shells[0].clone();
        lod0.faces[0].positions = vec![[0.0; 3]; MAX_VERTICES + 1];
        lod0.faces[0].triangles.clear();
        let error = mesh_from_lod0(&lod0).unwrap_err();
        assert!(error.message.contains("resource budget"));
    }

    #[test]
    fn public_dispatch_reaches_lod0_import() {
        let output = crate::dispatch(json!({
            "op":"brep_nurbs_import_step_lod0",
            "text":document("")
        }))
        .unwrap();
        assert_eq!(output["source"], json!(SOURCE));
        assert_eq!(output["requiresRetessellation"], json!(false));
    }

    #[test]
    fn sampled_hausdorff_is_two_sided_bvh_and_explicitly_uncertified() {
        let output = crate::dispatch(json!({
            "op":"brep_nurbs_step_lod0_hausdorff_estimate",
            "exactMesh":{"positions":[0.,0.,0.,1.,0.,0.,0.,1.,0.],"indices":[0,1,2]},
            "lod0Mesh":{"positions":[0.,0.,0.25,1.,0.,0.25,0.,1.,0.25],"indices":[0,1,2]}
        }))
        .unwrap();
        assert_eq!(
            output["method"],
            json!("two-sided-bvh-sampled-hausdorff-estimate")
        );
        assert_eq!(output["sampleCount"], json!(14));
        assert_eq!(output["certified"], json!(false));
        assert!((output["max"].as_f64().unwrap() - 0.25).abs() < 1e-12);
        assert!(
            crate::dispatch(json!({
                "op":"brep_nurbs_step_lod0_hausdorff_estimate",
                "exactMesh":{"positions":[0.,0.,0.,1.,0.,0.,0.,1.,0.],"indices":[0,1,2]},
                "lod0Mesh":{"positions":[0.,0.,0.,1.,0.,0.,0.,1.,0.],"indices":[0,1,2]},
                "extra":true
            }))
            .is_err()
        );
    }
}
