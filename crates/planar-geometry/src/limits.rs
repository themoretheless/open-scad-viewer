//! Resource policy shared by native algorithms and transport admission checks.
//! These are distinct budgets: a source path, an arrangement and an output
//! mesh consume different resources. Changing one does not relax the others.
pub const PATH_SEGMENTS: usize = 65_536;
pub const FLATTEN_POINTS: usize = PATH_SEGMENTS + 1;
pub const MESH_VERTICES: usize = 65_536;
pub const MESH_TRIANGLES: usize = 131_072;
pub const ARRANGEMENT_EDGES: usize = 65_536;
pub const ARRANGEMENT_SPLIT_POINTS: usize = 262_144;
pub const ARRANGEMENT_ATOMS: usize = 131_072;
pub const ARRANGEMENT_PAIR_CANDIDATES: usize = 8_000_000;
