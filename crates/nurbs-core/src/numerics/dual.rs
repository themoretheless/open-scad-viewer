//! Forward-mode automatic differentiation over the rational B-spline kernel.
//!
//! `Dual` carries a value and its first derivative (ε² = 0), `HyperDual`
//! carries a value, two first derivatives and one mixed second derivative
//! (ε₁² = ε₂² = ε₁ε₂ = 0). B-spline basis functions are polynomials on every
//! knot span, so the Cox–de Boor recurrence lifts verbatim to both algebras;
//! the rational quotient rule then yields exact analytic derivatives of NURBS
//! curves and surfaces. Seeding ε₁ = ε₂ = 1 on one parameter recovers the
//! unmixed second derivative from the mixed slot.
use crate::curve::Curve;
use crate::surface::Surface;
use crate::{Result, check, numeric, numeric_err};
use std::ops::{Add, Div, Mul, Neg, Sub};

/// First-order dual number `val + der·ε` with ε² = 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dual {
    pub val: f64,
    pub der: f64,
}

impl Dual {
    /// Constant (zero derivative).
    pub fn constant(val: f64) -> Self {
        Self { val, der: 0. }
    }
    /// Independent variable seeded at `val` (unit derivative).
    pub fn variable(val: f64) -> Self {
        Self { val, der: 1. }
    }
    pub fn sqrt(self) -> Self {
        let val = self.val.sqrt();
        Self {
            val,
            der: 0.5 * self.der / val,
        }
    }
    pub fn sin(self) -> Self {
        Self {
            val: self.val.sin(),
            der: self.val.cos() * self.der,
        }
    }
    pub fn cos(self) -> Self {
        Self {
            val: self.val.cos(),
            der: -self.val.sin() * self.der,
        }
    }
    pub fn exp(self) -> Self {
        let val = self.val.exp();
        Self {
            val,
            der: val * self.der,
        }
    }
    pub fn ln(self) -> Self {
        Self {
            val: self.val.ln(),
            der: self.der / self.val,
        }
    }
    pub fn powf(self, exponent: Dual) -> Self {
        // x^y = exp(y·ln x), so d = x^y·(y'·ln x + y·x'/x).
        let val = self.val.powf(exponent.val);
        Self {
            val,
            der: val * (exponent.der * self.val.ln() + exponent.val * self.der / self.val),
        }
    }
    pub fn powi(self, n: i32) -> Self {
        Self {
            val: self.val.powi(n),
            der: n as f64 * self.val.powi(n - 1) * self.der,
        }
    }
    /// Two-argument arctangent lifted along the chain rule.
    pub fn atan2(self, x: Dual) -> Self {
        Self {
            val: self.val.atan2(x.val),
            der: (x.val * self.der - self.val * x.der) / (x.val * x.val + self.val * self.val),
        }
    }
}

impl Add for Dual {
    type Output = Dual;
    fn add(self, rhs: Dual) -> Dual {
        Dual {
            val: self.val + rhs.val,
            der: self.der + rhs.der,
        }
    }
}
impl Sub for Dual {
    type Output = Dual;
    fn sub(self, rhs: Dual) -> Dual {
        Dual {
            val: self.val - rhs.val,
            der: self.der - rhs.der,
        }
    }
}
impl Mul for Dual {
    type Output = Dual;
    fn mul(self, rhs: Dual) -> Dual {
        Dual {
            val: self.val * rhs.val,
            der: self.der * rhs.val + self.val * rhs.der,
        }
    }
}
impl Div for Dual {
    type Output = Dual;
    fn div(self, rhs: Dual) -> Dual {
        Dual {
            val: self.val / rhs.val,
            der: (self.der * rhs.val - self.val * rhs.der) / (rhs.val * rhs.val),
        }
    }
}
impl Neg for Dual {
    type Output = Dual;
    fn neg(self) -> Dual {
        Dual {
            val: -self.val,
            der: -self.der,
        }
    }
}
impl Add<f64> for Dual {
    type Output = Dual;
    fn add(self, rhs: f64) -> Dual {
        self + Dual::constant(rhs)
    }
}
impl Sub<f64> for Dual {
    type Output = Dual;
    fn sub(self, rhs: f64) -> Dual {
        self - Dual::constant(rhs)
    }
}
impl Mul<f64> for Dual {
    type Output = Dual;
    fn mul(self, rhs: f64) -> Dual {
        Dual {
            val: self.val * rhs,
            der: self.der * rhs,
        }
    }
}
impl Div<f64> for Dual {
    type Output = Dual;
    fn div(self, rhs: f64) -> Dual {
        Dual {
            val: self.val / rhs,
            der: self.der / rhs,
        }
    }
}

