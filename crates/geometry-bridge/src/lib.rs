//! Application adapters for the native geometry libraries.
//! Owns cross-kernel sampling and the WASM/JSON transport. Geometry algorithms
//! remain in their domain libraries; native clients can use these typed adapters.
//!
//! Language compilation lives in `languages-bridge`; this crate executes
//! typed geometry and has no language frontend dependency.
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
mod cad_body_keypoints;
mod cad_boolean;
mod cad_boundary_agreement;
mod cad_bridge_curve;
mod cad_centered_lattice;
mod cad_clearance;
mod cad_diagnostics;
mod cad_dimensions;
mod cad_display;
mod cad_draft;
mod cad_edge_edit;
mod cad_face_contacts;
mod cad_face_distance;
mod cad_face_selection;
mod cad_hole;
mod cad_lattice;
mod cad_local_mesh_tools;
mod cad_material_chord;
mod cad_material_segment;
mod cad_material_wall;
mod cad_mesh_planes;
mod cad_mesh_topology;
mod cad_miter_layout;
mod cad_miter_owned;
mod cad_path;
mod cad_pattern;
mod cad_planar_edit;
mod cad_profile_prepare;
mod cad_profile_tools;
mod cad_quantity;
mod cad_sections;
mod cad_selection;
mod cad_shell_distance;
mod cad_sketch;
mod cad_sketch_offset;
mod cad_sketch_trim;
mod cad_solid_distance;
mod cad_source_body;
mod cad_split;
mod cad_surface_diagnostics;
mod cad_sweep_smoothness;
mod cad_texture;
mod cad_thread;
mod cad_whole_wall;
mod camera_gestures;
mod frame;
pub mod intersections;
#[cfg(feature = "gpu")]
pub mod lattice_gpu;
mod mesh;
pub mod mesh_analysis;
mod mesh_display;
mod mesh_editor;
mod mesh_export_file;
mod mesh_import;
pub mod mesh_picking;
mod mesh_render;
pub mod mesh_shell;
pub mod mesh_surface_groups;
pub mod print_geometry;
mod scene_picking;
mod transparent_bsp;
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

use bridge_codec::{Deserialize, Routed, Router, Serialize, response};
pub use math_core::{Error, Result};
fn input(message: impl Into<String>) -> Error {
    Error::new("GEOMETRY_INVALID_INPUT", message)
}
mod request_codec;
use request_codec::*;
pub(crate) use request_codec::{mass_model, diagnosis_json, mesh_from_triangles, triangles_from_mesh};

mod topology_requests;
use topology_requests::*;



mod nurbs_tessellation;
pub use nurbs_tessellation::{NurbsSurfaceAdapter, tessellate_nurbs, boundary_curves};

