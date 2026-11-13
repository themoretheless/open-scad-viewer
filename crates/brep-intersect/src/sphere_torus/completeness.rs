//! Completeness certificate for traced sphere/torus components.
//!
//! On each stereographic sphere patch the torus equation in homogeneous
//! form is a tensor-product Bernstein polynomial `f(u, v)` of bidegree
//! (8, 8) (the patch is rational biquadratic, the torus quartic). Its zero
//! set over the patch square is the intersection curve as seen in that
//! chart, and the eight squares cover the sphere.
//!
//! Each square is subdivided until every box is either *empty* (all
//! coefficients of `f` share a strict sign) or *monotone* (all coefficients
//! of `f_u`, or of `f_v`, share a strict sign). A monotone box contains no
//! closed zero curve (a closed level curve has a point with a vertical and
//! one with a horizontal tangent, where `f_v` resp. `f_u` vanishes), so
//! every zero arc meeting it crosses its boundary. On the boundary of every
//! monotone box the roots of `f` are isolated and counted (Bernstein sign
//! variations with subdivision) and compared with the crossings of the
//! traced curves, found the same way from the chart coordinates of each
//! traced cubic span (ratios of cubics). Equal counts everywhere leave no
//! room for an untraced component: it would cross the boundary of some
//! monotone box and add roots there.
//!
//! Anything ambiguous (a root at a box corner, a crossing too close to an
//! edge end, the work budget) answers `false`; the caller then keeps the
//! component set uncertified.
use super::*;
use nurbs_core::interval_eval::Interval;

type Grid = Vec<Vec<Interval>>;

fn binomial(n: usize, k: usize) -> f64 {
    let mut v = 1.;
    for j in 0..k.min(n - k) {
        v = v * (n - j) as f64 / (j + 1) as f64;
    }
    v.round()
}

/// Tensor-product Bernstein polynomial on the unit square.
#[derive(Clone)]
struct Bern2(Grid);

impl Bern2 {
    fn degree(&self) -> (usize, usize) {
        (self.0.len() - 1, self.0[0].len() - 1)
    }
    fn constant(x: f64) -> Self {
        Self(vec![vec![Interval::point(x)]])
    }
    fn elevate(&self, m: usize, n: usize) -> Result<Self> {
        let (p, q) = self.degree();
        if (p, q) == (m, n) {
            return Ok(self.clone());
        }
        // Elevation is multiplication by the all-ones polynomial of the
        // missing bidegree.
        self.mul(&Self(vec![vec![Interval::point(1.); n - q + 1]; m - p + 1]))
    }
    fn zip(&self, other: &Self, op: fn(Interval, Interval) -> Result<Interval>) -> Result<Self> {
        let (p, q) = self.degree();
        let (r, s) = other.degree();
        let (m, n) = (p.max(r), q.max(s));
        let (a, b) = (self.elevate(m, n)?, other.elevate(m, n)?);
        Ok(Self(
            a.0.iter()
                .zip(&b.0)
                .map(|(x, y)| x.iter().zip(y).map(|(x, y)| op(*x, *y)).collect())
                .collect::<Result<_>>()?,
        ))
    }
    fn add(&self, other: &Self) -> Result<Self> {
        self.zip(other, |x, y| x.add(y))
    }
    fn sub(&self, other: &Self) -> Result<Self> {
        self.zip(other, |x, y| x.sub(y))
    }
    fn scale(&self, s: f64) -> Result<Self> {
        Ok(Self(
            self.0
                .iter()
                .map(|r| r.iter().map(|x| x.mul(Interval::point(s))).collect())
                .collect::<Result<_>>()?,
        ))
    }
    fn mul(&self, other: &Self) -> Result<Self> {
        let (p, q) = self.degree();
        let (r, s) = other.degree();
        let mut out = vec![vec![Interval::point(0.); q + s + 1]; p + r + 1];
        for i in 0..=p {
            for j in 0..=q {
                for k in 0..=r {
                    for l in 0..=s {
                        let w = binomial(p, i) * binomial(q, j) * binomial(r, k) * binomial(s, l)
                            / (binomial(p + r, i + k) * binomial(q + s, j + l));
                        let term = self.0[i][j].mul(other.0[k][l])?.mul(Interval::point(w))?;
                        out[i + k][j + l] = out[i + k][j + l].add(term)?;
                    }
                }
            }
        }
        Ok(Self(out))
    }
    fn hull(&self) -> (f64, f64) {
        self.0.iter().flatten().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), x| {
            (lo.min(x.lo), hi.max(x.hi))
        })
    }
    /// Sign of the difference coefficients along u (`axis = 0`) or v.
    fn derivative_sign(&self, axis: usize) -> bool {
        let g = &self.0;
        let (p, q) = self.degree();
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        let diff = |i: usize, j: usize| -> Option<Interval> {
            if axis == 0 {
                (i < p).then(|| g[i + 1][j].sub(g[i][j]).ok()).flatten()
            } else {
                (j < q).then(|| g[i][j + 1].sub(g[i][j]).ok()).flatten()
            }
        };
        for i in 0..=p {
            for j in 0..=q {
                if let Some(d) = diff(i, j) {
                    lo = lo.min(d.lo);
                    hi = hi.max(d.hi);
                }
            }
        }
        lo > 0. || hi < 0.
    }
    /// Split along u (`axis = 0`) or v at ratio `t`.
    fn halves(&self, axis: usize, t: f64) -> Result<(Self, Self)> {
        let transpose = |g: &Grid| -> Grid {
            (0..g[0].len()).map(|j| g.iter().map(|r| r[j]).collect()).collect()
        };
        let g = if axis == 0 { transpose(&self.0) } else { self.0.clone() };
        let mut left = Vec::new();
        let mut right = Vec::new();
        for row in g {
            let (l, r) = split1(&row, t)?;
            left.push(l);
            right.push(r);
        }
        Ok(if axis == 0 {
            (Self(transpose(&left)), Self(transpose(&right)))
        } else {
            (Self(left), Self(right))
        })
    }
    /// Boundary restrictions as univariate Bernstein rows, in the order
    /// u = 0, u = 1, v = 0, v = 1.
    fn edges(&self) -> [Vec<Interval>; 4] {
        let g = &self.0;
        [
            g[0].clone(),
            g[g.len() - 1].clone(),
            g.iter().map(|r| r[0]).collect(),
            g.iter().map(|r| r[r.len() - 1]).collect(),
        ]
    }
}

