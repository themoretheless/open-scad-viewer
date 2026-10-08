//! Sweep surfaces: framed/profile/progressive/scaled/twist/two-guide/helical
//! sweeps, pipe and ribbon surfaces, plus sweep audit modules.
//!
//! Groups the former flat modules `framed_sweep`, `profile_sweep`,
//! `progressive_sweep`, `progressive_miter`, `scaled_sweep`, `twist_sweep`,
//! `two_guide_sweep`, `helical_sweep`, `pipe`, `ribbon` and the six
//! `sweep_*_audit`/`sweep_cap_*` modules (now `audit::{pair, wall, contour,
//! seam, cap_boundary, cap_wall}`). Old paths keep working through re-export
//! shims in `lib.rs`.
pub mod framed_sweep;
pub mod profile_sweep;
pub mod progressive_sweep;
pub mod progressive_miter;
pub mod scaled_sweep;
pub mod twist_sweep;
pub mod two_guide_sweep;
pub mod helical_sweep;
pub mod pipe;
pub mod ribbon;
pub mod audit;

/// Conditional filled-cap and full-boundary error composition.
pub mod filled_cap_error;
pub mod profile_certificate;
mod profile_seam;
mod profile_geometry;

/// Boundary-wide composition shared by sweep constructors.
pub mod certificates {
    pub use super::filled_cap_error;
    pub use crate::{sweep_section_correction, sweep_seam_set, retained_wall_domain};
    pub use super::progressive_miter::{law_certificates, boundary_certificates};
}

mod scalar;
pub use scalar::{scaled_sweep, checked_profile_sweep, checked_profile_sweep_with_cells};
