//! Root-valued endpoint binding. Exact source membership plus native uniqueness
//! proves world-point identity. Pole aliases use their exact constant image;
//! no nearest midpoint or rounded period translation authorizes a join.
use super::*;
pub(super) mod membership;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalSsPortKind {
    SameRootSelector,
    OriginalParameterRoot,
    CollapsedPole,
    OriginalSourceNode,
}
#[derive(Clone, Debug)]
pub struct QualifiedOriginalSsPort {
    point: Arc<OriginalSsRootPoint>,
    target: Arc<OriginalSsRootCurve>,
    phase: [i32; 4],
    parameter: [Q; 2],
    kind: OriginalSsPortKind,
    limits: OriginalSsCurveLimits,
}
impl QualifiedOriginalSsPort {
    pub fn point(&self) -> &Arc<OriginalSsRootPoint> {
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
    pub fn kind(&self) -> OriginalSsPortKind {
        self.kind
    }
    /// Replace an independent endpoint owner only after exact point identity.
    pub fn rebind_equivalent(&self, point: Arc<OriginalSsRootPoint>, limits: SsAlgebraicClockLimits) -> Result<Self> {
        check(self.point.same_source_point_in_charts(&point,limits,self.limits)?,"Independent SS endpoint identity remains unproved")?;
        bind_original_ss_port(point,self.target.clone(),self.phase,self.limits)
    }
    pub fn reverify(&self) -> Result<()> {
        self.point.reverify()?;
        self.target.reverify()?;
        let fresh = bind_original_ss_port(
            self.point.clone(),
            self.target.clone(),
            self.phase,
            self.limits,
        )?;
        check(
            self.parameter == fresh.parameter && self.kind == fresh.kind,
            "Original shared SS endpoint or its exact phase membership changed",
        )
    }
}
/// phase[k] adds an exact INTEGER original source period to coordinate k.
/// Nonzero shifts require native seam identity. The point selector remains
/// unchanged; its parameter on the target may remain algebraic/root-valued.
pub fn bind_original_ss_port(
    point: Arc<OriginalSsRootPoint>,
    target: Arc<OriginalSsRootCurve>,
    phase: [i32; 4],
    limits: OriginalSsCurveLimits,
) -> Result<QualifiedOriginalSsPort> {
    check(
        phase.iter().all(|v| (-2..=2).contains(v)),
        "Original root endpoint phase allowance exceeded",
    )?;
    if phase == [0; 4] && point.curve.same_definition(&target)? {
        return Ok(QualifiedOriginalSsPort {
            parameter: point.clock_bounds()?,
            point,
            target,
            phase,
            kind: OriginalSsPortKind::SameRootSelector,
            limits,
        });
    }
    if phase == [0;4] {
        if let Some(a)=point.source_node()? {
            if let Carrier::Blowup(b)=&target.carrier {
                if let Some(parameter)=b.node_parameter() {
                    if a.world()==b.source().node().world() {
                        return Ok(QualifiedOriginalSsPort{parameter:[membership::q(parameter)?,membership::q(parameter)?],point,target,phase,
                            kind:OriginalSsPortKind::OriginalSourceNode,limits});
                    }
                }
            }
        }
    }
    if phase == [0; 4] {
        if let (Some((parameter, a)), Some((end, b))) =
            (point.curve.pole_endpoint(), target.pole_endpoint())
        {
            if point.equals_binary_parameter(parameter)? && a == b {
                return Ok(QualifiedOriginalSsPort {
                    parameter: [membership::q(end)?, membership::q(end)?],
                    point,
                    target,
                    phase,
                    kind: OriginalSsPortKind::CollapsedPole,
                    limits,
                });
            }
        }
    }
    let parameter = membership::bind(&point, &target, phase, limits)?;
    Ok(QualifiedOriginalSsPort {
        point,
        target,
        phase,
        parameter,
        kind: OriginalSsPortKind::OriginalParameterRoot,
        limits,
    })
}
