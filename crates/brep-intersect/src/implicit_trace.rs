//! Certified tracing of the intersection of two polynomial implicit surfaces.
//!
//! Where an intersection curve has no exact rational form (an off-axis
//! sphere/torus pair meets in a space quartic), the curve is traced and
//! represented by cubic NURBS pieces whose distance from BOTH surfaces is
//! bounded rigorously:
//!
//! 1. Seeds come from sign changes of the second implicit along a grid of
//!    meridians on the first surface; Newton projection with the minimum-norm
//!    correction `-J^T (J J^T)^-1 g` puts points on both surfaces.
//! 2. Each closed component is traced with a tangent predictor
//!    `grad g1 x grad g2` and the same corrector. A vanishing tangent (the
//!    surfaces touch) is a refusal, never a guess.
//! 3. The loop is interpolated by cubic B-spline pieces (at most 180 points
//!    each) that share their end points exactly.
//! 4. On every Bezier span `g(C(t))` is a polynomial; its Bernstein
//!    coefficients, computed in outward-rounded interval arithmetic, bound
//!    `|g|` on the span by the convex-hull property, and interval de Casteljau
//!    halving tightens the bound. An implicit-specific factor turns `|g|` into
//!    a distance bound. A span that cannot be brought under the tolerance
//!    sends the whole component back for a finer trace.
//!
//! Each returned curve is certified against both surfaces. That the seeds
//! found EVERY component is not certified: a component smaller than the seed
//! grid could be missed, so reports keep a numerically-resolved coverage and
//! never authorize topology changes.
use nurbs_core::curve::Curve;
use nurbs_core::foundation::interpolation::{Parameterization, interpolate_curve};
use nurbs_core::interval_eval::Interval;
use nurbs_core::{Error, Result};

pub(crate) type P = [f64; 3];

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: P, b: P) -> P {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: P) -> f64 {
    dot(a, a).sqrt()
}
fn refuse(message: &str) -> Error {
    Error::new("BREP_INTERSECTION_TRACE_REFUSED", message)
}

/// A polynomial in the Bernstein basis of degree `len - 1` on `[0, 1]`, with
/// interval coefficients.
#[derive(Clone, Debug)]
pub(crate) struct Bern(pub(crate) Vec<Interval>);

fn binomial(n: usize, k: usize) -> f64 {
    let mut v = 1.;
    for j in 0..k.min(n - k) {
        v = v * (n - j) as f64 / (j + 1) as f64;
    }
    v.round()
}

