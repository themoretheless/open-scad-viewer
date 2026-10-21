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
mod native_snapshot;
mod drawing_projection;
mod cad_body_affine;
mod cad_body_keypoints;
mod cad_boolean;
mod cad_boundary_agreement;
mod cad_boundary_regularity;
mod cad_bridge_curve;
mod cad_centered_lattice;
mod cad_clearance;
mod cad_client_geometry;
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
mod cad_minkowski3d;
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
mod cad_geometric_g2;
mod cad_runout;
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
pub use request_codec::execute;
#[cfg(test)]
mod tests;

mod cad_mesh_buffer;
pub use cad_mesh_buffer::{CadMeshBuffer, import_cad_mesh};


/// Linear-memory ABI core shared by the geometry-wasm shell.
pub mod abi;