fn halves1(c: &[Interval]) -> Result<(Vec<Interval>, Vec<Interval>)> {
    split1(c, 0.5)
}

/// De Casteljau split at `t` into `[0, t]` and `[t, 1]`.
fn split1(c: &[Interval], t: f64) -> Result<(Vec<Interval>, Vec<Interval>)> {
    let (a, b) = (Interval::point(1. - t), Interval::point(t));
    let mut row = c.to_vec();
    let (mut left, mut right) = (vec![row[0]], vec![row[row.len() - 1]]);
    while row.len() > 1 {
        row = row
            .windows(2)
            .map(|w| w[0].mul(a)?.add(w[1].mul(b)?))
            .collect::<Result<_>>()?;
        left.push(row[0]);
        right.push(row[row.len() - 1]);
    }
    right.reverse();
    Ok((left, right))
}

fn variations(c: &[Interval]) -> Option<usize> {
    let mut count = 0;
    let mut last = 0_i8;
    for x in c {
        let sign = if x.lo > 0. {
            1
        } else if x.hi < 0. {
            -1
        } else {
            return None;
        };
        if last != 0 && sign != last {
            count += 1;
        }
        last = sign;
    }
    Some(count)
}

/// Exact root count of a univariate Bernstein polynomial on (0, 1) with
/// nonzero end values, by subdivision until every piece has zero or one
/// sign variation; `None` when that does not settle.
fn count_roots(c: &[Interval], depth: usize) -> Option<usize> {
    let ends = [c[0], c[c.len() - 1]];
    if ends.iter().any(|x| x.lo <= 0. && x.hi >= 0.) {
        return None;
    }
    match variations(c) {
        Some(0) => Some(0),
        Some(1) => Some(1),
        _ if depth == 0 => None,
        _ => {
            let (l, r) = halves1(c).ok()?;
            Some(count_roots(&l, depth - 1)? + count_roots(&r, depth - 1)?)
        }
    }
}

/// Chart coordinates of a traced span: `u = A / Q`, `v = B / Q` with cubic
/// Bernstein numerators and denominator.
#[derive(Clone)]
struct ChartSpan {
    a: Vec<Interval>,
    b: Vec<Interval>,
    q: Vec<Interval>,
}

fn ratio_hull(n: &[Interval], d: &[Interval]) -> Option<(f64, f64)> {
    let dl = d.iter().map(|x| x.lo).fold(f64::INFINITY, f64::min);
    let dh = d.iter().map(|x| x.hi).fold(f64::NEG_INFINITY, f64::max);
    if dl <= 0. {
        return None;
    }
    let nl = n.iter().map(|x| x.lo).fold(f64::INFINITY, f64::min);
    let nh = n.iter().map(|x| x.hi).fold(f64::NEG_INFINITY, f64::max);
    let q = Interval::new(nl, nh).ok()?.div(Interval::new(dl, dh).ok()?).ok()?;
    Some((q.lo, q.hi))
}

