//! Ports retain the algebraic point definition while bounding its clock on an
//! incident original graph. Affine frame membership never substitutes a float.
use super::*;
#[derive(Clone, Debug)]
pub struct QualifiedSsAlgebraicPort {
    point: Arc<OriginalSsAlgebraicPoint>,
    target: Arc<OriginalSsRootCurve>,
    parameter: [Q; 2],
    phase: [i32; 4],
    limits: SsAlgebraicPointLimits,
}
impl QualifiedSsAlgebraicPort {
    pub fn bind(
        point: Arc<OriginalSsAlgebraicPoint>,
        target: Arc<OriginalSsRootCurve>,
        phase: [i32; 4],
        limits: SsAlgebraicPointLimits,
    ) -> Result<Self> {
        check(
            phase.iter().all(|x| (-2..=2).contains(x)),
            "Algebraic root port phase allowance exceeded",
        )?;
        let parameter = if phase == [0; 4] && point.curve().same_definition(&target)? {
            point
                .clock()
                .enclose(limits.bisections, limits.clock)?
                .bounds
        } else {
            let bounds = point.enclose(limits)?;
            port::membership::bind_bounds(
                point.curve(),
                &bounds.geometry,
                bounds.clock,
                &target,
                phase,
            )?
        };
        Ok(Self {
            point,
            target,
            parameter,
            phase,
            limits,
        })
    }
    pub fn point(&self) -> &Arc<OriginalSsAlgebraicPoint> {
        &self.point
    }
    pub fn target(&self) -> &Arc<OriginalSsRootCurve> {
        &self.target
    }
    pub fn parameter_bounds(&self) -> &[Q; 2] {
        &self.parameter
    }
    pub fn phase(&self) -> [i32; 4] {
        self.phase
    }
    pub fn reverify(&self) -> Result<()> {
        self.point.reverify()?;
        self.target.reverify()?;
        let fresh = Self::bind(
            self.point.clone(),
            self.target.clone(),
            self.phase,
            self.limits,
        )?;
        check(
            fresh.parameter == self.parameter,
            "Algebraic root port membership changed",
        )
    }
}
#[derive(Clone, Debug)]
pub struct OriginalSsAlgebraicEdge {
    start: QualifiedSsAlgebraicPort,
    end: QualifiedSsAlgebraicPort,
    forward: bool,
}
impl OriginalSsAlgebraicEdge {
    pub fn new(start: QualifiedSsAlgebraicPort, end: QualifiedSsAlgebraicPort) -> Result<Self> {
        check(
            start.target().same_definition(end.target())?,
            "Algebraic edge ports belong to different original graphs",
        )?;
        let a = start.parameter_bounds();
        let b = end.parameter_bounds();
        let forward = if a[1] < b[0] {
            true
        } else {
            check(
                b[1] < a[0],
                "Algebraic edge endpoint order remains unproved",
            )?;
            false
        };
        Ok(Self {
            start,
            end,
            forward,
        })
    }
    pub fn start(&self) -> &QualifiedSsAlgebraicPort {
        &self.start
    }
    pub fn end(&self) -> &QualifiedSsAlgebraicPort {
        &self.end
    }
    pub fn forward(&self) -> bool {
        self.forward
    }
    pub fn parameter_bounds(&self) -> [Q; 2] {
        if self.forward {
            [
                self.start.parameter[0].clone(),
                self.end.parameter[1].clone(),
            ]
        } else {
            [
                self.end.parameter[0].clone(),
                self.start.parameter[1].clone(),
            ]
        }
    }
    pub fn enclose(&self, limits: OriginalSsCurveLimits) -> Result<OriginalSsCurveBounds> {
        let mut d = algebraic_clock::float_bounds(&self.parameter_bounds())?;
        let natural = self.start.target.parameter_domain();
        d[0] = d[0].max(natural[0]);
        d[1] = d[1].min(natural[1]);
        check(
            d[0] <= d[1],
            "Algebraic edge float enclosure leaves original graph",
        )?;
        self.start.target.enclose(d, limits)
    }
    pub fn reverify(&self) -> Result<()> {
        self.start.reverify()?;
        self.end.reverify()?;
        let fresh = Self::new(self.start.clone(), self.end.clone())?;
        check(
            fresh.forward == self.forward,
            "Algebraic edge direction changed",
        )
    }
}
