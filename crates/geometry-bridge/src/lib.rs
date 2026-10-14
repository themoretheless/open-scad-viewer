//! Application adapter between two independent Rust geometry libraries.
//! Owns the NURBS-to-polygon sampler and the WASM/JSON transport, not a third
//! geometry representation. Native clients can use the same typed adapters.
//!
//! Default feature `languages` pulls OpenSCAD and ModelGraph. Kernel-only
//! builds: `--no-default-features`.
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]
pub mod brep;
mod miter_smoothness;
mod polygon_station_body;
mod sweep_cap_evidence;
mod sweep_pipeline;
mod sweep_viewport;
pub use math_core::Acceleration;
pub mod brep_attestation;
mod brep_display;
pub mod brep_envelope;
pub mod brep_execution_plan;
pub mod brep_graph;
pub mod brep_graph_runner;
pub mod brep_identity;
mod brep_json_size;
pub mod brep_production;
pub mod brep_profile;
pub mod brep_provenance;
pub mod brep_result;
mod brep_scene_plan;
mod brep_semantic;
pub mod brep_session;
mod brep_session_abi;
mod dispatch;
mod cad_body_affine;
mod cad_boolean;
mod cad_centered_lattice;
mod cad_clearance;
mod cad_draft;
mod cad_display;
mod cad_edge_edit;
mod cad_source_body;
mod cad_face_selection;
mod cad_hole;
mod cad_lattice;
mod cad_mesh_planes;
mod cad_mesh_topology;
mod cad_path;
mod cad_pattern;
mod cad_planar_edit;
mod cad_sections;
mod cad_selection;
mod cad_sketch;
mod cad_profile_prepare;
mod cad_dimensions;
mod cad_quantity;
mod cad_diagnostics;
mod cad_face_distance;
mod cad_shell_distance;
mod cad_solid_distance;
mod cad_material_segment;
mod cad_material_chord;
mod cad_material_wall;
mod cad_whole_wall;
mod cad_boundary_agreement;
mod cad_face_contacts;
mod cad_surface_diagnostics;
mod cad_bridge_curve;
mod cad_sketch_offset;
mod cad_sketch_trim;
mod cad_split;
mod cad_texture;
mod cad_thread;
mod camera_gestures;
pub mod intersections;
#[cfg(feature = "cuda")]
mod lattice_cuda;
#[cfg(feature = "gpu")]
pub mod lattice_gpu;
mod mesh;
pub mod mesh_analysis;
mod mesh_export_file;
pub mod mesh_picking;
mod mesh_render;
pub mod mesh_shell;
pub mod mesh_surface_groups;
pub mod print_geometry;
mod scene_picking;
mod viewport;

#[cfg(feature = "gpu")]
pub fn gpu_backend_label() -> Option<&'static str> {
    gpu_backend_report().map(|report| report.label)
}

#[cfg(feature = "gpu")]
pub fn gpu_backend_report() -> Option<gpu_compute::BackendReport> {
    lattice_gpu::backend_report()
}
mod path2d;
pub mod reconstruction;
mod sdf_gpu;
use nurbs_core::{
    curve::Curve,
    surface::{Surface, SurfaceSampler},
};
use polygon_core::{
    BuiltMesh, Mesh, Seams,
    solid::tessellation::{self, Boundary, Options, ParametricSurface},
};
use value_codec::{Value, json};

pub use math_core::{Error, Result};
#[allow(unused_imports)]
pub(crate) use bridge_codec::{
    Routed, Router, encode, error_json, field, input, require_exact_fields, response, take_field,
};
pub(crate) fn mesh_from_triangles(t: geometry_ops::Triangles) -> Mesh {
    Mesh {
        positions: t.positions,
        indices: t.indices,
        uv: None,
    }
}
pub(crate) fn triangles_from_mesh(m: &Mesh) -> geometry_ops::Triangles {
    geometry_ops::Triangles {
        positions: m.positions.clone(),
        indices: m.indices.clone(),
    }
}

fn close_topology_role(value: &str) -> Result<brep_core::BodyRole> {
    Ok(match value {
        "wire" => brep_core::BodyRole::Wire,
        "face" => brep_core::BodyRole::Face,
        "sheet-shell" => brep_core::BodyRole::SheetShell,
        "open-shell" => brep_core::BodyRole::OpenShell,
        "solid" => brep_core::BodyRole::Solid,
        "compound" => brep_core::BodyRole::Compound,
        _ => return Err(input("Unknown close-topology body role")),
    })
}

