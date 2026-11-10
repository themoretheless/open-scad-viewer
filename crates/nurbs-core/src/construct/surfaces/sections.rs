//! Section curves and section-driven repair/projection utilities.
pub use crate::foundation::sections;
pub use crate::join::section_phase as phase;
pub mod circle_repair;
pub mod circle_sweep_repair;
pub mod miter_section_correction;
pub mod section_projection;
pub mod sweep_section_projection;
