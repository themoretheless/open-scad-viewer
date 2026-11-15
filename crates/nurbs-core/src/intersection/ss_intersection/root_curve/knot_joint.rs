//! Cross-carrier joints on an interior original knot line.
//!
//! SS carriers are qualified one original knot span at a time, so a root
//! branch that crosses a knot becomes two edges on two span carriers. This
//! module proves such a joint locally sound without extrapolating either span
//! polynomial past the knot:
//!
//! * Both carriers are affine deflated span graphs over the SAME original free
//!   coordinate `k`, their spans agree in every other coordinate, and they are
//!   adjacent across one interior knot `K` of multiplicity at most the degree,
//!   so the original surface is continuous across `K` and each closed span
//!   patch is source-equivalent there.
//! * The shared selector has the exact original free coordinate `K` on both
//!   carriers and binds into both closed graph windows. Graph uniqueness on
//!   each closed window then identifies it with both roots on the knot line:
//!   the two root selectors agree there.
//! * The two one-sided windows `W_A = [a, K] x C` and `W_B = [K, b] x C`, with
//!   `C` the common box of the other coordinates, are qualified graph windows.
//!   With `a < K < b` and the joint enclosure strictly inside `C`, their union
//!   contains the open box `N = (a, b) x int C` around the joint, and every
//!   original root in `N` lies on exactly one of the two graphs according to
//!   the side of `K`. Shortened incident subarcs are enclosed strictly inside
//!   `C`, so both edges actually enter `N`; the free clock orders the joint.
//!
//! Seams (nonzero phases or periodic wraps), poles, nonlinear blowdowns and
//! knots crossed by a non-free coordinate remain typed refusals.
use super::*;
use num_traits::ToPrimitive;
use crate::Error;

fn refused(s: &str) -> Error {
    Error::new("NURBS_SS_KNOT_JOINT_UNPROVED", s)
}
fn ensure(ok: bool, s: &str) -> Result<()> {
    if ok { Ok(()) } else { Err(refused(s)) }
}
fn q(x: f64) -> Result<Q> {
    Q::from_float(x).ok_or_else(|| refused("Nonfinite knot-joint coordinate"))
}

#[derive(Clone, Debug)]
pub struct QualifiedOriginalSsKnotJoint {
    incoming: OriginalSsRootEdge,
    outgoing: OriginalSsRootEdge,
    limits: OriginalSsCurveLimits,
    depth: usize,
    coordinate: usize,
    knot: Q,
    increasing: bool,
    windows: [QualifiedOriginalSsGraphWindow; 2],
}
impl QualifiedOriginalSsKnotJoint {
    /// Original UV4 coordinate index (0..4) whose interior knot is crossed.
    pub fn coordinate(&self) -> usize {
        self.coordinate
    }
    /// Exact original knot value of the shared joint.
    pub fn knot(&self) -> &Q {
        &self.knot
    }
    /// True when traversal increases the original knot coordinate.
    pub fn increasing(&self) -> bool {
        self.increasing
    }
    pub fn subdivision_depth(&self) -> usize {
        self.depth
    }
    /// The incoming and outgoing one-sided qualified graph windows.
    pub fn windows(&self) -> &[QualifiedOriginalSsGraphWindow; 2] {
        &self.windows
    }
    pub fn incoming(&self) -> &OriginalSsRootEdge {
        &self.incoming
    }
    pub fn outgoing(&self) -> &OriginalSsRootEdge {
        &self.outgoing
    }
    pub fn reverify(&self) -> Result<()> {
        self.incoming.reverify()?;
        self.outgoing.reverify()?;
        let fresh = attempt(&self.incoming, &self.outgoing, self.depth, self.limits)?;
        ensure(
            fresh.coordinate == self.coordinate
                && fresh.knot == self.knot
                && fresh.increasing == self.increasing
                && fresh.windows[0].original_window() == self.windows[0].original_window()
                && fresh.windows[1].original_window() == self.windows[1].original_window(),
            "Original SS knot joint changed",
        )
    }
}

/// A literal wire joint: either a regular single-graph joint or a certified
/// cross-carrier joint on an interior knot line.
#[derive(Clone, Debug)]
pub enum QualifiedOriginalSsWireJoint {
    Regular(QualifiedOriginalSsJoint),
    Knot(QualifiedOriginalSsKnotJoint),
}
impl QualifiedOriginalSsWireJoint {
    pub fn knot(&self) -> Option<&QualifiedOriginalSsKnotJoint> {
        match self {
            Self::Knot(k) => Some(k),
            Self::Regular(_) => None,
        }
    }
    pub fn reverify(&self) -> Result<()> {
        match self {
            Self::Regular(j) => j.reverify(),
            Self::Knot(j) => j.reverify(),
        }
    }
}