/// Crossings of the chart line `{fixed coordinate = c}` inside the open
/// range `(lo, hi)` of the other coordinate. `axis = 0` fixes u.
fn crossings(
    span: &ChartSpan,
    axis: usize,
    c: f64,
    lo: f64,
    hi: f64,
    near: f64,
    depth: usize,
) -> Option<usize> {
    // Inside the unit square q = 2r / (1 + u^2 + v^2) >= 2r / 3, so pieces
    // with a smaller denominator cannot touch the chart square.
    if span.q.iter().map(|x| x.hi).fold(f64::NEG_INFINITY, f64::max) < near {
        return Some(0);
    }
    let (num, other) = if axis == 0 { (&span.a, &span.b) } else { (&span.b, &span.a) };
    // Prune by the hulls first; a denominator not yet separated from zero
    // (a piece reaching toward the antipode) is split further.
    let hulls = ratio_hull(num, &span.q).zip(ratio_hull(other, &span.q));
    let Some(((fl, fh), (ol, oh))) = hulls else {
        if depth == 0 {
            return None;
        }
        let (al, ar) = halves1(&span.a).ok()?;
        let (bl, br) = halves1(&span.b).ok()?;
        let (ql, qr) = halves1(&span.q).ok()?;
        let left = ChartSpan { a: al, b: bl, q: ql };
        let right = ChartSpan { a: ar, b: br, q: qr };
        return Some(
            crossings(&left, axis, c, lo, hi, near, depth - 1)?
                + crossings(&right, axis, c, lo, hi, near, depth - 1)?,
        );
    };
    if fh < c || fl > c || oh < lo || ol > hi {
        return Some(0);
    }
    let g: Vec<Interval> = num
        .iter()
        .zip(&span.q)
        .map(|(n, q)| n.sub(q.mul(Interval::point(c))?))
        .collect::<Result<_>>()
        .ok()?;
    match variations(&g) {
        Some(0) => return Some(0),
        Some(1) if ol > lo && oh < hi => return Some(1),
        Some(1) if (oh < lo || ol > hi) => return Some(0),
        _ => {}
    }
    if depth == 0 {
        return None;
    }
    let split = |c: &[Interval]| halves1(c).ok();
    let next = depth - 1;
    let (al, ar) = split(&span.a)?;
    let (bl, br) = split(&span.b)?;
    let (ql, qr) = split(&span.q)?;
    let left = ChartSpan { a: al, b: bl, q: ql };
    let right = ChartSpan { a: ar, b: br, q: qr };
    Some(crossings(&left, axis, c, lo, hi, near, next)? + crossings(&right, axis, c, lo, hi, near, next)?)
}

/// Certify one chart square, retrying with shifted box grids when a traced
/// curve passes too close to a grid point to be counted.
fn certify_square(crossings: &dyn Crossings, f: Bern2) -> bool {
    [0.5, 0.4637, 0.5419, 0.4171].iter().any(|&ratio| {
        let mut chart = Chart {
            crossings,
            budget: 20_000,
            ratio,
        };
        chart.certify_box(f.clone(), [0., 1.], [0., 1.], 18)
    })
}

/// Traced crossings of the boundary of a chart box.
trait Crossings {
    fn boundary_crossings(&self, u: [f64; 2], v: [f64; 2]) -> Option<usize>;
}

/// Stereographic sphere chart.
struct Stereographic<'a> {
    spans: &'a [ChartSpan],
    /// Smallest chart denominator inside the unit square, with margin.
    near: f64,
}

impl Crossings for Stereographic<'_> {
    fn boundary_crossings(&self, u: [f64; 2], v: [f64; 2]) -> Option<usize> {
        let mut total = 0;
        for span in self.spans {
            for (axis, c, range) in [(0, u[0], v), (0, u[1], v), (1, v[0], u), (1, v[1], u)] {
                total += crossings(span, axis, c, range[0], range[1], self.near, 40)?;
            }
        }
        Some(total)
    }
}

struct Chart<'a> {
    crossings: &'a dyn Crossings,
    budget: usize,
    /// Split ratio; values other than 1/2 move the box grid off points a
    /// traced curve happens to pass through.
    ratio: f64,
}