/// Second-order hyper-dual number `val + d1·ε₁ + d2·ε₂ + d12·ε₁ε₂`
/// with ε₁² = ε₂² = ε₁ε₂ = 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HyperDual {
    pub val: f64,
    pub d1: f64,
    pub d2: f64,
    pub d12: f64,
}

impl HyperDual {
    /// Constant (all derivatives zero).
    pub fn constant(val: f64) -> Self {
        Self {
            val,
            d1: 0.,
            d2: 0.,
            d12: 0.,
        }
    }
    /// Variable seeded along ε₁ only (for ∂/∂u, ∂²/∂u∂v mixed slots).
    pub fn variable_e1(val: f64) -> Self {
        Self {
            val,
            d1: 1.,
            d2: 0.,
            d12: 0.,
        }
    }
    /// Variable seeded along ε₂ only.
    pub fn variable_e2(val: f64) -> Self {
        Self {
            val,
            d1: 0.,
            d2: 1.,
            d12: 0.,
        }
    }
    /// Variable seeded along both ε₁ and ε₂: the mixed slot then carries
    /// the unmixed second derivative ∂²/∂u² of any function of it.
    pub fn variable_second(val: f64) -> Self {
        Self {
            val,
            d1: 1.,
            d2: 1.,
            d12: 0.,
        }
    }
    /// Multiplicative inverse via the ε-expansion of 1/x.
    pub fn recip(self) -> Self {
        let v2 = self.val * self.val;
        Self {
            val: 1. / self.val,
            d1: -self.d1 / v2,
            d2: -self.d2 / v2,
            d12: (2. * self.d1 * self.d2 - self.val * self.d12) / (v2 * self.val),
        }
    }
    pub fn sqrt(self) -> Self {
        let s = self.val.sqrt();
        Self {
            val: s,
            d1: 0.5 * self.d1 / s,
            d2: 0.5 * self.d2 / s,
            d12: 0.5 * self.d12 / s - 0.25 * self.d1 * self.d2 / (s * self.val),
        }
    }
    pub fn sin(self) -> Self {
        let (s, c) = self.val.sin_cos();
        Self {
            val: s,
            d1: c * self.d1,
            d2: c * self.d2,
            d12: c * self.d12 - s * self.d1 * self.d2,
        }
    }
    pub fn cos(self) -> Self {
        let (s, c) = self.val.sin_cos();
        Self {
            val: c,
            d1: -s * self.d1,
            d2: -s * self.d2,
            d12: -s * self.d12 - c * self.d1 * self.d2,
        }
    }
    pub fn exp(self) -> Self {
        let e = self.val.exp();
        Self {
            val: e,
            d1: e * self.d1,
            d2: e * self.d2,
            d12: e * (self.d12 + self.d1 * self.d2),
        }
    }
    pub fn ln(self) -> Self {
        Self {
            val: self.val.ln(),
            d1: self.d1 / self.val,
            d2: self.d2 / self.val,
            d12: self.d12 / self.val - self.d1 * self.d2 / (self.val * self.val),
        }
    }
    pub fn powi(self, n: i32) -> Self {
        let val = self.val.powi(n);
        let first = n as f64 * self.val.powi(n - 1);
        let second = (n as f64) * ((n - 1) as f64) * self.val.powi(n - 2);
        Self {
            val,
            d1: first * self.d1,
            d2: first * self.d2,
            d12: first * self.d12 + second * self.d1 * self.d2,
        }
    }
}