struct Proof {
    coordinate: usize,
    knot: Q,
    increasing: bool,
    windows: [QualifiedOriginalSsGraphWindow; 2],
}

fn spans(curve: &OriginalSsRootCurve) -> Result<[usize; 4]> {
    match &curve.carrier {
        Carrier::Deflated(f) => Ok(f.source().original().original().spans()),
        Carrier::Pole(_) => Err(refused("Knot joint touches an excluded pole stratum")),
        Carrier::Blowup(_) => Err(refused("Nonlinear blowdown cannot use a knot joint")),
        _ => Err(refused("Knot joint needs original span carriers")),
    }
}

/// Two distinct affine span carriers over one free coordinate whose spans
/// differ in that coordinate only. This is the only configuration routed to
/// the knot proof; everything else stays with the regular joint.
fn candidate(a: &OriginalSsRootEdge, b: &OriginalSsRootEdge) -> bool {
    let (Ok(sa), Ok(sb)) = (spans(a.curve()), spans(b.curve())) else {
        return false;
    };
    let k = a.curve().free_axis().index();
    k == b.curve().free_axis().index()
        && sa[k] != sb[k]
        && (0..4).all(|j| j == k || sa[j] == sb[j])
}

fn attempt(
    a: &OriginalSsRootEdge,
    b: &OriginalSsRootEdge,
    depth: usize,
    limits: OriginalSsCurveLimits,
) -> Result<Proof> {
    ensure(
        a.end().point().same_selector(b.start().point()),
        "Knot joint has no literal shared root selector",
    )?;
    for port in [a.start(), a.end(), b.start(), b.end()] {
        ensure(
            port.kind() != OriginalSsPortKind::OriginalThirdSurface,
            "Knot joint needs a common-face UV proof",
        )?;
        ensure(port.phase() == [0; 4], "Knot joint needs explicit seam bridges")?;
    }
    let (ca, cb) = (a.curve(), b.curve());
    ensure(
        ca.source().first() == cb.source().first() && ca.source().second() == cb.source().second(),
        "Knot joint carriers have different ordered original supports",
    )?;
    ensure(!ca.same_definition(cb)?, "Knot joint carriers are one graph")?;
    let (sa, sb) = (spans(ca)?, spans(cb)?);
    let k = ca.free_axis().index();
    ensure(
        k == cb.free_axis().index(),
        "Knot joint needs one common original free coordinate",
    )?;
    ensure(
        sa[k] != sb[k] && (0..4).all(|j| j == k || sa[j] == sb[j]),
        "Knot joint carriers must differ in the crossed span only",
    )?;
    let surface = if k < 2 { ca.source().first() } else { ca.source().second() };
    let (knots, degree, count, periodic) = if k % 2 == 0 {
        (&surface.knots_u, surface.degree_u, surface.control_points.len(), surface.periodic_u)
    } else {
        (&surface.knots_v, surface.degree_v, surface.control_points[0].len(), surface.periodic_v)
    };
    ensure(!periodic, "Periodic knot lines need an explicit seam proof")?;
    let fa = frames::frame(ca)?;
    let fb = frames::frame(cb)?;
    ensure(
        fa.iter().chain(&fb).all(|f| f[1] > Q::zero()),
        "Knot joint span frames must increase",
    )?;
    let (knot, increasing) = if &fa[k][0] + &fa[k][1] == fb[k][0] {
        (fb[k][0].clone(), true)
    } else if &fb[k][0] + &fb[k][1] == fa[k][0] {
        (fa[k][0].clone(), false)
    } else {
        return Err(refused("Knot joint spans are not adjacent"));
    };
    // Interior knot of multiplicity <= degree: the original surface is
    // continuous across it, so both closed span patches agree on the line.
    let value = knot.to_f64().ok_or_else(|| refused("Knot overflow"))?;
    ensure(q(value)? == knot, "Knot joint value is not an original knot")?;
    let multiplicity = knots.iter().filter(|t| **t == value).count();
    ensure(
        q(knots[degree])? < knot && knot < q(knots[count])? && multiplicity <= degree,
        "Knot joint needs an interior continuous original knot",
    )?;
    // Exact original free clock of the shared selector on both carriers.
    let original = |f: &[[Q; 2]; 4], t: &[Q; 2]| [&f[k][0] + &f[k][1] * &t[0], &f[k][0] + &f[k][1] * &t[1]];
    let fresh_a = bind_original_ss_port(a.end().point().clone(), ca.clone(), [0; 4], limits)?;
    let fresh_b = bind_original_ss_port(b.start().point().clone(), cb.clone(), [0; 4], limits)?;
    for (f, t) in [(&fa, fresh_a.parameter_bounds()), (&fb, fresh_b.parameter_bounds())] {
        ensure(
            original(f, t) == [knot.clone(), knot.clone()],
            "Knot joint selector is not exactly on the knot line",
        )?;
    }
    // One-sided windows: the carrier's own knot-coordinate range, and the
    // common closed box of both carriers in every other coordinate.
    let da = ca.coordinate_domain();
    let db = cb.coordinate_domain();
    let mut wa: [[Q; 2]; 4] = std::array::from_fn(|_| [Q::zero(), Q::zero()]);
    let mut wb = wa.clone();
    for j in 0..4 {
        let map = |f: &[[Q; 2]; 4], x: f64| -> Result<Q> { Ok(&f[j][0] + &f[j][1] * q(x)?) };
        let ra = [map(&fa, da[j][0])?, map(&fa, da[j][1])?];
        let rb = [map(&fb, db[j][0])?, map(&fb, db[j][1])?];
        if j == k {
            wa[j] = ra;
            wb[j] = rb;
        } else {
            let common = [ra[0].clone().max(rb[0].clone()), ra[1].clone().min(rb[1].clone())];
            ensure(common[0] < common[1], "Knot joint carriers share no open transverse box")?;
            wa[j] = common.clone();
            wb[j] = common;
        }
    }
    let (low, high) = if increasing { (&wa, &wb) } else { (&wb, &wa) };
    ensure(
        low[k][0] < knot && low[k][1] == knot && high[k][0] == knot && knot < high[k][1],
        "Knot joint windows do not meet on both sides of the knot",
    )?;
    let windows = [ca.qualify_graph_window(wa.clone())?, cb.qualify_graph_window(wb)?];
    // The joint and both incident subarcs lie strictly inside the open box.
    let inside = |uv: &[[f64; 2]; 4]| -> Result<bool> {
        for j in (0..4).filter(|j| *j != k) {
            if !(wa[j][0] < q(uv[j][0])? && q(uv[j][1])? < wa[j][1]) {
                return Ok(false);
            }
        }
        Ok(true)
    };
    ensure(
        inside(&a.end().point().enclose(limits)?.uv)?,
        "Knot joint is not strictly inside both one-sided windows",
    )?;
    let left = joint::short(a, true, depth, limits)?;
    let right = joint::short(b, false, depth, limits)?;
    for edge in [&left, &right] {
        ensure(
            inside(&edge.enclose(limits)?.uv)?,
            "Knot joint subarcs leave the common open neighbourhood",
        )?;
    }
    Ok(Proof {
        coordinate: k,
        knot,
        increasing,
        windows,
    })
}

