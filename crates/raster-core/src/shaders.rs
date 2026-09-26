//! WGSL sources, embedded at compile time. These files are the single source
//! of truth: the browser renderer consumes generated copies, native wgpu
//! consumes these strings directly.

pub const MESH_WGSL: &str = include_str!("../shaders/mesh.wgsl");
pub const MESH_PBR_WGSL: &str = include_str!("../shaders/mesh_pbr.wgsl");
pub const MESH_MATCAP_WGSL: &str = include_str!("../shaders/mesh_matcap.wgsl");
pub const MESH_TOON_WGSL: &str = include_str!("../shaders/mesh_toon.wgsl");
pub const MESH_UNLIT_WGSL: &str = include_str!("../shaders/mesh_unlit.wgsl");
pub const DEEP_MESH_WGSL: &str = include_str!("../shaders/deep_mesh.wgsl");
pub const EDGE_WGSL: &str = include_str!("../shaders/edge.wgsl");
pub const LINE_WGSL: &str = include_str!("../shaders/line.wgsl");
pub const GRID_WGSL: &str = include_str!("../shaders/grid.wgsl");
pub const SELECTION_OVERLAY_WGSL: &str = include_str!("../shaders/selection_overlay.wgsl");

/// Shaders that declare the shared `Obj` uniform and morph blend. The first
/// element is a stable name used in diagnostics and tests.
pub const OBJECT_SHADERS: [(&str, &str); 7] = [
    ("mesh", MESH_WGSL),
    ("mesh_pbr", MESH_PBR_WGSL),
    ("mesh_matcap", MESH_MATCAP_WGSL),
    ("mesh_toon", MESH_TOON_WGSL),
    ("mesh_unlit", MESH_UNLIT_WGSL),
    ("deep_mesh", DEEP_MESH_WGSL),
    ("edge", EDGE_WGSL),
];

/// Vertex input locations shared by the object pipelines: slot 0 is the
/// interleaved position+normal mesh vertex (24 bytes), slot 1 is the morph
/// source positions (12 bytes, zeros while a mesh is not morphing).
pub const MESH_VERTEX_STRIDE: u64 = 24;
pub const MORPH_VERTEX_STRIDE: u64 = 12;

/// Per-instance storage record layout (used by the instanced pipelines):
/// the same 56 floats as [`crate::uniform::ObjectUniform`].
pub const INSTANCE_RECORD_FLOATS: usize = 56;
pub const INSTANCE_RECORD_BYTES: u64 = 224;
