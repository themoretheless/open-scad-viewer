//! A point on an original SS graph selected by an exact algebraic clock.
use super::*;
#[derive(Clone, Copy, Debug)]
pub struct SsAlgebraicPointLimits {
    pub bisections: usize,
    pub clock: SsAlgebraicClockLimits,
    pub geometry: OriginalSsCurveLimits,
}
impl Default for SsAlgebraicPointLimits {
    fn default() -> Self {
        Self {
            bisections: 48,
            clock: Default::default(),
            geometry: Default::default(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct OriginalSsAlgebraicPoint {
    curve: Arc<OriginalSsRootCurve>,
    clock: Arc<QualifiedSsAlgebraicClock>,
}
#[derive(Clone, Debug)]
pub struct SsAlgebraicPointBounds {
    pub clock: [Q; 2],
    pub clock_operations: usize,
    pub geometry: OriginalSsCurveBounds,
}
impl OriginalSsAlgebraicPoint {
    pub fn new(
        curve: Arc<OriginalSsRootCurve>,
        clock: Arc<QualifiedSsAlgebraicClock>,
    ) -> Result<Self> {
        let d = curve.parameter_domain();
        let p = clock.isolating_interval();
        check(
            Q::from_float(d[0]).is_some_and(|v| v <= p[0])
                && Q::from_float(d[1]).is_some_and(|v| p[1] <= v),
            "Algebraic point isolator leaves its original SS graph",
        )?;
        Ok(Self { curve, clock })
    }
    pub fn curve(&self) -> &Arc<OriginalSsRootCurve> {
        &self.curve
    }
    pub fn clock(&self) -> &Arc<QualifiedSsAlgebraicClock> {
        &self.clock
    }
    pub fn same_selector(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.curve, &other.curve) && Arc::ptr_eq(&self.clock, &other.clock)
    }
    pub fn enclose(&self, limits: SsAlgebraicPointLimits) -> Result<SsAlgebraicPointBounds> {
        let clock = self.clock.enclose(limits.bisections, limits.clock)?;
        let mut d = algebraic_clock::float_bounds(&clock.bounds)?;
        let natural = self.curve.parameter_domain();
        d[0] = d[0].max(natural[0]);
        d[1] = d[1].min(natural[1]);
        check(
            d[0] <= d[1],
            "Algebraic point float image leaves original graph",
        )?;
        let geometry = self.curve.enclose(d, limits.geometry)?;
        Ok(SsAlgebraicPointBounds {
            clock: clock.bounds,
            clock_operations: clock.operations,
            geometry,
        })
    }
    pub fn reverify(&self) -> Result<()> {
        self.clock.reverify()?;
        self.curve.reverify()?;
        Self::new(self.curve.clone(), self.clock.clone()).map(|_| ())
    }
}