impl Chart<'_> {
    fn boundary_crossings(&self, u: [f64; 2], v: [f64; 2]) -> Option<usize> {
        self.crossings.boundary_crossings(u, v)
    }

    fn certify_box(&mut self, f: Bern2, u: [f64; 2], v: [f64; 2], depth: usize) -> bool {
        if self.budget == 0 {
            return false;
        }
        self.budget -= 1;
        let (lo, hi) = f.hull();
        if lo > 0. || hi < 0. {
            return true;
        }
        if f.derivative_sign(0) || f.derivative_sign(1) {
            let mut roots = 0;
            for edge in f.edges() {
                match count_roots(&edge, 30) {
                    Some(n) => roots += n,
                    None => return self.split(f, u, v, depth),
                }
            }
            return match self.boundary_crossings(u, v) {
                Some(n) if n == roots => true,

                // A mismatch near a corner can be an artefact of where the
                // traced and true curves cross; smaller boxes decide.
                _ => self.split(f, u, v, depth),
            };
        }
        self.split(f, u, v, depth)
    }

    fn split(&mut self, f: Bern2, u: [f64; 2], v: [f64; 2], depth: usize) -> bool {
        if depth == 0 {
            return false;
        }
        let axis = if u[1] - u[0] >= v[1] - v[0] { 0 } else { 1 };
        let t = self.ratio;
        let Ok((a, b)) = f.halves(axis, t) else {
            return false;
        };
        let (ua, ub, va, vb) = if axis == 0 {
            let m = u[0] + t * (u[1] - u[0]);
            ([u[0], m], [m, u[1]], v, v)
        } else {
            let m = v[0] + t * (v[1] - v[0]);
            (u, u, [v[0], m], [m, v[1]])
        };
        self.certify_box(a, ua, va, depth - 1) && self.certify_box(b, ub, vb, depth - 1)
    }
}

/// The other surface of a sphere pair, as a homogeneous polynomial in
/// world coordinates.
pub(crate) enum Quadric<'a> {
    Torus(&'a CanonicalTorus),
    Cylinder {
        center: [f64; 3],
        /// In-plane x, in-plane y, axis.
        frame: [[f64; 3]; 3],
        radius: f64,
    },
    /// Radius `r_bottom + slope * z` above `bottom` along `frame[2]`.
    Cone {
        bottom: [f64; 3],
        frame: [[f64; 3]; 3],
        r_bottom: f64,
        slope: f64,
    },
}

/// Local cylindrical coordinates of a traced span (cubic Bernstein forms).
#[derive(Clone)]
struct LocalSpan {
    x: Vec<Interval>,
    y: Vec<Interval>,
    z: Vec<Interval>,
}

fn hull1(c: &[Interval]) -> (f64, f64) {
    c.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), x| (lo.min(x.lo), hi.max(x.hi)))
}

/// Angle of the 90-degree rational arc at parameter `t` (inverse of
/// `arc_parameter`): `s = t / (1 - t)`, `tan(phi / 2) = s / (sqrt 2 + s)`.
fn arc_angle(t: f64) -> f64 {
    if t >= 1. {
        return QUARTER;
    }
    let s = t / (1. - t);
    2. * (s / (std::f64::consts::SQRT_2 + s)).atan()
}

/// One side patch of a canonical cylinder: quadrant `i`, `u` the exact arc
/// parameter of the angle, `v = (z + h) / 2h`.
struct CylinderSide<'a> {
    spans: &'a [LocalSpan],
    quadrant: usize,
    /// Axial coordinate at v = 0 and the axial length of v in [0, 1].
    z0: f64,
    len: f64,
}

