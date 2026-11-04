//! Common SS vertex selectors. Algebraic clocks retain their original owners.
use super::*;
#[derive(Clone, Debug)]
enum Clock {
    Binary(f64),
    Algebraic { point: OriginalSsAlgebraicPoint, policy: SsAlgebraicPointLimits },
}
#[derive(Clone, Debug)]
pub struct OriginalSsRootPoint {
    pub(super) curve: Arc<OriginalSsRootCurve>,
    clock: Clock,
}
impl OriginalSsRootPoint {
    pub fn new(curve: Arc<OriginalSsRootCurve>, parameter: f64) -> Result<Self> {
        let point = Self { curve, clock: Clock::Binary(parameter) };
        point.validate_selector()?;
        Ok(point)
    }
    /// Retains the literal curve/clock owners and an explicit query allowance.
    /// No binary parameter is synthesized for this selector.
    pub fn from_algebraic(point: OriginalSsAlgebraicPoint, policy: SsAlgebraicPointLimits) -> Result<Self> {
        point.clock().enclose(policy.bisections, policy.clock)?;
        Ok(Self { curve: point.curve().clone(), clock: Clock::Algebraic { point, policy } })
    }
    pub fn curve(&self) -> &Arc<OriginalSsRootCurve> { &self.curve }
    pub fn binary_parameter(&self) -> Option<f64> {
        match self.clock { Clock::Binary(t) => Some(t), _ => None }
    }
    pub fn algebraic_point(&self) -> Option<&OriginalSsAlgebraicPoint> {
        match &self.clock { Clock::Algebraic {point,..} => Some(point), _ => None }
    }
    pub fn validate_selector(&self) -> Result<()> {
        let d = self.curve.parameter_domain();
        match &self.clock {
            Clock::Binary(t) => check(t.is_finite() && d[0] <= *t && *t <= d[1], "Original SS vertex selector leaves its owned root graph"),
            Clock::Algebraic {point,..} => OriginalSsAlgebraicPoint::new(self.curve.clone(),point.clock().clone()).map(|_|()),
        }
    }
    pub fn clock_bounds(&self) -> Result<[Q;2]> {
        match &self.clock {
            Clock::Binary(t) => { let q=Q::from_float(*t).ok_or_else(||crate::input("Nonfinite SS selector"))?; Ok([q.clone(),q]) },
            Clock::Algebraic {point,policy} => Ok(point.clock().enclose(policy.bisections,policy.clock)?.bounds),
        }
    }
    /// Exact endpoint equality, never a comparison of rounded enclosures.
    pub fn equals_binary_parameter(&self, parameter: f64) -> Result<bool> {
        let value=Q::from_float(parameter).ok_or_else(||crate::input("Nonfinite SS endpoint comparison"))?;
        match &self.clock {
            Clock::Binary(t) => Ok(*t==parameter),
            Clock::Algebraic {point,policy} => point.clock().equals_rational(&value,policy.clock).map(|v|v.0),
        }
    }
    pub(super) fn source_node(&self) -> Result<Option<&QualifiedSsRootNode>> {
        if let Carrier::Blowup(arc)=&self.curve.carrier {
            if let Some(t)=arc.node_parameter() {
                if self.equals_binary_parameter(t)? { return Ok(self.curve.node_at(t)); }
            }
        }
        Ok(None)
    }
    pub fn same_selector(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.curve,&other.curve) && match (&self.clock,&other.clock) {
            (Clock::Binary(a),Clock::Binary(b)) => a==b,
            (Clock::Algebraic {point:a,..},Clock::Algebraic {point:b,..}) => a.same_selector(b),
            _ => false,
        }
    }
    pub fn enclose(&self, limits: OriginalSsCurveLimits) -> Result<OriginalSsCurveBounds> {
        match &self.clock {
            Clock::Binary(t) => self.curve.enclose([*t;2],limits),
            Clock::Algebraic {point,policy} => {
                let result=point.enclose(SsAlgebraicPointLimits { geometry:limits,..*policy })?;
                let mut geometry=result.geometry;
                geometry.projection_work=geometry.projection_work.checked_add(result.clock_operations)
                    .ok_or_else(||crate::input("SS selector work count overflow"))?;
                Ok(geometry)
            }
        }
    }
    pub fn reverify(&self) -> Result<()> {
        self.validate_selector()?;
        match &self.clock {
            Clock::Binary(_) => self.curve.reverify(),
            Clock::Algebraic {point,policy} => { point.reverify()?; point.clock().enclose(policy.bisections,policy.clock)?; Ok(()) },
        }
    }
}
