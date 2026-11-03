use super::*;
use crate::ss_intersection::{FiberAxis, FiberStatus};
#[derive(Clone, Debug)]
pub struct QualifiedSsBlowupRootArc {
    source: QualifiedSsBlowupChart,
    domain: [[f64; 2]; 4],
    axis: FiberAxis,
    limits: DeflatedSsFiberLimits,
    report: DeflatedSsFiberReport,
    original_bounds: [[f64; 2]; 4],
    node_preimage: Option<([Q; 4], ExactSsAlgebraLimits)>,
}
impl QualifiedSsBlowupRootArc {
    pub fn source(&self) -> &QualifiedSsBlowupChart {
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
    pub fn original_bounds(&self) -> [[f64; 2]; 4] {
        self.original_bounds
    }
    pub fn reverify(&self) -> Result<()> {
        self.source.reverify()?;
        let mut fresh =
            qualify_ss_blowup_root_arc(&self.source, self.domain, self.axis, self.limits)?;
        if let Some((p, limits)) = &self.node_preimage {
            fresh = fresh.bind_node_preimage(p.clone(), *limits)?;
        }
        check(
            self.report == fresh.report && self.original_bounds == fresh.original_bounds,
            "Original SS blowup root changed",
        )
    }
    pub(crate) fn certify_restriction(
        &self,
        d: [[f64; 2]; 4],
        limits: DeflatedSsFiberLimits,
    ) -> Result<DeflatedSsFiberReport> {
        check(
            (0..4).all(|k| self.domain[k][0] <= d[k][0] && d[k][1] <= self.domain[k][1]),
            "Blowup restriction leaves its root chart",
        )?;
        fiber::certify_equations(self.source.equations(), d, self.axis, limits)
    }
}
/// A regular strict-transform graph is original SS geometry under the exact
/// polynomial blowdown. No regularity or injectivity of that image is implied.
pub fn qualify_ss_blowup_root_arc(
    source: &QualifiedSsBlowupChart,
    domain: [[f64; 2]; 4],
    axis: FiberAxis,
    limits: DeflatedSsFiberLimits,
) -> Result<QualifiedSsBlowupRootArc> {
    let report = fiber::certify_equations(source.equations(), domain, axis, limits)?;
    check(
        report.status == FiberStatus::UniformUniqueRoot,
        "Strict-transform uniform root remains unresolved",
    )?;
    // Check the ENTIRE chart image: no extrapolated source polynomial can
    // masquerade as a root of the bounded original knot patch.
    let mut projection = limits.projection;
    projection.max_work = projection
        .max_work
        .checked_sub(report.projection_work)
        .ok_or_else(|| crate::resource("Blowdown allowance exhausted"))?;
    let (original_bounds, _) = mapped_bounds(source, domain, projection)?;
    Ok(QualifiedSsBlowupRootArc {
        source: source.clone(),
        domain,
        axis,
        limits,
        report,
        original_bounds,
        node_preimage: None,
    })
}
impl QualifiedSsBlowupRootArc {
    pub fn node_parameter(&self) -> Option<f64> {
        let k = self.source.radial_axis();
        if self.axis.index() == k && self.domain[k][0] <= 0.5 && 0.5 <= self.domain[k][1] {
            return Some(0.5);
        }
        use num_traits::ToPrimitive;
        self.node_preimage
            .as_ref()
            .and_then(|(p, _)| p[self.axis.index()].to_f64())
    }
    pub fn bind_node_preimage(
        mut self,
        point: [Q; 4],
        limits: ExactSsAlgebraLimits,
    ) -> Result<Self> {
        validate_limits(limits)?;
        let mut w = Work { used: 0, limits };
        for k in 0..4 {
            w.coefficient(&point[k])?;
            check(
                q(self.domain[k][0])? <= point[k] && point[k] <= q(self.domain[k][1])?,
                "Exact node selector leaves the strict root graph",
            )?;
        }
        check(
            point[self.source.radial_axis()] == q(0.5)?,
            "Strict-transform node selector is not exceptional",
        )?;
        for p in self.source.equations() {
            check(
                localization::evaluate(p, &point, &mut w)?.is_zero(),
                "Exact node selector does not satisfy the strict transform",
            )?;
        }
        use num_traits::ToPrimitive;
        let parameter = point[self.axis.index()]
            .to_f64()
            .filter(|x| x.is_finite())
            .ok_or_else(|| crate::input("Node clock overflow"))?;
        check(
            q(parameter)? == point[self.axis.index()],
            "Node selector needs a root-valued rather than binary free clock",
        )?;
        self.node_preimage = Some((point, limits));
        Ok(self)
    }
    pub(crate) fn mapped_bounds(
        &self,
        domain: [[f64; 2]; 4],
        limits: ExactSsAlgebraLimits,
    ) -> Result<([[f64; 2]; 4], usize)> {
        check(
            (0..4).all(|k| self.domain[k][0] <= domain[k][0] && domain[k][1] <= self.domain[k][1]),
            "Blowdown query leaves its original root graph",
        )?;
        mapped_bounds(&self.source, domain, limits)
    }
}
fn mapped_bounds(
    source: &QualifiedSsBlowupChart,
    domain: [[f64; 2]; 4],
    limits: ExactSsAlgebraLimits,
) -> Result<([[f64; 2]; 4], usize)> {
    validate_limits(limits)?;
    check(
        domain
            .iter()
            .all(|d| d.iter().all(|x| x.is_finite()) && 0. <= d[0] && d[0] <= d[1] && d[1] <= 1.),
        "Invalid blowdown source box",
    )?;
    let mut w = Work { used: 0, limits };
    let mut bounds = [[0.; 2]; 4];
    for k in 0..4 {
        let mut lo = Q::one();
        let mut hi = -Q::one();
        for mask in 0..16 {
            let p = std::array::from_fn(|j| q(domain[j][(mask >> j) & 1]).unwrap());
            let value = localization::evaluate(&source.map[k], &p, &mut w)?;
            lo = lo.min(value.clone());
            hi = hi.max(value);
        }
        check(
            Q::zero() <= lo && hi <= Q::one(),
            "Strict-transform box extrapolates an original support",
        )?;
        use num_traits::ToPrimitive;
        let a = lo
            .to_f64()
            .ok_or_else(|| crate::input("Blowdown interval overflow"))?;
        let b = hi
            .to_f64()
            .ok_or_else(|| crate::input("Blowdown interval overflow"))?;
        bounds[k] = [
            if q(a)? > lo { a.next_down() } else { a },
            if q(b)? < hi { b.next_up() } else { b },
        ];
    }
    Ok((bounds, w.used))
}
