//! Full ORIGINAL parameter-product forest. Quotient charts cover roots through
//! exact period membership, including both seam aliases. No fitted geometry,
//! component count, physical embedding or retained-region claim is inferred.
use super::*;
use crate::ss_intersection::{CoverageLimits, FiberCoverageReport, FiberCoverageStatus};
mod discovery;
#[derive(Clone, Debug, PartialEq)]
pub enum QuotientCoverageKind {
    Chart(usize),
    RootImage {
        chart: usize,
        axis: FiberAxis,
        report: FiberReport,
    },
    Excluded(FiberReport),
    Split {
        axis: usize,
        parameter: f64,
        children: [usize; 2],
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct QuotientCoverageNode {
    domain: [[f64; 2]; 4],
    kind: Option<QuotientCoverageKind>,
}
impl QuotientCoverageNode {
    pub fn domain(&self) -> [[f64; 2]; 4] {
        self.domain
    }
    pub fn kind(&self) -> Option<&QuotientCoverageKind> {
        self.kind.as_ref()
    }
}
#[derive(Clone, Debug)]
pub struct QualifiedQuotientCoverage {
    fibers: Vec<QualifiedQuotientFiber>,
    domain: [[f64; 2]; 4],
    nodes: Vec<QuotientCoverageNode>,
    limits: CoverageLimits,
    report: FiberCoverageReport,
}
pub struct QuotientCoverageResult {
    report: FiberCoverageReport,
    coverage: Option<QualifiedQuotientCoverage>,
}
impl QuotientCoverageResult {
    pub fn report(&self) -> &FiberCoverageReport {
        &self.report
    }
    pub fn coverage(&self) -> Option<&QualifiedQuotientCoverage> {
        self.coverage.as_ref()
    }
    pub fn into_coverage(self) -> Option<QualifiedQuotientCoverage> {
        self.coverage
    }
}
impl QualifiedQuotientCoverage {
    pub fn fibers(&self) -> &[QualifiedQuotientFiber] {
        &self.fibers
    }
    pub fn nodes(&self) -> &[QuotientCoverageNode] {
        &self.nodes
    }
    pub fn report(&self) -> &FiberCoverageReport {
        &self.report
    }
    pub fn domain(&self) -> [[f64; 2]; 4] {
        self.domain
    }
    pub fn covers_entire_surfaces(&self) -> bool {
        self.domain == natural(self.fibers[0].source())
    }
    /// Reconstruct the entire deterministic original-root forest, including
    /// failed probes, with the same shared allowance. No stored verdict enters
    /// the reconstruction and no unresolved cell becomes a qualified leaf.
    pub fn reverify(&self) -> Result<()> {
        let r = cover_quotient_fiber_box(&self.fibers, self.domain, self.limits)?;
        let fresh = r
            .into_coverage()
            .ok_or_else(|| crate::input("Original quotient census is no longer complete"))?;
        check(
            self.nodes == fresh.nodes && self.report == fresh.report,
            "Original quotient root census changed",
        )
    }
}
fn natural(source: &FiberSources) -> [[f64; 2]; 4] {
    [
        axes::natural(source.first(), 0),
        axes::natural(source.first(), 1),
        axes::natural(source.second(), 0),
        axes::natural(source.second(), 1),
    ]
}
fn validate(
    fibers: &[QualifiedQuotientFiber],
    d: [[f64; 2]; 4],
    limits: CoverageLimits,
) -> Result<()> {
    check(
        (1..=256).contains(&fibers.len())
            && (1..=65536).contains(&limits.max_cells)
            && (1..=100000).contains(&limits.max_rectangles)
            && limits.max_depth <= 32,
        "Need bounded quotient census charts, cells, rectangles and depth",
    )?;
    let source = fibers[0].source();
    let n = natural(source);
    check(
        (0..4).all(|k| {
            d[k].iter().all(|v| v.is_finite())
                && n[k][0] <= d[k][0]
                && d[k][0] < d[k][1]
                && d[k][1] <= n[k][1]
        }),
        "Quotient census must be inside the complete original natural product",
    )?;
    for f in fibers {
        check(
            f.source().first() == source.first() && f.source().second() == source.second(),
            "Quotient census source carriers differ",
        )?;
    }
    Ok(())
}
pub fn cover_quotient_fiber_surfaces(
    fibers: &[QualifiedQuotientFiber],
    limits: CoverageLimits,
) -> Result<QuotientCoverageResult> {
    check(!fibers.is_empty(), "Quotient census has no original charts")?;
    cover_quotient_fiber_box(fibers, natural(fibers[0].source()), limits)
}
pub fn cover_quotient_fiber_box(
    fibers: &[QualifiedQuotientFiber],
    domain: [[f64; 2]; 4],
    limits: CoverageLimits,
) -> Result<QuotientCoverageResult> {
    validate(fibers, domain, limits)?;
    discovery::construct(fibers, domain, limits)
}
