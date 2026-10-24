//! Progressive profile transport with simultaneous scale and angular twist.
//!
//! Each level is a retained piecewise-linear NURBS patch set. Refinement compares
//! profile controls against fourfold finer transport. Open parameter-spaced
//! authored frames also carry original-law continuous retained-patch error
//! certificates. Other transport, global injectivity and capped-body admission
//! remain separate obligations; sampled acceptance is not their proof.
use crate::{Result, check, core::vec3_ext::norm, curve::Curve, surface::Surface};
use math_core::{cross, dot, sub};
use std::f64::consts::TAU;

type V = [f64; 3];
mod authored_error;
mod original_level_cache;
mod evidence_api;
mod section_frames;
mod arc_guide;
mod contact_anchor;
mod source_plane;
mod source_line;
mod retained_regularity;
mod retained_smoothness;
mod original_smoothness;
mod endpoint_jets;
mod rmf_transport;
pub use rmf_transport::{OriginalRmfTransportReport,OriginalRmfSectionImagesReport,RmfNormalCell,certify_original_rmf_transport,certify_original_rmf_section_images};
pub use original_smoothness::{OriginalFrameSmoothnessReport,ClosedAuthoredFrameSmoothnessReport,ClosedPathFrameSmoothnessReport};
pub use retained_smoothness::{RetainedSeamReport,RetainedSmoothnessReport,RetainedDecompositionReport,RetainedDecompositionSmoothnessReport};
mod endpoint_surface;
pub use endpoint_surface::EndpointSurfaceReport;
mod profile_domain;
pub use profile_domain::{IdealEndpointDomainsReport,IdealEndpointPlanesReport,EndpointCapProjectionReport};
pub use retained_regularity::{RetainedRegularityReport, inspect as inspect_retained_regularity};
pub use contact_anchor::{ContactAnchorReport, ContactFitReport};
pub use authored_error::{InitialCoordinatesReport, SectionInterpolationReport, PatchErrorReport};


#[cfg(feature = "codec")]
#[path = "../progressive_sweep/serialization.rs"]
mod serialization;

#[cfg(test)]
#[path = "../progressive_sweep/tests/mod.rs"]
mod tests;

mod orientation;
mod approximate;
pub use orientation::*;
pub use approximate::*;

mod source_jets;
mod rmf_first_images;

pub mod endpoint_points;
