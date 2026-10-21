//! Explicit envelope chart: solve the linear meshing equation for source roll.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartDomain {
    pub u: [f64; 2],
    pub beta: [f64; 2],
}
#[derive(Clone, Copy)]
struct J {
    x: I,
    d: [I; 2],
}
impl J {
    fn c(x: I) -> Self {
        Self {
            x,
            d: [I::point(0.); 2],
        }
    }
    fn variable(x: I, axis: usize) -> Self {
        let mut j = Self::c(x);
        j.d[axis] = I::point(1.);
        j
    }
    fn add(self, b: Self) -> Result<Self> {
        Ok(Self {
            x: self.x.add(b.x)?,
            d: [self.d[0].add(b.d[0])?, self.d[1].add(b.d[1])?],
        })
    }
    fn sub(self, b: Self) -> Result<Self> {
        Ok(Self {
            x: self.x.sub(b.x)?,
            d: [self.d[0].sub(b.d[0])?, self.d[1].sub(b.d[1])?],
        })
    }
    fn mul(self, b: Self) -> Result<Self> {
        Ok(Self {
            x: self.x.mul(b.x)?,
            d: [
                self.d[0].mul(b.x)?.add(self.x.mul(b.d[0])?)?,
                self.d[1].mul(b.x)?.add(self.x.mul(b.d[1])?)?,
            ],
        })
    }
    fn div(self, b: Self) -> Result<Self> {
        let denominator = b.x.mul(b.x)?;
        Ok(Self {
            x: self.x.div_signed(b.x)?,
            d: [
                self.d[0]
                    .mul(b.x)?
                    .sub(self.x.mul(b.d[0])?)?
                    .div(denominator)?,
                self.d[1]
                    .mul(b.x)?
                    .sub(self.x.mul(b.d[1])?)?
                    .div(denominator)?,
            ],
        })
    }
    fn sin_cos(self) -> Result<(Self, Self)> {
        let (s, c) = crate::involute::certified_trig(self.x)?;
        Ok((
            Self {
                x: s,
                d: [c.mul(self.d[0])?, c.mul(self.d[1])?],
            },
            Self {
                x: c,
                d: [neg(s.mul(self.d[0])?)?, neg(s.mul(self.d[1])?)?],
            },
        ))
    }
}
impl super::expression::Scalar for J {
    fn c(x: I) -> Self {
        J::c(x)
    }
    fn value(self) -> I {
        self.x
    }
    fn add(self, b: Self) -> Result<Self> {
        J::add(self, b)
    }
    fn sub(self, b: Self) -> Result<Self> {
        J::sub(self, b)
    }
    fn mul(self, b: Self) -> Result<Self> {
        J::mul(self, b)
    }
    fn div(self, b: Self) -> Result<Self> {
        J::div(self, b)
    }
    fn sin_cos(self) -> Result<(Self, Self)> {
        J::sin_cos(self)
    }
    fn sqrt(self) -> Result<Self> {
        let x = super::expression::sqrt_positive(self.x)?;
        let denominator = I::point(2.).mul(x)?;
        Ok(Self {
            x,
            d: [self.d[0].div(denominator)?, self.d[1].div(denominator)?],
        })
    }
    fn asin(self) -> Result<Self> {
        let denominator = super::expression::sqrt_positive(I::point(1.).sub(self.x.mul(self.x)?)?)?;
        Ok(Self {
            x: super::expression::asin_interval(self.x)?,
            d: [self.d[0].div(denominator)?, self.d[1].div(denominator)?],
        })
    }
    fn restrict_value(mut self, x: I) -> Result<Self> {
        self.x = self.x.intersect(x.lo, x.hi)?;
        Ok(self)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ChartBounds {
    point: [[f64; 2]; 3],
    du: [[f64; 2]; 3],
    dbeta: [[f64; 2]; 3],
    cross: [[f64; 2]; 3],
    roll: [f64; 2],
    phase: [f64; 2],
    denominator: [f64; 2],
    regular: bool,
}
impl ChartBounds {
    pub fn point(&self) -> [[f64; 2]; 3] {
        self.point
    }
    pub fn du(&self) -> [[f64; 2]; 3] {
        self.du
    }
    pub fn dbeta(&self) -> [[f64; 2]; 3] {
        self.dbeta
    }
    pub fn cross_product(&self) -> [[f64; 2]; 3] {
        self.cross
    }
    pub fn source_roll(&self) -> [f64; 2] {
        self.roll
    }
    pub fn worm_rotation(&self) -> [f64; 2] {
        self.phase
    }
    pub fn denominator(&self) -> [f64; 2] {
        self.denominator
    }
    pub fn regularity_proven(&self) -> bool {
        self.regular
    }
}
pub(super) fn validate(spec: Spec, flank: Flank, domain: ChartDomain) -> Result<()> {
    check(
        domain.u.iter().chain(&domain.beta).all(|x| x.is_finite())
            && 0. <= domain.u[0]
            && domain.u[0] <= domain.u[1]
            && domain.u[1] <= 1.
            && -1.4 <= domain.beta[0]
            && domain.beta[0] <= domain.beta[1]
            && domain.beta[1] <= 1.4,
        "Wheel chart requires ordered U in [0,1] and contact angle in [-1.4,1.4]",
    )?;
    state(spec, flank, mid(iv(domain.u)?), 0.)?;
    Ok(())
}
/// Whole-domain original-source positions and first derivatives. With
/// K=k*cos(beta)-(h/r)*sin(beta), contact Z=(rp-r*cos(beta))/K and
/// theta=(Z-profile_z)/h are the unique solution whenever h*K != 0.
/// This chart crosses angular-root folds; it does not prove global injectivity,
/// stock/cutter contact admissibility or a complete wheel domain.
pub fn enclose_chart(spec: Spec, flank: Flank, domain: ChartDomain) -> Result<ChartBounds> {
    validate(spec, flank, domain)?;
    let u = J::variable(iv(domain.u)?, 0);
    let beta = J::variable(iv(domain.beta)?, 1);
    let result = super::expression::evaluate(spec, flank, u, beta)?;
    let xyz = result.point;
    let theta = result.roll;
    let phi = result.phi;
    let denominator = result.denominator;
    let du = xyz.map(|j| j.d[0]);
    let db = xyz.map(|j| j.d[1]);
    let cross = [
        du[1].mul(db[2])?.sub(du[2].mul(db[1])?)?,
        du[2].mul(db[0])?.sub(du[0].mul(db[2])?)?,
        du[0].mul(db[1])?.sub(du[1].mul(db[0])?)?,
    ];
    let regular = cross.iter().any(|x| x.lo > 0. || x.hi < 0.);
    Ok(ChartBounds {
        point: xyz.map(|j| pair(j.x)),
        du: du.map(pair),
        dbeta: db.map(pair),
        cross: cross.map(pair),
        roll: pair(theta.x),
        phase: pair(phi.x),
        denominator: pair(denominator.x),
        regular,
    })
}

#[derive(Clone, Debug)]
enum Node {
    Leaf {
        domain: ChartDomain,
        bounds: Option<ChartBounds>,
    },
    Split {
        axis: usize,
        cut: f64,
        children: [usize; 2],
    },
}
pub struct ChartCover {
    spec: Spec,
    flank: Flank,
    domain: ChartDomain,
    nodes: Vec<Node>,
    complete: bool,
}
impl ChartCover {
    pub fn specification(&self) -> Spec {
        self.spec
    }
    pub fn flank(&self) -> Flank {
        self.flank
    }
    pub fn domain(&self) -> ChartDomain {
        self.domain
    }
    pub fn complete(&self) -> bool {
        self.complete
    }
    pub fn cells(&self) -> usize {
        self.nodes
            .iter()
            .filter(|n| matches!(n, Node::Leaf { .. }))
            .count()
    }
    pub fn leaves(&self) -> impl Iterator<Item = (ChartDomain, Option<&ChartBounds>)> {
        self.nodes.iter().filter_map(|n| match n {
            Node::Leaf { domain, bounds } => Some((*domain, bounds.as_ref())),
            _ => None,
        })
    }
    /// Replay the entire partition, original source boxes and Jacobian tests.
    /// Unvisited/duplicate nodes, gaps and stale bounds cannot authorize coverage.
    pub fn reverify(&self) -> Result<()> {
        validate(self.spec, self.flank, self.domain)?;
        check(
            !self.nodes.is_empty() && self.nodes.len() <= 65535,
            "Wheel chart node inventory is invalid",
        )?;
        let mut stack = vec![(0, self.domain, 0usize)];
        let mut seen = vec![false; self.nodes.len()];
        let mut complete = true;
        while let Some((index, domain, depth)) = stack.pop() {
            check(
                index < self.nodes.len() && !seen[index] && depth <= 32,
                "Wheel chart partition is cyclic, duplicate or too deep",
            )?;
            seen[index] = true;
            match &self.nodes[index] {
                Node::Leaf {
                    domain: saved,
                    bounds,
                } => {
                    check(*saved == domain, "Wheel chart leaf domain changed")?;
                    match bounds {
                        Some(b) => {
                            let fresh = enclose_chart(self.spec, self.flank, domain)?;
                            check(fresh == *b, "Wheel chart original bounds changed")?;
                            complete &= fresh.regular;
                        }
                        None => complete = false,
                    }
                }
                Node::Split {
                    axis,
                    cut,
                    children,
                } => {
                    let (a, b) = split(domain, *axis, *cut)?;
                    stack.push((children[1], b, depth + 1));
                    stack.push((children[0], a, depth + 1));
                }
            }
        }
        check(
            seen.iter().all(|x| *x) && complete == self.complete,
            "Wheel chart complete source partition or admission receipt changed",
        )
    }
}
fn split(domain: ChartDomain, axis: usize, cut: f64) -> Result<(ChartDomain, ChartDomain)> {
    check(axis < 2 && cut.is_finite(), "Invalid wheel chart split")?;
    let range = if axis == 0 { domain.u } else { domain.beta };
    check(
        range[0] < cut && cut < range[1],
        "Wheel chart cut does not strictly divide its parent",
    )?;
    let mut a = domain;
    let mut b = domain;
    if axis == 0 {
        a.u[1] = cut;
        b.u[0] = cut;
    } else {
        a.beta[1] = cut;
        b.beta[0] = cut;
    }
    Ok((a, b))
}
/// Complete adaptive cover of the requested original chart rectangle. A leaf
/// is admitted only by a whole-cell denominator and nonzero Jacobian proof.
/// Budget/actual singularities remain unqualified leaves, never exclusions.
pub fn cover_chart(
    spec: Spec,
    flank: Flank,
    domain: ChartDomain,
    max_nodes: usize,
    max_depth: usize,
) -> Result<ChartCover> {
    validate(spec, flank, domain)?;
    check(
        domain.u[0] < domain.u[1]
            && domain.beta[0] < domain.beta[1]
            && (1..=65535).contains(&max_nodes)
            && max_depth <= 32,
        "Wheel chart cover needs a nonempty rectangle and bounded work",
    )?;
    let mut nodes = vec![Node::Leaf {
        domain,
        bounds: None,
    }];
    let mut pending = vec![(0, domain, 0usize)];
    let mut complete = true;
    while let Some((index, cell, depth)) = pending.pop() {
        let bounds = enclose_chart(spec, flank, cell).ok();
        if bounds.as_ref().is_some_and(|b| b.regular) {
            nodes[index] = Node::Leaf {
                domain: cell,
                bounds,
            };
            continue;
        }
        if depth == max_depth || nodes.len() + 2 > max_nodes {
            complete = false;
            nodes[index] = Node::Leaf {
                domain: cell,
                bounds,
            };
            continue;
        }
        let axis = depth % 2;
        let range = if axis == 0 { cell.u } else { cell.beta };
        let cut = range[0] + (range[1] - range[0]) * 0.5;
        if cut <= range[0] || cut >= range[1] {
            complete = false;
            nodes[index] = Node::Leaf {
                domain: cell,
                bounds,
            };
            continue;
        }
        let (a, b) = split(cell, axis, cut)?;
        let children = [nodes.len(), nodes.len() + 1];
        nodes.push(Node::Leaf {
            domain: a,
            bounds: None,
        });
        nodes.push(Node::Leaf {
            domain: b,
            bounds: None,
        });
        nodes[index] = Node::Split {
            axis,
            cut,
            children,
        };
        pending.push((children[1], b, depth + 1));
        pending.push((children[0], a, depth + 1));
    }
    let report = ChartCover {
        spec,
        flank,
        domain,
        nodes,
        complete,
    };
    report.reverify()?;
    Ok(report)
}

/// The caller has validated the original source and the complete requested
/// domain; every query rectangle must be a contained part of that domain.
pub(super) fn radial_prepared(
    source: &super::expression::Prepared,
    domain: ChartDomain,
) -> Result<(I, I)> {
    let e = super::expression::radial_prepared(
        source,
        J::variable(iv(domain.u)?, 0),
        J::variable(iv(domain.beta)?, 1),
    )?;
    Ok((e.x, e.d[0]))
}

/// Gradient of the invariant original squared radial distance.
pub(super) fn radial_gradient(spec: Spec, flank: Flank, domain: ChartDomain) -> Result<[I; 2]> {
    validate(spec, flank, domain)?;
    Ok(super::expression::radial_squared(
        spec,
        flank,
        J::variable(iv(domain.u)?, 0),
        J::variable(iv(domain.beta)?, 1),
    )?
    .d)
}

pub(super) fn root_jets(
    spec: Spec,
    part: RootPart,
    domain: ChartDomain,
) -> Result<([I; 3], [I; 3], [I; 3])> {
    let p = super::expression::root(
        spec,
        part,
        J::variable(iv(domain.u)?, 0),
        J::variable(iv(domain.beta)?, 1),
    )?
    .point;
    Ok((p.map(|x| x.x), p.map(|x| x.d[0]), p.map(|x| x.d[1])))
}

pub(super) fn axial_root_jets(
    spec: Spec,
    part: RootPart,
    domain: AxialDomain,
) -> Result<([I; 3], [I; 3], [I; 3])> {
    let p = super::expression::axial_root(
        spec,
        part,
        J::variable(iv(domain.t)?, 0),
        J::variable(iv(domain.y)?, 1),
    )?
    .point;
    Ok((p.map(|x| x.x), p.map(|x| x.d[0]), p.map(|x| x.d[1])))
}

pub(super) fn axial_radial_jets(
    source: &super::expression::Prepared,
    d: AxialDomain,
) -> Result<(I, [I; 2])> {
    let r = super::expression::axial_radial(
        source,
        J::variable(iv(d.t)?, 0),
        J::variable(iv(d.y)?, 1),
    )?;
    Ok((r.x, r.d))
}
pub(super) fn axial_stock_jets(
    spec: Spec,
    flank: Flank,
    domain: AxialDomain,
    root: [f64; 2],
    root_dy: I,
) -> Result<([I; 3], [I; 3], [I; 3])> {
    let t = J::variable(iv(domain.t)?, 0);
    let y = J::variable(iv(domain.y)?, 1);
    let root = J {
        x: iv(root)?,
        d: [I::point(0.), root_dy],
    };
    let crest = J::c(I::point(if flank == Flank::Descending { 0. } else { 1. }));
    let mut u = crest.add(root.sub(crest)?.mul(t)?)?;
    u.x = u.x.intersect(0., 1.)?;
    let p = super::expression::axial_flank(spec, flank, u, y)?.point;
    Ok((p.map(|x| x.x), p.map(|x| x.d[0]), p.map(|x| x.d[1])))
}

pub(super) fn axial_tip_jets(
    spec: Spec,
    radius: f64,
    d: AxialDomain,
    roots: [[f64; 2]; 2],
    root_dy: [I; 2],
    point_bounds: [[[f64; 2]; 3]; 2],
) -> Result<([I; 3], [I; 3], [I; 3])> {
    let y = J::variable(iv(d.y)?, 1);
    let t = J::variable(iv(d.t)?, 0);
    let mut a = super::expression::axial_flank(
        spec,
        Flank::Descending,
        J {
            x: iv(roots[0])?,
            d: [I::point(0.), root_dy[0]],
        },
        y,
    )?
    .point;
    let mut b = super::expression::axial_flank(
        spec,
        Flank::Ascending,
        J {
            x: iv(roots[1])?,
            d: [I::point(0.), root_dy[1]],
        },
        y,
    )?
    .point;
    for k in 0..3 {
        a[k].x = a[k]
            .x
            .intersect(point_bounds[0][k][0], point_bounds[0][k][1])?;
        b[k].x = b[k]
            .x
            .intersect(point_bounds[1][k][0], point_bounds[1][k][1])?;
    }
    let p = super::expression::tip_blend(radius, t, y, a, b)?;
    Ok((p.map(|x| x.x), p.map(|x| x.d[0]), p.map(|x| x.d[1])))
}