fn close_topology_audit_value(value: &Value) -> Result<Value> {
    require_exact_fields(
        value,
        &["op", "parts", "sharedFaces", "radialRings", "vertexFans"],
        "close topology request",
    )?;
    let parts_value = value["parts"]
        .as_array()
        .ok_or_else(|| input("parts must be an array"))?;
    let mut parts = Vec::with_capacity(parts_value.len());
    for part in parts_value {
        require_exact_fields(part, &["role", "model"], "close topology part")?;
        parts.push(brep_core::ComplexPart {
            role: close_topology_role(
                part["role"]
                    .as_str()
                    .ok_or_else(|| input("part role must be a string"))?,
            )?,
            model: value_codec::from_value(part["model"].clone())
                .map_err(|e| input(e.to_string()))?,
        });
    }
    let mut shared_faces = Vec::new();
    for relation in value["sharedFaces"]
        .as_array()
        .ok_or_else(|| input("sharedFaces must be an array"))?
    {
        let uses = relation
            .as_array()
            .ok_or_else(|| input("shared face must be an array"))?;
        shared_faces.push(brep_core::SharedFace {
            uses: uses
                .iter()
                .map(|use_| {
                    require_exact_fields(use_, &["part", "face", "reversed"], "shared face use")?;
                    Ok(brep_core::FaceRef {
                        part: field(use_, "part")?,
                        face: field(use_, "face")?,
                        reversed: field(use_, "reversed")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        });
    }
    let mut radial_rings = Vec::new();
    for relation in value["radialRings"]
        .as_array()
        .ok_or_else(|| input("radialRings must be an array"))?
    {
        let uses = relation
            .as_array()
            .ok_or_else(|| input("radial ring must be an array"))?;
        radial_rings.push(brep_core::EdgeRadialRing {
            uses: uses
                .iter()
                .map(|use_| {
                    require_exact_fields(
                        use_,
                        &["part", "face", "edge", "reversed"],
                        "radial use",
                    )?;
                    Ok(brep_core::EdgeUseRef {
                        part: field(use_, "part")?,
                        face: field(use_, "face")?,
                        edge: field(use_, "edge")?,
                        reversed: field(use_, "reversed")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        });
    }
    let mut vertex_fans = Vec::new();
    for fan in value["vertexFans"]
        .as_array()
        .ok_or_else(|| input("vertexFans must be an array"))?
    {
        require_exact_fields(fan, &["part", "vertex", "closed", "uses"], "vertex fan")?;
        let uses = fan["uses"]
            .as_array()
            .ok_or_else(|| input("fan uses must be an array"))?;
        vertex_fans.push(brep_core::VertexFan {
            vertex: (field(fan, "part")?, field(fan, "vertex")?),
            closed: field(fan, "closed")?,
            uses: uses
                .iter()
                .map(|use_| {
                    require_exact_fields(use_, &["part", "face", "vertex"], "vertex fan use")?;
                    Ok(brep_core::VertexUseRef {
                        part: field(use_, "part")?,
                        face: field(use_, "face")?,
                        vertex: field(use_, "vertex")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        });
    }
    let audited = brep_core::MixedDimensionalBrep::new(
        parts,
        shared_faces,
        radial_rings,
        vertex_fans,
        vec![],
    )?
    .audit()?;
    let certificate = audited.certificate();
    let boundary_faces = audited.boundary_faces();
    Ok(json!({
        "certificate": {
            "capability": certificate.capability,
            "complete": certificate.complete,
            "partCount": certificate.part_count,
            "solidCellCount": certificate.solid_cell_count,
            "sheetCount": certificate.sheet_count,
            "openShellCount": certificate.open_shell_count,
            "sharedFaceCount": certificate.shared_face_count,
            "nonManifoldEdgeCount": certificate.non_manifold_edge_count,
            "vertexFanCount": certificate.vertex_fan_count,
            "boundaryFaceCount": certificate.boundary_face_count,
            "maxRadialValence": certificate.max_radial_valence,
            "namingComplete": certificate.naming_complete,
            "notes": certificate.notes
        },
        "boundaryFaces": boundary_faces.iter().map(|face| json!({
            "part":face.part,"face":face.face,"reversed":face.reversed
        })).collect::<Vec<_>>(),
        "decomposition": audited.manifold_decomposition().into_iter().map(|part| json!({
            "role":part.role.as_str(),"model":part.model
        })).collect::<Vec<_>>()
    }))
}


fn curved_graph_boolean_value(
    model: brep_core::Model,
    certificate: brep_core::CurvedGraphBooleanCertificate,
) -> Result<Value> {
    let axis = match certificate.axis {
        nurbs_intersect::nurbs_ss::ExactIsoAxis::U => "U",
        nurbs_intersect::nurbs_ss::ExactIsoAxis::V => "V",
    };
    Ok(json!({
        "model": encode(model)?,
        "certificate": {
            "capability": certificate.capability,
            "status": certificate.status,
            "operation": certificate.operation,
            "axis": axis,
            "fixedParameter": certificate.fixed_parameter,
            "lineage": {
                "sourceFace": certificate.source_face,
                "retainedFace": certificate.retained_face,
                "deletedRegion": certificate.deleted_region,
                "generatedIntersectionEdge": certificate.intersection_edge
            },
            "tensorCells": certificate.tensor_cells,
            "exactCorrespondence": certificate.exact_correspondence,
            "sew": {
                "matched": certificate.sew.matched,
                "complete": certificate.sew.complete,
                "displacementBudgetOk": certificate.sew.displacement_budget_ok
            },
            "audit": {
                "ok": certificate.audit.ok,
                "bodyCount": certificate.audit.body_count,
                "shellCount": certificate.audit.shell_count,
                "selfIntersectionPairsCandidate": certificate.audit.self_intersection_pairs_candidate,
                "selfIntersectionComplete": certificate.audit.self_intersection_complete,
                "selfIntersectionPairsChecked": certificate.audit.self_intersection_pairs_checked,
                "notes": certificate.audit.notes
            },
            "changeSet": encode(certificate.change_set)?,
            "namingComplete": certificate.naming_complete,
            "noFallback": certificate.no_fallback,
            "separationProof": certificate.separation_proof,
            "homogeneousRootProof": certificate.homogeneous_root_proof,
            "denominatorLowerBound": certificate.denominator_lower_bound,
            "weightConditionNumber": certificate.weight_condition_number,
            "resourceBound": certificate.resource_bound
        }
    }))
}

fn contained_graph_boolean_value(
    model: brep_core::Model,
    certificate: brep_core::ContainedGraphBooleanCertificate,
) -> Result<Value> {
    Ok(json!({
        "model": encode(model)?,
        "certificate": {
            "capability": certificate.capability,
            "status": certificate.status,
            "operation": certificate.operation,
            "relation": certificate.relation,
            "strictUvMargin": certificate.strict_uv_margin,
            "floorClearance": certificate.floor_clearance,
            "roofClearance": certificate.roof_clearance,
            "cavityProof": certificate.cavity_proof,
            "separationProof": certificate.separation_proof,
            "audit": {
                "ok": certificate.audit.ok,
                "bodyCount": certificate.audit.body_count,
                "shellCount": certificate.audit.shell_count,
                "notes": certificate.audit.notes
            },
            "changeSet": encode(certificate.change_set)?,
            "namingComplete": certificate.naming_complete,
            "noFallback": certificate.no_fallback
        }
    }))
}

fn general_nurbs_boolean_value(
    model: brep_core::Model,
    certificate: brep_core::GeneralNurbsBooleanCertificate,
) -> Result<Value> {
    Ok(json!({
        "model": encode(model)?,
        "certificate": {
            "capability": certificate.capability,
            "authority": certificate.authority,
            "status": certificate.status,
            "operation": certificate.operation,
            "operandOrder": certificate.operand_order,
            "exactRegionMembership": certificate.exact_region_membership,
            "partitionCells": certificate.partition_cells,
            "cavityCount": certificate.cavity_count,
            "branchGraph": {
                "components": certificate.branch_graph.certificate.component_count,
                "fragments": certificate.branch_graph.certificate.fragment_count,
                "candidateSpanPairs": certificate.branch_graph.certificate.candidate_span_pairs,
                "sourceSpanCount": certificate.branch_graph.certificate.source_span_count,
                "denominatorLowerBound": certificate.branch_graph.certificate.denominator_lower_bound,
                "complete": certificate.branch_graph.permits_topology_authorship()
            },
            "uv": {
                "tensorCells": certificate.uv.tensor_cell_count,
                "branches": certificate.uv.branch_count,
                "materialCells": certificate.uv.material_cell_count,
                "holeCells": certificate.uv.hole_cell_count,
                "complete": certificate.uv.permits_trim_classification()
            },
            "exactCurvePcurveCount": certificate.exact_curve_pcurve_count,
            "ssReportsComplete": certificate.ss_reports_complete,
            "ssFacePairs": certificate.ss_face_pairs,
            "sew": {
                "matched": certificate.sew.matched,
                "complete": certificate.sew.complete,
                "displacementBudgetOk": certificate.sew.displacement_budget_ok
            },
            "audit": {
                "ok": certificate.audit.ok,
                "bodyCount": certificate.audit.body_count,
                "shellCount": certificate.audit.shell_count,
                "selfIntersectionPairsCandidate": certificate.audit.self_intersection_pairs_candidate,
                "selfIntersectionComplete": certificate.audit.self_intersection_complete,
                "selfIntersectionPairsChecked": certificate.audit.self_intersection_pairs_checked,
                "notes": certificate.audit.notes
            },
            "changeSet": encode(certificate.change_set)?,
            "naming": {
                "split": certificate.naming.split,
                "retained": certificate.naming.retained,
                "deleted": certificate.naming.deleted,
                "generated": certificate.naming.generated,
                "operationStable": certificate.naming.operation_stable
            },
            "resultComponents": certificate.result_components,
            "resultFaces": certificate.result_faces,
            "noFallback": certificate.no_fallback
        }
    }))
}

pub struct NurbsSurfaceAdapter {
    sampler: SurfaceSampler,
}
impl NurbsSurfaceAdapter {
    pub fn new(surface: &Surface) -> Result<Self> {
        Ok(Self {
            sampler: SurfaceSampler::new(surface)?,
        })
    }
}
impl ParametricSurface for NurbsSurfaceAdapter {
    fn domain(&self) -> [f64; 4] {
        let s = self.sampler.definition();
        [
            s.knots_u[s.degree_u],
            s.knots_u[s.control_points.len()],
            s.knots_v[s.degree_v],
            s.knots_v[s.control_points[0].len()],
        ]
    }
    fn point(&self, u: f64, v: f64) -> polygon_core::Result<[f64; 3]> {
        self.sampler.evaluate(u, v).map(|e| e.point)
    }
    fn boundary(&self) -> Boundary {
        let s = self.sampler.definition();
        let d = self.domain();
        let clamped = |k: &[f64], p: usize, a: f64, b: f64| {
            k[..=p].iter().all(|v| *v == a) && k[k.len() - p - 1..].iter().all(|v| *v == b)
        };
        let cu = clamped(&s.knots_u, s.degree_u, d[0], d[1]);
        let cv = clamped(&s.knots_v, s.degree_v, d[2], d[3]);
        let fu = s.control_points[0].clone();
        let lu = s.control_points.last().unwrap().clone();
        let fv: Vec<_> = s.control_points.iter().map(|r| r[0].clone()).collect();
        let lv: Vec<_> = s
            .control_points
            .iter()
            .map(|r| r.last().unwrap().clone())
            .collect();
        let equal = |a: &[Vec<f64>], b: &[Vec<f64>], wa: &[f64], wb: &[f64]| {
            let ratio = wb[0] / wa[0];
            a.iter()
                .enumerate()
                .all(|(i, p)| *p == b[i] && (wb[i] / wa[i] - ratio).abs() <= ratio.abs() * 1e-12)
        };
        let wfv: Vec<_> = s.weights.iter().map(|r| r[0]).collect();
        let wlv: Vec<_> = s.weights.iter().map(|r| *r.last().unwrap()).collect();
        let seams = Seams {
            u: s.periodic_u || (cu && equal(&fu, &lu, &s.weights[0], s.weights.last().unwrap())),
            v: s.periodic_v || (cv && equal(&fv, &lv, &wfv, &wlv)),
        };
        let rows = [&fu, &lu, &fv, &lv];
        let clamps = [cu, cu, cv, cv];
        let collapsed = std::array::from_fn(|i| {
            if clamps[i] && rows[i].iter().all(|p| *p == rows[i][0]) {
                Some([rows[i][0][0], rows[i][0][1], rows[i][0][2]])
            } else {
                None
            }
        });
        Boundary { seams, collapsed }
    }
}
/// NURBS definitions remain owned by the caller. The result is a derived mesh
/// with sampled UV correspondence; no exact/global error certificate is implied.
pub fn tessellate_nurbs(surface: &Surface, options: &Options) -> Result<BuiltMesh> {
    tessellation::tessellate(
        &NurbsSurfaceAdapter::new(surface)?,
        options,
    )
}
/// Exact piecewise-linear NURBS curves from ordered mesh boundary vertices.
/// This transfers polygon data into the spline library; it does not infer the
/// original smooth surface. Closed loops are clamped curves, not periodic data.
pub fn boundary_curves(mesh: &Mesh) -> Result<Vec<Curve>> {
    mesh.boundary_loops()?
        .iter()
        .map(|l| {
            let points = l
                .iter()
                .map(|i| mesh.point(*i).map(|p| p.to_vec()))
                .collect::<polygon_core::Result<Vec<_>>>()?;
            Curve::from_polyline(points)
        })
        .collect()
}
/// Domain crates tried before the local operations, in order.
const DOMAINS: &[Router] = &[bridge_cam::dispatch, bridge_analysis::dispatch, bridge_svg::dispatch];

pub fn dispatch(mut v: Value) -> Result<Value> {
    for domain in DOMAINS {
        match domain(v) {
            Routed::Handled(result) => return result,
            Routed::Unhandled(request) => v = request,
        }
    }
    dispatch_local(v)
}

/// Violation of a B-rep semantic contract (identity, envelope, provenance, attestation,
/// execution plan, production, result): one code for the whole family of contract checks.
fn semantic_contract_error(message: &str) -> Error {
    Error::new("BREP_SEMANTIC_CONTRACT", message)
}

fn dispatch_local(mut v: Value) -> Result<Value> {
    for route in dispatch::ROUTES {
        match route(v) {
            Routed::Handled(result) => return result,
            Routed::Unhandled(request) => v = request,
        }
    }
    dispatch::fallback::dispatch(v)
}
pub fn execute(input_text: &str) -> String {
    if input_text.len() > 32 * 1024 * 1024 {
        return response(Err(input("Geometry request exceeds 32 MiB")));
    }
    response(
        value_codec::from_str(input_text)
            .map_err(|e| input(e.to_string()))
            .and_then(dispatch),
    )
}
#[cfg(test)]
mod tests;

/// Owned binary transport snapshot. Pointers are borrowed until `free()`;
/// clients must reacquire the WASM memory buffer after allocating this object.
pub struct CadMeshBuffer {
    pub(crate) positions: Vec<f64>,
    pub(crate) indices: Vec<u32>,
    pub(crate) face_ids: Vec<u32>,
}
impl value_codec::Serialize for CadMeshBuffer {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "positions".into(),
            value_codec::Serialize::to_value(&self.positions),
        );
        object.insert(
            "indices".into(),
            value_codec::Serialize::to_value(&self.indices),
        );
        object.insert(
            "faceIds".into(),
            value_codec::Serialize::to_value(&self.face_ids),
        );
        value_codec::Value::Object(object)
    }
}
impl CadMeshBuffer {
    pub fn positions_ptr(&self) -> usize {
        self.positions.as_ptr() as usize
    }
    pub fn positions_len(&self) -> usize {
        self.positions.len()
    }
    pub fn indices_ptr(&self) -> usize {
        self.indices.as_ptr() as usize
    }
    pub fn indices_len(&self) -> usize {
        self.indices.len()
    }
    pub fn face_ids_ptr(&self) -> usize {
        self.face_ids.as_ptr() as usize
    }
    pub fn face_ids_len(&self) -> usize {
        self.face_ids.len()
    }
}
/// Typed native entry point shared with the WASM import adapter.
pub fn import_cad_mesh(stride: usize, vertices: &[f64], indices: &[u32]) -> Result<u32> {
    mesh::import_buffers(stride, vertices, indices)
}

/// Linear-memory ABI core shared by the geometry-wasm shell.
pub mod abi;