impl CylinderSide<'_> {
    /// Crossings of one span with the iso line `{coordinate axis = c}` with
    /// the other coordinate inside `(lo, hi)`.
    fn crossings(&self, span: &LocalSpan, axis: usize, c: f64, lo: f64, hi: f64, depth: usize) -> Option<usize> {
        let mid = self.quadrant as f64 * QUARTER + 0.5 * QUARTER;
        let (cm, sm) = (mid.cos(), mid.sin());
        // Rotated coordinates: X toward the patch middle.
        let rot = |x: &[Interval], y: &[Interval], cs: f64, sn: f64| -> Option<Vec<Interval>> {
            x.iter()
                .zip(y)
                .map(|(x, y)| x.mul(Interval::point(cs)).and_then(|a| a.add(y.mul(Interval::point(sn))?)).ok())
                .collect()
        };
        let big_x = rot(&span.x, &span.y, cm, sm)?;
        let big_y = rot(&span.y, &span.x, cm, -sm)?;
        let (xl, xh) = hull1(&big_x);
        let (zl, zh) = hull1(&span.z);
        let vrange = ((zl - self.z0) / self.len, (zh - self.z0) / self.len);
        let split = |this: &Self| -> Option<usize> {
            if depth == 0 {
                return None;
            }
            let (xa, xb) = halves1(&span.x).ok()?;
            let (ya, yb) = halves1(&span.y).ok()?;
            let (za, zb) = halves1(&span.z).ok()?;
            Some(
                this.crossings(&LocalSpan { x: xa, y: ya, z: za }, axis, c, lo, hi, depth - 1)?
                    + this.crossings(&LocalSpan { x: xb, y: yb, z: zb }, axis, c, lo, hi, depth - 1)?,
            )
        };
        // The patch is the wedge |Y| <= X (45 degrees each side of its
        // middle); a span wholly outside the wedge has no crossing.
        let (yl0, yh0) = hull1(&big_y);
        if xh <= 0. || yl0 > xh || yh0 < -xh {
            return Some(0);
        }
        if xl <= 0. {
            return split(self);
        }
        // Local angle range from the box of (X, Y), X > 0.
        let (yl, yh) = hull1(&big_y);
        let angles = [yl.atan2(xl), yl.atan2(xh), yh.atan2(xl), yh.atan2(xh)];
        let (al, ah) = angles.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), a| (l.min(*a), h.max(*a)));
        // The arc parameter t(phi) = s / (1 + s), s = sqrt2 T / (1 - T),
        // T = tan(phi / 2), is increasing on (-pi/2, pi/2): it extends past
        // the seam at phi = 0 to negative t, and past phi = 90 degrees we
        // only need "beyond 1".
        let arc_t = |phi: f64| {
            let t = (0.5 * phi).tan();
            let s = std::f64::consts::SQRT_2 * t / (1. - t);
            s / (1. + s)
        };
        let (ul, uh) = (
            arc_t(al + 0.5 * QUARTER) - 1e-12,
            if ah + 0.5 * QUARTER >= QUARTER { f64::INFINITY } else { arc_t(ah + 0.5 * QUARTER) + 1e-12 },
        );
        let (g, other) = if axis == 0 {
            // u = c: the ruling at local angle arc_angle(c) - 45 degrees.
            let a = arc_angle(c) - 0.5 * QUARTER;
            let (ca, sa) = (a.cos(), a.sin());
            let g: Vec<Interval> = big_x
                .iter()
                .zip(&big_y)
                .map(|(x, y)| y.mul(Interval::point(ca)).and_then(|p| p.sub(x.mul(Interval::point(sa))?)).ok())
                .collect::<Option<_>>()?;
            (g, vrange)
        } else {
            let z0 = self.z0 + self.len * c;
            let g: Vec<Interval> = span.z.iter().map(|z| z.sub(Interval::point(z0)).ok()).collect::<Option<_>>()?;
            (g, (ul, uh))
        };
        let (gl, gh) = hull1(&g);
        if gl > 0. || gh < 0. || other.1 < lo || other.0 > hi {
            return Some(0);
        }
        match variations(&g) {
            Some(0) => return Some(0),
            Some(1) if other.0 > lo && other.1 < hi => return Some(1),
            _ => {}
        }
        split(self)
    }
}

impl Crossings for CylinderSide<'_> {
    fn boundary_crossings(&self, u: [f64; 2], v: [f64; 2]) -> Option<usize> {
        let mut total = 0;
        for span in self.spans {
            for (axis, c, range) in [(0, u[0], v), (0, u[1], v), (1, v[0], u), (1, v[1], u)] {
                // A span ending exactly on the line (traces seeded on a
                // symmetry plane) has an undecidable end sign; a line
                // shifted by far less than any crossing separation gives the
                // same transversal count.
                let n = [0., 1e-11, -1e-11]
                    .iter()
                    .find_map(|&eps| self.crossings(span, axis, c + eps, range[0], range[1], 40))?;
                total += n;
            }
        }
        Some(total)
    }
}

/// A planar cap chart: `u = (x / R + 1) / 2`, `v = (y / R + 1) / 2` in the
/// cap frame, so both iso lines are linear in the point.
struct Planar<'a> {
    spans: &'a [LocalSpan],
    radius: f64,
}

impl Planar<'_> {
    fn crossings(&self, span: &LocalSpan, axis: usize, c: f64, lo: f64, hi: f64, depth: usize) -> Option<usize> {
        let to_uv = |x: f64| (x / self.radius + 1.) * 0.5;
        let (fixed, free) = if axis == 0 { (&span.x, &span.y) } else { (&span.y, &span.x) };
        let level = self.radius * (2. * c - 1.);
        let g: Vec<Interval> = fixed.iter().map(|x| x.sub(Interval::point(level)).ok()).collect::<Option<_>>()?;
        let (fl, fh) = hull1(free);
        let other = (to_uv(fl), to_uv(fh));
        let (gl, gh) = hull1(&g);
        if gl > 0. || gh < 0. || other.1 < lo || other.0 > hi {
            return Some(0);
        }
        match variations(&g) {
            Some(0) => return Some(0),
            Some(1) if other.0 > lo && other.1 < hi => return Some(1),
            _ => {}
        }
        if depth == 0 {
            return None;
        }
        let (xa, xb) = halves1(&span.x).ok()?;
        let (ya, yb) = halves1(&span.y).ok()?;
        let (za, zb) = halves1(&span.z).ok()?;
        Some(
            self.crossings(&LocalSpan { x: xa, y: ya, z: za }, axis, c, lo, hi, depth - 1)?
                + self.crossings(&LocalSpan { x: xb, y: yb, z: zb }, axis, c, lo, hi, depth - 1)?,
        )
    }
}

