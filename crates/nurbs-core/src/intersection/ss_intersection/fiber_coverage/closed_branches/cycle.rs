//! A first-return traversal of an ALREADY proved embedded original circle.
//! Uniform source charts are oriented intervals on that circle. Shared exact
//! root selectors join consecutive arcs. Strict clock order gives positive
//! progress, and exclusion/order of the start point on EVERY intermediate arc
//! prevents a missed return or multiple winding. Thus the first closed walk
//! visits the whole selected component exactly once. Adjacency alone proves none
//! of these hypotheses. No saved curve approximation or material role follows.
use super::*;
mod discovery;
pub use discovery::order_closed_fiber_cycle;
#[derive(Clone, Copy, Debug)]
pub struct ClosedFiberCycleLimits {
    pub max_rectangles: usize,
    pub max_steps: usize,
    pub max_candidates: usize,
    pub max_start_attempts: usize,
}
impl Default for ClosedFiberCycleLimits {
    fn default() -> Self {
        Self {
            max_rectangles: 100000,
            max_steps: 128,
            max_candidates: 2048,
            max_start_attempts: 8,
        }
    }
}
#[derive(Clone, Debug)]
pub struct ClosedFiberRootPort {
    point: QualifiedFiber,
    bounds: Box4,
}
impl ClosedFiberRootPort {
    /// One original root at one exactly fixed binary64 source coordinate;
    /// other coordinates stay outward enclosures, never snapped identities.
    pub fn selector(&self) -> &QualifiedFiber {
        &self.point
    }
    pub fn parameter_bounds(&self) -> Box4 {
        self.bounds
    }
}
#[derive(Clone, Debug)]
pub struct OrderedClosedFiberArc {
    fiber: QualifiedFiber,
    forward: bool,
}
impl OrderedClosedFiberArc {
    pub fn fiber(&self) -> &QualifiedFiber {
        &self.fiber
    }
    pub fn forward(&self) -> bool {
        self.forward
    }
}
#[derive(Clone, Debug)]
pub struct OrderedClosedFiberCycle {
    owner: QualifiedClosedFiberBranches,
    branch: usize,
    first_chart: usize,
    arcs: Vec<OrderedClosedFiberArc>,
    ports: Vec<ClosedFiberRootPort>,
    transitions: Vec<QualifiedFiberTransition>,
    limits: ClosedFiberCycleLimits,
    rectangles: usize,
}
// A point fixes one source coordinate. It inherits support immersion from
// the complete closed owner; a collapsed interval need not re-prove a 2D
// surface differential. Original root existence/uniqueness is still fresh.
pub(super) fn point_bounds(point: &QualifiedFiber) -> Result<Box4> {
    let report = point.report();
    check(
        report.status == FiberStatus::UniformUniqueRoot
            && report.operator.is_some()
            && report
                .contraction_upper
                .is_some_and(|q| q.is_finite() && q >= 0. && q < 1.),
        "Missing original point root proof",
    )?;
    let image = report
        .root_image
        .ok_or_else(|| crate::input("Missing original point enclosure"))?;
    let mut bounds = point.domain();
    let mut j = 0;
    for k in 0..4 {
        if k != point.free_axis().index() {
            bounds[k] = [bounds[k][0].max(image[j][0]), bounds[k][1].min(image[j][1])];
            j += 1;
        }
    }
    check(
        bounds.iter().all(|d| d[0] <= d[1]),
        "Empty original point enclosure",
    )?;
    Ok(bounds)
}
fn same(a: &QualifiedFiber, b: &QualifiedFiber) -> bool {
    a.report() == b.report()
        && a.limits().max_rectangles == b.limits().max_rectangles
        && a.limits().preconditioner == b.limits().preconditioner
        && a.domain() == b.domain()
        && a.free_axis() == b.free_axis()
        && a.source().first() == b.source().first()
        && a.source().second() == b.source().second()
}
fn inside(point: Box4, domain: Box4, strict: bool) -> bool {
    (0..4).all(|k| {
        if strict {
            domain[k][0] < point[k][0] && point[k][1] < domain[k][1]
        } else {
            domain[k][0] <= point[k][0] && point[k][1] <= domain[k][1]
        }
    })
}
fn disjoint(point: Box4, domain: Box4) -> bool {
    (0..4).any(|k| point[k][1] < domain[k][0] || domain[k][1] < point[k][0])
}
fn before(a: Box4, b: Box4, axis: usize, forward: bool) -> bool {
    if forward {
        a[axis][1] < b[axis][0]
    } else {
        b[axis][1] < a[axis][0]
    }
}
fn orientation(
    t: &QualifiedFiberTransition,
    a: &QualifiedFiber,
    b: &QualifiedFiber,
) -> Option<bool> {
    ((same(t.first(), a) && same(t.second(), b)) || (same(t.first(), b) && same(t.second(), a)))
        .then_some(t.orientation() == TransitionOrientation::Forward)
}
impl OrderedClosedFiberCycle {
    pub fn owner(&self) -> &QualifiedClosedFiberBranches {
        &self.owner
    }
    pub fn branch(&self) -> usize {
        self.branch
    }
    /// Port i is shared by arc i's start and arc i-1's end, cyclically.
    pub fn ports(&self) -> &[ClosedFiberRootPort] {
        &self.ports
    }
    pub fn arcs(&self) -> &[OrderedClosedFiberArc] {
        &self.arcs
    }
    pub fn rectangles(&self) -> usize {
        self.rectangles
    }
    pub fn reverify(&self) -> Result<()> {
        self.verify(true)
    }
    fn verify(&self, fresh: bool) -> Result<()> {
        if fresh {
            self.owner.reverify()?;
        }
        check(
            self.branch < self.owner.branches().len(),
            "Unknown closed source component",
        )?;
        let n = self.arcs.len();
        check(
            n >= 2
                && n <= self.limits.max_steps
                && self.ports.len() == n
                && self.transitions.len() == n
                && self.rectangles <= self.limits.max_rectangles,
            "Incomplete closed source traversal",
        )?;
        let ids = &self.owner.branches()[self.branch].charts;
        let fs = self.owner.owner().proof().coverage().fibers();
        check(
            ids.contains(&self.first_chart)
                && same(&self.arcs[0].fiber, &fs[self.first_chart])
                && self.arcs[0].forward,
            "Traversal not bound to its original component",
        )?;
        let mut replay = 0usize;
        for port in &self.ports {
            if fresh {
                port.point.reverify()?;
            }
            check(
                port.point.source().first() == fs[self.first_chart].source().first()
                    && port.point.source().second() == fs[self.first_chart].source().second(),
                "Shared port belongs to foreign source carriers",
            )?;
            let axis = port.point.free_axis().index();
            check(
                port.point.domain()[axis][0] == port.point.domain()[axis][1]
                    && point_bounds(&port.point)? == port.bounds,
                "Original shared root selector changed",
            )?;
            replay += port.point.report().rectangles;
        }
        let start = self.ports[0].bounds;
        check(
            inside(start, self.arcs[0].fiber.domain(), true),
            "Cycle start is not a strict original chart point",
        )?;
        for i in 0..n {
            let arc = &self.arcs[i];
            let a = self.ports[i].bounds;
            let b = self.ports[(i + 1) % n].bounds;
            check(
                arc.fiber.source().first() == fs[self.first_chart].source().first()
                    && arc.fiber.source().second() == fs[self.first_chart].source().second()
                    && inside(a, arc.fiber.domain(), false)
                    && inside(b, arc.fiber.domain(), false)
                    && before(a, b, arc.fiber.free_axis().index(), arc.forward),
                "An original arc reverses or leaves its shared ports",
            )?;
            if i > 0 && i + 1 < n {
                check(
                    disjoint(start, arc.fiber.domain())
                        || (inside(start, arc.fiber.domain(), false)
                            && before(start, a, arc.fiber.free_axis().index(), arc.forward)),
                    "An intermediate arc may already cross the start root",
                )?;
            }
            let transition = &self.transitions[i];
            if fresh {
                transition.reverify()?;
            }
            replay += transition.rectangles();
            let next = &self.arcs[(i + 1) % n];
            let agrees = orientation(transition, &arc.fiber, &next.fiber)
                .ok_or_else(|| crate::input("Transition belongs to different original charts"))?;
            check(
                next.forward == if agrees { arc.forward } else { !arc.forward },
                "Closed traversal orientation does not glue",
            )?;
        }
        check(
            replay <= self.rectangles,
            "Closed traversal replay exceeds the shared work ledger",
        )
    }
}