impl Bern {
    fn constant(x: f64, degree: usize) -> Self {
        Self(vec![Interval::point(x); degree + 1])
    }
    fn degree(&self) -> usize {
        self.0.len() - 1
    }
    fn elevate_to(&self, degree: usize) -> Result<Self> {
        let mut current = self.clone();
        while current.degree() < degree {
            let n = current.degree() + 1;
            let mut next = Vec::with_capacity(n + 1);
            for k in 0..=n {
                let a = if k > 0 {
                    current.0[k - 1]
                } else {
                    Interval::point(0.)
                };
                let b = if k < n {
                    current.0[k]
                } else {
                    Interval::point(0.)
                };
                let wa = Interval::point(k as f64).div(Interval::point(n as f64))?;
                let wb = Interval::point((n - k) as f64).div(Interval::point(n as f64))?;
                next.push(a.mul(wa)?.add(b.mul(wb)?)?);
            }
            current = Self(next);
        }
        Ok(current)
    }
    fn add(&self, other: &Self) -> Result<Self> {
        let d = self.degree().max(other.degree());
        let (a, b) = (self.elevate_to(d)?, other.elevate_to(d)?);
        Ok(Self(
            a.0.iter()
                .zip(&b.0)
                .map(|(x, y)| x.add(*y))
                .collect::<Result<_>>()?,
        ))
    }
    fn sub(&self, other: &Self) -> Result<Self> {
        let d = self.degree().max(other.degree());
        let (a, b) = (self.elevate_to(d)?, other.elevate_to(d)?);
        Ok(Self(
            a.0.iter()
                .zip(&b.0)
                .map(|(x, y)| x.sub(*y))
                .collect::<Result<_>>()?,
        ))
    }
    fn scale(&self, s: f64) -> Result<Self> {
        Ok(Self(
            self.0
                .iter()
                .map(|x| x.mul(Interval::point(s)))
                .collect::<Result<_>>()?,
        ))
    }
    fn mul(&self, other: &Self) -> Result<Self> {
        let (m, n) = (self.degree(), other.degree());
        let mut out = Vec::with_capacity(m + n + 1);
        for k in 0..=m + n {
            let mut sum = Interval::point(0.);
            let first = k.saturating_sub(n);
            for (i, a) in self.0.iter().enumerate().take(m.min(k) + 1).skip(first) {
                let j = k - i;
                let w = Interval::point(binomial(m, i) * binomial(n, j))
                    .div(Interval::point(binomial(m + n, k)))?;
                sum = sum.add(a.mul(other.0[j])?.mul(w)?)?;
            }
            out.push(sum);
        }
        Ok(Self(out))
    }
    /// Hull of the coefficients: encloses `p(t)` on `[0, 1]`.
    fn hull(&self) -> Result<Interval> {
        let lo = self.0.iter().map(|x| x.lo).fold(f64::INFINITY, f64::min);
        let hi = self.0.iter().map(|x| x.hi).fold(f64::NEG_INFINITY, f64::max);
        Interval::new(lo, hi)
    }
    /// De Casteljau halves on `[0, 1/2]` and `[1/2, 1]`.
    fn halves(&self) -> Result<(Self, Self)> {
        let half = Interval::point(0.5);
        let mut row = self.0.clone();
        let (mut left, mut right) = (vec![row[0]], vec![*row.last().unwrap()]);
        while row.len() > 1 {
            row = row
                .windows(2)
                .map(|w| w[0].add(w[1])?.mul(half))
                .collect::<Result<_>>()?;
            left.push(row[0]);
            right.push(*row.last().unwrap());
        }
        right.reverse();
        Ok((Self(left), Self(right)))
    }
    /// Upper bound of `|p(t)|` on `[0, 1]` (convex-hull property).
    fn magnitude(&self) -> f64 {
        self.0
            .iter()
            .map(|x| x.lo.abs().max(x.hi.abs()))
            .fold(0., f64::max)
    }
}

/// A polynomial implicit surface `g = 0` with a factor that turns `|g|` into
/// a bound on the distance from the surface.
pub(crate) trait Implicit {
    fn value(&self, p: P) -> f64;
    fn gradient(&self, p: P) -> P;
    /// Bernstein form of `g(C(t))` for a curve span given by the Bernstein
    /// forms of its world coordinates.
    fn compose(&self, xyz: &[Bern; 3]) -> Result<Bern>;
    /// `distance <= |g| * factor` for points of a span whose control points
    /// (and so the whole span, by the convex-hull property) lie in `xyz`.
    fn distance_factor(&self, xyz: &[Bern; 3]) -> f64;
    /// Homogeneous form for rational points `X / W`: returns `(G, k)` with
    /// `g(X / W) = G / W^k`.
    fn compose_homogeneous(&self, xyzw: &[Bern; 4]) -> Result<(Bern, i32)>;
}

/// `|p - c|^2 - r^2`; `|d| = |g| / (|p - c| + r) <= |g| / r`.
pub(crate) struct SphereImplicit {
    pub(crate) center: P,
    pub(crate) radius: f64,
}
impl Implicit for SphereImplicit {
    fn value(&self, p: P) -> f64 {
        let d = sub(p, self.center);
        dot(d, d) - self.radius * self.radius
    }
    fn gradient(&self, p: P) -> P {
        sub(p, self.center).map(|x| 2. * x)
    }
    fn compose(&self, xyz: &[Bern; 3]) -> Result<Bern> {
        let degree = xyz[0].degree();
        let mut sum = Bern::constant(-self.radius * self.radius, 0);
        for k in 0..3 {
            let d = xyz[k].sub(&Bern::constant(self.center[k], degree))?;
            sum = sum.add(&d.mul(&d)?)?;
        }
        Ok(sum)
    }
    fn distance_factor(&self, _: &[Bern; 3]) -> f64 {
        1. / self.radius
    }
    fn compose_homogeneous(&self, xyzw: &[Bern; 4]) -> Result<(Bern, i32)> {
        let w = &xyzw[3];
        let mut sum = w.mul(w)?.scale(-self.radius * self.radius)?;
        for k in 0..3 {
            let d = xyzw[k].sub(&w.scale(self.center[k])?)?;
            sum = sum.add(&d.mul(&d)?)?;
        }
        Ok((sum, 2))
    }
}

