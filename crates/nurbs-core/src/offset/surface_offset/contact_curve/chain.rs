//! Literal C0 paired-UV/world chains with original-root chart-switch proofs.
use crate::{Result, check, curve::Curve, offset_contact_pcurve as uv, surface::Surface};
mod joining;
mod piece;
pub mod region;
pub use joining::Join;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    pub owner: usize,
    pub axis: usize,
    pub drive: [f64; 2],
    pub free: [f64; 2],
    pub other: [[f64; 2]; 2],
    /// Traversal only. Each stored curve retains its increasing original clock.
    pub reversed: bool,
}
#[derive(Clone, Copy)]
pub struct Limits {
    /// Independent allowances for EACH piece, not shared across pieces.
    pub uv_fit: uv::FitLimits,
    pub world_fit: uv::FitLimits,
    /// Independent final allowance for each of two UV curves, the world curve
    /// and each of two direct original normal-offset lift checks.
    pub final_cells_per_curve: usize,
    pub spans: usize,
    pub refinements: usize,
    pub max_pieces: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            uv_fit: uv::FitLimits::default(),
            world_fit: uv::FitLimits::default(),
            final_cells_per_curve: 100000,
            spans: 128,
            refinements: 8,
            max_pieces: 32,
        }
    }
}
pub struct Piece {
    spec: Span,
    curves: [Curve; 2],
    world: Curve,
    uv_fit: uv::FitReport,
    world_fit: super::FitReport,
    uv_proofs: Vec<uv::Report>,
    world_proof: Option<super::Report>,
    lift_proofs: Vec<super::lift::Report>,
}
impl Piece {
    pub fn span(&self) -> Span {
        self.spec
    }
    /// Always original source order, independent of the driving owner.
    pub fn source_uv(&self) -> &[Curve; 2] {
        &self.curves
    }
    pub fn world(&self) -> &Curve {
        &self.world
    }
    pub fn uv_fitting(&self) -> &uv::FitReport {
        &self.uv_fit
    }
    pub fn world_fitting(&self) -> &super::FitReport {
        &self.world_fit
    }
    pub fn uv_proofs(&self) -> &[uv::Report] {
        &self.uv_proofs
    }
    pub fn world_proof(&self) -> &super::Report {
        self.world_proof
            .as_ref()
            .expect("published pieces have final authority")
    }
    pub fn lift_proofs(&self) -> &[super::lift::Report] {
        &self.lift_proofs
    }
    pub(super) fn endpoint(&self, start: bool) -> usize {
        if start ^ self.spec.reversed {
            0
        } else {
            self.world.control_points.len() - 1
        }
    }
    pub(super) fn bind(&mut self, start: bool, params: [[f64; 2]; 2], point: [f64; 3]) {
        let end = self.endpoint(start);
        self.world.control_points[end] = point.to_vec();
        self.world.weights[end] = 1.;
        for s in 0..2 {
            let end = if start ^ self.spec.reversed {
                0
            } else {
                self.curves[s].control_points.len() - 1
            };
            self.curves[s].control_points[end] = params[s].to_vec();
            self.curves[s].weights[end] = 1.;
        }
    }
}
pub struct Chain {
    sources: [Surface; 2],
    distances: [f64; 2],
    spans: Vec<Span>,
    tolerance_uv: f64,
    tolerance_mm: f64,
    limits: Limits,
    pieces: Vec<Piece>,
    joins: Vec<Join>,
}
impl Chain {
    pub fn close_to_boundary(
        &self,
        side: usize,
        far_side: usize,
        limits: region::Limits,
    ) -> Result<region::Region> {
        region::to_boundary(self, side, far_side, limits)
    }
    pub fn close_with(
        &self,
        side: usize,
        closure: &[Curve],
        limits: region::Limits,
    ) -> Result<region::Region> {
        region::with_closure(self, side, closure, limits)
    }
    pub fn pieces(&self) -> &[Piece] {
        &self.pieces
    }
    pub fn joins(&self) -> &[Join] {
        &self.joins
    }
    pub fn reverify(&self) -> Result<()> {
        let r = build(
            self.sources.each_ref(),
            self.distances,
            &self.spans,
            self.tolerance_uv,
            self.tolerance_mm,
            self.limits,
        )?;
        check(
            r.pieces.len() == self.pieces.len()
                && r.joins.len() == self.joins.len()
                && r.pieces
                    .iter()
                    .zip(&self.pieces)
                    .all(|(a, b)| a.spec == b.spec && a.curves == b.curves && a.world == b.world),
            "Contact chain differs from full original reconstruction",
        )
    }
}
pub(super) fn ordered<'a>(
    s: [&'a Surface; 2],
    d: [f64; 2],
    span: Span,
) -> ([&'a Surface; 2], [f64; 2]) {
    (
        [s[span.owner], s[1 - span.owner]],
        [d[span.owner], d[1 - span.owner]],
    )
}
pub(super) fn endpoint_clock(s: Span, start: bool) -> f64 {
    s.drive[usize::from(!(start ^ s.reversed))]
}
fn validate_fit(f: uv::FitLimits) -> bool {
    (1..=16).contains(&f.max_degree)
        && (1..=10000000).contains(&f.root_queries)
        && (1..=10000000).contains(&f.coefficient_work)
        && (1..=100000).contains(&f.proof_cells)
        && (1..=100000).contains(&f.spans)
        && f.root_refinements <= 16
}
/// Build actual paired original-UV and world curve nets. Adjacent endpoints are
/// bound to one literal UV pair/world point, including changes of source owner,
/// axis, traversal direction and nonunit clocks. Every modified COMPLETE curve
/// is requalified after binding. A separately proved transition branch covers
/// any small original-root gap at a chart switch within both error tolerances.
///
/// This is an open C0 contact chain. It is not a simple Jordan region, an exact
/// ideal-offset curve, G1/G2 at joins, complete natural root coverage, trimmed
/// material selection, world embedding or BRep sewing authority.
pub fn build(
    sources: [&Surface; 2],
    distances: [f64; 2],
    spans: &[Span],
    tolerance_uv: f64,
    tolerance_mm: f64,
    limits: Limits,
) -> Result<Chain> {
    check(
        (1..=32).contains(&limits.max_pieces)
            && !spans.is_empty()
            && spans.len() <= limits.max_pieces
            && (1..=100000).contains(&limits.final_cells_per_curve)
            && (1..=100000).contains(&limits.spans)
            && limits.refinements <= 16
            && validate_fit(limits.uv_fit)
            && validate_fit(limits.world_fit)
            && tolerance_uv.is_finite()
            && tolerance_uv * 0.25 > 0.
            && tolerance_mm.is_finite()
            && tolerance_mm * 0.25 > 0.,
        "Choose positive chain tolerances and bounded per-piece work",
    )?;
    for span in spans {
        check(
            span.owner < 2 && span.axis < 2,
            "Invalid original contact clock owner",
        )?;
    }
    let mut pieces = Vec::with_capacity(spans.len());
    for &span in spans {
        pieces.push(piece::fit(
            sources,
            distances,
            span,
            tolerance_uv * 0.25,
            tolerance_mm * 0.25,
            limits,
        )?);
    }
    let mut joins = vec![];
    for i in 0..pieces.len() - 1 {
        let join = joining::prove(
            sources,
            distances,
            spans[i],
            spans[i + 1],
            tolerance_uv,
            tolerance_mm,
            limits,
        )?;
        pieces[i].bind(false, join.parameters, join.point);
        pieces[i + 1].bind(true, join.parameters, join.point);
        joins.push(join);
    }
    for piece in &mut pieces {
        piece::qualify(
            sources,
            distances,
            piece,
            tolerance_uv,
            tolerance_mm,
            limits,
        )?;
    }
    Ok(Chain {
        sources: sources.map(Clone::clone),
        distances,
        spans: spans.to_vec(),
        tolerance_uv,
        tolerance_mm,
        limits,
        pieces,
        joins,
    })
}
