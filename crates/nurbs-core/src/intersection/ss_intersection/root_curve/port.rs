//! Root-valued endpoint binding. Exact source membership plus native uniqueness
//! proves world-point identity. Pole aliases use their exact constant image;
//! no nearest midpoint or rounded period translation authorizes a join.
use super::*;
pub(super) mod membership;
mod third;
pub use third::bind_original_ss_third_port;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalSsPortKind {
    SameRootSelector,
    OriginalParameterRoot,
    CollapsedPole,
    OriginalSourceNode,
    OriginalThirdSurface,
}
#[derive(Clone, Debug)]
pub struct QualifiedOriginalSsPort {
    point: Arc<OriginalSsRootPoint>,
    target: Arc<OriginalSsRootCurve>,
    phase: [i32; 4],
    parameter: [Q; 2],
    kind: OriginalSsPortKind,
    limits: OriginalSsCurveLimits,
    third: Option<(Arc<QualifiedSsThirdSurfacePoint>, [usize; 2], [[Q; 2]; 4])>,
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
    /// Exact normalized target UV enclosure for a sealed triple-surface port.
    /// These coordinates belong to the target graph, not the global point graph.
    pub fn third_target_local_uv_bounds(&self) -> Option<&[[Q; 2]; 4]> {
        self.third.as_ref().map(|(_, _, uv)| uv)
    }
    /// Prove identical native UV on a common surface using the same sealed
    /// triple witness and surface role. World incidence alone is insufficient
    /// on self-intersecting or periodic surfaces. False means unproved.
    pub fn same_third_surface_uv(&self, other: &Self, support: usize, other_support: usize) -> Result<bool> {
        check(support < 2 && other_support < 2, "Invalid triple-port surface support")?;
        Ok(match (&self.third, &other.third) {
            (Some((a, ar, _)), Some((b, br, _))) => Arc::ptr_eq(a,b)
                && ar[support] == br[other_support]
                && self.point.same_selector(&other.point)
                && self.phase == [0;4] && other.phase == [0;4],
            _ => false,
        })
    }
    /// Replace an independent endpoint owner only after exact point identity.
    pub fn rebind_equivalent(&self, point: Arc<OriginalSsRootPoint>, limits: SsAlgebraicClockLimits) -> Result<Self> {
        if let Some((witness, roles, _)) = &self.third {
            check(point.same_selector(witness.point()), "Third-surface port requires its literal global vertex owner")?;
            let mut fresh = bind_original_ss_third_port(witness.clone(), self.target.clone(), *roles, self.limits)?;
            fresh.point = point;
            return Ok(fresh);
        }
        check(self.point.same_source_point_in_charts(&point,limits,self.limits)?,"Independent SS endpoint identity remains unproved")?;
        bind_original_ss_port(point,self.target.clone(),self.phase,self.limits)
    }
    pub fn reverify(&self) -> Result<()> {
        self.point.reverify()?;
        self.target.reverify()?;
        if let Some((witness, roles, _)) = &self.third {
            witness.reverify()?;
            let fresh = bind_original_ss_third_port(witness.clone(), self.target.clone(), *roles, self.limits)?;
            return check(self.point.same_selector(witness.point()) && self.parameter == fresh.parameter
                && self.kind == fresh.kind && self.phase == [0;4]
                && self.third_target_local_uv_bounds() == fresh.third_target_local_uv_bounds(), "Third-surface port proof changed");
        }
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
            third: None,
        });
    }
    if phase == [0;4] {
        if let Some(a)=point.source_node()? {
            if let Carrier::Blowup(b)=&target.carrier {
                if let Some(parameter)=b.node_parameter() {
                    if a.world()==b.source().node().world() {
                        return Ok(QualifiedOriginalSsPort{parameter:[membership::q(parameter)?,membership::q(parameter)?],point,target,phase,
                            kind:OriginalSsPortKind::OriginalSourceNode,limits,third:None});
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
                    third: None,
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
        third: None,
    })
}