/// Infinite circular cylinder about `axis` through `center`:
/// `g = x^2 + y^2 - R^2` in local coordinates, so `|d| = |g| / (rho + R)`
/// and `|d| <= |g| / R`.
pub(crate) struct CylinderImplicit {
    pub(crate) center: P,
    /// In-plane x, in-plane y, axis.
    pub(crate) frame: [P; 3],
    pub(crate) radius: f64,
}
impl CylinderImplicit {
    fn planar(&self, p: P) -> [f64; 2] {
        let d = sub(p, self.center);
        [dot(d, self.frame[0]), dot(d, self.frame[1])]
    }
}
impl Implicit for CylinderImplicit {
    fn value(&self, p: P) -> f64 {
        let [x, y] = self.planar(p);
        x * x + y * y - self.radius * self.radius
    }
    fn gradient(&self, p: P) -> P {
        let [x, y] = self.planar(p);
        std::array::from_fn(|k| 2. * (x * self.frame[0][k] + y * self.frame[1][k]))
    }
    fn compose(&self, xyz: &[Bern; 3]) -> Result<Bern> {
        let w = Bern::constant(1., 0);
        let (g, _) = self.compose_homogeneous(&[xyz[0].clone(), xyz[1].clone(), xyz[2].clone(), w])?;
        Ok(g)
    }
    fn distance_factor(&self, _: &[Bern; 3]) -> f64 {
        1. / self.radius
    }
    fn compose_homogeneous(&self, xyzw: &[Bern; 4]) -> Result<(Bern, i32)> {
        let w = &xyzw[3];
        let mut sum = w.mul(w)?.scale(-self.radius * self.radius)?;
        for axis in &self.frame[..2] {
            let mut l = xyzw[0].sub(&w.scale(self.center[0])?)?.scale(axis[0])?;
            for k in 1..3 {
                l = l.add(&xyzw[k].sub(&w.scale(self.center[k])?)?.scale(axis[k])?)?;
            }
            sum = sum.add(&l.mul(&l)?)?;
        }
        Ok((sum, 2))
    }
}