impl OriginalSsRootEdge {
    /// Bounded search for a certified cross-carrier joint on an interior knot.
    pub fn qualify_knot_successor(
        &self,
        next: &Self,
        max_depth: usize,
        limits: OriginalSsCurveLimits,
    ) -> Result<QualifiedOriginalSsKnotJoint> {
        ensure(max_depth <= 24, "Knot joint subdivision budget exceeds limit")?;
        let mut last = refused("Original SS knot joint remains unresolved");
        for depth in 0..=max_depth {
            match attempt(self, next, depth, limits) {
                Ok(p) => {
                    return Ok(QualifiedOriginalSsKnotJoint {
                        incoming: self.clone(),
                        outgoing: next.clone(),
                        limits,
                        depth,
                        coordinate: p.coordinate,
                        knot: p.knot,
                        increasing: p.increasing,
                        windows: p.windows,
                    });
                }
                Err(e) => last = e,
            }
        }
        Err(last)
    }
}

impl OriginalSsRootWire {
    /// Like `qualify_joints`, but joints between adjacent span carriers over
    /// one free coordinate use the knot certificate instead of refusing.
    pub fn qualify_knot_aware_joints(
        &self,
        max_depth: usize,
        limits: OriginalSsCurveLimits,
    ) -> Result<Vec<QualifiedOriginalSsWireJoint>> {
        let edges = self.edges();
        edges
            .iter()
            .enumerate()
            .map(|(i, edge)| {
                let next = &edges[(i + 1) % edges.len()];
                if candidate(edge, next) {
                    edge.qualify_knot_successor(next, max_depth, limits)
                        .map(QualifiedOriginalSsWireJoint::Knot)
                } else {
                    edge.qualify_successor(next, max_depth, limits)
                        .map(QualifiedOriginalSsWireJoint::Regular)
                }
            })
            .collect()
    }
}
