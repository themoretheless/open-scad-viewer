//! Local no-retracing authority. This is not a global Jordan certificate.
use super::*;
use num_traits::ToPrimitive;
#[derive(Clone, Debug)]
pub struct QualifiedOriginalSsJoint {
    incoming: OriginalSsRootEdge,
    outgoing: OriginalSsRootEdge,
    limits: OriginalSsCurveLimits,
    depth: usize,
    receiver: usize,
    increasing: bool,
}
impl QualifiedOriginalSsJoint {
    pub fn increasing(&self) -> bool {
        self.increasing
    }
    pub fn subdivision_depth(&self) -> usize {
        self.depth
    }
    pub fn reverify(&self) -> Result<()> {
        self.incoming.reverify()?;
        self.outgoing.reverify()?;
        let increasing = attempt(
            &self.incoming,
            &self.outgoing,
            self.depth,
            self.receiver,
            self.limits,
        )?;
        check(
            increasing == self.increasing,
            "Original SS joint order changed",
        )
    }
}
fn short(
    edge: &OriginalSsRootEdge,
    end: bool,
    depth: usize,
    limits: OriginalSsCurveLimits,
) -> Result<OriginalSsRootEdge> {
    frames::frame(edge.curve())?; // affine injectivity, not nonlinear blowdown
    check(
        edge.start().phase() == [0; 4] && edge.end().phase() == [0; 4],
        "Joint needs explicit seam bridges",
    )?;
    let a = edge.start().parameter_bounds();
    let b = edge.end().parameter_bounds();
    let (lo, hi) = if edge.forward() {
        (&a[1], &b[0])
    } else {
        (&b[1], &a[0])
    };
    let near_low = if end { !edge.forward() } else { edge.forward() };
    let fraction = Q::new(1.into(), (1u64 << (depth + 1)).into());
    let exact = if near_low {
        lo + (hi - lo) * fraction
    } else {
        hi - (hi - lo) * fraction
    };
    let parameter = exact
        .to_f64()
        .ok_or_else(|| crate::input("Joint clock overflow"))?;
    let q = Q::from_float(parameter).ok_or_else(|| crate::input("Nonfinite joint clock"))?;
    check(
        *lo < q && q < *hi,
        "Joint subdivision has no strict binary interior",
    )?;
    let point = Arc::new(OriginalSsRootPoint::new(edge.curve().clone(), parameter)?);
    let port = bind_original_ss_port(point, edge.curve().clone(), [0; 4], limits)?;
    if end {
        OriginalSsRootEdge::new(port, edge.end().clone())
    } else {
        OriginalSsRootEdge::new(edge.start().clone(), port)
    }
}
fn attempt(
    a: &OriginalSsRootEdge,
    b: &OriginalSsRootEdge,
    depth: usize,
    receiver: usize,
    limits: OriginalSsCurveLimits,
) -> Result<bool> {
    check(
        a.end().point().same_selector(b.start().point()),
        "Joint has no literal shared root selector",
    )?;
    let left = short(a, true, depth, limits)?;
    let right = short(b, false, depth, limits)?;
    let target = if receiver == 0 { a.curve() } else { b.curve() };
    for edge in [&left, &right] {
        let uv = edge
            .enclose(limits)?
            .uv
            .map(|d| d.map(|x| Q::from_float(x).unwrap()));
        target.qualify_graph_window(uv)?;
    }
    let before =
        bind_original_ss_port(left.start().point().clone(), target.clone(), [0; 4], limits)?;
    let join = bind_original_ss_port(a.end().point().clone(), target.clone(), [0; 4], limits)?;
    let after = bind_original_ss_port(right.end().point().clone(), target.clone(), [0; 4], limits)?;
    // A full graph neighborhood must exist at the joint, not just a one-sided chart.
    let bounds = join
        .point()
        .enclose(limits)?
        .uv
        .map(|d| d.map(|x| Q::from_float(x).unwrap()));
    let window = target.qualify_graph_window(bounds)?;
    for k in 0..4 {
        let d = target.coordinate_domain()[k];
        check(
            Q::from_float(d[0]).unwrap() < window.local_window()[k][0]
                && window.local_window()[k][1] < Q::from_float(d[1]).unwrap(),
            "Joint lacks an interior graph neighborhood",
        )?;
    }
    let (p, j, n) = (
        before.parameter_bounds(),
        join.parameter_bounds(),
        after.parameter_bounds(),
    );
    if p[1] < j[0] && j[1] < n[0] {
        Ok(true)
    } else {
        check(
            n[1] < j[0] && j[1] < p[0],
            "Joint retraces or its order is unproved",
        )?;
        Ok(false)
    }
}
impl OriginalSsRootEdge {
    /// Bounded search for two complete incident subarcs in one unique affine
    /// original-root graph, with strict opposite order about their shared root.
    pub fn qualify_successor(
        &self,
        next: &Self,
        max_depth: usize,
        limits: OriginalSsCurveLimits,
    ) -> Result<QualifiedOriginalSsJoint> {
        check(max_depth <= 24, "Joint subdivision budget exceeds limit")?;
        for depth in 0..=max_depth {
            for receiver in 0..2 {
                if let Ok(increasing) = attempt(self, next, depth, receiver, limits) {
                    return Ok(QualifiedOriginalSsJoint {
                        incoming: self.clone(),
                        outgoing: next.clone(),
                        limits,
                        depth,
                        receiver,
                        increasing,
                    });
                }
            }
        }
        Err(crate::input(
            "Original SS joint no-retracing proof remains unresolved",
        ))
    }
}

impl OriginalSsRootWire {
    /// Prove no local retracing at every literal joint. This deliberately does
    /// not assert global embedding or absence of repeated complete traversal.
    pub fn qualify_joints(
        &self,
        max_depth: usize,
        limits: OriginalSsCurveLimits,
    ) -> Result<Vec<QualifiedOriginalSsJoint>> {
        self.edges()
            .iter()
            .enumerate()
            .map(|(i, edge)| {
                edge.qualify_successor(
                    &self.edges()[(i + 1) % self.edges().len()],
                    max_depth,
                    limits,
                )
            })
            .collect()
    }
}