/// Domain crates tried before the local operations, in order.
const DOMAINS: &[Router] = &[
    bridge_cam::dispatch,
    bridge_analysis::dispatch,
    bridge_svg::dispatch,
];

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
        "frame_solve" => frame::solve(v),
        "frame_envelope" => frame::envelope(v),
        "frame_diagnose" => frame::diagnose(v),
        "frame_buckling" => frame::buckling(v),
        "frame_modal" => frame::modal(v),
        "frame_collapse" => frame::collapse(v),
        "frame_influence" => frame::influence(v),
        "brep_sweep_viewport_evidence" => sweep_viewport::read(v),
        "brep_progressive_profile_body" => sweep_pipeline::profile_body(v),
        "brep_sweep_law_payload" => sweep_pipeline::law_payload(v),
        "brep_sweep_constructor" => sweep_pipeline::constructor(v),
        "brep_sweep_stream_start" => sweep_pipeline::stream_start(v),
        "brep_sweep_stream_next" => sweep_pipeline::stream_next(v),
        "brep_sweep_stream_release" => sweep_pipeline::stream_release(v),
        "brep_miter_body" => sweep_pipeline::miter(v),
        "brep_transform_certified_miter" => sweep_pipeline::transform(v),
        "brep_smooth_certified_miter" => sweep_pipeline::smooth(v, false),
        "brep_reconstruct_certified_miter" => sweep_pipeline::smooth(v, true),
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
        "sketch_solve_diagnostics" => {
            let (solution, diagnostics) = sketch_core::solve_with_diagnostics_options(
                &field(&v, "sketch")?,
                sketch_core::SolverOptions {
                    tolerance: field(&v, "tolerance")?,
                    max_iterations: 64,
                },
            )?;
            encode(json!({"solution": solution, "diagnostics": {
                "redundantEquations": diagnostics.redundant_equations,
                "degenerateConstraints": diagnostics.degenerate_constraints,
                "constraintResiduals": diagnostics.constraint_residuals,
                "inconsistent": diagnostics.inconsistent,
            }}))
        }
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
            triangles_from_mesh(&reconstruction::valid_source(&field(&v, "mesh")?, 4096)?),
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
            let section = mesh_section::MeshSectionIndex::new(&mesh.view())
                .map_err(legacy_mesh_error)?
                .section(z_mm)
                .map_err(legacy_mesh_error)?;
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
            let report = brep_core::miter_seams::inspect_charts(
                &field(&v, "model")?,
                &field::<Vec<usize>>(&v, "capFaces")?,
                field(&v, "maxCells")?,
            )?;
            let charts: Vec<_> = report.charts.iter().map(|(face,a)| json!({"face":face,"audit":{
                "certified":a.certified,"cells":a.cells,"projection":a.projection,"reason":a.reason,"globalEmbeddingCertified":false}})).collect();
            Ok(
                json!({"allChartsCertified":report.certified,"cells":report.cells,"charts":charts,
                "unresolvedFaces":report.unresolved,"globalEmbeddingCertified":false}),
            )
        }
        "brep_sweep_cap_pairs_audit" => sweep_cap_evidence::inspect_pairs(
            &field(&v, "model")?,
            &field::<Vec<usize>>(&v, "capFaces")?,
            &v["budgets"],
        ),
        "brep_sweep_cap_evidence_audit" => {
            let options = &v["boundaryOptions"];
            sweep_cap_evidence::inspect(
                &field(&v, "model")?,
                &field::<Vec<usize>>(&v, "capFaces")?,
                field(&v, "maxWalls")?,
                field(options, "tolerance")?,
                field(options, "maxProducts")?,
                field(options, "maxCells")?,
                field(options, "maxWork")?,
            )
        }
        "brep_nurbs_section_loft_source_audit" => {
            let exact = brep_core::analytic::section_loft_source_matches(
                &field(&v, "model")?,
                &field::<Vec<Vec<Vec<Curve>>>>(&v, "sections")?,
                field(&v, "closed")?,
            )?;
            Ok(json!({"geometryAndTopologyIdentical":exact,"globalEmbeddingCertified":false}))
        }
        "brep_nurbs_affine_lattice" => {
            let report = brep_core::affine_lattice::place(
                &field(&v, "model")?,
                field(&v, "matrix")?,
                field(&v, "quantum")?,
                field(&v, "maxWork")?,
            )?;
            Ok(
                json!({"model":report.model,"operatorNormUpper":report.operator_norm_upper,"arithmeticErrorUpper":report.arithmetic_error_upper,"work":report.work,"reason":report.reason}),
            )
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
        "brep_sweep_retained_cap_decomposition_audit" => {
            let r = brep_core::sweep_retained::cap_decomposition(
                &field(&v, "model")?,
                &field::<Vec<Vec<Vec<Curve>>>>(&v, "endpoints")?,
                brep_core::sweep_cap_contacts::Budgets {
                    max_walls: field(&v, "maxWalls")?,
                    max_exact_work: field(&v, "maxExactWork")?,
                    max_chart_cells: field(&v, "maxChartCells")?,
                    max_trim_pairs: field(&v, "maxTrimPairs")?,
                    max_trim_cells: field(&v, "maxTrimCells")?,
                    max_trim_domain_cells: field(&v, "maxTrimDomainCells")?,
                },
                field(&v, "maxProducts")?,
                field(&v, "maxEdges")?,
            )?;
            let regions=r.regions.map(|r|json!({"exact":r.exact,"capErrorUpper":if r.exact {Some(0)} else {None},"exactWork":r.exact_work,"faces":r.faces,"reason":r.reason}));
            Ok(
                json!({"certified":r.error_upper.is_some(),"capErrorUpper":r.error_upper,"products":r.products,"regions":regions,"reason":r.reason}),
            )
        }
        "brep_sweep_retained_caps_audit" => {
            let model = field::<brep_core::Model>(&v, "model")?;
            let endpoints = field::<[Vec<Vec<Curve>>; 2]>(&v, "endpoints")?;
            let report = brep_core::sweep_retained_caps::inspect(
                &model,
                &endpoints,
                brep_core::sweep_cap_contacts::Budgets {
                    max_walls: field(&v, "maxWalls")?,
                    max_exact_work: field(&v, "maxExactWork")?,
                    max_chart_cells: field(&v, "maxChartCells")?,
                    max_trim_pairs: field(&v, "maxTrimPairs")?,
                    max_trim_cells: field(&v, "maxTrimCells")?,
                    max_trim_domain_cells: field(&v, "maxTrimDomainCells")?,
                },
                field(&v, "maxEdges")?,
            )?;
            Ok(
                json!({"exact":report.exact,"capErrorUpper":if report.exact {Some(0.)}else{None},
                "exactWork":report.exact_work,"faces":[model.faces.len().saturating_sub(2),model.faces.len().saturating_sub(1)],
                "inspectedEdges":report.inspected_edges,"reason":report.reason,
                "continuousBound":false,"globalEmbeddingCertified":false,"solidCertified":false}),
            )
        }
        "brep_sweep_cap_contacts_audit" => {
            let report = brep_core::sweep_cap_contacts::inspect(
                &field::<brep_core::Model>(&v, "model")?,
                field(&v, "capFace")?,
                &field::<Vec<usize>>(&v, "capFaces")?,
                brep_core::sweep_cap_contacts::Budgets {
                    max_walls: field(&v, "maxWalls")?,
                    max_exact_work: field(&v, "maxExactWork")?,
                    max_chart_cells: field(&v, "maxChartCells")?,
                    max_trim_pairs: field(&v, "maxTrimPairs")?,
                    max_trim_cells: field(&v, "maxTrimCells")?,
                    max_trim_domain_cells: field(&v, "maxTrimDomainCells")?,
                },
            )?;
            Ok(
                json!({"capCertified":report.cap_certified,"planarControlHullCertified":report.planar_control_hull_certified,"allCapWallContactsCertified":report.all_cap_wall_contacts_certified,
                "separatedWalls":report.separated_walls,"allowedBoundaries":report.allowed_boundaries,
                "unresolvedWalls":report.unresolved_walls,"exactWork":report.exact_work,"reason":report.reason,
                "globalEmbeddingCertified":false}),
            )
        }
        "brep_sweep_embedding_audit" => cad_face_contacts::diagnose_sweep_embedding(v),
        "brep_nurbs_capped_loft_with_caps" => {
            let definitions: Vec<Value> = field(&v, "caps")?;
            if definitions.len() != 2 {
                return Err(input("Loft requires two authored caps"));
            }
            let caps = definitions
                .iter()
                .map(|c| {
                    Ok(brep_core::LoftCap {
                        surface: field(c, "surface")?,
                        trims: field(c, "trims")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let l: Value = field(&v, "embeddingLimits")?;
            let limits = brep_core::boundary_embedding::Limits {
                exact_work: field(&l, "exactWork")?,
                trim_pairs: field(&l, "trimPairs")?,
                trim_cells: field(&l, "trimCells")?,
                trim_domain_cells: field(&l, "trimDomainCells")?,
                spans: field(&l, "spans")?,
                contacts: brep_core::face_contacts::Limits {
                    pairs: field(&l, "facePairs")?,
                    cells: field(&l, "faceCells")?,
                    domain_cells: field(&l, "faceDomainCells")?,
                    cells_per_pair: field(&l, "faceCellsPerPair")?,
                    domain_cells_per_pair: field(&l, "faceDomainCellsPerPair")?,
                },
            };
            encode(brep_core::capped_loft_with_caps_checked(
                &field::<Vec<Vec<Curve>>>(&v, "start")?,
                &field::<Vec<Vec<Curve>>>(&v, "end")?,
                &field::<Vec<Vec<Surface>>>(&v, "sides")?,
                [&caps[0], &caps[1]],
                field(&v, "toleranceUv")?,
                limits,
            )?)
        }
        "brep_sweep_retained_decomposition_audit" => {
            cad_sweep_smoothness::diagnose_decomposition(v)
        }
        "brep_sweep_retained_correspondence_audit" => {
            cad_sweep_smoothness::diagnose_correspondence(v)
        }
        "brep_sweep_retained_charts_audit" => cad_sweep_smoothness::diagnose_charts(v),
        "brep_miter_owned_construct" => cad_miter_owned::construct(v),
        "brep_miter_owned_reconstruct" => cad_miter_owned::reconstruct(v),
        "brep_miter_owned_place" => cad_miter_owned::place(v),
        "brep_miter_affine_boundary" => cad_miter_layout::affine_boundary(v),
        "brep_miter_reconstruct_stations" => cad_miter_layout::reconstruct(v),
        "brep_miter_body_plan" => cad_miter_layout::plan(v),
        "brep_miter_section_partition" => cad_miter_layout::partition(v),
        "brep_miter_sharp_stations" => cad_miter_layout::sharp(v),
        "brep_miter_wall_preview" => cad_miter_layout::preview(v),
        "brep_miter_profile_smoothness_audit" => cad_sweep_smoothness::diagnose(v, true),
        "brep_miter_station_smoothness_audit" => cad_sweep_smoothness::diagnose(v, false),
        "brep_sweep_volume_audit" => cad_face_contacts::diagnose_sweep_volume(v),
        "brep_nurbs_natural_section_loft" => encode(brep_core::natural_section_loft(
            &field::<Vec<Vec<Vec<Curve>>>>(&v, "sections")?,
            &field::<Vec<f64>>(&v, "parameters")?,
        )?),
        "brep_nurbs_section_loft_surfaces" => encode(brep_core::analytic::section_loft_surfaces(
            &field::<Vec<Vec<Vec<Curve>>>>(&v, "sections")?,
            &field::<Vec<Vec<Surface>>>(&v, "sides")?,
            field(&v, "closed")?,
        )?),
        "brep_nurbs_smooth_station_walls" => {
            let report = brep_core::analytic::smooth_station_walls(
                &field::<Vec<Vec<Vec<Curve>>>>(&v, "sections")?,
                &field::<Vec<usize>>(&v, "sharp")?,
                field(&v, "closed")?,
                field(&v, "quantum")?,
                field(&v, "tolerance")?,
                field(&v, "maxWork")?,
            )?;
            encode(json!({"sides": report.sides,
                "wallDisplacementUpper": report.wall_displacement_upper,
                "work": report.work, "reason": report.reason}))
        }
        "brep_nurbs_capped_loft_surfaces" => encode(brep_core::capped_loft_surfaces(
            &field::<Vec<Vec<Curve>>>(&v, "start")?,
            &field::<Vec<Vec<Curve>>>(&v, "end")?,
            &field::<Vec<Vec<Surface>>>(&v, "sides")?,
        )?),
        "brep_nurbs_periodic_section_loft" => encode(brep_core::periodic_section_loft(&field::<
            Vec<Vec<Vec<Curve>>>,
        >(
            &v, "sections",
        )?)?),
        "brep_nurbs_rational_section_loft" => encode(brep_core::rational_section_loft(&field::<
            Vec<Vec<Vec<Curve>>>,
        >(
            &v, "sections",
        )?)?),
        "brep_nurbs_progressive_profile_body" => {
            use nurbs_core::progressive_sweep::{Options, Orientation, Spacing};
            let orientation = match field::<String>(&v, "orientation")?.as_str() {
                "rmf" => Orientation::RotationMinimizing,
                "fixed" | "authored" => Orientation::Fixed,
                "fixed_normal" => Orientation::FixedNormal,
                "frenet" => Orientation::Frenet,
                "corrected_frenet" => Orientation::CorrectedFrenet,
                _ => {
                    return Err(Error::new(
                        "BREP_RATIONAL_SWEEP_REFUSED",
                        "Unknown orientation",
                    ));
                }
            };
            let spacing = match field::<String>(&v, "spacing")?.as_str() {
                "parameter" => Spacing::Parameter,
                "arc_length" => Spacing::ArcLength {
                    tolerance: field(&v, "length_tolerance")?,
                    max_cells: field(&v, "length_max_cells")?,
                },
                _ => return Err(Error::new("BREP_RATIONAL_SWEEP_REFUSED", "Unknown spacing")),
            };
            let options = Options {
                normal: field(&v, "normal")?,
                orientation,
                spacing,
                initial_sections: field(&v, "initial_sections")?,
                max_sections: field(&v, "max_sections")?,
                max_deviation: field(&v, "max_deviation")?,
            };
            let loops = field::<Vec<Vec<Curve>>>(&v, "loops")?;
            let path = field::<Curve>(&v, "path")?;
            let scale = field::<Curve>(&v, "scale")?;
            let twist = field::<Curve>(&v, "twist")?;
            let axes = optional_field::<Curve>(&v, "axis_scale")?;
            let center = optional_field::<Curve>(&v, "center_law")?;
            let guide = optional_field::<Curve>(&v, "orientation_guide")?;
            let contact = optional_field::<f64>(&v, "contact_parameter")?;
            let contact_profile = optional_field::<usize>(&v, "contact_profile")?;
            if contact.is_some() && guide.is_none()
                || contact_profile.is_some() && contact.is_none()
            {
                return Err(Error::new(
                    "BREP_RATIONAL_SWEEP_REFUSED",
                    "Contact anchor requires orientation guide and parameter",
                ));
            }
            let authored = field::<String>(&v, "orientation")? == "authored";
            if authored && guide.is_some() {
                return Err(Error::new(
                    "BREP_RATIONAL_SWEEP_REFUSED",
                    "Orientation guide cannot be combined with authored frames",
                ));
            }
            let frame_axis = if authored {
                Some(field::<Curve>(&v, "frame_axis")?)
            } else {
                None
            };
            let frame_normal = if authored {
                Some(field::<Curve>(&v, "frame_normal")?)
            } else {
                None
            };
            let default_axes = nurbs_core::progressive_sweep::constant_vector_law([1.; 3])?;
            let default_center = nurbs_core::progressive_sweep::constant_vector_law([0.; 3])?;
            let affine = if authored || guide.is_some() || axes.is_some() || center.is_some() {
                Some((
                    axes.as_ref().unwrap_or(&default_axes),
                    center.as_ref().unwrap_or(&default_center),
                ))
            } else {
                None
            };
            let options = if authored {
                Options {
                    orientation: Orientation::Fixed,
                    ..options
                }
            } else {
                options
            };
            let tolerance = optional_field::<f64>(&v, "cap_correction_tolerance")?;
            let quantum = optional_field::<f64>(&v, "cap_correction_quantum")?;
            let max_work = optional_field::<u64>(&v, "cap_correction_max_work")?;
            if tolerance.is_none() && (quantum.is_some() || max_work.is_some()) {
                return Err(Error::new(
                    "BREP_RATIONAL_SWEEP_REFUSED",
                    "Cap correction requires a displacement tolerance",
                ));
            }
            let correction =
                tolerance.map(|tolerance| brep_core::analytic::EndpointCapCorrection {
                    tolerance,
                    quantum: quantum.unwrap_or(2_f64.powi(-40)),
                    max_work: max_work.unwrap_or(1000000),
                });
            let rmf_steps = optional_field::<usize>(&v, "rmf_transport_steps")?;
            let rmf_cells = optional_field::<usize>(&v, "error_max_cells")?;
            let rmf_products = optional_field::<usize>(&v, "error_max_products")?;
            let rmf_policy = match (rmf_steps, rmf_cells, rmf_products) {
                (None, None, None) => None,
                (Some(steps), Some(cells), Some(products))
                    if field::<String>(&v, "orientation")? == "rmf" =>
                {
                    Some((steps, cells, products))
                }
                _ => {
                    return Err(Error::new(
                        "BREP_RATIONAL_SWEEP_REFUSED",
                        "Spatial RMF policy requires unguided RMF and three proof budgets",
                    ));
                }
            };
            let result = brep_core::analytic::progressive_profile_body_with_rmf_policy(
                &loops,
                &path,
                &scale,
                &twist,
                affine,
                frame_axis.as_ref().zip(frame_normal.as_ref()),
                guide
                    .as_ref()
                    .map(|g| (g, contact.map(|p| (contact_profile.unwrap_or(0), p)))),
                options,
                correction,
                rmf_policy,
            )?;
            let retained_caps=result.retained_caps.as_ref().map(|r|json!({"exact":r.exact,
                "capErrorUpper":if r.exact {Some(0.)}else{None},"exactWork":r.exact_work,"inspectedEdges":r.inspected_edges,"reason":r.reason,
                "scope":"constructor-owned-retained-endpoint-regions","continuousBound":false,"globalEmbeddingCertified":false}));
            let cap_projection = result.cap_projection.as_ref().map(|r| {
                json!({
                "idealCapDomainsCertified":r.original.domains.ideal_endpoint_domains_certified,
                "normalDots":r.normal_dots,"reversesOrientation":r.reverses_orientation,
                "cells":r.cells,"exactWork":r.exact_work,"reason":r.reason,
                "scope":"constructor-owned-endpoint-plane-projection"})
            });
            let mut output = encode(
                json!({"model":result.model,"approximation":result.approximation,"retainedCaps":retained_caps,
                "capProjection":cap_projection,"filledCapErrorUpper":result.filled_cap_error_upper,
                "capCorrectionErrorUpper":result.cap_correction_error_upper,
                "bodyDecompositionErrorUpper":result.body_decomposition_error_upper,
                "bodyDecompositionProducts":result.body_decomposition_products,
                "retainedWalls":{"certified":result.retained_walls.certified,"faceCoverageCertified":result.retained_walls.face_coverage_certified,
                    "coefficientFamilyCertified":result.retained_walls.coefficient_family_certified,"inspectedFaces":result.retained_walls.inspected_faces,
                    "exactWork":result.retained_walls.exact_work,"reason":result.retained_walls.reason},
                "boundaryErrorUpper":result.boundary_error_upper,"boundaryContinuousBound":result.boundary_error_upper.is_some(),
                "boundaryErrorWithinBudget":result.boundary_error_within_budget,
                "boundaryErrorScope":"constructor-owned-retained-wall-and-cap-union","globalEmbeddingCertified":false}),
            )?;
            if let Some((_, cells, products)) = rmf_policy {
                output["approximation"]["report"]["errorCertificateMaxCells"] = json!(cells);
                output["approximation"]["report"]["decompositionMaxProducts"] = json!(products);
                if let Some(levels) = output["approximation"]["levels"].as_array_mut() {
                    for level in levels {
                        level["errorCertificateMaxCells"] = json!(cells);
                        level["decompositionMaxProducts"] = json!(products);
                    }
                }
            }
            Ok(output)
        }
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
        "brep_nurbs_constant_fillet_family" => encode(
            brep_core::feature_family::constant_fillet_family(
                &field(&v, "model")?,
                &field::<Vec<usize>>(&v, "edges")?,
            )?
            .name(),
        ),
        "brep_nurbs_exact_convex_prism_fillet"
        | "brep_nurbs_exact_simple_prism_fillet"
        | "brep_nurbs_exact_annular_fillet"
        | "brep_nurbs_exact_layered_prism_fillet" => {
            require_exact_fields(
                &v,
                &["op", "model", "edges", "radius"],
                "exact fillet request",
            )?;
            let author = match v["op"].as_str() {
                Some("brep_nurbs_exact_simple_prism_fillet") => {
                    brep_core::exact_simple_prism_fillet
                }
                Some("brep_nurbs_exact_annular_fillet") => brep_core::exact_annular_fillet,
                Some("brep_nurbs_exact_layered_prism_fillet") => {
                    brep_core::exact_layered_prism_fillet
                }
                _ => brep_core::exact_convex_prism_fillet,
            };
            encode(author(
                &field(&v, "model")?,
                &field::<Vec<usize>>(&v, "edges")?,
                field(&v, "radius")?,
            )?)
        }
        "brep_nurbs_partial_annular_preview" => {
            require_exact_fields(
                &v,
                &["op", "model", "edge", "radius"],
                "partial annular preview request",
            )?;
            let model = brep_core::analytic_features::build_partial_annular_preview(
                &field(&v, "model")?,
                field(&v, "edge")?,
                field(&v, "radius")?,
            )?;
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
        "stl_decode_binary" | "mesh_soup_render" | "mesh_import" | "mesh_import_finalize" => {
            mesh_import::dispatch(v)
        }
        "cad_sampled_corner" | "cad_profile_revolve" => cad_profile_tools::dispatch(v),
        "cad_sampled_shell" => cad_local_mesh_tools::shell(v),
        "cad_local_mesh_blend" => cad_local_mesh_tools::blend(v),
        "mesh_display_inspect" | "mesh_hit_point" | "mesh_hit_normal" | "mesh_measure" => {
            mesh_display::dispatch(v)
        }
        "truss_force_markers" => encode(geometry_ops::force_markers::build(
            &field::<Vec<[f64; 3]>>(&v, "nodes")?,
            &field::<Vec<[usize; 2]>>(&v, "members")?,
            &field::<Vec<f64>>(&v, "forces")?,
            field(&v, "marker")?,
        )?),
        "mesh_editor" => mesh_editor::dispatch(v),
        "affine_matrix" => {
            let operation: String = field(&v, "operation")?;
            let vector = field(&v, "vector")?;
            let matrix = match operation.as_str() {
                "translate" => Some(math_core::affine::translation(vector)),
                "scale" => Some(math_core::affine::scaling(vector)),
                "rotate" => Some(math_core::affine::euler_degrees(vector)),
                "mirror" => math_core::affine::reflection(vector),
                _ => return Err(input("Unknown affine operation")),
            };
            encode(matrix)
        }
        "planar_rectangle_corners" => encode(planar_geometry::primitives::rectangle_corners(
            field(&v, "size")?,
            field(&v, "center")?,
        )?),
        "nurbs_circle_quadrants" => encode(nurbs_core::primitives::circle_quadrants(field(
            &v, "radius",
        )?)?),
        "mesh_world_area" => {
            let state: [f64; 2] = field(&v, "state")?;
            let mut area = mesh_topology::measure::SurfaceArea {
                sum: state[0],
                correction: state[1],
            };
            area.add_triangles(&field::<Vec<f64>>(&v, "points")?, field(&v, "matrix")?)?;
            encode([area.sum, area.correction])
        }
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
        "solid_program_execute_report" => {
            let program: geometry_ops::solid_program::Program = field(&v, "program")?;
            let report = polygon_core::solid::program::execute_report(&program, 200_000)?;
            let mut warnings = report
                .slice_reductions
                .iter()
                .map(|reduction| {
                    format!(
                        "linear_extrude slices were clamped to the engine limit {}",
                        reduction.maximum
                    )
                })
                .collect::<Vec<_>>();
            warnings.extend(report.profile_diagnostics.iter().map(|diagnostic| {
                match diagnostic.issue {
                    polygon_core::solid::profile_program::Issue::EmptyContours => {
                        "polygon() outlines produced an empty cross-section".to_string()
                    }
                    polygon_core::solid::profile_program::Issue::InvalidContours => {
                        "polygon() outlines did not produce a valid cross-section".to_string()
                    }
                }
            }));
            warnings.extend(
                report
                    .mesh_failures
                    .iter()
                    .map(|_| "polyhedron() topology did not produce a manifold solid".to_string()),
            );
            warnings.extend(
                report
                    .resize_diagnostics
                    .iter()
                    .map(|event| event.message()),
            );
            warnings.extend(
                report
                    .sweep_warnings
                    .iter()
                    .map(|event| event.warning.message(event.maximum)),
            );
            let reduced = !report.slice_reductions.is_empty()
                || report.sweep_warnings.iter().any(|event| {
                    matches!(
                        event.warning.kind,
                        geometry_ops::fragment_resolution::WarningKind::Clamped
                    )
                });
            Ok(json!({"meshes":report.meshes,"warnings":warnings,"reduced":reduced}))
        }
        "solid_program_execute" => {
            let program: geometry_ops::solid_program::Program = field(&v, "program")?;
            encode(polygon_core::solid::program::execute(&program, 200_000)?)
        }
        "transparent_bsp_build" => transparent_bsp::build(v),
        "transparent_triangle_split" => transparent_bsp::split(v),
        "cad_body_keypoints" => cad_body_keypoints::geometry(v),
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
pub use request_codec::execute;
#[cfg(test)]
mod tests;

mod cad_mesh_buffer;
pub use cad_mesh_buffer::{CadMeshBuffer, import_cad_mesh};


/// Linear-memory ABI core shared by the geometry-wasm shell.
pub mod abi;
