//! Recognition of analytic primitives (plane, sphere, cylinder, cone, torus)
//! from the coefficients of a rational B-spline surface.
//!
//! Samples are used ONLY to propose parameters (via [`super::primitive_fit`]).
//! Acceptance is decided from the original coefficients: the surface is split
//! into rational Bezier patches, the primitive's implicit equation is
//! substituted into the homogeneous patch polynomials in Bernstein form, and
//! the largest Bernstein coefficient bounds the implicit residual everywhere
//! on the patch (convex hull property). Each primitive's residual is then
//! converted into a distance bound with a factorization lower bound:
//!
//! * plane `n.x + o`: distance `= |F| / W`;
//! * sphere/cylinder `rho^2 - r^2 = (rho - r)(rho + r)`: distance `<= |G| / (W^2 r)`;
//! * cone (single nappe, patch strictly off the apex, `h >= h_min > 0`):
//!   `cos^2 a rho^2 - sin^2 a h^2 = (rho cos a - h sin a)(rho cos a + h sin a)`,
//!   distance `<= |G| / (W^2 h_min sin a)`;
//! * ring torus `T = (s - r^2)(s' - r^2)`, `s' - r^2 >= R^2 - r^2`:
//!   distance `<= |T| / (W^4 r (R^2 - r^2))`.
//!
//! Floating-point rounding of the Bernstein products is covered by a
//! relative slack on the tracked coefficient magnitudes; the decomposition
//! into Bezier patches itself is a binary64 knot insertion and is not
//! separately interval-certified. Cones whose patch touches the apex and
//! non-ring tori are refused (no factor lower bound), never guessed.
use crate::analysis::primitive_fit::{fit_cone, fit_cylinder, fit_plane, fit_sphere, fit_torus};
use crate::{Result, check, surface::Surface};

/// Recognized analytic primitive with its parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnalyticPrimitive {
    /// `normal . x + offset = 0`, unit normal.
    Plane { normal: [f64; 3], offset: f64 },
    Sphere { center: [f64; 3], radius: f64 },
    Cylinder { point: [f64; 3], axis: [f64; 3], radius: f64 },
    /// Single nappe opening along `axis` from `apex`, half angle in radians.
    Cone { apex: [f64; 3], axis: [f64; 3], half_angle: f64 },
    Torus { center: [f64; 3], axis: [f64; 3], major_radius: f64, minor_radius: f64 },
}

/// Accepted recognition: the primitive plus the coefficient-derived bound on
/// the distance of every surface point to it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Recognition {
    pub primitive: AnalyticPrimitive,
    pub distance_upper: f64,
}

/// Relative rounding slack applied to the tracked coefficient magnitudes.
const SLACK: f64 = 1e-12;
const GRID: usize = 13;

/// Tensor Bernstein polynomial with tracked magnitude (sum of |terms|).
#[derive(Clone)]
struct Poly {
    p: usize,
    q: usize,
    c: Vec<f64>,
    m: Vec<f64>,
}

fn binom(n: usize, k: usize) -> f64 {
    (0..k).fold(1., |acc, i| acc * (n - i) as f64 / (i + 1) as f64)
}

impl Poly {
    fn at(&self, i: usize, j: usize) -> usize {
        i * (self.q + 1) + j
    }
    fn mul(&self, o: &Poly) -> Poly {
        let (p, q) = (self.p + o.p, self.q + o.q);
        let mut c = vec![0.; (p + 1) * (q + 1)];
        let mut m = vec![0.; (p + 1) * (q + 1)];
        for i1 in 0..=self.p {
            for j1 in 0..=self.q {
                let a = self.c[self.at(i1, j1)];
                let am = self.m[self.at(i1, j1)];
                let fa = binom(self.p, i1) * binom(self.q, j1);
                for i2 in 0..=o.p {
                    for j2 in 0..=o.q {
                        let f = fa * binom(o.p, i2) * binom(o.q, j2)
                            / (binom(p, i1 + i2) * binom(q, j1 + j2));
                        let k = (i1 + i2) * (q + 1) + j1 + j2;
                        c[k] += f * a * o.c[o.at(i2, j2)];
                        m[k] += f * am * o.m[o.at(i2, j2)];
                    }
                }
            }
        }
        Poly { p, q, c, m }
    }
    fn lin(&self, a: f64, o: &Poly, b: f64) -> Poly {
        debug_assert!(self.p == o.p && self.q == o.q);
        Poly {
            p: self.p,
            q: self.q,
            c: self.c.iter().zip(&o.c).map(|(x, y)| a * x + b * y).collect(),
            m: self.m.iter().zip(&o.m).map(|(x, y)| a.abs() * x + b.abs() * y).collect(),
        }
    }
    fn scale(&self, a: f64) -> Poly {
        Poly {
            p: self.p,
            q: self.q,
            c: self.c.iter().map(|x| a * x).collect(),
            m: self.m.iter().map(|x| a.abs() * x).collect(),
        }
    }
    /// Upper bound of |poly| over the patch.
    fn abs_max(&self) -> f64 {
        self.c
            .iter()
            .zip(&self.m)
            .map(|(c, m)| c.abs() + SLACK * m)
            .fold(0., f64::max)
    }
    fn min_lower(&self) -> f64 {
        self.c
            .iter()
            .zip(&self.m)
            .map(|(c, m)| c - SLACK * m)
            .fold(f64::INFINITY, f64::min)
    }
    fn max_upper(&self) -> f64 {
        self.c
            .iter()
            .zip(&self.m)
            .map(|(c, m)| c + SLACK * m)
            .fold(f64::NEG_INFINITY, f64::max)
    }
}

