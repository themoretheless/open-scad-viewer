//! Full ORIGINAL parameter-product forest. Quotient charts cover roots through
//! exact period membership, including both seam aliases. No fitted geometry,
//! component count, physical embedding or retained-region claim is inferred.
use super::*;
use crate::ss_intersection::{
    CoverageLimits, FiberCoverageReport, FiberCoverageStatus, QualifiedMixedFiberCoverage,
};
use std::sync::Arc;
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
    nonperiodic: Option<Arc<QualifiedMixedFiberCoverage>>,
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
    /// Retain a complete original-product proof and its independently
    /// certified quotient charts. With no periodic axes the quotient
    /// identifies no parameters, so both proofs describe the same roots.
    pub fn from_nonperiodic_mixed(
        coverage: &QualifiedMixedFiberCoverage,
        fibers: Vec<QualifiedQuotientFiber>,
        limits: CoverageLimits,
    ) -> Result<Self> {
        let domain = coverage.domain();
        validate(&fibers, domain, limits)?;
        let sources = fibers[0].source();
        check(
            [sources.first(), sources.second()].into_iter().all(|s| !s.periodic_u && !s.periodic_v),
            "A periodic quotient requires its own original-product forest",
        )?;
        check(
            fibers.len() == coverage.fibers().len()
                && fibers.iter().zip(coverage.fibers()).all(|(fiber, native)| {
                    fiber.domain() == native.domain()
                        && fiber.free_axis() == native.free_axis()
                        && fiber.source().first() == native.source().first()
                        && fiber.source().second() == native.source().second()
                }),
            "Quotient charts differ from the nonperiodic coverage proof",
        )?;
        let mut report = coverage.report().clone();
        report.reason = "complete-nonperiodic-mixed-root-coverage";
        for fiber in &fibers {
            report.rectangles = report.rectangles.saturating_add(fiber.report().rectangles);
        }
        check(
            report.cells <= limits.max_cells
                && report.rectangles <= limits.max_rectangles
                && coverage.depth_limit() <= limits.max_depth,
            "Nonperiodic quotient coverage exceeds the requested work limits",
        )?;
        Ok(Self {
            fibers,
            domain,
            nodes: vec![],
            limits,
            report,
            nonperiodic: Some(Arc::new(coverage.clone())),
        })
    }
    pub fn fibers(&self) -> &[QualifiedQuotientFiber] {
        &self.fibers
    }
    /// Original quotient forest nodes. A retained nonperiodic mixed proof
    /// can certify complete coverage without materializing this forest.
    pub fn nodes(&self) -> &[QuotientCoverageNode] {
        &self.nodes
    }
    /// Closed original-coordinate enclosures of every root, paired with the
    /// index of the verified owning chart. An enclosure may contain no roots.
    pub fn root_enclosures(&self) -> Result<Vec<(usize, [[f64; 2]; 4])>> {
        if self.nonperiodic.is_some() {
            return Ok(self.fibers.iter().enumerate().map(|(i, fiber)| {
                (i, fiber.domain())
            }).collect());
        }
        let mut enclosures = Vec::new();
        for node in &self.nodes {
            let mut root = node.domain;
            let chart = match &node.kind {
                Some(QuotientCoverageKind::Chart(chart)) => *chart,
                Some(QuotientCoverageKind::RootImage { chart, axis, report }) => {
                    let image = report.root_image.ok_or_else(|| {
                        crate::input("Missing quotient census root enclosure")
                    })?;
                    let free = axis.index();
                    for (bounds, image) in root
                        .iter_mut()
                        .enumerate()
                        .filter_map(|(axis, bounds)| (axis != free).then_some(bounds))
                        .zip(image)
                    {
                        bounds[0] = bounds[0].max(image[0]);
                        bounds[1] = bounds[1].min(image[1]);
                    }
                    *chart
                }
                Some(QuotientCoverageKind::Excluded(_) | QuotientCoverageKind::Split { .. }) => continue,
                None => return Err(crate::input("Incomplete quotient census root enclosure")),
            };
            enclosures.push((chart, root));
        }
        Ok(enclosures)
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
        if let Some(native) = &self.nonperiodic {
            native.reverify()?;
            for fiber in &self.fibers {
                fiber.reverify()?;
            }
            return Ok(());
        }
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
            && (1..=1000000).contains(&limits.max_rectangles)
            && limits.max_depth <= 64,
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
