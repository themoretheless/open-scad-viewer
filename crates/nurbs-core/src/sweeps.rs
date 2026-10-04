pub mod progressive_miter;
pub mod filled_cap_error;
pub mod progressive_sweep;

/// Boundary-wide composition shared by sweep constructors.
pub mod certificates {
    pub use super::filled_cap_error;
    pub use crate::{sweep_section_correction, sweep_seam_set, retained_wall_domain};
    pub use super::progressive_miter::{law_certificates, boundary_certificates};
}