/// Ring torus about `axis` through `center`: in local coordinates
/// `g = (|q|^2 + R^2 - r^2)^2 - 4 R^2 (x^2 + y^2)`, which factors as
/// `(q_d^2 - r^2)((rho + R)^2 + z^2 - r^2)` with `q_d` the distance to the
/// core circle. The second factor is at least `R^2 - r^2` and
/// `|q_d^2 - r^2| >= r |d|`, so `|d| <= |g| / (r (R^2 - r^2))`.
pub(crate) struct TorusImplicit {
    pub(crate) center: P,
    pub(crate) frame: [P; 3],
    pub(crate) major: f64,
    pub(crate) minor: f64,
}
impl TorusImplicit {
    fn local(&self, p: P) -> P {
        let d = sub(p, self.center);
        self.frame.map(|axis| dot(d, axis))
    }
}
impl Implicit for TorusImplicit {
    fn value(&self, p: P) -> f64 {
        let [x, y, z] = self.local(p);
        let (r2, m2) = (self.major * self.major, self.minor * self.minor);
        let s = x * x + y * y + z * z + r2 - m2;
        s * s - 4. * r2 * (x * x + y * y)
    }
    fn gradient(&self, p: P) -> P {
        let [x, y, z] = self.local(p);
        let (r2, m2) = (self.major * self.major, self.minor * self.minor);
        let s = x * x + y * y + z * z + r2 - m2;
        let local = [
            4. * s * x - 8. * r2 * x,
            4. * s * y - 8. * r2 * y,
            4. * s * z,
        ];
        std::array::from_fn(|k| (0..3).map(|i| local[i] * self.frame[i][k]).sum())
    }
    fn compose(&self, xyz: &[Bern; 3]) -> Result<Bern> {
        let degree = xyz[0].degree();
        // Local coordinates are affine in world ones, so their Bernstein
        // coefficients are the same affine map of the world coefficients.
        let shifted: Vec<Bern> = (0..3)
            .map(|k| xyz[k].sub(&Bern::constant(self.center[k], degree)))
            .collect::<Result<_>>()?;
        let mut local = Vec::with_capacity(3);
        for axis in self.frame {
            let mut sum = Bern::constant(0., degree);
            for k in 0..3 {
                sum = sum.add(&shifted[k].scale(axis[k])?)?;
            }
            local.push(sum);
        }
        let (r2, m2) = (self.major * self.major, self.minor * self.minor);
        let planar = local[0].mul(&local[0])?.add(&local[1].mul(&local[1])?)?;
        let s = planar
            .add(&local[2].mul(&local[2])?)?
            .add(&Bern::constant(r2 - m2, 0))?;
        s.mul(&s)?.sub(&planar.scale(4. * r2)?)
    }
    fn compose_homogeneous(&self, xyzw: &[Bern; 4]) -> Result<(Bern, i32)> {
        let w = &xyzw[3];
        let shifted: Vec<Bern> = (0..3)
            .map(|k| xyzw[k].sub(&w.scale(self.center[k])?))
            .collect::<Result<_>>()?;
        let mut local = Vec::with_capacity(3);
        for axis in self.frame {
            let mut sum = shifted[0].scale(axis[0])?;
            for k in 1..3 {
                sum = sum.add(&shifted[k].scale(axis[k])?)?;
            }
            local.push(sum);
        }
        let (r2, m2) = (self.major * self.major, self.minor * self.minor);
        let w2 = w.mul(w)?;
        let planar = local[0].mul(&local[0])?.add(&local[1].mul(&local[1])?)?;
        let s = planar.add(&local[2].mul(&local[2])?)?.add(&w2.scale(r2 - m2)?)?;
        Ok((s.mul(&s)?.sub(&planar.mul(&w2)?.scale(4. * r2)?)?, 4))
    }
    fn distance_factor(&self, xyz: &[Bern; 3]) -> f64 {
        // The second factor is at least (rho_min + R)^2 - r^2, where rho_min
        // is a lower bound of the distance to the axis over the span's box of
        // control points, enclosed by interval arithmetic on that box. The
        // global floor R^2 - r^2 (rho_min = 0) always holds.
        let r2 = self.minor * self.minor;
        let rho_low = (|| -> Result<f64> {
            let boxes: Vec<Interval> = (0..3)
                .map(|k| {
                    let lo = xyz[k].0.iter().map(|x| x.lo).fold(f64::INFINITY, f64::min);
                    let hi = xyz[k]
                        .0
                        .iter()
                        .map(|x| x.hi)
                        .fold(f64::NEG_INFINITY, f64::max);
                    Interval::new(lo, hi)?.sub(Interval::point(self.center[k]))
                })
                .collect::<Result<_>>()?;
            let mut planar = Interval::point(0.);
            for axis in [self.frame[0], self.frame[1]] {
                let mut coordinate = Interval::point(0.);
                for k in 0..3 {
                    coordinate = coordinate.add(boxes[k].mul(Interval::point(axis[k]))?)?;
                }
                // Square of an interval, nonnegative.
                let low = if coordinate.lo <= 0. && coordinate.hi >= 0. {
                    0.
                } else {
                    coordinate.lo.abs().min(coordinate.hi.abs())
                };
                planar = planar.add(Interval::point(low).mul(Interval::point(low))?)?;
            }
            Ok(planar.lo.max(0.).sqrt().next_down().max(0.))
        })()
        .unwrap_or(0.);
        let second = ((rho_low + self.major) * (rho_low + self.major) - r2).next_down();
        let floor = self.major * self.major - r2;
        (1. / (self.minor * second.max(floor))).next_up()
    }
}