impl Crossings for Planar<'_> {
    fn boundary_crossings(&self, u: [f64; 2], v: [f64; 2]) -> Option<usize> {
        let mut total = 0;
        for span in self.spans {
            for (axis, c, range) in [(0, u[0], v), (0, u[1], v), (1, v[0], u), (1, v[1], u)] {
                let n = [0., 1e-11, -1e-11]
                    .iter()
                    .find_map(|&eps| self.crossings(span, axis, c + eps, range[0], range[1], 40))?;
                total += n;
            }
        }
        Some(total)
    }
}

/// True when the traced `cap_loops[slot]` provably are all the components
/// of `other` with each cap plane of a canonical cylinder, over the whole
/// square cap patch (which contains the cap disk).
pub(crate) fn certify_complete_on_caps(
    cylinder: &crate::sphere_cylinder::CanonicalCylinder,
    cylinder_model: &Model,
    other: &Quadric<'_>,
    cap_loops: &[Vec<Vec<Curve>>; 2],
) -> bool {
    (|| -> Result<bool> {
        for (slot, &face) in cylinder.caps.iter().enumerate() {
            let sign = if slot == 0 { -1. } else { 1. };
            let origin: [f64; 3] =
                std::array::from_fn(|k| cylinder.center[k] + sign * cylinder.half_height * cylinder.axis[k]);
            let frame = [cylinder.frame[0], cylinder.frame[1], cylinder.axis];
            let surface = &cylinder_model.faces[face].surface;
            for (u, v) in [(0.2, 0.3), (0.7, 0.9)] {
                let p = surface.evaluate(u, v)?.point;
                let d = sub([p[0], p[1], p[2]], origin);
                let pu = (dot(d, frame[0]) / cylinder.radius + 1.) * 0.5;
                let pv = (dot(d, frame[1]) / cylinder.radius + 1.) * 0.5;
                if (pu - u).abs() > 1e-9 || (pv - v).abs() > 1e-9 || dot(d, frame[2]).abs() > 1e-9 {
                    return Ok(false);
                }
            }
            let mut spans = Vec::new();
            for piece in cap_loops[slot].iter().flatten() {
                for segment in piece.decompose()? {
                    let c = &segment.definition().control_points;
                    let local = |axis: [f64; 3]| -> Vec<Interval> {
                        c.iter()
                            .map(|p| Interval::point(dot(sub([p[0], p[1], p[2]], origin), axis)))
                            .collect()
                    };
                    spans.push(LocalSpan { x: local(frame[0]), y: local(frame[1]), z: local(frame[2]) });
                }
            }
            let f = on_patch(other, surface)?;
            let chart = Planar { spans: &spans, radius: cylinder.radius };
            if !certify_square(&chart, f) {
                return Ok(false);
            }
        }
        Ok(true)
    })()
    .unwrap_or(false)
}

/// True when the traced `loops` provably are all the components of the
/// intersection of `other` with the side wall of a canonical cylinder (the
/// caller guarantees the contact stays between the cap planes).
pub(crate) fn certify_complete_on_cylinder(
    cylinder: &crate::sphere_cylinder::CanonicalCylinder,
    cylinder_model: &Model,
    other: &Quadric<'_>,
    loops: &[Vec<Curve>],
) -> bool {
    let h = cylinder.half_height;
    certify_complete_on_side(
        cylinder.center,
        [cylinder.frame[0], cylinder.frame[1], cylinder.axis],
        cylinder.sides,
        (-h, 2. * h),
        cylinder_model,
        other,
        loops,
    )
}

/// The same on the side wall of a canonical frustum.
pub(crate) fn certify_complete_on_cone(
    cone: &brep_core::intersections::CanonicalCone,
    cone_model: &Model,
    other: &Quadric<'_>,
    loops: &[Vec<Curve>],
) -> bool {
    certify_complete_on_side(
        cone.bottom,
        [cone.frame[0], cone.frame[1], cone.axis],
        cone.sides,
        (0., cone.height),
        cone_model,
        other,
        loops,
    )
}

