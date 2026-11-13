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
    /// Halves along u (`axis = 0`) or v.
    fn halves(&self, axis: usize) -> Result<(Self, Self)> {
        let transpose = |g: &Grid| -> Grid {
            (0..g[0].len()).map(|j| g.iter().map(|r| r[j]).collect()).collect()
        };
        let g = if axis == 0 { transpose(&self.0) } else { self.0.clone() };
        let mut left = Vec::new();
        let mut right = Vec::new();
        for row in g {
            let (l, r) = halves1(&row)?;
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
    let half = Interval::point(0.5);
    let mut row = c.to_vec();
    let (mut left, mut right) = (vec![row[0]], vec![row[row.len() - 1]]);
    while row.len() > 1 {
        row = row
            .windows(2)
            .map(|w| w[0].add(w[1])?.mul(half))
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

struct Chart<'a> {
    spans: &'a [ChartSpan],
    budget: usize,
    /// Smallest chart denominator inside the unit square, with margin.
    near: f64,
}

impl Chart<'_> {
    /// Traced crossings of the boundary of `[u0, u1] x [v0, v1]`.
    fn boundary_crossings(&self, u: [f64; 2], v: [f64; 2]) -> Option<usize> {
        let mut total = 0;
        for span in self.spans {
            for (axis, c, range) in [(0, u[0], v), (0, u[1], v), (1, v[0], u), (1, v[1], u)] {
                total += crossings(span, axis, c, range[0], range[1], self.near, 40)?;
            }
        }
        Some(total)
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
        let Ok((a, b)) = f.halves(axis) else {
            return false;
        };
        let (ua, ub, va, vb) = if axis == 0 {
            let m = 0.5 * (u[0] + u[1]);
            ([u[0], m], [m, u[1]], v, v)
        } else {
            let m = 0.5 * (v[0] + v[1]);
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
            let mut chart = Chart {
                spans: &spans,
                budget: 20_000,
                near: 0.6 * sphere.radius,
            };
            if !chart.certify_box(f, [0., 1.], [0., 1.], 16) {
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