/// Minimum-norm Newton projection onto both surfaces.
fn correct(a: &dyn Implicit, b: &dyn Implicit, mut p: P, scale: f64) -> Option<P> {
    for _ in 0..40 {
        let (ga, gb) = (a.gradient(p), b.gradient(p));
        let (va, vb) = (a.value(p), b.value(p));
        let (na, nb) = (norm(ga), norm(gb));
        if !(na > 0. && nb > 0.) {
            return None;
        }
        if (va / na).abs() <= 1e-15 * scale && (vb / nb).abs() <= 1e-15 * scale {
            return Some(p);
        }
        // Solve (J J^T) w = g, delta = -J^T w.
        let (aa, ab, bb) = (dot(ga, ga), dot(ga, gb), dot(gb, gb));
        let det = aa * bb - ab * ab;
        if det.is_nan() || det <= 1e-20 * aa * bb {
            return None;
        }
        let wa = (bb * va - ab * vb) / det;
        let wb = (aa * vb - ab * va) / det;
        let step: P = std::array::from_fn(|k| wa * ga[k] + wb * gb[k]);
        p = sub(p, step);
        if !p.iter().all(|x| x.is_finite()) || norm(step) > scale {
            return None;
        }
    }
    None
}

/// Unit tangent of the intersection curve, or an error at a tangency.
fn tangent(a: &dyn Implicit, b: &dyn Implicit, p: P) -> Result<P> {
    let (ga, gb) = (a.gradient(p), b.gradient(p));
    let t = cross(ga, gb);
    let n = norm(t);
    if n.is_nan() || n <= 1e-6 * norm(ga) * norm(gb) {
        return Err(refuse(
            "Intersection trace reached a tangency: the surfaces touch and the curve is not regular",
        ));
    }
    Ok(t.map(|x| x / n))
}

/// Trace one closed component starting at a point on both surfaces.
fn trace_loop(
    a: &dyn Implicit,
    b: &dyn Implicit,
    start: P,
    step: f64,
    scale: f64,
) -> Result<Vec<P>> {
    let mut points = vec![start];
    let mut direction = tangent(a, b, start)?;
    let mut p = start;
    let mut local = step;
    for _ in 0..20000 {
        let predicted: P = std::array::from_fn(|k| p[k] + local * direction[k]);
        let accepted = correct(a, b, predicted, scale).and_then(|q| {
            let moved = norm(sub(q, p));
            let t = tangent(a, b, q).ok()?;
            let t = if dot(t, direction) < 0. {
                t.map(|x| -x)
            } else {
                t
            };
            (moved > 0.3 * local && moved < 2. * local && dot(t, direction) > 0.94)
                .then_some((q, t))
        });
        match accepted {
            Some((q, t)) => {
                if points.len() > 8 && norm(sub(q, start)) < 1.2 * step {
                    return Ok(points);
                }
                points.push(q);
                p = q;
                direction = t;
                local = step;
            }
            None => {
                local *= 0.5;
                if local < step / 64. {
                    return Err(refuse("Intersection trace could not continue the curve"));
                }
            }
        }
    }
    Err(refuse("Intersection trace exceeded its step budget"))
}

/// A cubic Bezier span with interval control points.
type Span = [[Interval; 3]; 4];

/// Interval de Casteljau halves of a cubic span.
fn split(span: &Span) -> Result<(Span, Span)> {
    let half = Interval::point(0.5);
    let mid = |a: [Interval; 3], b: [Interval; 3]| -> Result<[Interval; 3]> {
        let mut out = [Interval::point(0.); 3];
        for k in 0..3 {
            out[k] = a[k].add(b[k])?.mul(half)?;
        }
        Ok(out)
    };
    let [p0, p1, p2, p3] = *span;
    let (q0, q1, q2) = (mid(p0, p1)?, mid(p1, p2)?, mid(p2, p3)?);
    let (r0, r1) = (mid(q0, q1)?, mid(q1, q2)?);
    let s = mid(r0, r1)?;
    Ok(([p0, q0, r0, s], [s, r1, q2, p3]))
}