/// Side patches of a solid of revolution: quadrant `i` patch has
/// `u = arc_parameter(angle - i * 90 deg)` and `v = (z - z0) / len`.
fn certify_complete_on_side(
    origin: [f64; 3],
    frame: [[f64; 3]; 3],
    sides: [usize; 4],
    (z0, len): (f64, f64),
    model: &Model,
    other: &Quadric<'_>,
    loops: &[Vec<Curve>],
) -> bool {
    (|| -> Result<bool> {
        let mut spans = Vec::new();
        for piece in loops.iter().flatten() {
            for segment in piece.decompose()? {
                let c = &segment.definition().control_points;
                let local = |axis: [f64; 3]| -> Vec<Interval> {
                    c.iter()
                        .map(|p| Interval::point(dot(sub([p[0], p[1], p[2]], origin), axis)))
                        .collect()
                };
                spans.push(LocalSpan {
                    x: local(frame[0]),
                    y: local(frame[1]),
                    z: local(frame[2]),
                });
            }
        }
        for (quadrant, &face) in sides.iter().enumerate() {
            let surface = &model.faces[face].surface;
            for (u, v) in [(0.2, 0.3), (0.7, 0.9)] {
                let p = surface.evaluate(u, v)?.point;
                let d = sub([p[0], p[1], p[2]], origin);
                let angle = dot(d, frame[1]).atan2(dot(d, frame[0])).rem_euclid(std::f64::consts::TAU)
                    - quadrant as f64 * QUARTER;
                let pu = crate::plane_torus::arc_parameter(angle.clamp(0., QUARTER));
                let pv = (dot(d, frame[2]) - z0) / len;
                if (pu - u).abs() > 1e-9 || (pv - v).abs() > 1e-9 {
                    return Ok(false);
                }
            }
            let f = on_patch(other, surface)?;
            let side = CylinderSide {
                spans: &spans,
                quadrant,
                z0,
                len,
            };
            if !certify_square(&side, f) {
                return Ok(false);
            }
        }
        Ok(true)
    })()
    .unwrap_or(false)
}

/// Homogeneous equation of `other` on a rational biquadratic patch.
fn on_patch(other: &Quadric<'_>, surface: &nurbs_core::surface::Surface) -> Result<Bern2> {
    match other {
        Quadric::Torus(torus) => torus_on_patch(torus, surface),
        Quadric::Cylinder { center, frame, radius } => {
            let (w, local) = local_coordinates(surface, *center, *frame)?;
            local[0]
                .mul(&local[0])?
                .add(&local[1].mul(&local[1])?)?
                .sub(&w.mul(&w)?.scale(radius * radius)?)
        }
        Quadric::Cone { bottom, frame, r_bottom, slope } => {
            let (w, local) = local_coordinates(surface, *bottom, *frame)?;
            let c = w.scale(*r_bottom)?.add(&local[2].scale(*slope)?)?;
            local[0].mul(&local[0])?.add(&local[1].mul(&local[1])?)?.sub(&c.mul(&c)?)
        }
    }
}

/// Weight and local homogeneous coordinates of a patch.
fn local_coordinates(
    surface: &nurbs_core::surface::Surface,
    center: [f64; 3],
    frame: [[f64; 3]; 3],
) -> Result<(Bern2, Vec<Bern2>)> {
    let coordinate = |k: usize| -> Bern2 {
        Bern2(
            surface
                .control_points
                .iter()
                .zip(&surface.weights)
                .map(|(row, w)| {
                    row.iter()
                        .zip(w)
                        .map(|(p, w)| Interval::point(if k < 3 { p[k] * w } else { *w }))
                        .collect()
                })
                .collect(),
        )
    };
    let w = coordinate(3);
    let mut local = Vec::with_capacity(3);
    for axis in frame {
        let mut sum = Bern2::constant(0.);
        for k in 0..3 {
            let shifted = coordinate(k).sub(&w.scale(center[k])?)?;
            sum = sum.add(&shifted.scale(axis[k])?)?;
        }
        local.push(sum);
    }
    Ok((w, local))
}

/// Homogeneous torus equation on a rational biquadratic patch.
fn torus_on_patch(torus: &CanonicalTorus, surface: &nurbs_core::surface::Surface) -> Result<Bern2> {
    let coordinate = |k: usize| -> Bern2 {
        Bern2(
            surface
                .control_points
                .iter()
                .zip(&surface.weights)
                .map(|(row, w)| {
                    row.iter()
                        .zip(w)
                        .map(|(p, w)| Interval::point(if k < 3 { p[k] * w } else { *w }))
                        .collect()
                })
                .collect(),
        )
    };
    let w = coordinate(3);
    let frame = [torus.frame[0], torus.frame[1], torus.axis];
    let mut local = Vec::with_capacity(3);
    for axis in frame {
        let mut sum = Bern2::constant(0.);
        for k in 0..3 {
            let shifted = coordinate(k).sub(&w.scale(torus.center[k])?)?;
            sum = sum.add(&shifted.scale(axis[k])?)?;
        }
        local.push(sum);
    }
    let (r2, m2) = (torus.major * torus.major, torus.minor * torus.minor);
    let w2 = w.mul(&w)?;
    let planar = local[0].mul(&local[0])?.add(&local[1].mul(&local[1])?)?;
    let s = planar.add(&local[2].mul(&local[2])?)?.add(&w2.scale(r2 - m2)?)?;
    s.mul(&s)?.sub(&planar.mul(&w2)?.scale(4. * r2)?)
}

