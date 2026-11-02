//! Composition of independently recomputed original-source pole facts.
use super::*;
use crate::{
    surface_join::Boundary,
    surface_quotient_injectivity::{PolarCollar, certify_polar_collar},
};

/// A local source quotient homeomorphism with a uniform tangent-plane limit.
/// This token does not assert a complete incident fan is a regular disk, does
/// not exclude a different face contacting this collar, and does not imply G2.
pub struct TangentPoleCollar<'a> {
    plane: PlaneLimit<'a>,
    quotient: PolarCollar<'a>,
}
impl TangentPoleCollar<'_> {
    pub fn plane(&self) -> &PlaneLimit<'_> {
        &self.plane
    }
    pub fn quotient(&self) -> &PolarCollar<'_> {
        &self.quotient
    }
}
pub struct TangentPoleCollarReport<'a> {
    pub certificate: Option<TangentPoleCollar<'a>>,
    pub exact_work: u64,
    pub cells: usize,
    pub reason: &'static str,
}
/// Neither caller-authored reports nor successful sampled probes can admit a
/// pole. Both private tokens are freshly derived from this same original
/// surface and boundary, using separate explicit exact-work and cell limits.
pub fn prepare_tangent_pole_collar(
    s: &Surface,
    boundary: Boundary,
    inward_extent: f64,
    subdivisions: usize,
    max_work: u64,
    max_cells: usize,
) -> Result<TangentPoleCollarReport<'_>> {
    let plane = prepare_plane_limit(s, boundary, max_work)?;
    let mut out = TangentPoleCollarReport {
        certificate: None,
        exact_work: plane.work,
        cells: 0,
        reason: plane.reason,
    };
    let Some(plane) = plane.certificate else {
        return Ok(out);
    };
    let quotient = certify_polar_collar(s, boundary, inward_extent, subdivisions, max_cells)?;
    out.cells = quotient.cells;
    out.reason = quotient.reason;
    if let Some(quotient) = quotient.certificate {
        out.certificate = Some(TangentPoleCollar { plane, quotient });
        out.reason = "original-pole-collar-homeomorphism-and-uniform-plane";
    }
    Ok(out)
}
