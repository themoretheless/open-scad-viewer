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
        brep_core::nurbs_ss_g6::ExactIsoAxis::U => "U",
        brep_core::nurbs_ss_g6::ExactIsoAxis::V => "V",
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

fn dispatch_local(mut v: Value) -> Result<Value> {
    match v["op"].as_str().unwrap_or("") {
        "brep_sweep_viewport_evidence" => sweep_viewport::read(v),
        "brep_progressive_profile_body" => sweep_pipeline::profile_body(v),
        "brep_sweep_law_payload" => sweep_pipeline::law_payload(v),
        "brep_sweep_constructor" => sweep_pipeline::constructor(v),
        "brep_sweep_stream_start" => sweep_pipeline::stream_start(v),
        "brep_sweep_stream_next" => sweep_pipeline::stream_next(v),
        "brep_sweep_stream_release" => sweep_pipeline::stream_release(v),
        "brep_miter_body" => sweep_pipeline::miter(v),
        "brep_transform_certified_miter" => sweep_pipeline::transform(v),
        "brep_smooth_certified_miter" => sweep_pipeline::smooth(v,false),
        "brep_reconstruct_certified_miter" => sweep_pipeline::smooth(v,true),
        "brep_progressive_miter_body" => sweep_pipeline::progressive_miter(v),
        "brep_sweep_release_owner" => sweep_pipeline::release(v),
        "brep_sweep_solid_admission" => sweep_pipeline::admission(v),
        "brep_miter_correct_sections" => sweep_pipeline::correct(v),
        "brep_intersect_surface_surface"
        | "brep_intersect_curve_segment"
        | "brep_intersect_curve_plane"
        | "brep_intersect_surface_plane"
        | "brep_intersect_curve_surface"
        | "brep_intersect_curve_ruled_surface"
        | "brep_intersect_curve_curve"
        | "brep_intersect_sphere_sphere"
        | "brep_intersect_sphere_cylinder"
        | "brep_intersect_sphere_cone"
        | "brep_intersect_cone_cone"
        | "brep_intersect_cylinder_cylinder"
        | "brep_intersect_plane_sphere"
        | "brep_intersect_plane_cylinder"
        | "brep_intersect_plane_cone"
        | "brep_intersect_plane_torus"
        | "brep_intersect_sphere_torus"
        | "brep_intersect_cylinder_torus"
        | "brep_intersect_cone_torus"
        | "brep_intersect_torus_torus"
        | "brep_intersection_trace_curve"
        | "brep_intersection_trace_curve_segments"
        | "brep_intersection_trace_evaluate" => intersections::dispatch(v),
        "brep_session" => brep_session_abi::dispatch(v),
        "mesh_picking" => mesh_picking::dispatch(v),
        "scene_picking" => scene_picking::dispatch(v),
        "viewport" => viewport::dispatch(v),
        "camera_gesture" => camera_gestures::dispatch(v),
        "mesh_export_file" => mesh_export_file::dispatch(v),
        "brep_graph" => brep_graph::dispatch(v),
        "brep_graph_report" => Ok(brep_graph::report(v)),
        "brep_scene_plan" => brep_scene_plan::plan(&v),
        "brep_semantic_geometry" => brep_semantic::execute(v),
        "brep_profile_transform"
        | "brep_profile_author"
        | "brep_profile_validate"
        | "brep_profile_offset"
        | "brep_profile_boolean"
        | "brep_profile_intersections"
        | "brep_profile_signed_area" => brep_profile::dispatch(v),
        "cad" | "mesh" => mesh::dispatch(v),
        "path2d" => path2d::dispatch(v),
        "subdivision_extrude" => encode(subdivision_core::Cage::extrude(
            &field::<Vec<[f64; 3]>>(&v, "profile")?,
            field(&v, "vector")?,
        )?),
        "subdivision_loft" => encode(subdivision_core::Cage::loft(
            &field::<Vec<Vec<[f64; 3]>>>(&v, "sections")?,
            field(&v, "caps")?,
        )?),
        "subdivision_sweep" => encode(subdivision_core::Cage::sweep(
            &field::<Vec<[f64; 2]>>(&v, "profile")?,
            &field::<Vec<[f64; 3]>>(&v, "path")?,
            field(&v, "up")?,
            field(&v, "caps")?,
        )?),
        "subdivision_revolve" => encode(subdivision_core::Cage::revolve(
            &field::<Vec<[f64; 2]>>(&v, "profile")?,
            field(&v, "segments")?,
        )?),
        "sketch_solve" => encode(sketch_core::solve(
            &field(&v, "sketch")?,
            field(&v, "tolerance")?,
        )?),
        "polygon_deform" => encode(polygon_core::solid::edit::deform(
            &field(&v, "mesh")?,
            &field(&v, "deformation")?,
        )?),
        "polygon_sculpt" => encode(polygon_core::solid::edit::sculpt(
            &field(&v, "mesh")?,
            &field(&v, "brush")?,
        )?),
        "subdivision_sculpt" => {
            encode(field::<subdivision_core::Cage>(&v, "cage")?.sculpt(&field(&v, "brush")?)?)
        }
        "polygon_brush" => encode(polygon_core::solid::edit::brush(
            &field(&v, "mesh")?,
            &field(&v, "brush")?,
        )?),
        "polygon_extrude_faces" => encode(polygon_core::solid::edit::extrude_faces(
            &field(&v, "mesh")?,
            &field::<Vec<usize>>(&v, "triangles")?,
            field(&v, "vector")?,
        )?),
        "subdivision_deform" => {
            encode(field::<subdivision_core::Cage>(&v, "cage")?.deform(&field(&v, "deformation")?)?)
        }
        "subdivision_brush" => {
            encode(field::<subdivision_core::Cage>(&v, "cage")?.brush(&field(&v, "brush")?)?)
        }
        "sdf_deform" => {
            encode(field::<sdf_core::Field>(&v, "field")?.deform(field(&v, "deformation")?)?)
        }
        "sdf_sculpt" => encode(field::<sdf_core::Field>(&v, "field")?.sculpt(&field::<
            sdf_core::SdfStroke,
        >(
            &v, "stroke"
        )?)?),
        "sdf_sculpt_sphere" => encode(field::<sdf_core::Field>(&v, "field")?.sculpt_sphere(
            field(&v, "center")?,
            field(&v, "radius")?,
            field(&v, "remove")?,
        )?),
        "polygon_extrude" => encode(polygon_core::solid::modeling::extrude(
            &field(&v, "profile")?,
            field(&v, "vector")?,
        )?),
        "polygon_revolve" => encode(polygon_core::solid::modeling::revolve(
            &field::<Vec<[f64; 2]>>(&v, "profile")?,
            field(&v, "angle")?,
            field(&v, "segments")?,
            field(&v, "caps")?,
        )?),
        "polygon_loft" => encode(polygon_core::solid::modeling::loft(
            &field::<Vec<Vec<[f64; 3]>>>(&v, "sections")?,
            field(&v, "caps")?,
        )?),
        "polygon_sweep" => encode(polygon_core::solid::modeling::sweep(
            &field::<Vec<[f64; 2]>>(&v, "profile")?,
            &field::<Vec<[f64; 3]>>(&v, "path")?,
            field(&v, "up")?,
            field(&v, "caps")?,
        )?),
        "mesh_to_nurbs_brep" => encode(reconstruction::nurbs_brep_from_mesh(&field(&v, "mesh")?)?),
        "mesh_to_sdf" => encode(sdf_core::Field::from_triangles(
            triangles_from_mesh(&polygon_core::solid::proximity::valid_source(
                &field(&v, "mesh")?,
                4096,
            )?),
            field(&v, "signed")?,
        )?),
        "mesh_to_subdivision" => encode(reconstruction::mesh_to_subdivision(
            &field(&v, "mesh")?,
            field(&v, "iterations")?,
        )?),
        "mesh_to_nurbs" => encode(reconstruction::nurbs_from_mesh(
            &field(&v, "mesh")?,
            field(&v, "mode")?,
            field(&v, "maxDeviationMm")?,
        )?),
        "nurbs_patches_tessellate" => encode(reconstruction::tessellate_surface_set(
            &field(&v, "patches")?,
            field(&v, "segments")?,
        )?),
        "subdivision_refine" => {
            encode(field::<subdivision_core::Cage>(&v, "cage")?.subdivide(field(&v, "levels")?)?)
        }
        "subdivision_tessellate" => {
            let refined =
                field::<subdivision_core::Cage>(&v, "cage")?.subdivide(field(&v, "levels")?)?;
            let (triangles, face_ids) = refined.triangulate()?;
            let mesh = mesh_from_triangles(triangles);
            let report = mesh.inspect()?;
            let mut value = encode(BuiltMesh { mesh, report })?;
            value["faceIds"] = json!(face_ids);
            Ok(value)
        }
        "sdf_evaluate" => {
            encode(field::<sdf_core::Field>(&v, "field")?.evaluate(field(&v, "point")?)?)
        }
        "mesh_spatial_lattice" => encode(mesh_shell::lattice(
            &field(&v, "mesh")?,
            field(&v, "nodes")?,
            field(&v, "edges")?,
            field(&v, "radius")?,
            field(&v, "skin")?,
            field(&v, "step")?,
            field(&v, "organic")?,
            field(&v, "openTop")?,
            v.get("wallDepth").and_then(|x| x.as_f64()).unwrap_or(0.),
            v.get("keepCore").and_then(|x| x.as_bool()).unwrap_or(false),
        )?),
        "mesh_shell_adaptive" => encode(mesh_shell::shell_options(
            &field(&v, "mesh")?,
            &field::<Vec<usize>>(&v, "openings")?,
            field(&v, "thickness")?,
            field(&v, "step")?,
            true,
        )?),
        "mesh_shell_sampled" => encode(mesh_shell::shell(
            &field(&v, "mesh")?,
            &field::<Vec<usize>>(&v, "openings")?,
            field(&v, "thickness")?,
            field(&v, "step")?,
        )?),
        "sdf_tessellate" => {
            let mesh = mesh_from_triangles(sdf_core::polygonize(
                &field(&v, "field")?,
                &field(&v, "grid")?,
            )?);
            let report = mesh.inspect()?;
            encode(BuiltMesh { mesh, report })
        }
        "sdf_prepare" => sdf_gpu::prepare(&v),
        "sdf_prepare_batch" => sdf_gpu::prepare_batch(&v),
        "sdf_finish" => sdf_gpu::finish(&v),
        "surface_tessellate" => encode(tessellate_nurbs(
            &field(&v, "surface")?,
            &field(&v, "options")?,
        )?),
        "mesh_boolean" => encode(polygon_core::solid::boolean::boolean(
            &field::<Mesh>(&v, "a")?,
            &field::<Mesh>(&v, "b")?,
            field(&v, "operation")?,
            &match v.get("options") {
                Some(options) => {
                    value_codec::from_value(options.clone()).map_err(|e| input(e.to_string()))?
                }
                None => polygon_core::solid::boolean::Options::default(),
            },
        )?),
        // Mesh sections, planned toolpaths and validated G-code previews.
        "cad_surface_boundary_measure" => cad_surface_diagnostics::measure(v),
        "cad_mesh_intersection" => cad_diagnostics::intersection(v),
        "cad_mesh_intersections" => cad_diagnostics::intersections(v),
        "cad_mesh_diagnostic_locations" => cad_diagnostics::locations(v),
        "cad_plane_section" => cad_diagnostics::section(v),
        "cad_measure_points" => cad_diagnostics::measure_points(v),
        "cad_curve_curvature" => cad_diagnostics::curvature(v),
        "cad_face_distance" => cad_face_distance::measure(v),
        "cad_shell_distance" => cad_shell_distance::measure(v),
        "cad_solid_distance" => cad_solid_distance::measure(v),
        "cad_material_segment" => cad_material_segment::inspect(v),
        "cad_source_body_restore" => cad_source_body::restore(v),
        "cad_material_chord" => cad_material_chord::inspect(v),
        "cad_material_wall" => cad_material_wall::inspect(v),
        "cad_whole_wall" => cad_whole_wall::inspect(v),
        "cad_boundary_agreement" => cad_boundary_agreement::diagnose(v),
        "cad_face_contacts" => cad_face_contacts::diagnose(v),
        "cad_self_intersection" => cad_face_contacts::diagnose_self_intersection(v),
        "mesh_section" => {
            let mesh: Mesh = field(&v, "mesh")?;
            let z_mm: f64 = field(&v, "z")?;
            let section =
                polygon_core::solid::section::MeshSectionIndex::new(&mesh)?.section(z_mm)?;
            Ok(json!({
                "z_mm": section.z_mm,
                "candidateTriangles": section.candidate_triangles,
                "contours": section.contours.iter().map(|contour| json!({
                    "points": contour.points,
                    "sourceTriangles": contour.source_triangles,
                })).collect::<Vec<_>>(),
            }))
        }
        "brep_nurbs_sketch_extrude" => {
            let sketch = v.get("sketch").ok_or_else(|| input("Missing sketch"))?;
            let profile = match sketch.get("analytic") {
                Some(analytic)
                    if analytic.get("kind").and_then(Value::as_str) == Some("circle") =>
                {
                    brep_core::sketch::Profile::Circle {
                        center: field(analytic, "center")?,
                        radius: field(analytic, "radius")?,
                    }
                }
                _ => brep_core::sketch::Profile::Polygon(field(sketch, "points")?),
            };
            let plane = match sketch.get("plane") {
                None | Some(Value::Null) => None,
                Some(plane) => Some([
                    field(plane, "origin")?,
                    field(plane, "u")?,
                    field(plane, "v")?,
                ]),
            };
            encode(brep_core::sketch::extrude(
                profile,
                field(sketch, "closed")?,
                field(&v, "height")?,
                field(&v, "baseZ")?,
                plane,
            )?)
        }
        "brep_sweep_retained_wall_charts_audit" => {
            let report = brep_core::miter_seams::inspect_charts(&field(&v,"model")?,&field::<Vec<usize>>(&v,"capFaces")?,field(&v,"maxCells")?)?;
            let charts: Vec<_> = report.charts.iter().map(|(face,a)| json!({"face":face,"audit":{
                "certified":a.certified,"cells":a.cells,"projection":a.projection,"reason":a.reason,"globalEmbeddingCertified":false}})).collect();
            Ok(json!({"allChartsCertified":report.certified,"cells":report.cells,"charts":charts,
                "unresolvedFaces":report.unresolved,"globalEmbeddingCertified":false}))
        }
        "brep_sweep_cap_pairs_audit" => {
            sweep_cap_evidence::inspect_pairs(&field(&v,"model")?,&field::<Vec<usize>>(&v,"capFaces")?,&v["budgets"])
        }
        "brep_sweep_cap_evidence_audit" => {
            let options=&v["boundaryOptions"];
            sweep_cap_evidence::inspect(&field(&v,"model")?,&field::<Vec<usize>>(&v,"capFaces")?,field(&v,"maxWalls")?,
                field(options,"tolerance")?,field(options,"maxProducts")?,field(options,"maxCells")?,field(options,"maxWork")?)
        }
        "brep_miter_station_smoothness_audit" => {
            let model = field(&v,"model")?; let caps: Vec<usize> = field(&v,"capFaces")?; let max_work = field(&v,"maxWork")?;
            let report = brep_core::miter_seams::inspect(&model,&caps,max_work,true)?;
            Ok(miter_smoothness::station(&report,max_work))
        }
        "brep_miter_profile_smoothness_audit" => {
            let model = field(&v,"model")?; let caps: Vec<usize> = field(&v,"capFaces")?; let max_work = field(&v,"maxWork")?;
            let report = brep_core::miter_seams::inspect_profile(&model,&caps,max_work)?;
            Ok(miter_smoothness::profile(&report,max_work,!caps.is_empty()))
        }
        "brep_sweep_retained_decomposition_audit" => {
            let r=brep_core::sweep_retained::decomposition(&field(&v,"model")?,&field::<Vec<Vec<Vec<Curve>>>>(&v,"sections")?,field(&v,"closed")?,field(&v,"maxProducts")?,field(&v,"maxFaces")?,field(&v,"maxExactWork")?)?;
            Ok(json!({"certified":r.error_upper.is_some(),"wallErrorUpper":r.error_upper,"inspectedFaces":r.inspected_faces,"products":r.products,"reason":r.reason}))
        }
        "brep_sweep_retained_correspondence_audit" => {
            let r=brep_core::sweep_retained::inspect(&field(&v,"model")?,&field::<Vec<Vec<Vec<Curve>>>>(&v,"sections")?,field(&v,"closed")?,field(&v,"maxFaces")?,field(&v,"maxExactWork")?)?;
            Ok(json!({"exact":r.exact,"wallErrorUpper":if r.exact {Some(0)} else {None},"inspectedFaces":r.inspected_faces,"exactWork":r.work,"reason":r.reason}))
        }
        "brep_nurbs_section_loft_source_audit" => {
            let exact=brep_core::analytic::section_loft_source_matches(&field(&v,"model")?,&field::<Vec<Vec<Vec<Curve>>>>(&v,"sections")?,field(&v,"closed")?)?;
            Ok(json!({"geometryAndTopologyIdentical":exact,"globalEmbeddingCertified":false}))
        }
        "brep_nurbs_affine_lattice" => {
            let report=brep_core::affine_lattice::place(&field(&v,"model")?,field(&v,"matrix")?,field(&v,"quantum")?,field(&v,"maxWork")?)?;
            Ok(json!({"model":report.model,"operatorNormUpper":report.operator_norm_upper,"arithmeticErrorUpper":report.arithmetic_error_upper,"work":report.work,"reason":report.reason}))
        }
        "brep_nurbs_transform" => encode(brep_core::transform::affine(
            &field(&v, "model")?,
            field(&v, "matrix")?,
        )?),
        "brep_nurbs_workplane" => encode(brep_core::transform::workplane(
            &field(&v, "model")?,
            field(&v, "origin")?,
            field(&v, "u")?,
            field(&v, "v")?,
            field(&v, "offset")?,
        )?),
        "brep_nurbs_box" => encode(brep_core::cuboid(field(&v, "min")?, field(&v, "max")?)?),
        "brep_nurbs_freeform_cuboid" => encode(brep_core::freeform_cuboid_solid(
            field(&v, "min")?,
            field(&v, "max")?,
        )?),
        "brep_nurbs_freeform_cuboid_bump" => encode(brep_core::freeform_cuboid_with_bump_face(
            field(&v, "min")?,
            field(&v, "max")?,
        )?),
        "brep_nurbs_revolve" => encode(brep_core::revolve_angle(
            &field::<Vec<[f64; 2]>>(&v, "profile")?,
            v.get("angleDegrees")
                .and_then(Value::as_f64)
                .unwrap_or(360.),
        )?),
        "brep_nurbs_sphere" => encode(brep_core::sphere(field(&v, "radius")?)?),
        "brep_nurbs_gear" => {
            let number = |name: &str, default: f64| -> f64 {
                v.get(name).and_then(Value::as_f64).unwrap_or(default)
            };
            let flag = |name: &str| v.get(name).and_then(Value::as_bool).unwrap_or(false);
            let teeth: f64 = field(&v, "teeth")?;
            let spec = brep_core::GearSpec {
                module: field(&v, "module")?,
                teeth: teeth.round().max(0.) as usize,
                pressure_angle_deg: number("pressureAngle", 20.),
                height: field(&v, "height")?,
                helix_angle_deg: number("helixAngle", 0.),
                herringbone: flag("herringbone"),
                bore: number("bore", 0.),
                internal: flag("internal"),
                rim_width: number("rimWidth", 2.),
                clearance: number("clearance", 0.25),
                backlash: number("backlash", 0.),
            };
            encode(brep_core::gear(&spec)?)
        }
        "brep_nurbs_torus" => encode(brep_core::torus(
            field(&v, "majorRadius")?,
            field(&v, "minorRadius")?,
        )?),
        "brep_nurbs_push_face" => encode(brep_core::operations::push_planar_face(
            &field(&v, "model")?,
            field(&v, "face")?,
            field(&v, "distance")?,
        )?),
        "brep_nurbs_shell" => encode(brep_core::operations::shell_planar(
            &field(&v, "model")?,
            &field::<Vec<usize>>(&v, "openings")?,
            field(&v, "thickness")?,
        )?),
        "brep_nurbs_exact_analytic_shell" => {
            require_exact_fields(
                &v,
                &["op", "model", "openings", "thickness", "direction"],
                "exact analytic shell request",
            )?;
            encode(brep_core::exact_analytic_shell(
                &field(&v, "model")?,
                &field::<Vec<usize>>(&v, "openings")?,
                field(&v, "thickness")?,
                v.get("direction")
                    .and_then(Value::as_str)
                    .ok_or_else(|| input("Invalid shell direction"))?,
            )?)
        }
        "brep_nurbs_close_topology_v1" => close_topology_audit_value(&v),
        "brep_nurbs_split" => encode(brep_core::operations::split_planar(
            &field(&v, "model")?,
            field(&v, "normal")?,
            field(&v, "offset")?,
        )?),
        "brep_nurbs_mass_properties" => encode(brep_core::analysis::mass_properties(
            &field::<brep_core::Model>(&v, "model")?,
            v.get("relativeTolerance")
                .and_then(Value::as_f64)
                .unwrap_or(1e-7),
            v.get("maxEvaluations")
                .and_then(Value::as_u64)
                .unwrap_or(300000) as usize,
        )?),
        "brep_nurbs_certified_mass_properties" => {
            let model: brep_core::Model = field(&v, "model")?;
            encode(brep_core::analysis::certified_mass_properties(&model)?)
        }
        "brep_nurbs_certified_freeform_mass_properties" => {
            let model: brep_core::Model = field(&v, "model")?;
            encode(brep_core::analysis::certified_freeform_mass_properties(
                &model,
            )?)
        }
        "brep_nurbs_authorized_heal_v2" => {
            require_exact_fields(&v, &["op", "model", "operation"], "heal request")?;
            let model: brep_core::Model = field(&v, "model")?;
            let operation = v
                .get("operation")
                .ok_or_else(|| input("Missing operation"))?;
            let kind = operation
                .get("kind")
                .and_then(Value::as_str)
                .ok_or_else(|| input("Invalid operation kind"))?;
            match kind {
                "endpointSnap" => {
                    require_exact_fields(operation, &["kind", "vertex", "to"], "endpoint snap")?;
                    encode(brep_core::transactions::authorized_heal_endpoint(
                        &model,
                        field(operation, "vertex")?,
                        field(operation, "to")?,
                    )?)
                }
                "rationalRefit" => {
                    require_exact_fields(
                        operation,
                        &["kind", "edge", "replacement"],
                        "rational refit",
                    )?;
                    encode(brep_core::transactions::authorized_heal_refit(
                        &model,
                        field(operation, "edge")?,
                        field(operation, "replacement")?,
                    )?)
                }
                _ => Err(input("Unsupported authorized heal operation")),
            }
        }
        "brep_nurbs_cylinder" => encode(brep_core::cylinder(
            field(&v, "radius")?,
            field(&v, "height")?,
        )?),
        "brep_nurbs_frustum" => encode(brep_core::frustum(
            field(&v, "bottomRadius")?,
            field(&v, "topRadius")?,
            field(&v, "height")?,
        )?),
        "brep_nurbs_tube" => encode(brep_core::tube(
            field(&v, "outerRadius")?,
            field(&v, "innerRadius")?,
            field(&v, "height")?,
        )?),
        "brep_nurbs_extrude_curves" => encode(brep_core::prism::extrude(
            &field::<Vec<Vec<Curve>>>(&v, "loops")?,
            field(&v, "zMin")?,
            field(&v, "zMax")?,
        )?),
        "brep_nurbs_extrude_polygon" => {
            let holes = v
                .get("holes")
                .map(|_| field::<Vec<Vec<[f64; 2]>>>(&v, "holes"))
                .transpose()?
                .unwrap_or_default();
            encode(brep_core::extrude_polygon_with_holes(
                &field::<Vec<[f64; 2]>>(&v, "profile")?,
                &holes,
                field(&v, "zMin")?,
                field(&v, "zMax")?,
            )?)
        }
        operation @ ("brep_nurbs_smooth_polygon_station_body" | "brep_nurbs_section_loft_surfaces" | "brep_nurbs_rational_section_loft") => polygon_station_body::dispatch(operation,&v),
        "brep_nurbs_smooth_station_walls" => {
            let report = brep_core::analytic::smooth_station_walls(
                &field::<Vec<Vec<Vec<Curve>>>>(&v, "sections")?,
                &field::<Vec<usize>>(&v, "sharp")?,
                field(&v, "closed")?, field(&v, "quantum")?,
                field(&v, "tolerance")?, field(&v, "maxWork")?,
            )?;
            encode(json!({"sides": report.sides,
                "wallDisplacementUpper": report.wall_displacement_upper,
                "work": report.work, "reason": report.reason}))
        },
        "brep_sweep_retained_cap_decomposition_audit" => {
            let r=brep_core::sweep_retained::cap_decomposition(&field(&v,"model")?,&field::<Vec<Vec<Vec<Curve>>>>(&v,"endpoints")?,brep_core::sweep_cap_contacts::Budgets {
                max_walls:field(&v,"maxWalls")?,max_exact_work:field(&v,"maxExactWork")?,max_chart_cells:field(&v,"maxChartCells")?,max_trim_pairs:field(&v,"maxTrimPairs")?,max_trim_cells:field(&v,"maxTrimCells")?,max_trim_domain_cells:field(&v,"maxTrimDomainCells")?,
            },field(&v,"maxProducts")?,field(&v,"maxEdges")?)?;
            let regions=r.regions.map(|r|json!({"exact":r.exact,"capErrorUpper":if r.exact {Some(0)} else {None},"exactWork":r.exact_work,"faces":r.faces,"reason":r.reason}));
            Ok(json!({"certified":r.error_upper.is_some(),"capErrorUpper":r.error_upper,"products":r.products,"regions":regions,"reason":r.reason}))
        }
        "brep_sweep_retained_caps_audit" => {
            let r=brep_core::sweep_cap_contacts::inspect_retained_caps(&field(&v,"model")?,&field::<Vec<Vec<Vec<Curve>>>>(&v,"endpoints")?,brep_core::sweep_cap_contacts::Budgets {
                max_walls:field(&v,"maxWalls")?,max_exact_work:field(&v,"maxExactWork")?,max_chart_cells:field(&v,"maxChartCells")?,max_trim_pairs:field(&v,"maxTrimPairs")?,max_trim_cells:field(&v,"maxTrimCells")?,max_trim_domain_cells:field(&v,"maxTrimDomainCells")?,
            },field(&v,"maxEdges")?)?;
            Ok(json!({"exact":r.exact,"capErrorUpper":if r.exact {Some(0)} else {None},"exactWork":r.exact_work,"faces":r.faces,"reason":r.reason}))
        }
        "brep_sweep_cap_contacts_audit" => {
            let report=brep_core::sweep_cap_contacts::inspect(&field::<brep_core::Model>(&v,"model")?,field(&v,"capFace")?,
                &field::<Vec<usize>>(&v,"capFaces")?,brep_core::sweep_cap_contacts::Budgets {
                    max_walls:field(&v,"maxWalls")?,max_exact_work:field(&v,"maxExactWork")?,
                    max_chart_cells:field(&v,"maxChartCells")?,max_trim_pairs:field(&v,"maxTrimPairs")?,
                    max_trim_cells:field(&v,"maxTrimCells")?,max_trim_domain_cells:field(&v,"maxTrimDomainCells")?,
                })?;
            Ok(json!({"capCertified":report.cap_certified,"planarControlHullCertified":report.planar_control_hull_certified,"allCapWallContactsCertified":report.all_cap_wall_contacts_certified,
                "separatedWalls":report.separated_walls,"allowedBoundaries":report.allowed_boundaries,
                "unresolvedWalls":report.unresolved_walls,"exactWork":report.exact_work,"reason":report.reason,
                "globalEmbeddingCertified":false}))
        }
        "brep_sweep_embedding_audit" => cad_face_contacts::diagnose_sweep_embedding(v),
        "brep_sweep_volume_audit" => cad_face_contacts::diagnose_sweep_volume(v),
        "brep_nurbs_natural_section_loft" => encode(brep_core::natural_section_loft(&field::<Vec<Vec<Vec<Curve>>>>(&v,"sections")?, &field::<Vec<f64>>(&v,"parameters")?)?),
        "brep_nurbs_capped_loft_with_caps" => {
            let definitions: Vec<Value> = field(&v,"caps")?;
            if definitions.len()!=2 { return Err(input("Loft requires two authored caps")); }
            let caps = definitions.iter().map(|c| Ok(brep_core::LoftCap {
                surface:field(c,"surface")?,trims:field(c,"trims")?,
            })).collect::<Result<Vec<_>>>()?;
            let l:Value=field(&v,"embeddingLimits")?;
            let limits=brep_core::boundary_embedding::Limits {
                exact_work:field(&l,"exactWork")?,trim_pairs:field(&l,"trimPairs")?,
                trim_cells:field(&l,"trimCells")?,trim_domain_cells:field(&l,"trimDomainCells")?,
                spans:field(&l,"spans")?,contacts:brep_core::face_contacts::Limits {
                    pairs:field(&l,"facePairs")?,cells:field(&l,"faceCells")?,
                    domain_cells:field(&l,"faceDomainCells")?,cells_per_pair:field(&l,"faceCellsPerPair")?,
                    domain_cells_per_pair:field(&l,"faceDomainCellsPerPair")?,
                },
            };
            encode(brep_core::capped_loft_with_caps_checked(
                &field::<Vec<Vec<Curve>>>(&v,"start")?, &field::<Vec<Vec<Curve>>>(&v,"end")?,
                &field::<Vec<Vec<Surface>>>(&v,"sides")?, [&caps[0],&caps[1]],
                field(&v,"toleranceUv")?, limits)?)
        }
        "brep_nurbs_capped_loft_surfaces" => encode(brep_core::capped_loft_surfaces(&field::<Vec<Vec<Curve>>>(&v,"start")?, &field::<Vec<Vec<Curve>>>(&v,"end")?, &field::<Vec<Vec<Surface>>>(&v,"sides")?)?),
        "brep_nurbs_periodic_section_loft" => encode(brep_core::periodic_section_loft(&field::<Vec<Vec<Vec<Curve>>>>(&v,"sections")?)?),
        "brep_nurbs_ruled_loft" => encode(brep_core::ruled_loft(&field::<Vec<Vec<[f64; 3]>>>(
            &v, "sections",
        )?)?),
        "brep_nurbs_faceted_loft" => encode(brep_core::faceted_loft(
            &field::<Vec<Vec<[f64; 3]>>>(&v, "sections")?,
        )?),
        "brep_nurbs_faceted_sweep" => encode(brep_core::faceted_sweep(
            &field::<Vec<[f64; 2]>>(&v, "profile")?,
            &field::<Vec<[f64; 3]>>(&v, "path")?,
            field(&v, "up")?,
        )?),
        "brep_nurbs_faceted_revolve" => encode(brep_core::faceted_revolve(
            &field::<Vec<[f64; 2]>>(&v, "profile")?,
            field(&v, "segments")?,
        )?),
        "brep_nurbs_faceted_cylinder" => encode(brep_core::faceted_cylinder(
            field(&v, "radius")?,
            field(&v, "height")?,
            field(&v, "segments")?,
        )?),
        "brep_nurbs_faceted_sphere" => encode(brep_core::faceted_sphere(
            field(&v, "radius")?,
            field(&v, "radialSegments")?,
            field(&v, "latitudeSegments")?,
        )?),
        "brep_nurbs_canonical_graph_solid_v3" => encode(brep_core::canonical_bezier_graph_solid(
            field(&v, "degreeU")?,
            field(&v, "degreeV")?,
        )?),
        "brep_nurbs_canonical_rational_graph_solid_v5" => {
            encode(brep_core::canonical_rational_graph_solid(
                field(&v, "degreeU")?,
                field(&v, "degreeV")?,
            )?)
        }
        "brep_nurbs_canonical_multispan_graph_solid" => {
            require_exact_fields(&v, &["op", "spansU", "spansV"], "multispan graph request")?;
            encode(brep_core::canonical_multispan_graph_solid(
                field(&v, "spansU")?,
                field(&v, "spansV")?,
            )?)
        }
        "brep_nurbs_boolean_bezier_le3_v3" => {
            require_exact_fields(
                &v,
                &["op", "a", "b", "operation"],
                "V3 graph Boolean request",
            )?;
            let a: brep_core::Model = field(&v, "a")?;
            let b: brep_core::Model = field(&v, "b")?;
            let operation = v
                .get("operation")
                .and_then(Value::as_str)
                .ok_or_else(|| input("Invalid operation"))?;
            let (model, certificate) = brep_core::nurbs_boolean_graph_patch_v3(&a, &b, operation)?;
            curved_graph_boolean_value(model, certificate)
        }
        "brep_nurbs_boolean_bezier_le3_v4_unequal" => {
            require_exact_fields(
                &v,
                &["op", "a", "b", "operation"],
                "V4 unequal graph Boolean request",
            )?;
            let (model, certificate) = brep_core::nurbs_boolean_graph_patch_unequal_v4(
                &field(&v, "a")?,
                &field(&v, "b")?,
                v.get("operation")
                    .and_then(Value::as_str)
                    .ok_or_else(|| input("Invalid operation"))?,
            )?;
            curved_graph_boolean_value(model, certificate)
        }
        "brep_nurbs_boolean_bezier_le3_v4_containment" => {
            require_exact_fields(
                &v,
                &["op", "graph", "cutter", "operation"],
                "V4 containment graph Boolean request",
            )?;
            let (model, certificate) = brep_core::nurbs_boolean_graph_containment_v4(
                &field(&v, "graph")?,
                &field(&v, "cutter")?,
                v.get("operation")
                    .and_then(Value::as_str)
                    .ok_or_else(|| input("Invalid operation"))?,
            )?;
            contained_graph_boolean_value(model, certificate)
        }
        "brep_nurbs_boolean_bezier_le3_v5_rational" => {
            require_exact_fields(
                &v,
                &["op", "a", "b", "operation"],
                "V5 rational graph Boolean request",
            )?;
            let (model, certificate) = brep_core::nurbs_boolean_rational_graph_patch_v5(
                &field(&v, "a")?,
                &field(&v, "b")?,
                v.get("operation")
                    .and_then(Value::as_str)
                    .ok_or_else(|| input("Invalid operation"))?,
            )?;
            curved_graph_boolean_value(model, certificate)
        }
        "brep_nurbs_boolean_general" => {
            require_exact_fields(
                &v,
                &["op", "a", "b", "operation"],
                "general NURBS Boolean request",
            )?;
            let (model, certificate) = brep_core::author_general_nurbs_boolean(
                &field(&v, "a")?,
                &field(&v, "b")?,
                v.get("operation")
                    .and_then(Value::as_str)
                    .ok_or_else(|| input("Invalid operation"))?,
            )?;
            general_nurbs_boolean_value(model, certificate)
        }
        "brep_nurbs_boolean" => encode(brep_core::boolean(
            &field(&v, "a")?,
            &field(&v, "b")?,
            v.get("operation")
                .and_then(|value| value.as_str())
                .ok_or_else(|| input("Invalid operation"))?,
        )?),
        "brep_nurbs_chamfer" => encode(brep_core::chamfer(
            &field(&v, "model")?,
            field(&v, "edge")?,
            field(&v, "size")?,
        )?),
        "brep_nurbs_chamfer_edges" => encode(brep_core::chamfer_edges(
            &field(&v, "model")?,
            &field::<Vec<usize>>(&v, "edges")?,
            field(&v, "size")?,
        )?),
        "brep_nurbs_fillet" => encode(brep_core::fillet(
            &field(&v, "model")?,
            field(&v, "edge")?,
            field(&v, "radius")?,
            field(&v, "segments")?,
        )?),
        "brep_nurbs_fillet_edges" => encode(brep_core::fillet_edges(
            &field(&v, "model")?,
            &field::<Vec<usize>>(&v, "edges")?,
            field(&v, "radius")?,
            field(&v, "segments")?,
        )?),
        "brep_nurbs_audited_multi_edge_fillet" => {
            let model: brep_core::Model = field(&v, "model")?;
            encode(brep_core::audited_multi_edge_fillet(
                &model,
                &field::<Vec<usize>>(&v, "edges")?,
                field(&v, "radius")?,
            )?)
        }
        "brep_nurbs_exact_convex_chamfer" => {
            require_exact_fields(
                &v,
                &["op", "model", "edges", "distance"],
                "exact chamfer request",
            )?;
            encode(brep_core::exact_convex_chamfer(
                &field(&v, "model")?,
                &field::<Vec<usize>>(&v, "edges")?,
                field(&v, "distance")?,
            )?)
        }
        "brep_nurbs_exact_convex_prism_fillet" | "brep_nurbs_exact_simple_prism_fillet" | "brep_nurbs_exact_annular_fillet" | "brep_nurbs_exact_layered_prism_fillet" => {
            require_exact_fields(
                &v,
                &["op", "model", "edges", "radius"],
                "exact fillet request",
            )?;
            let author = match v["op"].as_str() {
                Some("brep_nurbs_exact_simple_prism_fillet") => brep_core::exact_simple_prism_fillet,
                Some("brep_nurbs_exact_annular_fillet") => brep_core::exact_annular_fillet,
                Some("brep_nurbs_exact_layered_prism_fillet") => brep_core::exact_layered_prism_fillet,
                _ => brep_core::exact_convex_prism_fillet,
            };
            encode(author(
                &field(&v, "model")?,
                &field::<Vec<usize>>(&v, "edges")?,
                field(&v, "radius")?,
            )?)
        }
        "brep_nurbs_partial_annular_preview" => {
            require_exact_fields(&v,&["op","model","edge","radius"],"partial annular preview request")?;
            let model=brep_core::analytic_features::build_partial_annular_preview(
                &field(&v,"model")?,field(&v,"edge")?,field(&v,"radius")?)?;
            Ok(json!({
                "model":model,
                "changeSet":model.1.change_set,
                "qualification":{
                    "status":"preview-only", "commitAllowed":false,
                    "boundaryIntersectionProof":"unqualified",
                    "transitionContinuityProof":"unqualified"
                }
            }))
        }
        "brep_nurbs_exact_variable_radius_fillet" => {
            require_exact_fields(
                &v,
                &["op", "model", "edges", "radii"],
                "variable-radius fillet request",
            )?;
            encode(brep_core::exact_variable_radius_fillet(
                &field(&v, "model")?,
                &field::<Vec<usize>>(&v, "edges")?,
                &field::<Vec<[f64; 2]>>(&v, "radii")?,
            )?)
        }
        "brep_nurbs_exact_valence3_corner_blend" => {
            require_exact_fields(
                &v,
                &["op", "model", "edges", "radius"],
                "valence-3 corner blend request",
            )?;
            encode(brep_core::exact_valence3_corner_blend(
                &field(&v, "model")?,
                &field::<Vec<usize>>(&v, "edges")?,
                field::<f64>(&v, "radius")?,
            )?)
        }
        "brep_nurbs_audited_parallel_frame_sweep" => {
            let frame_law: String = field(&v, "frameLaw")?;
            encode(brep_core::audited_parallel_frame_sweep(
                &field::<Vec<[f64; 2]>>(&v, "profile")?,
                &field::<Vec<[f64; 3]>>(&v, "path")?,
                &frame_law,
            )?)
        }
        "brep_nurbs_audited_multi_section_loft_v2" => {
            require_exact_fields(&v, &["op", "sections"], "multi-section loft request")?;
            encode(brep_core::audited_multi_section_loft(&field::<
                Vec<Vec<[f64; 3]>>,
            >(
                &v, "sections"
            )?)?)
        }
        "brep_nurbs_audited_bent_rmf_sweep_v2" => {
            require_exact_fields(
                &v,
                &["op", "profile", "path", "twistRadians", "scales"],
                "bent RMF sweep request",
            )?;
            encode(brep_core::audited_bent_rmf_sweep(
                &field::<Vec<[f64; 2]>>(&v, "profile")?,
                &field::<Vec<[f64; 3]>>(&v, "path")?,
                &field::<Vec<f64>>(&v, "twistRadians")?,
                &field::<Vec<f64>>(&v, "scales")?,
            )?)
        }
        "brep_nurbs_inspect" => {
            encode(take_field::<brep_core::Model>(&mut v, "model")?.validate()?)
        }
        "brep_nurbs_export_step" => {
            let (text, cert) = brep_core::export_step(&field(&v, "model")?)?;
            encode(json!({
                "text": text,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                }
            }))
        }
        "brep_nurbs_import_step" => {
            let (model, cert) = brep_core::import_step(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model": model,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                }
            }))
        }
        "brep_nurbs_export_step_v2" => {
            let (text, cert, identity) = brep_core::export_step_v2(&field(&v, "model")?)?;
            encode(json!({
                "text": text,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                },
                "identity": {
                    "preserved": identity.preserved,
                    "source": identity.source,
                    "preservedCount": identity.preserved_count,
                    "createdCount": identity.created_count,
                    "lostCount": identity.lost_count,
                }
            }))
        }
        "brep_nurbs_import_step_v2" => {
            let (model, cert, identity) = brep_core::import_step_v2(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model": model,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                },
                "identity": {
                    "preserved": identity.preserved,
                    "source": identity.source,
                    "preservedCount": identity.preserved_count,
                    "createdCount": identity.created_count,
                    "lostCount": identity.lost_count,
                }
            }))
        }
        "brep_nurbs_export_step_v3" => {
            let (text, cert, report) = brep_core::export_step_v3(&field(&v, "model")?)?;
            encode(json!({
                "text": text,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                },
                "identity": {
                    "preserved": report.identity.preserved,
                    "source": report.identity.source,
                    "preservedCount": report.identity.preserved_count,
                    "createdCount": report.identity.created_count,
                    "lostCount": report.identity.lost_count,
                },
                "ignoredEntities": report.ignored_entities,
                "instanceCount": report.instance_count,
                "reachableCount": report.reachable_count,
            }))
        }
        "brep_nurbs_import_step_v3" => {
            let (model, cert, report) = brep_core::import_step_v3(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model": model,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                },
                "identity": {
                    "preserved": report.identity.preserved,
                    "source": report.identity.source,
                    "preservedCount": report.identity.preserved_count,
                    "createdCount": report.identity.created_count,
                    "lostCount": report.identity.lost_count,
                },
                "ignoredEntities": report.ignored_entities,
                "instanceCount": report.instance_count,
                "reachableCount": report.reachable_count,
            }))
        }
        "brep_nurbs_export_step_v4" => {
            let (text, cert, report) = brep_core::export_step_v4(&field(&v, "model")?)?;
            encode(json!({
                "text":text,"certificate":{"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"instanceCount":report.instance_count,"reachableCount":report.reachable_count,
            }))
        }
        "brep_nurbs_import_step_v4" => {
            let (model, cert, report) = brep_core::import_step_v4(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model":model,"certificate":{"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"instanceCount":report.instance_count,"reachableCount":report.reachable_count,
            }))
        }
        "brep_nurbs_export_step_v5" => {
            let (text, cert, report) = brep_core::export_step_v5(&field(&v, "model")?)?;
            encode(json!({
                "text":text,"certificate":{"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"metadataLoss":report.metadata_loss,
                "instanceCount":report.instance_count,"reachableCount":report.reachable_count,
            }))
        }
        "brep_nurbs_import_step_v5" => {
            let (model, cert, report) = brep_core::import_step_v5(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model":model,"certificate":{"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"metadataLoss":report.metadata_loss,
                "instanceCount":report.instance_count,"reachableCount":report.reachable_count,
            }))
        }
        "brep_nurbs_export_step_v6" => {
            let (text, cert, report) = brep_core::export_step_v6(&field(&v, "model")?)?;
            encode(json!({
                "text":text,"certificate":{"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"metadataLoss":report.metadata_loss,
                "instanceCount":report.instance_count,"reachableCount":report.reachable_count,
                "definitionIdentities":report.definition_identities,"occurrenceIdentities":report.occurrence_identities,
                "productHierarchy":report.product_hierarchy,"externalReferences":report.external_references,
            }))
        }
        "brep_nurbs_import_step_v6" => {
            let (model, cert, report) = brep_core::import_step_v6(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model":model,"certificate":{"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"metadataLoss":report.metadata_loss,
                "instanceCount":report.instance_count,"reachableCount":report.reachable_count,
                "definitionIdentities":report.definition_identities,"occurrenceIdentities":report.occurrence_identities,
                "productHierarchy":report.product_hierarchy,"externalReferences":report.external_references,
            }))
        }
        "brep_nurbs_export_step_v7" => {
            let (text, cert, report) = brep_core::export_step_v7(&field(&v, "model")?)?;
            encode(json!({
                "text":text,"certificate":{"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"metadataLoss":report.metadata_loss,
                "instanceCount":report.instance_count,"reachableCount":report.reachable_count,
                "definitionIdentities":report.definition_identities,"occurrenceIdentities":report.occurrence_identities,
                "productHierarchy":report.product_hierarchy,"externalReferences":report.external_references,
            }))
        }
        "brep_nurbs_import_step_v7" => {
            let (model, cert, report) = brep_core::import_step_v7(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model":model,"certificate":{"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"metadataLoss":report.metadata_loss,
                "instanceCount":report.instance_count,"reachableCount":report.reachable_count,
                "definitionIdentities":report.definition_identities,"occurrenceIdentities":report.occurrence_identities,
                "productHierarchy":report.product_hierarchy,"externalReferences":report.external_references,
            }))
        }
        "brep_nurbs_export_step_v8" => {
            let (text, cert, report) = brep_core::export_step_v8(&field(&v, "model")?)?;
            let regularity=cert.regularity.iter().map(|row|json!({
                "carrier":row.carrier,"parameterU":row.parameter_u,"parameterV":row.parameter_v,
                "liftedPeriods":row.lifted_periods,"denominatorLowerBound":row.denominator_lower_bound,
                "jacobianLowerBound":row.jacobian_lower_bound,"collapsedBoundaries":row.collapsed_boundaries,
                "regularOpenDomain":row.regular_open_domain,"identity":row.identity,
            })).collect::<Vec<_>>();
            encode(json!({
                "text":text,"certificate":{"capability":cert.capability,"complete":cert.complete,
                    "regularity":regularity,"senseLayers":cert.sense_layers,
                    "coupledSenseCases":cert.coupled_sense_cases,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"metadataLoss":report.metadata_loss,
                "instanceCount":report.instance_count,"reachableCount":report.reachable_count,
                "definitionIdentities":report.definition_identities,"occurrenceIdentities":report.occurrence_identities,
                "productHierarchy":report.product_hierarchy,"externalReferences":report.external_references,
            }))
        }
        "brep_nurbs_import_step_v8" => {
            let (model, cert, report) = brep_core::import_step_v8(&field::<String>(&v, "text")?)?;
            let regularity=cert.regularity.iter().map(|row|json!({
                "carrier":row.carrier,"parameterU":row.parameter_u,"parameterV":row.parameter_v,
                "liftedPeriods":row.lifted_periods,"denominatorLowerBound":row.denominator_lower_bound,
                "jacobianLowerBound":row.jacobian_lower_bound,"collapsedBoundaries":row.collapsed_boundaries,
                "regularOpenDomain":row.regular_open_domain,"identity":row.identity,
            })).collect::<Vec<_>>();
            encode(json!({
                "model":model,"certificate":{"capability":cert.capability,"complete":cert.complete,
                    "regularity":regularity,"senseLayers":cert.sense_layers,
                    "coupledSenseCases":cert.coupled_sense_cases,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"metadataLoss":report.metadata_loss,
                "instanceCount":report.instance_count,"reachableCount":report.reachable_count,
                "definitionIdentities":report.definition_identities,"occurrenceIdentities":report.occurrence_identities,
                "productHierarchy":report.product_hierarchy,"externalReferences":report.external_references,
            }))
        }
        "brep_nurbs_export_step_v9" => {
            let (text, cert, report) = brep_core::export_step_v9(&field(&v, "model")?)?;
            let regularity=cert.regularity.iter().map(|row|json!({
                "carrier":row.carrier,"parameterU":row.parameter_u,"parameterV":row.parameter_v,
                "liftedPeriods":row.lifted_periods,"denominatorLowerBound":row.denominator_lower_bound,
                "jacobianLowerBound":row.jacobian_lower_bound,"collapsedBoundaries":row.collapsed_boundaries,
                "regularOpenDomain":row.regular_open_domain,"identity":row.identity,
            })).collect::<Vec<_>>();
            encode(json!({
                "text":text,"certificate":{"capability":cert.capability,"complete":cert.complete,
                    "regularity":regularity,"senseLayers":cert.sense_layers,
                    "coupledSenseCases":cert.coupled_sense_cases,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"metadataLoss":report.metadata_loss,
                "instanceCount":report.instance_count,"reachableCount":report.reachable_count,
                "definitionIdentities":report.definition_identities,"occurrenceIdentities":report.occurrence_identities,
                "productHierarchy":report.product_hierarchy,"externalReferences":report.external_references,
            }))
        }
        "brep_nurbs_import_step_v9" => {
            let (model, cert, report) = brep_core::import_step_v9(&field::<String>(&v, "text")?)?;
            let regularity=cert.regularity.iter().map(|row|json!({
                "carrier":row.carrier,"parameterU":row.parameter_u,"parameterV":row.parameter_v,
                "liftedPeriods":row.lifted_periods,"denominatorLowerBound":row.denominator_lower_bound,
                "jacobianLowerBound":row.jacobian_lower_bound,"collapsedBoundaries":row.collapsed_boundaries,
                "regularOpenDomain":row.regular_open_domain,"identity":row.identity,
            })).collect::<Vec<_>>();
            encode(json!({
                "model":model,"certificate":{"capability":cert.capability,"complete":cert.complete,
                    "regularity":regularity,"senseLayers":cert.sense_layers,
                    "coupledSenseCases":cert.coupled_sense_cases,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"metadataLoss":report.metadata_loss,
                "instanceCount":report.instance_count,"reachableCount":report.reachable_count,
                "definitionIdentities":report.definition_identities,"occurrenceIdentities":report.occurrence_identities,
                "productHierarchy":report.product_hierarchy,"externalReferences":report.external_references,
            }))
        }
        "brep_nurbs_import_step_v10" => {
            let (model, cert, document) =
                brep_core::import_step_v10(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model":model,
                "certificate":{"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "document":{
                    "source":document.source,"graphIdentity":document.graph_identity,
                    "definitionIdentities":document.definition_identities,
                    "occurrenceIdentities":document.occurrence_identities,
                    "productHierarchy":document.product_hierarchy,
                    "operatorIdentities":document.operator_identities,
                    "metadataLoss":document.metadata_loss,
                }
            }))
        }
        "brep_nurbs_export_step_v10" => {
            let document = brep_core::StepV10Document {
                source: field(&v, "source")?,
                graph_identity: field(&v, "graphIdentity")?,
                definition_identities: Vec::new(),
                occurrence_identities: Vec::new(),
                product_hierarchy: Vec::new(),
                operator_identities: Vec::new(),
                metadata_loss: Vec::new(),
            };
            encode(json!({"text":brep_core::export_step_v10(&document)?,
                "certificate":{"capability":"step-interchange/10","complete":true,
                    "notes":["retained_affine_occurrence_graph","exact_graph_isomorphism_identity"]}}))
        }
        "brep_nurbs_compose_step_v7" => {
            let models = field::<Vec<brep_core::Model>>(&v, "models")?;
            encode(brep_core::compose_step_v7_occurrences(&models)?)
        }
        "brep_nurbs_compose_step_v8" => {
            let models = field::<Vec<brep_core::Model>>(&v, "models")?;
            encode(brep_core::compose_step_v8_occurrences(&models)?)
        }
        "brep_nurbs_compose_step_v9" => {
            let models = field::<Vec<brep_core::Model>>(&v, "models")?;
            encode(brep_core::compose_step_v9_occurrences(&models)?)
        }
        "brep_nurbs_export_iges_v2" => {
            let (text, cert, report) = brep_core::export_iges_v2(&field(&v, "model")?)?;
            encode(json!({
                "text": text,
                "certificate": {"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "identity": {
                    "preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,
                    "createdCount":report.identity.created_count,"lostCount":report.identity.lost_count,
                },
                "ignoredMetadata":report.ignored_metadata,
                "entityCount":report.entity_count,
                "topologyCount":report.topology_count,
            }))
        }
        "brep_nurbs_import_iges_v2" => {
            let (model, cert, report) = brep_core::import_iges_v2(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model":model,
                "certificate": {"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "identity": {
                    "preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,
                    "createdCount":report.identity.created_count,"lostCount":report.identity.lost_count,
                },
                "ignoredMetadata":report.ignored_metadata,
                "entityCount":report.entity_count,
                "topologyCount":report.topology_count,
            }))
        }
        "brep_nurbs_export_step_freeform" => {
            let (text, cert) = brep_core::export_nurbs_step(&field(&v, "model")?)?;
            encode(json!({
                "text": text,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                }
            }))
        }
        "brep_nurbs_import_step_freeform" => {
            let (model, cert) = brep_core::import_nurbs_step(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model": model,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                }
            }))
        }
        "brep_nurbs_export_step_trimmed" => {
            let (text, cert) = brep_core::export_nurbs_step_trimmed(&field(&v, "model")?)?;
            encode(json!({
                "text": text,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                }
            }))
        }
        "brep_nurbs_import_step_trimmed" => {
            let (model, cert) =
                brep_core::import_nurbs_step_trimmed(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model": model,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                }
            }))
        }
        "brep_nurbs_export_step_solid" => {
            let (text, cert) = brep_core::export_nurbs_step_solid(&field(&v, "model")?)?;
            encode(json!({
                "text": text,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                }
            }))
        }
        "brep_nurbs_import_step_solid" => {
            let (model, cert) = brep_core::import_nurbs_step_solid(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model": model,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                }
            }))
        }
        "brep_nurbs_export_step_solid_v2" => {
            let (text, cert, identity) =
                brep_core::export_nurbs_step_solid_v2(&field(&v, "model")?)?;
            encode(json!({
                "text": text,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                },
                "identity": {
                    "preserved": identity.preserved,
                    "source": identity.source,
                    "preservedCount": identity.preserved_count,
                    "createdCount": identity.created_count,
                    "lostCount": identity.lost_count,
                }
            }))
        }
        "brep_nurbs_import_step_solid_v2" => {
            let (model, cert, identity) =
                brep_core::import_nurbs_step_solid_v2(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model": model,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                },
                "identity": {
                    "preserved": identity.preserved,
                    "source": identity.source,
                    "preservedCount": identity.preserved_count,
                    "createdCount": identity.created_count,
                    "lostCount": identity.lost_count,
                }
            }))
        }
        "brep_nurbs_tessellate" => encode(brep::nurbs(
            &take_field(&mut v, "model")?,
            field(&v, "segments")?,
        )?),
        "brep_nurbs_certified_tessellate" => encode(brep::certified_nurbs(
            &take_field(&mut v, "model")?,
            field(&v, "chordToleranceMm")?,
            v.get("maxTriangles")
                .and_then(Value::as_u64)
                .unwrap_or(20_000) as usize,
        )?),
        "brep_nurbs_certified_freeform_tessellate" => encode(brep::certified_freeform_nurbs(
            &take_field(&mut v, "model")?,
            field(&v, "chordToleranceMm")?,
            v.get("maxTriangles")
                .and_then(Value::as_u64)
                .unwrap_or(20_000) as usize,
        )?),
        "brep_nurbs_display" => brep_display::dispatch(v),
        "brep_nurbs_to_polygon" => {
            let t = brep::nurbs(&take_field(&mut v, "model")?, field(&v, "segments")?)?;
            encode(polygon_core::solid::brep::from_mesh(
                &t.built.mesh,
                Some(&t.face_ids),
            )?)
        }
        "brep_polygon_from_mesh" => {
            let ids: Option<Vec<usize>> =
                v.get("faceIds").map(|_| field(&v, "faceIds")).transpose()?;
            encode(polygon_core::solid::brep::from_mesh(
                &field(&v, "mesh")?,
                ids.as_deref(),
            )?)
        }
        "brep_polygon_inspect" => {
            polygon_core::solid::brep::validate(&field(&v, "model")?)?;
            encode(json!({"topologyValid":true,"solidGeometryStatus":"not_certified"}))
        }
        "brep_polygon_tessellate" => encode(brep::polygons(&field(&v, "model")?)?),
        "mesh_inspect" => encode(field::<Mesh>(&v, "mesh")?.inspect()?),
        "mesh_build_surfaces" => print_geometry::dispatch(v),
        "scene_flatten" => {
            let meshes = field::<Vec<Value>>(&v, "meshes")?
                .into_iter()
                .map(|m| {
                    Ok(polygon_core::scene_flatten::Input {
                        vertices: field(&m, "vertices")?,
                        indices: field(&m, "indices")?,
                        transform: field(&m, "transform")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            encode(polygon_core::scene_flatten::flatten(&meshes)?)
        }
        "body_bounds" => {
            let (min, max) =
                polygon_core::scene_flatten::bounds(&field::<Vec<Vec<f64>>>(&v, "positions")?)?;
            encode(json!({"min":min,"max":max}))
        }
        "cad_resize_bodies" => cad_body_affine::resize(v),
        "cad_draft_bodies" => cad_draft::draft(v),
        "cad_mirror_bodies" => cad_body_affine::mirror(v),
        "cad_prepare_retained_profile" => cad_profile_prepare::prepare_retained(v),
        "cad_prepare_profile" => cad_profile_prepare::prepare(v),
        "cad_slot" => cad_sketch::slot(v),
        "cad_dimensions" => cad_dimensions::dimensions(v),
        "cad_quantity" => cad_quantity::parse(v),
        "cad_bridge_curve" => cad_bridge_curve::bridge(v),
        "cad_trim_sketch" => cad_sketch_trim::trim(v),
        "cad_extend_sketch" => cad_sketch_trim::extend(v),
        "cad_validate_sketch" => cad_sketch_offset::validate_request(v),
        "cad_offset_sketch" => cad_sketch_offset::offset(v),
        "cad_ruled_sketch_loft" => cad_sections::ruled(v),
        "cad_build_sections" => cad_sections::build(v),
        "cad_path_points" => cad_path::sample(v),
        "cad_thread_body" => cad_thread::apply(v),
        "cad_thread_geometry" => mechanical_core::thread_geometry(&field::<Value>(&v, "options")?)
            .map_err(|e| input(e.message)),
        "cad_thread_radius" => encode(
            mechanical_core::thread_radius(
                &field::<Value>(&v, "options")?,
                field(&v, "angle")?,
                field(&v, "z")?,
            )
            .map_err(|e| input(e.message))?,
        ),
        "cad_transform_points" => cad_sketch::transform_points(v),
        "cad_world_points" => cad_sketch::world_points(v),
        "cad_sample_curve" => cad_sketch::sample(field(&v, "curve")?),
        "cad_transform_sketch" => cad_sketch::transform(v),
        "cad_inspect_pairs" => cad_clearance::inspect(v),
        "cad_hole_body" => cad_hole::hole(v),
        "cad_boolean_bodies" => cad_boolean::boolean(v),
        "cad_edge_edit" => cad_edge_edit::edit(v),
        "cad_planar_edit" => cad_planar_edit::edit(v),
        "cad_face_plane" => cad_mesh_topology::face_plane(v),
        "cad_mesh_topology" => cad_mesh_topology::topology(v),
        "cad_select_brep_edge" => cad_face_selection::edge(v),
        "cad_select_brep_support" => cad_face_selection::select(v),
        "cad_split_body" => cad_split::split(v),
        "cad_lattice_components" => cad_lattice::components(v),
        "cad_lattice_decimate" => cad_lattice::decimate(v),
        "cad_lattice_print_fit" => cad_lattice::print_fit(v),
        "cad_lattice_bridge_warning" => cad_lattice::bridge_warning(v),
        "cad_lightening_cells" => cad_lattice::lightening_cells(v),
        "cad_spatial_graph" => cad_lattice::graph(v),
        "cad_lightening" => cad_lattice::lightening(v),
        "cad_texture_height" => cad_texture::height(v),
        "cad_texture_body" => cad_texture::apply(v),
        "cad_pattern_bodies" => cad_pattern::pattern(v),
        "cad_joint_bodies" => cad_body_affine::joint(v),
        "cad_arrange_bodies" => cad_body_affine::arrange(v),
        "cad_transform_bodies" => cad_body_affine::transform(v),
        "cad_instance_transform" => cad_body_affine::instance_transform(v),
        "cad_display_mesh" => cad_display::prepare(v),
        "cad_instances" => cad_body_affine::instances(v),
        "cad_transform_selection" => cad_selection::transform(v),
        "mesh_thicken" => encode(field::<Mesh>(&v, "mesh")?.thicken(field(&v, "vector")?)?),
        "mesh_transform" => {
            let mesh = field::<Mesh>(&v, "mesh")?.transform(field(&v, "matrix")?)?;
            let report = mesh.inspect()?;
            encode(BuiltMesh { mesh, report })
        }
        "mesh_boundary_loops" => encode(field::<Mesh>(&v, "mesh")?.boundary_loops()?),
        "mesh_boundary_curves" => encode(boundary_curves(&field(&v, "mesh")?)?),
        "mesh_export_stl" => encode(field::<Mesh>(&v, "mesh")?.export_stl()?),
        "mesh_export_format" => {
            let bytes = polygon_core::mesh_export::export(
                &field(&v, "mesh")?,
                &field::<String>(&v, "format")?,
            )?;
            encode(mesh_analysis::store(
                mesh_analysis::AnalysisBuffers::Bytes { bytes },
            ))
        }
        "mesh_export_3mf_model" => {
            let bytes = polygon_core::model_3mf::export(
                &field(&v, "mesh")?,
                &field::<Vec<Mesh>>(&v, "parts")?,
            )?;
            encode(mesh_analysis::store(
                mesh_analysis::AnalysisBuffers::Bytes { bytes },
            ))
        }
        "mesh_export_3mf" => {
            let bytes = polygon_core::package_3mf::export(
                &field(&v, "mesh")?,
                &field::<Vec<Mesh>>(&v, "parts")?,
                field(&v, "compressed")?,
            )?;
            encode(mesh_analysis::store(
                mesh_analysis::AnalysisBuffers::Bytes { bytes },
            ))
        }
        _ => Ok(nurbs_core::dispatch(v)?),
    }
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