/// Largest distance bound of a span from either surface, halving the span
/// until both are below `tolerance`. `None` when the budget or depth runs out.
fn certify_span(
    surfaces: [&dyn Implicit; 2],
    span: Span,
    tolerance: f64,
    depth: usize,
    cells: &mut usize,
) -> Result<Option<f64>> {
    *cells += 1;
    if *cells > 400_000 {
        return Ok(None);
    }
    let xyz: [Bern; 3] = std::array::from_fn(|k| Bern(span.iter().map(|p| p[k]).collect()));
    let mut worst = 0_f64;
    for surface in surfaces {
        worst = worst.max(surface.compose(&xyz)?.magnitude() * surface.distance_factor(&xyz));
    }
    if worst <= tolerance {
        return Ok(Some(worst));
    }
    if depth == 0 {
        return Ok(None);
    }
    let (left, right) = split(&span)?;
    let Some(l) = certify_span(surfaces, left, tolerance, depth - 1, cells)? else {
        return Ok(None);
    };
    let Some(r) = certify_span(surfaces, right, tolerance, depth - 1, cells)? else {
        return Ok(None);
    };
    Ok(Some(l.max(r)))
}

/// A traced, certified intersection component.
#[derive(Clone, Debug)]
pub(crate) struct TracedLoop {
    pub(crate) pieces: Vec<Curve>,
    pub(crate) distance_bound: f64,
    pub(crate) spans: usize,
}

/// Fit and certify one closed polyline; `None` when some span cannot be
/// certified, which asks the caller for a finer trace.
fn fit_loop(
    surfaces: [&dyn Implicit; 2],
    points: &[P],
    tolerance: f64,
) -> Result<Option<TracedLoop>> {
    let mut closed = points.to_vec();
    closed.push(points[0]);
    let n = closed.len() - 1;
    let pieces_count = n.div_ceil(179).max(1);
    let mut pieces = Vec::new();
    let mut bound = 0_f64;
    let mut spans = 0;
    let mut cells = 0;
    for k in 0..pieces_count {
        let (lo, hi) = (k * n / pieces_count, (k + 1) * n / pieces_count);
        let slice: Vec<P> = closed[lo..=hi].to_vec();
        if slice.len() < 4 {
            return Err(refuse("Intersection loop is too short to fit"));
        }
        let curve = interpolate_curve(&slice, 3, Parameterization::ChordLength, None)?;
        for segment in curve.decompose()? {
            let c = &segment.definition().control_points;
            if c.len() != 4 {
                return Err(refuse("Unexpected span degree in a traced intersection"));
            }
            let span: Span =
                std::array::from_fn(|i| std::array::from_fn(|k| Interval::point(c[i][k])));
            let Some(b) = certify_span(surfaces, span, tolerance, 14, &mut cells)? else {
                return Ok(None);
            };
            bound = bound.max(b);
            spans += 1;
        }
        pieces.push(curve);
    }
    Ok(Some(TracedLoop {
        pieces,
        distance_bound: bound,
        spans,
    }))
}

/// Trace and certify every component found from the seed grid. `seed` maps
/// grid parameters `(polar, azimuth)` to points of surface `a`; components
/// are certified against both surfaces within `tolerance`.
pub(crate) fn trace_intersection(
    a: &dyn Implicit,
    b: &dyn Implicit,
    seed: impl Fn(f64, f64) -> P,
    scale: f64,
    feature: f64,
    tolerance: f64,
) -> Result<Vec<TracedLoop>> {
    // Seeds: sign changes of g_b along polar lines of the parameterization of a.
    let (rows, columns) = (97, 128);
    let mut seeds = Vec::new();
    for j in 0..columns {
        let phi = std::f64::consts::TAU * j as f64 / columns as f64;
        let theta = |i: usize| std::f64::consts::PI * i as f64 / (rows - 1) as f64;
        for i in 0..rows - 1 {
            let (t0, t1) = (theta(i), theta(i + 1));
            let (v0, v1) = (b.value(seed(t0, phi)), b.value(seed(t1, phi)));
            if v0 == 0. || v0.signum() != v1.signum() {
                let (mut lo, mut hi, mut vlo) = (t0, t1, v0);
                for _ in 0..60 {
                    let mid = 0.5 * (lo + hi);
                    let v = b.value(seed(mid, phi));
                    if v.signum() == vlo.signum() && v != 0. {
                        lo = mid;
                        vlo = v;
                    } else {
                        hi = mid;
                    }
                }
                if let Some(p) = correct(a, b, seed(0.5 * (lo + hi), phi), scale) {
                    seeds.push(p);
                }
            }
        }
    }
    let mut loops: Vec<Vec<P>> = Vec::new();
    let base_step = 0.05 * feature;
    let near = |p: P, loops: &Vec<Vec<P>>, step: f64| {
        loops.iter().flatten().any(|q| norm(sub(p, *q)) < 2. * step)
    };
    for s in &seeds {
        if near(*s, &loops, base_step) {
            continue;
        }
        loops.push(trace_loop(a, b, *s, base_step, scale)?);
    }
    let mut out = Vec::with_capacity(loops.len());
    for first in &loops {
        // Retrace at halving steps until every span certifies.
        let mut step = base_step;
        let mut points = first.clone();
        let mut certified = None;
        for _ in 0..5 {
            if points.len() > 3000 {
                break;
            }
            if let Some(fit) = fit_loop([a, b], &points, tolerance)? {
                certified = Some(fit);
                break;
            }
            step *= 0.5;
            points = trace_loop(a, b, first[0], step, scale)?;
        }
        out.push(certified.ok_or_else(|| {
            refuse("Intersection component could not be certified within the tolerance")
        })?);
    }
    Ok(out)
}

