//! Geometric G2 proposals via the original support's ambient curvature tensor.
//! All floating tensors/linearizations are proposals. Only the independent
//! whole original pcurve G2 owner may admit the actual final coefficients.
use super::*;
use crate::constrained_surface::JetEquation;
#[derive(Clone, Copy)]
pub struct CurvatureConstraint<'a> {
    pub spatial: &'a Curve,
    pub parameters: &'a Curve,
    pub reverse: bool,
    pub support: SupportTrace<'a>,
}
#[derive(Clone, Copy)]
pub struct CurvatureOptions {
    pub network: Options,
    /// Frobenius norm of the world-coordinate curvature tensor difference.
    pub curvature_tolerance: f64,
}
impl Default for CurvatureOptions {
    fn default() -> Self {
        Self {
            network: Options::default(),
            curvature_tolerance: 1e-5,
        }
    }
}
/// Nonlinear geometric G2 along whole original curves, mixed with optional
/// G0/G1 guides and full original boundary locks. No UV Jacobian is supplied:
/// the support's ambient tensor acts on the candidate's actual tangent vectors.
/// Fixed weights/basis remain authored. Bounded Newton/collocation proposals
/// require fresh complete original-curve G2 before any Qualified is returned.
pub fn solve_with_curvature(
    seed: &Surface,
    guides: &[CurveConstraint<'_>],
    curvatures: &[CurvatureConstraint<'_>],
    constraints: &[Constraint],
    locks: Option<BoundaryLocks<'_>>,
    options: CurvatureOptions,
) -> Result<Qualified> {
    if !options.curvature_tolerance.is_finite() || options.curvature_tolerance <= 0. {
        return Err(error(
            "Whole interior G2 needs a finite positive curvature tolerance",
        ));
    }
    solve_impl(
        seed,
        guides,
        curvatures,
        constraints,
        locks,
        options.network,
        Some(options.curvature_tolerance),
        None,
    )
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| a[k] * b[k]).sum()
}
fn product(b: [[f64; 3]; 3], a: [f64; 3]) -> [f64; 3] {
    b.map(|row| dot(row, a))
}
struct Shape {
    normal: [f64; 3],
    tensor: [[f64; 3]; 3],
}
fn shape(s: &Surface, uv: [f64; 2]) -> Result<Shape> {
    let p = s.evaluate(uv[0], uv[1])?;
    let (u, v) = p
        .first_derivatives()
        .ok_or_else(|| error("Original curvature tangents unresolved"))?;
    let (uu, uv, vv) = p
        .second_derivatives()
        .ok_or_else(|| error("Original second jet unresolved at a G2 proposal site"))?;
    let normal = p
        .unit_normal()
        .ok_or_else(|| error("Singular curvature proposal chart"))?;
    let (e, f, g) = (dot(u, u), dot(u, v), dot(v, v));
    let det = e * g - f * f;
    if !det.is_finite() || det <= 1e-14 * e * g || det <= 0. {
        return Err(error("Curvature proposal metric is unresolved"));
    }
    let a = std::array::from_fn::<_, 3, _>(|k| (g * u[k] - f * v[k]) / det);
    let b = std::array::from_fn::<_, 3, _>(|k| (e * v[k] - f * u[k]) / det);
    let h = [dot(normal, uu), dot(normal, uv), dot(normal, vv)];
    let tensor = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            h[0] * a[i] * a[j] + h[1] * (a[i] * b[j] + b[i] * a[j]) + h[2] * b[i] * b[j]
        })
    });
    if tensor.iter().flatten().any(|x| !x.is_finite()) {
        return Err(error("Curvature proposal tensor overflow"));
    }
    Ok(Shape { normal, tensor })
}
pub(super) fn equations(
    current: &Surface,
    uv: [f64; 2],
    support: &Surface,
    support_uv: [f64; 2],
    out: &mut Vec<Constraint>,
) -> Result<()> {
    let source = shape(support, support_uv)?;
    let (u, v) = current
        .evaluate(uv[0], uv[1])?
        .first_derivatives()
        .ok_or_else(|| error("Candidate G2 linearization tangents unresolved"))?;
    let b = source.tensor;
    let tangents = [u, v];
    for (second, [i, j]) in [[0, 0], [0, 1], [1, 1]].into_iter().enumerate() {
        let mut equation = JetEquation {
            first: [[0.; 3]; 2],
            second: [[0.; 3]; 3],
            value: -dot(tangents[i], product(b, tangents[j])),
        };
        equation.second[second] = source.normal;
        let left = product(b, tangents[j]);
        let right = product(b, tangents[i]);
        for k in 0..3 {
            equation.first[i][k] -= left[k];
            equation.first[j][k] -= right[k];
        }
        out.push(Constraint::Jet { uv, equation });
    }
    Ok(())
}
pub(super) fn needs_update(
    current: &Surface,
    g: &OwnedGuide,
    parameters: &[f64],
    curvature: f64,
    normal: f64,
) -> Result<bool> {
    let support = g
        .tangent
        .as_ref()
        .ok_or_else(|| error("G2 proposal lost its original support"))?;
    for &t in parameters {
        let world = if g.reverse { 1. - t } else { t };
        let st = if support.reverse { 1. - world } else { world };
        let a = g
            .parameters
            .evaluate(sites::parameter(&g.parameters, t))?
            .point;
        let b = support
            .parameters
            .evaluate(sites::parameter(&support.parameters, st))?
            .point;
        let actual = shape(current, [a[0], a[1]])?;
        let original = shape(&support.surface, [b[0], b[1]])?;
        let sign = if dot(actual.normal, original.normal) < 0. {
            -1.
        } else {
            1.
        };
        let n = (0..3)
            .map(|k| (actual.normal[k] * sign - original.normal[k]).powi(2))
            .sum::<f64>()
            .sqrt();
        let tensor = (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| (actual.tensor[i][j] * sign - original.tensor[i][j]).powi(2))
                    .sum::<f64>()
            })
            .sum::<f64>()
            .sqrt();
        if n > normal || tensor > curvature {
            return Ok(true);
        }
    }
    Ok(false)
}
