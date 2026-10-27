//! Complete closed/singular intersections of admitted original polynomial graphs.
//! Exact checked Bernstein identities, not marching or endpoint proximity.
use super::fiber::FiberSources;
use crate::exact_curve_segments::{exact_add, exact_mul};
use crate::{Result, check, distance_bounds::Interval as I, surface::Surface};
type Poly = [[f64; 3]; 3];
mod diagonalize;
mod line_clip;
mod ranges;
pub use ranges::OriginalGraphSSRange;
#[path = "quadratic_graph/linear_factors.rs"]
mod linear_factors;
#[derive(Clone, Copy, Debug)]
pub struct QuadraticGraphSSLimits {
    pub max_operations: usize,
}
impl Default for QuadraticGraphSSLimits {
    fn default() -> Self {
        Self {
            max_operations: 1024,
        }
    }
}
struct Work {
    used: usize,
    max: usize,
}
impl Work {
    fn new(max: usize) -> Result<Self> {
        check(
            (1..=4096).contains(&max),
            "Exact graph work allowance must be1..4096",
        )?;
        Ok(Self { used: 0, max })
    }
    fn tick(&mut self) -> Result<()> {
        if self.used >= self.max {
            return Err(crate::Error::new(
                crate::RESOURCE_LIMIT,
                "Exact quadratic graph work exhausted",
            ));
        }
        self.used += 1;
        Ok(())
    }
    fn add(&mut self, a: f64, b: f64) -> Result<f64> {
        self.tick()?;
        exact_add(a, b)
            .ok_or_else(|| crate::input("Graph identity requires an exactly representable sum"))
    }
    fn mul(&mut self, a: f64, b: f64) -> Result<f64> {
        self.tick()?;
        exact_mul(a, b)
            .ok_or_else(|| crate::input("Graph identity requires an exactly representable product"))
    }
    fn div(&mut self, a: f64, b: f64) -> Result<f64> {
        check(b != 0., "Zero exact graph divisor")?;
        let q = a / b;
        check(q.is_finite(), "Exact graph quotient exceeded range")?;
        check(
            self.mul(q, b)? == a,
            "Graph identity requires an exact binary64 quotient",
        )?;
        Ok(q)
    }
    fn ia(&mut self, a: I, b: I) -> Result<I> {
        self.tick()?;
        a.add(b)
    }
    fn is(&mut self, a: I, b: I) -> Result<I> {
        self.tick()?;
        a.sub(b)
    }
    fn im(&mut self, a: I, b: I) -> Result<I> {
        self.tick()?;
        a.mul(b)
    }
    fn id(&mut self, a: I, b: I) -> Result<I> {
        self.tick()?;
        a.div(b)
    }
    fn sqrt(&mut self, a: I) -> Result<I> {
        self.tick()?;
        check(a.hi >= 0., "Negative squared graph radius")?;
        I::new(
            a.lo.max(0.).sqrt().next_down().max(0.),
            a.hi.sqrt().next_up(),
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuadraticGraphSSKind {
    Empty,
    ClosedEllipse,
    IsolatedTangentPoint,
    TangentDoubleLine,
    CrossingLines,
    ParallelLineFactors,
    CoincidentPatch,
}
#[derive(Clone, Debug, PartialEq)]
pub enum OriginalGraphSSPoint {
    Normalized([f64; 2]),
    EllipseCardinal(usize),
}
#[derive(Clone, Debug, PartialEq)]
pub struct OriginalGraphSSVertex {
    pub point: OriginalGraphSSPoint,
    pub singular: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub enum OriginalGraphSSEdgeGeometry {
    Segment([[f64; 2]; 2]),
    EllipseQuarter(usize),
}
#[derive(Clone, Debug, PartialEq)]
pub struct OriginalGraphSSEdge {
    pub vertices: [usize; 2],
    pub geometry: OriginalGraphSSEdgeGeometry,
    pub multiplicity: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalGraphSSBranch {
    pub edges: Vec<usize>,
    pub closed: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalGraphSSTrim {
    pub edge: usize,
    pub surface: usize,
}
#[derive(Clone, Debug, PartialEq)]
struct Data {
    xy: [f64; 4],
    domains: [[[f64; 2]; 2]; 2],
    heights: [Poly; 2],
    /// Centered diagonal polynomial, mapped to original normalized UV by basis.
    diagonal: [f64; 3],
    center: [f64; 2],
    /// Original normalized UV displacement from centered diagonal coordinates.
    basis: [[f64; 2]; 2],
    kind: QuadraticGraphSSKind,
    vertices: Vec<OriginalGraphSSVertex>,
    edges: Vec<OriginalGraphSSEdge>,
    branches: Vec<OriginalGraphSSBranch>,
    trims: Vec<OriginalGraphSSTrim>,
    work: usize,
}
#[derive(Clone, Debug)]
pub struct QualifiedQuadraticGraphSS {
    source: FiberSources,
    limits: QuadraticGraphSSLimits,
    data: Data,
}
#[derive(Clone, Debug)]
pub struct OriginalGraphSSStation {
    pub normalized_uv: [[f64; 2]; 2],
    pub first_uv: [[f64; 2]; 2],
    pub second_uv: [[f64; 2]; 2],
    /// Contains the selected exact original world point, not a sampled point.
    pub world: [[f64; 2]; 3],
    pub operations: usize,
}
fn power(values: &[f64], w: &mut Work) -> Result<[f64; 3]> {
    let linear = w.add(values[1], -values[0])?;
    if values.len() == 2 {
        return Ok([values[0], linear, 0.]);
    }
    let linear = w.mul(2., linear)?;
    let twice = w.mul(-2., values[1])?;
    let a = w.add(values[0], twice)?;
    Ok([values[0], linear, w.add(a, values[2])?])
}
fn graph(s: &Surface, w: &mut Work) -> Result<([f64; 4], [[f64; 2]; 2], Poly)> {
    check(
        (1..=2).contains(&s.degree_u)
            && (1..=2).contains(&s.degree_v)
            && s.control_points.len() == s.degree_u + 1
            && s.control_points
                .iter()
                .all(|row| row.len() == s.degree_v + 1 && row.iter().all(|p| p.len() == 3))
            && s.knots_u.len() == 2 * (s.degree_u + 1)
            && s.knots_v.len() == 2 * (s.degree_v + 1)
            && !s.periodic_u
            && !s.periodic_v,
        "Need a single original degree1/2 unwrapped polynomial graph patch",
    )?;
    s.validate()?;
    check(
        s.weights.iter().flatten().all(|x| *x == s.weights[0][0]),
        "Rational varying weights need a separate graph proof",
    )?;
    let domain = [
        [s.knots_u[0], *s.knots_u.last().unwrap()],
        [s.knots_v[0], *s.knots_v.last().unwrap()],
    ];
    for (axis, (knots, degree)) in [(&s.knots_u, s.degree_u), (&s.knots_v, s.degree_v)]
        .into_iter()
        .enumerate()
    {
        check(
            knots[..degree + 1].iter().all(|v| *v == domain[axis][0])
                && knots[degree + 1..].iter().all(|v| *v == domain[axis][1]),
            "Graph must retain complete original clamped knot spans",
        )?;
        w.is(I::point(domain[axis][1]), I::point(domain[axis][0]))?;
    }
    let x = s.control_points[0][0][0];
    let y = s.control_points[0][0][1];
    let dx = w.add(s.control_points[s.degree_u][0][0], -x)?;
    let dy = w.add(s.control_points[0][s.degree_v][1], -y)?;
    check(
        dx != 0. && dy != 0.,
        "Original graph XY projection must be invertible",
    )?;
    for i in 0..=s.degree_u {
        for j in 0..=s.degree_v {
            let u = i as f64 / s.degree_u as f64;
            let v = j as f64 / s.degree_v as f64;
            let a = w.mul(dx, u)?;
            let b = w.mul(dy, v)?;
            check(
                s.control_points[i][j][0] == w.add(x, a)?
                    && s.control_points[i][j][1] == w.add(y, b)?,
                "Original graph XY coefficients are not exactly affine",
            )?;
        }
    }
    let mut rows = Vec::new();
    for row in &s.control_points {
        rows.push(power(&row.iter().map(|p| p[2]).collect::<Vec<_>>(), w)?);
    }
    let mut height = [[0.; 3]; 3];
    for v in 0..3 {
        let p = power(&rows.iter().map(|row| row[v]).collect::<Vec<_>>(), w)?;
        for u in 0..3 {
            height[u][v] = p[u];
        }
    }
    Ok(([x, dx, y, dy], domain, height))
}
fn segment(d: &mut Data, points: [[f64; 2]; 2], multiplicity: usize) {
    let first = d.vertices.len();
    for p in points {
        d.vertices.push(OriginalGraphSSVertex {
            point: OriginalGraphSSPoint::Normalized(p),
            singular: false,
        });
    }
    let edge = d.edges.len();
    d.edges.push(OriginalGraphSSEdge {
        vertices: [first, first + 1],
        geometry: OriginalGraphSSEdgeGeometry::Segment(points),
        multiplicity,
    });
    d.branches.push(OriginalGraphSSBranch {
        edges: vec![edge],
        closed: false,
    });
}
fn derive(source: &FiberSources, limits: QuadraticGraphSSLimits) -> Result<Data> {
    let mut w = Work::new(limits.max_operations)?;
    let (xy, first, p) = graph(source.first(), &mut w)?;
    let (second_xy, second, q) = graph(source.second(), &mut w)?;
    check(
        xy == second_xy,
        "Both original graph patches need the same affine normalized XY correspondence",
    )?;
    let mut f = [[0.; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            f[i][j] = w.add(p[i][j], -q[i][j])?;
            if !matches!((i, j), (0, 0) | (1, 0) | (2, 0) | (0, 1) | (0, 2) | (1, 1)) {
                check(
                    f[i][j] == 0.,
                    "Higher mixed graph difference needs another exact classifier",
                )?;
            }
        }
    }
    let ([mut a, mut b, mut k], center, basis) = diagonalize::form(&f, &mut w)?;
    if a < 0. && b <= 0. || a == 0. && b < 0. {
        a = -a;
        b = -b;
        k = -k;
    }
    let mut d = Data {
        xy,
        domains: [first, second],
        heights: [p, q],
        diagonal: [a, b, k],
        center,
        basis,
        kind: QuadraticGraphSSKind::Empty,
        vertices: vec![],
        edges: vec![],
        branches: vec![],
        trims: vec![],
        work: 0,
    };
    if a == 0. && b == 0. {
        if k == 0. {
            d.kind = QuadraticGraphSSKind::CoincidentPatch;
        }
    } else if a > 0. && b > 0. {
        if k == 0. {
            if center.iter().all(|c| (0.0..=1.0).contains(c)) {
                d.kind = QuadraticGraphSSKind::IsolatedTangentPoint;
                d.vertices.push(OriginalGraphSSVertex {
                    point: OriginalGraphSSPoint::Normalized(center),
                    singular: true,
                });
            }
        } else if k < 0. {
            diagonalize::contained(&d, &mut w)?;
            d.kind = QuadraticGraphSSKind::ClosedEllipse;
            for quadrant in 0..4 {
                d.vertices.push(OriginalGraphSSVertex {
                    point: OriginalGraphSSPoint::EllipseCardinal(quadrant),
                    singular: false,
                });
                d.edges.push(OriginalGraphSSEdge {
                    vertices: [quadrant, (quadrant + 1) % 4],
                    geometry: OriginalGraphSSEdgeGeometry::EllipseQuarter(quadrant),
                    multiplicity: 1,
                });
            }
            d.branches.push(OriginalGraphSSBranch {
                edges: vec![0, 1, 2, 3],
                closed: true,
            });
        }
    } else if a == 0. || b == 0. {
        if k < 0. {
            linear_factors::parallel(&mut d, &mut w)?;
        } else if k == 0. {
            line_clip::double(&mut d, &mut w)?;
        }
    } else {
        linear_factors::crossing(&mut d, &mut w)?;
    }
    for edge in 0..d.edges.len() {
        for surface in 0..2 {
            d.trims.push(OriginalGraphSSTrim { edge, surface });
        }
    }
    d.work = w.used;
    Ok(d)
}
impl QualifiedQuadraticGraphSS {
    pub fn source(&self) -> &FiberSources {
        &self.source
    }
    pub fn kind(&self) -> QuadraticGraphSSKind {
        self.data.kind
    }
    pub fn vertices(&self) -> &[OriginalGraphSSVertex] {
        &self.data.vertices
    }
    pub fn edges(&self) -> &[OriginalGraphSSEdge] {
        &self.data.edges
    }
    pub fn branches(&self) -> &[OriginalGraphSSBranch] {
        &self.data.branches
    }
    pub fn trims(&self) -> &[OriginalGraphSSTrim] {
        &self.data.trims
    }
    pub fn operations(&self) -> usize {
        self.data.work
    }
    pub fn covers_entire_surfaces(&self) -> bool {
        true
    }
    pub fn reverify(&self) -> Result<()> {
        check(
            derive(&self.source, self.limits)? == self.data,
            "Original closed/singular graph ownership changed",
        )
    }
    fn ellipse(&self, quadrant: usize, t: f64, w: &mut Work) -> Result<[I; 2]> {
        self.ellipse_interval(quadrant, I::point(t), w)
    }
    fn ellipse_interval(&self, quadrant: usize, t: I, w: &mut Work) -> Result<[I; 2]> {
        check(
            self.kind() == QuadraticGraphSSKind::ClosedEllipse && quadrant < 4,
            "Unknown original ellipse chart",
        )?;
        let square = w.im(t, t)?;
        let den = w.ia(I::point(1.), square)?;
        let num = w.is(I::point(1.), square)?;
        let x = w.id(num, den)?;
        let twice = w.im(I::point(2.), t)?;
        let y = w.id(twice, den)?;
        let neg = |i: I| I {
            lo: -i.hi,
            hi: -i.lo,
        };
        let unit = match quadrant {
            0 => [x, y],
            1 => [neg(y), x],
            2 => [neg(x), neg(y)],
            _ => [y, neg(x)],
        };
        let mut delta = [I::point(0.); 2];
        for axis in 0..2 {
            let square = w.id(
                I::point(-self.data.diagonal[2]),
                I::point(self.data.diagonal[axis]),
            )?;
            let radius = w.sqrt(square)?;
            delta[axis] = w.im(radius, unit[axis])?;
        }
        let mut uv = [I::point(0.); 2];
        for axis in 0..2 {
            uv[axis] = I::point(self.data.center[axis]);
            for j in 0..2 {
                let term = w.im(I::point(self.data.basis[axis][j]), delta[j])?;
                uv[axis] = w.ia(uv[axis], term)?;
            }
        }
        Ok(uv)
    }
    fn enclose(&self, uv: [I; 2], w: &mut Work) -> Result<OriginalGraphSSStation> {
        let mut world = [I::point(0.); 3];
        for axis in 0..2 {
            let term = w.im(I::point(self.data.xy[2 * axis + 1]), uv[axis])?;
            world[axis] = w.ia(I::point(self.data.xy[2 * axis]), term)?;
        }
        let mut rows = [I::point(0.); 3];
        for i in 0..3 {
            let mut row = I::point(self.data.heights[0][i][2]);
            for j in (0..2).rev() {
                let product = w.im(row, uv[1])?;
                row = w.ia(product, I::point(self.data.heights[0][i][j]))?;
            }
            rows[i] = row;
        }
        let a = w.im(rows[2], uv[0])?;
        let a = w.ia(a, rows[1])?;
        let a = w.im(a, uv[0])?;
        world[2] = w.ia(a, rows[0])?;
        let mut native = [[[0.; 2]; 2]; 2];
        for surface in 0..2 {
            for axis in 0..2 {
                let span = w.is(
                    I::point(self.data.domains[surface][axis][1]),
                    I::point(self.data.domains[surface][axis][0]),
                )?;
                let term = w.im(span, uv[axis])?;
                let value = w.ia(I::point(self.data.domains[surface][axis][0]), term)?;
                native[surface][axis] = [value.lo, value.hi];
            }
        }
        Ok(OriginalGraphSSStation {
            normalized_uv: uv.map(|v| [v.lo, v.hi]),
            first_uv: native[0],
            second_uv: native[1],
            world: world.map(|v| [v.lo, v.hi]),
            operations: w.used,
        })
    }
    pub fn edge_station(
        &self,
        edge: usize,
        t: f64,
        max_operations: usize,
    ) -> Result<OriginalGraphSSStation> {
        check(
            t.is_finite() && (0.0..=1.0).contains(&t),
            "Original edge clock must lie in0..1",
        )?;
        let mut w = Work::new(max_operations)?;
        let edge = self
            .edges()
            .get(edge)
            .ok_or_else(|| crate::input("Unknown original graph edge"))?;
        let uv = match edge.geometry {
            OriginalGraphSSEdgeGeometry::EllipseQuarter(q) => self.ellipse(q, t, &mut w)?,
            OriginalGraphSSEdgeGeometry::Segment(points) => {
                let mut uv = [I::point(0.); 2];
                for axis in 0..2 {
                    let delta = w.is(I::point(points[1][axis]), I::point(points[0][axis]))?;
                    let delta = w.im(delta, I::point(t))?;
                    uv[axis] = w.ia(I::point(points[0][axis]), delta)?;
                }
                uv
            }
        };
        self.enclose(uv, &mut w)
    }
    pub fn vertex_station(
        &self,
        vertex: usize,
        max_operations: usize,
    ) -> Result<OriginalGraphSSStation> {
        let mut w = Work::new(max_operations)?;
        let vertex = self
            .vertices()
            .get(vertex)
            .ok_or_else(|| crate::input("Unknown original graph vertex"))?;
        let uv = match vertex.point {
            OriginalGraphSSPoint::Normalized(p) => p.map(I::point),
            OriginalGraphSSPoint::EllipseCardinal(q) => self.ellipse(q, 0., &mut w)?,
        };
        self.enclose(uv, &mut w)
    }
    pub fn overlap_station(
        &self,
        uv: [f64; 2],
        max_operations: usize,
    ) -> Result<OriginalGraphSSStation> {
        check(
            self.kind() == QuadraticGraphSSKind::CoincidentPatch
                && uv.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v)),
            "Need an original coincident patch point in0..1",
        )?;
        let mut w = Work::new(max_operations)?;
        self.enclose(uv.map(I::point), &mut w)
    }
}
pub fn qualify_quadratic_graph_ss(
    source: &FiberSources,
    limits: QuadraticGraphSSLimits,
) -> Result<QualifiedQuadraticGraphSS> {
    let data = derive(source, limits)?;
    Ok(QualifiedQuadraticGraphSS {
        source: source.clone(),
        limits,
        data,
    })
}

#[cfg(test)]
mod proof_tests {
    use super::*;
    #[test]
    fn replay_rejects_forged_closure_multiplicity_and_rebound_original_world_height() {
        let q = [0.25, -0.25, 0.25];
        let surface = |z: bool| Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| {
                            vec![
                                i as f64 / 2.,
                                j as f64 / 2.,
                                if z { q[i] + q[j] - 0.0625 } else { 0. },
                            ]
                        })
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let mut a = surface(true);
        let mut b = surface(false);
        let source = FiberSources::new(&a, &b).unwrap();
        let o = qualify_quadratic_graph_ss(&source, Default::default()).unwrap();
        let mut bad = o.clone();
        bad.data.branches[0].closed = false;
        assert!(bad.reverify().is_err());
        let mut bad = o.clone();
        bad.data.edges[0].multiplicity = 2;
        assert!(bad.reverify().is_err());
        let mut bad = o.clone();
        bad.data.basis[0][1] = 0.5;
        assert!(bad.reverify().is_err());
        for s in [&mut a, &mut b] {
            for row in &mut s.control_points {
                for p in row {
                    p[2] += 1.;
                }
            }
        }
        let mut bad = o;
        bad.source = FiberSources::new(&a, &b).unwrap();
        assert!(bad.reverify().is_err());
    }
}