impl Add for HyperDual {
    type Output = HyperDual;
    fn add(self, rhs: HyperDual) -> HyperDual {
        HyperDual {
            val: self.val + rhs.val,
            d1: self.d1 + rhs.d1,
            d2: self.d2 + rhs.d2,
            d12: self.d12 + rhs.d12,
        }
    }
}
impl Sub for HyperDual {
    type Output = HyperDual;
    fn sub(self, rhs: HyperDual) -> HyperDual {
        HyperDual {
            val: self.val - rhs.val,
            d1: self.d1 - rhs.d1,
            d2: self.d2 - rhs.d2,
            d12: self.d12 - rhs.d12,
        }
    }
}
impl Mul for HyperDual {
    type Output = HyperDual;
    fn mul(self, rhs: HyperDual) -> HyperDual {
        HyperDual {
            val: self.val * rhs.val,
            d1: self.d1 * rhs.val + self.val * rhs.d1,
            d2: self.d2 * rhs.val + self.val * rhs.d2,
            d12: self.d12 * rhs.val + self.val * rhs.d12 + self.d1 * rhs.d2 + self.d2 * rhs.d1,
        }
    }
}
impl Div for HyperDual {
    type Output = HyperDual;
    fn div(self, rhs: HyperDual) -> HyperDual {
        self * rhs.recip()
    }
}
impl Neg for HyperDual {
    type Output = HyperDual;
    fn neg(self) -> HyperDual {
        HyperDual {
            val: -self.val,
            d1: -self.d1,
            d2: -self.d2,
            d12: -self.d12,
        }
    }
}
impl Mul<f64> for HyperDual {
    type Output = HyperDual;
    fn mul(self, rhs: f64) -> HyperDual {
        HyperDual {
            val: self.val * rhs,
            d1: self.d1 * rhs,
            d2: self.d2 * rhs,
            d12: self.d12 * rhs,
        }
    }
}

/// Point in R³ with first-derivative jets on every coordinate.
pub type Dual3 = [Dual; 3];

/// Scalar algebra shared by the lifted basis recurrence.
pub(crate) trait Scalar:
    Clone + Add<Output = Self> + Sub<Output = Self> + Mul<Output = Self> + Div<Output = Self>
{
    fn constant(x: f64) -> Self;
    fn value(&self) -> f64;
}
impl Scalar for Dual {
    fn constant(x: f64) -> Self {
        Dual::constant(x)
    }
    fn value(&self) -> f64 {
        self.val
    }
}
impl Scalar for HyperDual {
    fn constant(x: f64) -> Self {
        HyperDual::constant(x)
    }
    fn value(&self) -> f64 {
        self.val
    }
}

/// Knot-span index for `u`, matching `curve::basis`: the right domain end
/// snaps back to the last nonzero-length span.
fn find_span(degree: usize, knots: &[f64], n: usize, u: f64, domain: [f64; 2]) -> Result<usize> {
    check(
        u.is_finite() && u >= domain[0] && u <= domain[1],
        "Parameter is outside the active knot domain",
    )?;
    let mut span = degree;
    if u == domain[1] {
        span = n - 1;
        while span > 0 && knots[span] == u {
            span -= 1;
        }
    } else {
        while span + 1 < knots.len() && knots[span + 1] <= u {
            span += 1;
        }
    }
    Ok(span)
}

/// Cox–de Boor basis functions on the span of `u`, lifted to any scalar
/// algebra (Piegl & Tiller A2.2). Knot differences are plain constants, so
/// every division below is by a derivative-free denominator.
fn basis_funs<S: Scalar>(degree: usize, knots: &[f64], span: usize, u: &S) -> Vec<S> {
    let mut n = vec![S::constant(0.); degree + 1];
    let mut left = vec![S::constant(0.); degree + 1];
    let mut right = vec![S::constant(0.); degree + 1];
    n[0] = S::constant(1.);
    for j in 1..=degree {
        left[j] = u.clone() - S::constant(knots[span + 1 - j]);
        right[j] = S::constant(knots[span + j]) - u.clone();
        let mut saved = S::constant(0.);
        for r in 0..j {
            let temp = n[r].clone() / (right[r + 1].clone() + left[j - r].clone());
            n[r] = saved + right[r + 1].clone() * temp.clone();
            saved = left[j - r].clone() * temp;
        }
        n[j] = saved;
    }
    n
}