/// Homogeneous Bernstein coordinates `[X, Y, Z, W]` of a single-span
/// rational Bezier patch composed with a Bezier UV span given by the
/// Bernstein forms of `u(t)` and `v(t)` (already mapped to the patch's
/// parameter domain).
pub(crate) fn compose_patch(surface: &nurbs_core::surface::Surface, u: &Bern, v: &Bern) -> Result<[Bern; 4]> {
    let (p, q) = (surface.degree_u, surface.degree_v);
    if surface.control_points.len() != p + 1 || surface.control_points[0].len() != q + 1 {
        return Err(refuse("lift certificate needs a single-span patch"));
    }
    let map = |b: &Bern, knots: &[f64], degree: usize| -> Result<Bern> {
        let (a, z) = (knots[degree], knots[knots.len() - degree - 1]);
        Ok(Bern(
            b.0.iter()
                .map(|x| x.sub(Interval::point(a))?.div(Interval::point(z - a)))
                .collect::<Result<_>>()?,
        ))
    };
    let (u, v) = (map(u, &surface.knots_u, p)?, map(v, &surface.knots_v, q)?);
    // Bernstein basis polynomials B_i^n(s(t)) as polynomials in t.
    let basis = |s: &Bern, n: usize| -> Result<Vec<Bern>> {
        let one_minus = Bern::constant(1., 0).sub(s)?;
        (0..=n)
            .map(|i| {
                let mut b = Bern::constant(binomial(n, i), 0);
                for _ in 0..i {
                    b = b.mul(s)?;
                }
                for _ in 0..n - i {
                    b = b.mul(&one_minus)?;
                }
                Ok(b)
            })
            .collect()
    };
    let (bu, bv) = (basis(&u, p)?, basis(&v, q)?);
    let mut out: [Bern; 4] = std::array::from_fn(|_| Bern::constant(0., 0));
    for i in 0..=p {
        for j in 0..=q {
            let w = surface.weights[i][j];
            let b = bu[i].mul(&bv[j])?.scale(w)?;
            for k in 0..3 {
                out[k] = out[k].add(&b.scale(surface.control_points[i][j][k])?)?;
            }
            out[3] = out[3].add(&b)?;
        }
    }
    Ok(out)
}