/// Homogeneous patch polynomials [wx, wy, wz, w].
fn homogeneous(patch: &Surface) -> [Poly; 4] {
    let (p, q) = (patch.degree_u, patch.degree_v);
    std::array::from_fn(|k| {
        let mut c = Vec::with_capacity((p + 1) * (q + 1));
        for i in 0..=p {
            for j in 0..=q {
                let w = patch.weights[i][j];
                c.push(if k == 3 { w } else { w * patch.control_points[i][j][k] });
            }
        }
        let m = c.iter().map(|x: &f64| x.abs()).collect();
        Poly { p, q, c, m }
    })
}

fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn unit(a: [f64; 3]) -> Option<[f64; 3]> {
    let n = dot3(a, a).sqrt();
    (n.is_finite() && n > 0.).then(|| a.map(|x| x / n))
}

/// D = P - c W, the homogeneous offset from `c`.
fn offset(h: &[Poly; 4], c: [f64; 3]) -> [Poly; 3] {
    std::array::from_fn(|k| h[k].lin(1., &h[3], -c[k]))
}

fn along(d: &[Poly; 3], e: [f64; 3]) -> Poly {
    d[0].scale(e[0]).lin(1., &d[1], e[1]).lin(1., &d[2], e[2])
}

fn square_norm(d: &[Poly; 3]) -> Poly {
    d[0].mul(&d[0]).lin(1., &d[1].mul(&d[1]), 1.).lin(1., &d[2].mul(&d[2]), 1.)
}

/// Coefficient-certified distance bound of one patch to `primitive`, or `None`
/// when the factorization lower bound is unavailable on this patch.
fn patch_bound(patch: &Surface, primitive: &AnalyticPrimitive) -> Option<f64> {
    let h = homogeneous(patch);
    let w = &h[3];
    let w_min = w.min_lower();
    if !(w_min > 0.) {
        return None;
    }
    let w2 = w.mul(w);
    let bound = match *primitive {
        AnalyticPrimitive::Plane { normal, offset: o } => {
            let f = h[0].scale(normal[0]).lin(1., &h[1], normal[1]).lin(1., &h[2], normal[2]).lin(1., w, o);
            f.abs_max() / w_min
        }
        AnalyticPrimitive::Sphere { center, radius } => {
            let g = square_norm(&offset(&h, center)).lin(1., &w2, -radius * radius);
            g.abs_max() / (w_min * w_min * radius)
        }
        AnalyticPrimitive::Cylinder { point, axis, radius } => {
            let d = offset(&h, point);
            let a = along(&d, axis);
            let g = square_norm(&d).lin(1., &a.mul(&a), -1.).lin(1., &w2, -radius * radius);
            g.abs_max() / (w_min * w_min * radius)
        }
        AnalyticPrimitive::Cone { apex, axis, half_angle } => {
            let d = offset(&h, apex);
            let a = along(&d, axis);
            let w_max = w.max_upper();
            let h_min = a.min_lower() / w_max;
            if !(h_min > 0.) {
                return None;
            }
            let (s, c) = half_angle.sin_cos();
            let a2 = a.mul(&a);
            let rho2 = square_norm(&d).lin(1., &a2, -1.);
            let g = rho2.lin(c * c, &a2, -s * s);
            g.abs_max() / (w_min * w_min * h_min * s)
        }
        AnalyticPrimitive::Torus { center, axis, major_radius: big, minor_radius: r } => {
            let d = offset(&h, center);
            let s = square_norm(&d);
            let a = along(&d, axis);
            let rho2 = s.lin(1., &a.mul(&a), -1.);
            let aa = s.lin(1., &w2, big * big - r * r);
            let t = aa.mul(&aa).lin(1., &rho2.mul(&w2), -4. * big * big);
            t.abs_max() / (w_min.powi(4) * r * (big * big - r * r))
        }
    };
    bound.is_finite().then_some(bound)
}

