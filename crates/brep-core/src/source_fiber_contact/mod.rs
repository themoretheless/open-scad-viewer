//! Sufficient allowed-contact proof on a natural source chart boundary.
//! Original restrictions and exact shared ownership are retained unchanged.
use crate::{
    source_contour_proposal::SourceRegion,
    source_fiber_boundary::{self, Locus},
    source_plane_fiber,
    source_shared_edge::SharedEdge,
    source_shell_incidence::{Address, Shell},
};
use nurbs_core::{Error, Result};

mod certificate;
#[cfg(test)]
mod tests;
pub use certificate::*;
