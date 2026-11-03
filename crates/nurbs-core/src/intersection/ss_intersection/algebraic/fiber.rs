//! Uniform roots of source-equivalent deflated equations. Original rank may
//! be two at a multiple/tangent contact; exact zero-set equivalence, not a
//! proxy surface or a sampled derivative, binds this operator to original SS.
use super::*;
use crate::{
    distance_bounds::Interval as I,
    ss_intersection::{FiberAxis, FiberStatus},
};
mod interval;
mod operator;
#[derive(Clone, Copy, Debug)]
pub struct DeflatedSsFiberLimits {
    pub max_interval_operations: usize,
    pub projection: ExactSsAlgebraLimits,
    pub preconditioner: Option<[[f64; 3]; 3]>,
}
impl Default for DeflatedSsFiberLimits {
    fn default() -> Self {
        Self {
            max_interval_operations: 100000,
            projection: Default::default(),
            preconditioner: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct DeflatedSsFiberReport {
    pub status: FiberStatus,
    pub reason: &'static str,
    pub interval_operations: usize,
    pub projection_work: usize,
    pub root_image: Option<[[f64; 2]; 3]>,
    pub contraction_upper: Option<f64>,
    pub preconditioner: Option<[[f64; 3]; 3]>,
    pub scales: Option<[f64; 3]>,
}
#[derive(Clone, Debug)]
pub struct QualifiedDeflatedSsFiber {
    source: QualifiedSsDeflatedSystem,
    domain: [[f64; 2]; 4],
    axis: FiberAxis,
    limits: DeflatedSsFiberLimits,
    report: DeflatedSsFiberReport,
}
pub struct DeflatedSsFiberResult {
    report: DeflatedSsFiberReport,
    fiber: Option<QualifiedDeflatedSsFiber>,
}
impl DeflatedSsFiberResult {
    pub fn report(&self) -> &DeflatedSsFiberReport {
        &self.report
    }
    pub fn into_fiber(self) -> Option<QualifiedDeflatedSsFiber> {
        self.fiber
    }
    pub fn fiber(&self) -> Option<&QualifiedDeflatedSsFiber> {
        self.fiber.as_ref()
    }
}
impl QualifiedDeflatedSsFiber {
    pub(crate) fn certify_restriction(
        &self,
        domain: [[f64; 2]; 4],
        limits: DeflatedSsFiberLimits,
    ) -> Result<DeflatedSsFiberReport> {
        check(
            (0..4).all(|k| self.domain[k][0] <= domain[k][0] && domain[k][1] <= self.domain[k][1]),
            "Deflated query leaves its owned root chart",
        )?;
        operator::certify(self.source.equations(), domain, self.axis, limits)
    }

    pub fn source(&self) -> &QualifiedSsDeflatedSystem {
        &self.source
    }
    pub fn domain(&self) -> [[f64; 2]; 4] {
        self.domain
    }
    pub fn free_axis(&self) -> FiberAxis {
        self.axis
    }
    pub fn report(&self) -> &DeflatedSsFiberReport {
        &self.report
    }
    pub fn reverify(&self) -> Result<()> {
        self.source.reverify()?;
        let fresh =
            operator::certify(self.source.equations(), self.domain, self.axis, self.limits)?;
        check(
            fresh.status == FiberStatus::UniformUniqueRoot && fresh == self.report,
            "Original source-equivalent tangent SS fiber changed",
        )
    }
    pub fn at(
        &self,
        parameter: f64,
        limits: DeflatedSsFiberLimits,
    ) -> Result<DeflatedSsFiberResult> {
        let k = self.axis.index();
        check(
            parameter.is_finite()
                && self.domain[k][0] <= parameter
                && parameter <= self.domain[k][1],
            "Deflated SS specialization leaves its original span clock",
        )?;
        let mut d = self.domain;
        d[k] = [parameter; 2];
        qualify_deflated_ss_fiber(&self.source, d, self.axis, limits)
    }
}
/// Normalized clocks belong to the original four knot spans. A positive result
/// owns one original parameter-root graph, not a full census or a world curve.
pub fn qualify_deflated_ss_fiber(
    source: &QualifiedSsDeflatedSystem,
    domain: [[f64; 2]; 4],
    axis: FiberAxis,
    limits: DeflatedSsFiberLimits,
) -> Result<DeflatedSsFiberResult> {
    let report = operator::certify(source.equations(), domain, axis, limits)?;
    let fiber =
        (report.status == FiberStatus::UniformUniqueRoot).then(|| QualifiedDeflatedSsFiber {
            source: source.clone(),
            domain,
            axis,
            limits,
            report: report.clone(),
        });
    Ok(DeflatedSsFiberResult { report, fiber })
}
struct IntervalWork {
    used: usize,
    max: usize,
}
impl IntervalWork {
    fn charge(&mut self) -> Result<()> {
        if self.used == self.max {
            return Err(crate::resource(
                "Source-equivalent SS interval work exhausted",
            ));
        }
        self.used += 1;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct QualifiedPoleSsRootArc {
    source: QualifiedSsPoleSaturation,
    domain: [[f64; 2]; 4],
    axis: FiberAxis,
    limits: DeflatedSsFiberLimits,
    report: DeflatedSsFiberReport,
}
impl QualifiedPoleSsRootArc {
    pub(crate) fn certify_restriction(
        &self,
        domain: [[f64; 2]; 4],
        limits: DeflatedSsFiberLimits,
    ) -> Result<DeflatedSsFiberReport> {
        check(
            (0..4).all(|k| self.domain[k][0] <= domain[k][0] && domain[k][1] <= self.domain[k][1]),
            "Pole query leaves its owned root chart",
        )?;
        operator::certify(self.source.equations(), domain, self.axis, limits)
    }

    pub fn source(&self) -> &QualifiedSsPoleSaturation {
        &self.source
    }
    pub fn domain(&self) -> [[f64; 2]; 4] {
        self.domain
    }
    pub fn report(&self) -> &DeflatedSsFiberReport {
        &self.report
    }
    pub fn free_axis(&self) -> FiberAxis {
        self.axis
    }
    pub fn pole_world_point(&self) -> &[Q; 3] {
        self.source.pole().point()
    }
    pub fn reverify(&self) -> Result<()> {
        self.source.reverify()?;
        let fresh = qualify_pole_ss_root_arc(&self.source, self.domain, self.limits)?;
        check(
            fresh.report == self.report,
            "Original pole root arc or endpoint presence changed",
        )
    }
}
/// Uniform original off-pole roots and a genuine original SS endpoint at the
/// collapsed shore. The radial source clock is free; exceptional angular
/// aliases are identified by the proved constant source world-point stratum.
/// No full branch census, speed, orientation or trim topology is inferred.
pub fn qualify_pole_ss_root_arc(
    source: &QualifiedSsPoleSaturation,
    domain: [[f64; 2]; 4],
    limits: DeflatedSsFiberLimits,
) -> Result<QualifiedPoleSsRootArc> {
    let k = 2 * source.pole().support() + source.pole().axis();
    let shore = source.pole().shore();
    check(
        domain[k][0] < domain[k][1] && domain[k][shore] == shore as f64,
        "Pole SS root arc must reach the proved original radial shore",
    )?;
    let axis = [
        FiberAxis::FirstU,
        FiberAxis::FirstV,
        FiberAxis::SecondU,
        FiberAxis::SecondV,
    ][k];
    let report = operator::certify(source.equations(), domain, axis, limits)?;
    check(
        report.status == FiberStatus::UniformUniqueRoot,
        "Original saturated pole root arc remains unproved",
    )?;
    Ok(QualifiedPoleSsRootArc {
        source: source.clone(),
        domain,
        axis,
        limits,
        report,
    })
}
pub(super) fn certify_equations(equations:&[ExactSsPolynomial],d:[[f64;2];4],axis:FiberAxis,limits:DeflatedSsFiberLimits)->Result<DeflatedSsFiberReport> {
    operator::certify(equations,d,axis,limits)
}