/// Horner evaluation of `Σ coefficients[i]·x^i` in any scalar algebra.
pub(crate) fn horner<S: Scalar>(coefficients: &[f64], x: &S) -> S {
    let mut acc = S::constant(*coefficients.last().unwrap_or(&0.));
    for c in coefficients.iter().rev().skip(1) {
        acc = acc * x.clone() + S::constant(*c);
    }
    acc
}

/// Homogeneous rational evaluation of `curve` at the lifted parameter `u`:
/// accumulate `Σ Nᵢwᵢ(Pᵢ, 1)` and divide by the weight. The returned `axis`
/// count matches the control-point dimension.
fn evaluate_curve_scalar<S: Scalar>(curve: &Curve, u: &S) -> Result<Vec<S>> {
    let n = curve.control_points.len();
    let domain = crate::curve::validate_basis(curve.degree, &curve.knots, n)?;
    let span = find_span(curve.degree, &curve.knots, n, u.value(), domain)?;
    let basis = basis_funs(curve.degree, &curve.knots, span, u);
    let dimension = curve.control_points[0].len();
    let mut weight = S::constant(0.);
    let mut homogeneous = vec![S::constant(0.); dimension];
    for (j, b) in basis.iter().enumerate() {
        let i = span - curve.degree + j;
        let w = b.clone() * S::constant(curve.weights[i]);
        for (axis, h) in homogeneous.iter_mut().enumerate() {
            *h = h.clone() + w.clone() * S::constant(curve.control_points[i][axis]);
        }
        weight = weight + w;
    }
    let point: Vec<S> = homogeneous
        .into_iter()
        .map(|h| h / weight.clone())
        .collect();
    numeric(
        point.iter().all(|p| p.value().is_finite()),
        "Rational dual evaluation exhausted finite precision",
    )?;
    Ok(point)
}

/// Point and first derivative of `curve` at `u.val`, as one lifted
/// evaluation: `result[axis].der` is the exact `dC/du` component at `u.val`
/// whenever `u.der == 1`.
pub fn evaluate_curve_dual(curve: &Curve, u: Dual) -> Result<Vec<Dual>> {
    curve.validate()?;
    evaluate_curve_scalar(curve, &u)
}

/// 3D convenience wrapper around [`evaluate_curve_dual`].
pub fn evaluate_curve_dual3(curve: &Curve, u: Dual) -> Result<Dual3> {
    let point = evaluate_curve_dual(curve, u)?;
    check(point.len() == 3, "Curve must have three coordinates")?;
    Ok([point[0], point[1], point[2]])
}

/// Homogeneous tensor-product evaluation of `surface` at lifted parameters
/// `(u, v)`; the derivative slots of the result follow the seeds of `u`,`v`.
fn evaluate_surface_scalar<S: Scalar>(
    surface: &Surface,
    u: &S,
    v: &S,
) -> Result<[S; 3]> {
    let nu = surface.control_points.len();
    let nv = surface.control_points[0].len();
    let domain_u = crate::curve::validate_basis(surface.degree_u, &surface.knots_u, nu)?;
    let domain_v = crate::curve::validate_basis(surface.degree_v, &surface.knots_v, nv)?;
    let span_u = find_span(surface.degree_u, &surface.knots_u, nu, u.value(), domain_u)?;
    let span_v = find_span(surface.degree_v, &surface.knots_v, nv, v.value(), domain_v)?;
    let basis_u = basis_funs(surface.degree_u, &surface.knots_u, span_u, u);
    let basis_v = basis_funs(surface.degree_v, &surface.knots_v, span_v, v);
    let mut weight = S::constant(0.);
    let mut homogeneous = [S::constant(0.), S::constant(0.), S::constant(0.)];
    for (ju, bu) in basis_u.iter().enumerate() {
        let iu = span_u - surface.degree_u + ju;
        for (jv, bv) in basis_v.iter().enumerate() {
            let iv = span_v - surface.degree_v + jv;
            let w = bu.clone() * bv.clone() * S::constant(surface.weights[iu][iv]);
            for (axis, h) in homogeneous.iter_mut().enumerate() {
                *h = h.clone() + w.clone() * S::constant(surface.control_points[iu][iv][axis]);
            }
            weight = weight + w;
        }
    }
    let point: Vec<S> = homogeneous
        .into_iter()
        .map(|h| h / weight.clone())
        .collect();
    numeric(
        point.iter().all(|p| p.value().is_finite()),
        "Rational hyper-dual evaluation exhausted finite precision",
    )?;
    Ok([point[0].clone(), point[1].clone(), point[2].clone()])
}

