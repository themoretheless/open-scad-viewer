//! Exact endpoint UV incidence. This is independent of Jordan/material proofs.
use super::*;

#[derive(Clone, Debug)]
pub struct QualifiedRootUvClosure {
    source: Arc<RootBrep>,
    max_joints: usize,
    joints: usize,
}
fn ordinary(port: &ss::QualifiedOriginalSsPort) -> bool {
    matches!(
        port.kind(),
        ss::OriginalSsPortKind::SameRootSelector | ss::OriginalSsPortKind::OriginalParameterRoot
    )
}
/// Prove that the two endpoints have identical native face coordinates.
pub fn qualify_root_uv_joint(
    a: &ss::QualifiedOriginalSsPort,
    support_a: usize,
    b: &ss::QualifiedOriginalSsPort,
    support_b: usize,
) -> Result<()> {
    if qualify_root_uv_joint_period_delta(a, support_a, b, support_b)? == [0, 0] {
        return Ok(());
    }
    Err(unproved(
        "Native UV joint requires a periodic chart transition",
    ))
}

/// Prove the integer period offset from the first face endpoint to the second.
/// A nonzero result requires a chart transition before planar UV operations.
pub fn qualify_root_uv_joint_period_delta(
    a: &ss::QualifiedOriginalSsPort,
    support_a: usize,
    b: &ss::QualifiedOriginalSsPort,
    support_b: usize,
) -> Result<[i32; 2]> {
    if support_a >= 2 || support_b >= 2 || !a.point().same_selector(b.point()) {
        return Err(unproved(
            "UV joint lacks valid supports or a literal shared selector",
        ));
    }
    if let Some(delta) = a.third_surface_uv_period_delta(b, support_a, support_b)? {
        return Ok(delta);
    }
    for (triple, ts, plain, ps, sign) in [
        (a, support_a, b, support_b, -1),
        (b, support_b, a, support_a, 1),
    ] {
        if let Some((witness, roles)) = triple.third_surface_binding() {
            let original = witness.source().original().source();
            let target = plain.target().source();
            if ordinary(plain)
                && roles[ts] == ps
                && roles[ts] < 2
                && target.first() == original.first()
                && target.second() == original.second()
            {
                let a = triple.phase();
                let b = plain.phase();
                return Ok(std::array::from_fn(|axis| sign * (a[2 * ts + axis] - b[2 * ps + axis])));
            }
        }
    }
    let x = a.target().source();
    let y = b.target().source();
    if ordinary(a)
        && ordinary(b)
        && support_a == support_b
        && x.first() == y.first()
        && x.second() == y.second()
    {
        // Both ports bind the same exact selector. Each nonzero shift was
        // admitted by the original seam identity in its endpoint binder.
        let first = a.phase();
        let second = b.phase();
        return Ok(std::array::from_fn(|axis| {
            second[2 * support_b + axis] - first[2 * support_a + axis]
        }));
    }
    Err(unproved("Common-face native UV identity remains unproved"))
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RootUvLoopLift {
    pub face: usize,
    pub loop_id: usize,
    /// Translation of each native coedge chart, in periods of the face.
    pub coedge_offsets: Vec<[i32; 2]>,
    /// End minus start in the lifted contour, in periods of the face.
    pub winding: [i32; 2],
}

impl RootUvLoopLift {
    fn advance(&mut self, delta: [i32; 2]) -> Result<()> {
        self.coedge_offsets.push(self.winding);
        // Translate the next chart to cancel the jump in native UV.
        for (offset, jump) in self.winding.iter_mut().zip(delta) {
            *offset = offset
                .checked_sub(jump)
                .ok_or_else(|| unproved("UV contour period offset overflow"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifted_charts_cancel_native_jumps_and_preserve_net_winding() {
        let mut lift = RootUvLoopLift {
            face: 0,
            loop_id: 0,
            coedge_offsets: Vec::new(),
            winding: [0, 0],
        };
        lift.advance([1, -1]).unwrap();
        lift.advance([-1, 0]).unwrap();
        assert_eq!(lift.coedge_offsets, [[0, 0], [-1, 1]]);
        assert_eq!(lift.winding, [0, 1]);
        lift.advance([0, 1]).unwrap();
        assert_eq!(lift.winding, [0, 0]);
    }
}

#[derive(Clone, Debug)]
pub struct QualifiedRootPeriodicUvClosure {
    source: Arc<RootBrep>,
    max_joints: usize,
    loops: Vec<RootUvLoopLift>,
}

fn collect_loop_lifts(
    source: &RootBrep,
    max_joints: usize,
    planar: bool,
) -> Result<(usize, Vec<RootUvLoopLift>)> {
    if !(1..=262144).contains(&max_joints) {
        return Err(unproved("Invalid UV closure joint budget"));
    }
    let topology = source.topology();
    let mut joints = 0usize;
    for face in &topology.faces {
        for id in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            joints = joints
                .checked_add(topology.loops[id].coedges.len())
                .ok_or_else(|| unproved("UV closure count overflow"))?;
            if joints > max_joints {
                return Err(unproved("UV closure joint budget exhausted"));
            }
        }
    }
    if joints == 0 {
        return Err(unproved("UV closure has no face contours"));
    }
    let mut loops = Vec::new();
    for (face_index, face) in topology.faces.iter().enumerate() {
        for id in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            let lift = qualify_loop_lift(source, face_index, id, planar)?;
            if !planar {
                loops.push(lift);
            }
        }
    }
    Ok((joints, loops))
}

pub(super) fn qualify_loop_lift(
    source: &RootBrep,
    face_index: usize,
    id: usize,
    planar: bool,
) -> Result<RootUvLoopLift> {
    let topology = source.topology();
    let coedges = &topology.loops[id].coedges;
    let mut lift = RootUvLoopLift {
        face: face_index,
        loop_id: id,
        coedge_offsets: Vec::new(),
        winding: [0, 0],
    };
    for (i, a) in coedges.iter().enumerate() {
        let b = &coedges[(i + 1) % coedges.len()];
        let ea = &topology.edges[a.edge].curve;
        let eb = &topology.edges[b.edge].curve;
        let end = if a.reversed { ea.start() } else { ea.end() };
        let start = if b.reversed { eb.end() } else { eb.start() };
        let delta =
            qualify_root_uv_joint_period_delta(end, a.pcurve.support, start, b.pcurve.support)?;
        if planar {
            if delta != [0, 0] {
                return Err(unproved(
                    "Native UV joint requires a periodic chart transition",
                ));
            }
        } else {
            lift.advance(delta)?;
        }
    }
    Ok(lift)
}

impl QualifiedRootPeriodicUvClosure {
    /// Qualify cyclic endpoint incidence modulo exact surface periods.
    /// Winding is reported per loop; planar trim still requires separate proof.
    pub fn qualify(source: Arc<RootBrep>, max_joints: usize) -> Result<Self> {
        let (_, loops) = collect_loop_lifts(&source, max_joints, false)?;
        Ok(Self {
            source,
            max_joints,
            loops,
        })
    }

    pub fn loops(&self) -> &[RootUvLoopLift] {
        &self.loops
    }

    pub fn reverify(&self) -> Result<()> {
        self.source.reverify()?;
        let (_, loops) = collect_loop_lifts(&self.source, self.max_joints, false)?;
        if loops != self.loops {
            return Err(unproved("Periodic UV closure incidence changed"));
        }
        Ok(())
    }
}

impl QualifiedRootUvClosure {
    pub fn qualify(source: Arc<RootBrep>, max_joints: usize) -> Result<Self> {
        let (joints, _) = collect_loop_lifts(&source, max_joints, true)?;
        Ok(Self {
            source,
            max_joints,
            joints,
        })
    }
    pub fn joints(&self) -> usize {
        self.joints
    }
    pub fn source(&self) -> &Arc<RootBrep> {
        &self.source
    }
    pub fn reverify(&self) -> Result<()> {
        self.source.reverify()?;
        let fresh = Self::qualify(self.source.clone(), self.max_joints)?;
        if fresh.joints != self.joints {
            return Err(unproved("UV closure incidence changed"));
        }
        Ok(())
    }
}
