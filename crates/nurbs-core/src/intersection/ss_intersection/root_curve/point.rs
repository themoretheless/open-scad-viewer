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
    /// Semantic identity on the same qualified original graph, proved exactly.
    /// Different graph charts require a separate original membership proof.
    pub fn same_source_point(&self, other: &Self, limits: SsAlgebraicClockLimits) -> Result<bool> {
        if self.same_selector(other) {return Ok(true);}
        if !self.curve.same_definition(&other.curve)? {return Ok(false);}
        match (&self.clock,&other.clock) {
            (Clock::Binary(a),Clock::Binary(b)) => Ok(a==b),
            (Clock::Algebraic {point:a,..},Clock::Algebraic {point:b,..}) => a.clock().equals_clock(b.clock(),limits).map(|v|v.0),
            (Clock::Binary(t),Clock::Algebraic {point,..}) | (Clock::Algebraic {point,..},Clock::Binary(t)) => {
                let value=Q::from_float(*t).ok_or_else(||crate::input("Nonfinite SS point identity"))?;
                point.clock().equals_rational(&value,limits).map(|v|v.0)
            }
        }
    }
    /// Proves identity across regular affine charts using their original free
    /// coordinate and full native target-graph membership. No phase is inferred.
    pub fn same_source_point_in_charts(&self, other: &Self, clock_limits: SsAlgebraicClockLimits, geometry_limits: OriginalSsCurveLimits) -> Result<bool> {
        if self.curve.same_definition(&other.curve)? {
            return self.same_source_point(other,clock_limits);
        }
        let a=self.curve.source();
        let b=other.curve.source();
        if a.first()!=b.first() || a.second()!=b.second() {
            return Ok(false);
        }
        if self.curve.free_axis()!=other.curve.free_axis() {
            return cross_axis::prove(self,other,clock_limits,geometry_limits);
        }
        let k=self.curve.free_axis().index();
        let fa=frames::frame(&self.curve)?;
        let fb=frames::frame(&other.curve)?;
        check(fa[k][1]>Q::zero() && fb[k][1]>Q::zero(),"SS affine clock frames must increase")?;
        let scale=&fb[k][1]/&fa[k][1];
        let shift=(&fb[k][0]-&fa[k][0])/&fa[k][1];
        let same=match (&self.clock,&other.clock) {
            (Clock::Algebraic {point:a,..},Clock::Algebraic {point:b,..}) => a.clock().equals_affine(b.clock(),&scale,&shift,clock_limits)?.0,
            (Clock::Binary(t),Clock::Algebraic {point,..}) => {
                let value=(Q::from_float(*t).unwrap()-&shift)/&scale;
                point.clock().equals_rational(&value,clock_limits)?.0
            },
            (Clock::Algebraic {point,..},Clock::Binary(t)) => {
                let value=&scale*Q::from_float(*t).unwrap()+&shift;
                point.clock().equals_rational(&value,clock_limits)?.0
            },
            (Clock::Binary(a),Clock::Binary(b)) => Q::from_float(*a).unwrap()==scale*Q::from_float(*b).unwrap()+shift,
        };
        if !same {return Ok(false);}
        // Equal free clocks do not identify the branch. Prove that the whole
        // original point enclosure lies in the target's unique root graph.
        port::membership::bind(self,&other.curve,[0;4],geometry_limits)?;
        Ok(true)
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

mod cross_axis;