fn admissible(primitive: &AnalyticPrimitive) -> bool {
    match *primitive {
        AnalyticPrimitive::Plane { normal, offset } => {
            offset.is_finite() && (dot3(normal, normal) - 1.).abs() < 1e-12
        }
        AnalyticPrimitive::Sphere { center, radius } => {
            center.iter().all(|x| x.is_finite()) && radius.is_finite() && radius > 0.
        }
        AnalyticPrimitive::Cylinder { point, axis, radius } => {
            point.iter().all(|x| x.is_finite())
                && (dot3(axis, axis) - 1.).abs() < 1e-12
                && radius.is_finite()
                && radius > 0.
        }
        AnalyticPrimitive::Cone { apex, axis, half_angle } => {
            apex.iter().all(|x| x.is_finite())
                && (dot3(axis, axis) - 1.).abs() < 1e-12
                && half_angle > 0.
                && half_angle < std::f64::consts::FRAC_PI_2
        }
        AnalyticPrimitive::Torus { center, axis, major_radius, minor_radius } => {
            center.iter().all(|x| x.is_finite())
                && (dot3(axis, axis) - 1.).abs() < 1e-12
                && minor_radius > 0.
                && major_radius > minor_radius
                && major_radius.is_finite()
        }
    }
}

/// Certify `surface` against a given primitive from its coefficients.
/// Returns the distance upper bound, or `None` when no bound is available.
pub fn certify_primitive(surface: &Surface, primitive: &AnalyticPrimitive) -> Result<Option<f64>> {
    surface.validate()?;
    check(admissible(primitive), "Primitive parameters must be finite with unit axes and admissible radii")?;
    let mut worst = 0_f64;
    for patch in crate::surface_edit::decompose(surface)? {
        match patch_bound(&patch, primitive) {
            Some(b) => worst = worst.max(b),
            None => return Ok(None),
        }
    }
    Ok(Some(worst))
}

fn samples(surface: &Surface) -> Result<Vec<[f64; 3]>> {
    let (p, q) = (surface.degree_u, surface.degree_v);
    let (n, m) = (surface.control_points.len(), surface.control_points[0].len());
    let du = [surface.knots_u[p], surface.knots_u[n]];
    let dv = [surface.knots_v[q], surface.knots_v[m]];
    let mut out = Vec::with_capacity(GRID * GRID);
    for i in 0..GRID {
        for j in 0..GRID {
            let s = (i as f64 + 0.5) / GRID as f64;
            let t = (j as f64 + 0.5) / GRID as f64;
            let e = surface.evaluate(du[0] + s * (du[1] - du[0]), dv[0] + t * (dv[1] - dv[0]))?;
            out.push([e.point[0], e.point[1], e.point[2]]);
        }
    }
    Ok(out)
}

/// Candidate primitives proposed from samples, simplest class first.
fn proposals(points: &[[f64; 3]]) -> Vec<AnalyticPrimitive> {
    let mut out = Vec::new();
    if let Ok(f) = fit_plane(points) {
        if let Some(normal) = unit(f.normal) {
            out.push(AnalyticPrimitive::Plane { normal, offset: f.offset });
        }
    }
    if let Ok(f) = fit_sphere(points) {
        out.push(AnalyticPrimitive::Sphere { center: f.center, radius: f.radius });
    }
    if let Ok(f) = fit_cylinder(points) {
        if let Some(axis) = unit(f.direction) {
            out.push(AnalyticPrimitive::Cylinder { point: f.point_on_axis, axis, radius: f.radius });
        }
    }
    if let Ok(f) = fit_cone(points) {
        if let Some(axis) = unit(f.axis) {
            out.push(AnalyticPrimitive::Cone { apex: f.vertex, axis, half_angle: f.half_angle });
        }
    }
    if let Ok(f) = fit_torus(points) {
        if let Some(axis) = unit(f.axis) {
            out.push(AnalyticPrimitive::Torus {
                center: f.center,
                axis,
                major_radius: f.major_radius,
                minor_radius: f.minor_radius,
            });
        }
    }
    out
}

/// Recognize the simplest analytic primitive the surface lies on within
/// `tolerance` (absolute distance), certified from its coefficients.
/// Returns `None` for freeform surfaces and near-misses.
pub fn recognize_analytic_surface(surface: &Surface, tolerance: f64) -> Result<Option<Recognition>> {
    check(tolerance.is_finite() && tolerance > 0., "Recognition tolerance must be positive and finite")?;
    surface.validate()?;
    let points = samples(surface)?;
    for primitive in proposals(&points).into_iter().filter(admissible) {
        if let Some(distance_upper) = certify_primitive(surface, &primitive)? {
            if distance_upper <= tolerance {
                return Ok(Some(Recognition { primitive, distance_upper }));
            }
        }
    }
    Ok(None)
}
