//! Original rational SS fibers on lifted periodic charts. No surface is
//! reparameterized or unflagged. Exact period translations select original
//! sections; their common outward operator crosses either/both support seams.
//! C0 seam identity and source-wide piecewise C1 continuity suffice for the
//! Lipschitz fixed-point proof. This is local root authority, not a census.
use super::*;
mod axes;
mod operator;
#[derive(Clone, Debug)]
pub struct QualifiedQuotientFiber {
    source: FiberSources,
    domain: [[f64; 2]; 4],
    axis: FiberAxis,
    limits: FiberLimits,
    report: FiberReport,
}
#[derive(Clone, Debug)]
pub struct QuotientFiberResult {
    report: FiberReport,
    fiber: Option<QualifiedQuotientFiber>,
}
impl QuotientFiberResult {
    pub fn report(&self) -> &FiberReport {
        &self.report
    }
    pub fn fiber(&self) -> Option<&QualifiedQuotientFiber> {
        self.fiber.as_ref()
    }
    pub fn into_fiber(self) -> Option<QualifiedQuotientFiber> {
        self.fiber
    }
}
impl FiberSources {
    pub(crate) fn verify_continuous_ss_source(&self) -> Result<()> {
        axes::validate_source(self)
    }
    /// Qualify original F=S1-S2 in lifted coordinates. Every periodic axis has
    /// an exact native seam proof; every local interval covers less than one
    /// period. A positive result never claims whole-surface completeness.
    pub fn certify_quotient(
        &self,
        domain: [[f64; 2]; 4],
        axis: FiberAxis,
        limits: FiberLimits,
    ) -> Result<QuotientFiberResult> {
        let report = operator::certify(self, domain, axis, limits)?;
        let fiber =
            (report.status == FiberStatus::UniformUniqueRoot).then(|| QualifiedQuotientFiber {
                source: self.clone(),
                domain,
                axis,
                limits,
                report: report.clone(),
            });
        Ok(QuotientFiberResult { report, fiber })
    }
}
impl QualifiedQuotientFiber {
    pub fn source(&self) -> &FiberSources {
        &self.source
    }
    pub fn domain(&self) -> [[f64; 2]; 4] {
        self.domain
    }
    pub fn free_axis(&self) -> FiberAxis {
        self.axis
    }
    pub fn report(&self) -> &FiberReport {
        &self.report
    }
    /// Exact membership, including natural high/low aliases. Rounded section
    /// hulls may be used for evaluation, never to prove census ownership.
    pub fn contains_original_box(&self, domain: [[f64; 2]; 4]) -> Result<bool> {
        axes::contains(&self.source, self.domain, domain)
    }
    pub fn reverify(&self) -> Result<()> {
        let report = operator::certify(&self.source, self.domain, self.axis, self.limits)?;
        check(
            report.status == FiberStatus::UniformUniqueRoot && report == self.report,
            "Original quotient fiber or its seam/operator proof changed",
        )
    }
    pub fn at(&self, parameter: f64, limits: FiberLimits) -> Result<QuotientFiberResult> {
        let k = self.axis.index();
        check(
            parameter.is_finite()
                && self.domain[k][0] <= parameter
                && parameter <= self.domain[k][1],
            "Quotient specialization leaves original lifted chart",
        )?;
        let mut domain = self.domain;
        domain[k] = [parameter; 2];
        self.source.certify_quotient(domain, self.axis, limits)
    }
}
fn evaluate_lifted(
    source: &FiberSources,
    support: usize,
    domain: [[f64; 2]; 4],
    jacobian: bool,
    budget: &mut Budget,
) -> Result<Option<Evaluation>> {
    evaluate_sections(source, support, domain, jacobian, None, budget)
}
fn evaluate_lifted_projected(
    source: &FiberSources,
    support: usize,
    domain: [[f64; 2]; 4],
    jacobian: bool,
    r: [[f64; 3]; 3],
    origin: &[f64],
    budget: &mut Budget,
) -> Result<Option<Evaluation>> {
    evaluate_sections(source, support, domain, jacobian, Some((r, origin)), budget)
}
fn evaluate_sections(
    source: &FiberSources,
    support: usize,
    domain: [[f64; 2]; 4],
    jacobian: bool,
    projection: Option<([[f64; 3]; 3], &[f64])>,
    budget: &mut Budget,
) -> Result<Option<Evaluation>> {
    let s = if support == 0 {
        source.first()
    } else {
        source.second()
    };
    let us = axes::sections(s, 0, domain[2 * support])?;
    let vs = axes::sections(s, 1, domain[2 * support + 1])?;
    let empty = I {
        lo: f64::INFINITY,
        hi: f64::NEG_INFINITY,
    };
    let mut values = [empty; 3];
    let mut derivatives = [[empty; 2]; 3];
    for u in &us {
        for v in &vs {
            let evaluation = if let Some((r, origin)) = projection {
                evaluate_projected(s, [*u, *v], r, origin, jacobian, budget)?
            } else {
                evaluate(s, [*u, *v], jacobian, budget)?
            };
            let Some((next, j)) = evaluation else {
                return Ok(None);
            };
            for k in 0..3 {
                values[k].lo = values[k].lo.min(next[k].lo);
                values[k].hi = values[k].hi.max(next[k].hi);
                if jacobian {
                    for a in 0..2 {
                        derivatives[k][a].lo = derivatives[k][a].lo.min(j[k][a].lo);
                        derivatives[k][a].hi = derivatives[k][a].hi.max(j[k][a].hi);
                    }
                }
            }
        }
    }
    Ok(Some((values, derivatives)))
}

mod coverage;
pub use coverage::{
    QualifiedQuotientCoverage, QuotientCoverageKind, QuotientCoverageNode, QuotientCoverageResult,
    cover_quotient_fiber_box, cover_quotient_fiber_surfaces,
};

mod discover;
pub use discover::{
    QuotientDiscoveryLimits, QuotientDiscoveryReport, QuotientDiscoveryResult,
    discover_quotient_fiber_surfaces,
};

pub(crate) fn original_axis_sections(s:&Surface,axis:usize,d:[f64;2])->Result<Vec<[f64;2]>>{axes::sections(s,axis,d)}
