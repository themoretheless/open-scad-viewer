//! Progressive profile transport with simultaneous scale and angular twist.
//!
//! Each level is a retained piecewise-linear NURBS patch set. Refinement compares
//! profile controls against fourfold finer transport; this is sampled evidence,
//! not a continuous RMF, injectivity, or rounding-inclusive certificate.
use crate::{Result, check, sweep_support::vec3_ext::norm, curve::Curve, surface::Surface};
use math_core::{cross, dot, sub};
use std::f64::consts::TAU;

type V = [f64; 3];


#[cfg(feature = "codec")]
mod serialization;

#[cfg(test)]
#[path="tests/mod.rs"]
mod tests;

mod orientation;
mod sweep;
mod approximate;
pub use orientation::*;
pub use approximate::*;

mod authored_error;
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
mod profile_domain;
pub use profile_domain::{IdealEndpointDomainsReport,IdealEndpointPlanesReport,EndpointCapProjectionReport};
pub use retained_regularity::{RetainedRegularityReport, inspect as inspect_retained_regularity};
pub use contact_anchor::{ContactAnchorReport, ContactFitReport};
pub use authored_error::{InitialCoordinatesReport, SectionInterpolationReport, PatchErrorReport};