/// Point of `surface` at `(u.val, v.val)` with the derivative slots implied
/// by the seeds of `u` and `v`: seeding ε₁ on `u` and ε₂ on `v` yields
/// `Sᵤ` in `d1`, `Sᵥ` in `d2` and `Sᵤᵥ` in `d12`; seeding both ε₁ and ε₂ on
/// one parameter yields the unmixed second derivative in `d12`.
pub fn evaluate_surface_hyper(
    surface: &Surface,
    u: HyperDual,
    v: HyperDual,
) -> Result<[HyperDual; 3]> {
    surface.validate()?;
    evaluate_surface_scalar(surface, &u, &v)
}

/// First and second fundamental forms and curvatures of a surface, all
/// recovered from hyper-dual partial derivatives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceCurvature {
    pub point: [f64; 3],
    pub du: [f64; 3],
    pub dv: [f64; 3],
    pub duu: [f64; 3],
    pub duv: [f64; 3],
    pub dvv: [f64; 3],
    /// First fundamental form coefficients (E, F, G).
    pub first_form: [f64; 3],
    /// Second fundamental form coefficients (L, M, N).
    pub second_form: [f64; 3],
    pub gaussian: f64,
    pub mean: f64,
}

fn sub3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Gaussian and mean curvature at `(u, v)` from three hyper-dual surface
/// evaluations (ε₁/ε₂ seeding gives `Sᵤ,Sᵥ,Sᵤᵥ`; the both-ε seeding on one
/// parameter gives `Sᵤᵤ` and `Sᵥᵥ` from the mixed slot). Errors on a
/// degenerate normal, matching the analytic evaluator's singular status.
pub fn surface_curvature_dual(surface: &Surface, u: f64, v: f64) -> Result<SurfaceCurvature> {
    surface.validate()?;
    let mixed = evaluate_surface_scalar(
        surface,
        &HyperDual::variable_e1(u),
        &HyperDual::variable_e2(v),
    )?;
    let uu = evaluate_surface_scalar(
        surface,
        &HyperDual::variable_second(u),
        &HyperDual::constant(v),
    )?;
    let vv = evaluate_surface_scalar(
        surface,
        &HyperDual::constant(u),
        &HyperDual::variable_second(v),
    )?;
    let point = [mixed[0].val, mixed[1].val, mixed[2].val];
    let du = [mixed[0].d1, mixed[1].d1, mixed[2].d1];
    let dv = [mixed[0].d2, mixed[1].d2, mixed[2].d2];
    let duv = [mixed[0].d12, mixed[1].d12, mixed[2].d12];
    let duu = [uu[0].d12, uu[1].d12, uu[2].d12];
    let dvv = [vv[0].d12, vv[1].d12, vv[2].d12];
    let normal = math_core::cross(du, dv);
    let length = math_core::norm(normal);
    numeric(length > 0. && length.is_finite(), "Surface normal degenerated")?;
    if !(length > 0.) {
        return Err(numeric_err("Surface normal degenerated"));
    }
    let unit = [normal[0] / length, normal[1] / length, normal[2] / length];
    let e = math_core::dot(du, du);
    let f = math_core::dot(du, dv);
    let g = math_core::dot(dv, dv);
    let l = math_core::dot(duu, unit);
    let m = math_core::dot(duv, unit);
    let nn = math_core::dot(dvv, unit);
    let det = e * g - f * f;
    numeric(det > 0. && det.is_finite(), "First fundamental form degenerated")?;
    let _ = sub3;
    Ok(SurfaceCurvature {
        point,
        du,
        dv,
        duu,
        duv,
        dvv,
        first_form: [e, f, g],
        second_form: [l, m, nn],
        gaussian: (l * nn - m * m) / det,
        mean: (e * nn - 2. * f * m + g * l) / (2. * det),
    })
}

#[cfg(test)]
#[path = "tests/dual.rs"]
mod tests;