/// True when the traced `loops` provably are all the components of the
/// sphere/torus intersection. Requires stereographic sphere charts
/// (checked numerically against the authored patches).
pub(super) fn certify_complete(
    sphere: &CanonicalSphere,
    torus: &CanonicalTorus,
    sphere_model: &Model,
    loops: &[Vec<Curve>],
) -> bool {
    certify_complete_against(sphere, &Quadric::Torus(torus), sphere_model, loops)
}

/// [`certify_complete`] against any supported quadric.
pub(crate) fn certify_complete_against(
    sphere: &CanonicalSphere,
    other: &Quadric<'_>,
    sphere_model: &Model,
    loops: &[Vec<Curve>],
) -> bool {
    (|| -> Result<bool> {
        for (face, model_face) in sphere_model.faces.iter().enumerate() {
            let surface = &model_face.surface;
            if surface.degree_u != 2
                || surface.degree_v != 2
                || surface.knots_u != [0., 0., 0., 1., 1., 1.]
                || surface.knots_v != surface.knots_u
            {
                return Ok(false);
            }
            let (a, b, pole) = sphere.frame(face);
            // The chart must be this patch's own parameterization.
            for (u, v) in [(0.2, 0.3), (0.7, 0.1), (0.5, 0.9), (1., 1.)] {
                let p = surface.evaluate(u, v)?.point;
                let d = sub([p[0], p[1], p[2]], sphere.center);
                let q = sphere.radius + dot(d, pole);
                if q <= 0. || (dot(d, a) / q - u).abs() > 1e-9 || (dot(d, b) / q - v).abs() > 1e-9 {
                    return Ok(false);
                }
            }
            // Chart forms of every traced span: numerators d.a, d.b and
            // denominator r + d.pole are affine in the cubic control points.
            let mut spans = Vec::new();
            for piece in loops.iter().flatten() {
                for segment in piece.decompose()? {
                    let c = &segment.definition().control_points;
                    let affine = |axis: [f64; 3], offset: f64| -> Vec<Interval> {
                        c.iter()
                            .map(|p| {
                                let d = sub([p[0], p[1], p[2]], sphere.center);
                                Interval::point(dot(d, axis) + offset)
                            })
                            .collect()
                    };
                    spans.push(ChartSpan {
                        a: affine(a, 0.),
                        b: affine(b, 0.),
                        q: affine(pole, sphere.radius),
                    });
                }
            }
            let f = on_patch(other, surface)?;
            let stereographic = Stereographic {
                spans: &spans,
                near: 0.6 * sphere.radius,
            };
            if !certify_square(&stereographic, f) {
                return Ok(false);
            }
        }
        Ok(true)
    })()
    .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::super::super::test_utils::translated;
    use super::*;

    #[test]
    fn a_missing_component_breaks_the_certificate() {
        let torus_model = brep_core::analytic::torus(3., 1.).unwrap();
        let torus = recognize_torus(&torus_model).unwrap().unwrap();
        let mut seen_multi = false;
        for (radius, at) in [
            (5.2_f64.sqrt(), [0.5, 0., 0.]),
            (1., [4.5, 0., 0.]),
            (1., [3.5, 0.4, 0.3]),
            (2.5, [2., 1., 0.]),
        ] {
            let sphere_model = translated(&brep_core::analytic::sphere(radius).unwrap(), at);
            let sphere = sphere_sphere::recognize_any(&sphere_model).unwrap().unwrap();
            let report = intersect_sphere_torus(&sphere_model, &torus_model, Options::default()).unwrap();
            let loops: Vec<Vec<Curve>> = report
                .components
                .iter()
                .filter_map(|c| match c {
                    SphereTorusComponent::Traced { pieces, .. } => Some(pieces.clone()),
                    _ => None,
                })
                .collect();
            assert!(!loops.is_empty(), "{radius} at {at:?}");
            assert!(certify_complete(&sphere, &torus, &sphere_model, &loops));
            // No traced loops: every root on the boxes' boundaries is unmatched.
            assert!(!certify_complete(&sphere, &torus, &sphere_model, &[]));
            // Dropping one of several loops is detected too.
            if loops.len() > 1 {
                seen_multi = true;
                assert!(!certify_complete(&sphere, &torus, &sphere_model, &loops[1..]));
            }
        }
        assert!(seen_multi, "no configuration with several loops was exercised");
    }
}
