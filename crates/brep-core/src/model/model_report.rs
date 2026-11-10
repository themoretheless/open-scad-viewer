//! Structural validation report shared by native topology clients.
#[derive(Debug)]
pub struct Report {
    pub vertex_count: usize,
    pub edge_count: usize,
    pub loop_count: usize,
    pub face_count: usize,
    pub shell_count: usize,
    pub body_count: usize,
    pub boundary_edge_count: usize,
    pub topology_valid: bool,
    pub geometry_agreement: &'static str,
    pub solid_geometry_status: &'static str,
}