/// Certified bound on the distance from `other` of every point of a
/// rational span `X / W`, or `None` when subdivision to `depth` cannot
/// reach `tolerance` (or a weight is not provably positive).
pub(crate) fn certify_rational_span(
    other: &dyn Implicit,
    xyzw: [Bern; 4],
    tolerance: f64,
    depth: usize,
    cells: &mut usize,
) -> Result<Option<f64>> {
    *cells += 1;
    if *cells > 200_000 {
        return Ok(None);
    }
    let w = xyzw[3].hull()?;
    if w.lo > 0. {
        let (g, k) = other.compose_homogeneous(&xyzw)?;
        let box3: [Bern; 3] = std::array::from_fn(|i| {
            xyzw[i]
                .hull()
                .and_then(|x| x.div(w))
                .map(|x| Bern(vec![x]))
                .unwrap_or_else(|_| Bern(vec![Interval::point(f64::INFINITY)]))
        });
        let bound = g.magnitude() / w.lo.powi(k) * other.distance_factor(&box3);
        if bound.is_finite() && bound <= tolerance {
            return Ok(Some(bound));
        }
    }
    if depth == 0 {
        return Ok(None);
    }
    let halves: Vec<(Bern, Bern)> = xyzw.iter().map(Bern::halves).collect::<Result<_>>()?;
    let left = std::array::from_fn(|i| halves[i].0.clone());
    let right = std::array::from_fn(|i| halves[i].1.clone());
    let Some(l) = certify_rational_span(other, left, tolerance, depth - 1, cells)? else {
        return Ok(None);
    };
    let Some(r) = certify_rational_span(other, right, tolerance, depth - 1, cells)? else {
        return Ok(None);
    };
    Ok(Some(l.max(r)))
}

/// Certified distance bound from `other` of the image of a UV pcurve
/// (any degree, B-spline) on a single-span patch.
pub(crate) fn certify_lift(
    surface: &nurbs_core::surface::Surface,
    pcurve: &Curve,
    other: &dyn Implicit,
    tolerance: f64,
) -> Result<Option<f64>> {
    let mut worst = 0_f64;
    let mut cells = 0;
    for segment in pcurve.decompose()? {
        let c = &segment.definition().control_points;
        let u = Bern(c.iter().map(|p| Interval::point(p[0])).collect());
        let v = Bern(c.iter().map(|p| Interval::point(p[1])).collect());
        let xyzw = compose_patch(surface, &u, &v)?;
        match certify_rational_span(other, xyzw, tolerance, 12, &mut cells)? {
            Some(b) => worst = worst.max(b),
            None => return Ok(None),
        }
    }
    Ok(Some(worst))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bernstein_products_bound_polynomials() {
        // (t)(1 - t) has Bernstein coefficients [0, 1/2, 0] in degree 2 and a
        // maximum of 1/4.
        let t = Bern(vec![Interval::point(0.), Interval::point(1.)]);
        let u = Bern(vec![Interval::point(1.), Interval::point(0.)]);
        let p = t.mul(&u).unwrap();
        assert_eq!(p.degree(), 2);
        assert!((p.magnitude() - 0.5).abs() < 1e-15);
        let elevated = p.elevate_to(4).unwrap();
        assert!(elevated.magnitude() <= p.magnitude() + 1e-15);
        assert!(elevated.magnitude() >= 0.25);
    }

    #[test]
    fn certified_off_axis_sphere_torus_curves() {
        let torus = TorusImplicit {
            center: [0.; 3],
            frame: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            major: 3.,
            minor: 1.,
        };
        let sphere = SphereImplicit {
            center: [3.5, 0.4, 0.3],
            radius: 1.,
        };
        let seed = |theta: f64, phi: f64| -> P {
            let (st, ct) = theta.sin_cos();
            let (sp, cp) = phi.sin_cos();
            [3.5 + st * cp, 0.4 + st * sp, 0.3 + ct]
        };
        let loops = trace_intersection(&sphere, &torus, seed, 5., 1., 1e-7).unwrap();
        assert!(!loops.is_empty());
        for l in &loops {
            assert!(l.distance_bound <= 1e-7, "{}", l.distance_bound);
            // Independent check: dense samples lie on both surfaces.
            for piece in &l.pieces {
                for k in 0..=50 {
                    let p = piece.evaluate(k as f64 / 50.).unwrap().point;
                    let p = [p[0], p[1], p[2]];
                    let ds = (norm(sub(p, sphere.center)) - 1.).abs();
                    let rho = p[0].hypot(p[1]);
                    let dt = ((rho - 3.).hypot(p[2]) - 1.).abs();
                    assert!(ds <= 1e-7 && dt <= 1e-7, "{ds} {dt}");
                }
            }
            // Pieces chain into a closed loop with exactly shared ends.
            for w in l.pieces.windows(2) {
                assert_eq!(w[0].control_points.last(), w[1].control_points.first());
            }
            assert_eq!(
                l.pieces.last().unwrap().control_points.last(),
                l.pieces[0].control_points.first()
            );
        }
    }
}
