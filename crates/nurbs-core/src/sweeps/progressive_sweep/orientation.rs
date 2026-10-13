use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    RotationMinimizing,
    /// Preserve the initial world orientation; twist uses the initial tangent.
    Fixed,
    /// Project the authored normal perpendicular to each path tangent.
    FixedNormal,
    /// Principal normal. Zero curvature and undefined second derivatives fail.
    Frenet,
    /// Sign-continuous sampled principal normal; RMF transport at zero curvature.
    CorrectedFrenet,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Spacing {
    Parameter,
    /// Certified inverse arc-length residual, in model length units.
    ArcLength {
        tolerance: f64,
        max_cells: usize,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub normal: V,
    pub orientation: Orientation,
    pub spacing: Spacing,
    pub initial_sections: usize,
    pub max_sections: usize,
    pub max_deviation: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LevelReport {
    pub accepted: bool,
    pub sections: usize,
    pub stations: usize,
    pub sampled_control_deviation: f64,
    pub budget: f64,
    pub closed_path: bool,
    pub length_residual_upper: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct Level {
    /// Preview is always available, even when this level misses the budget.
    pub patches: Vec<Surface>,
    pub report: LevelReport,
}

#[derive(Clone, Debug)]
pub struct Approximation {
    /// Only an accepted final level is promoted to a construction result.
    pub patches: Option<Vec<Surface>>,
    pub levels: Vec<LevelReport>,
}

/// Caller-controlled progression: consume one level for preview, stop or resume.
/// Scale is positive [value,0,0]; twist is signed [radians,0,0]. Each law domain
/// independently maps to normalized traversal (parameter or arc length).
pub struct Sweep<'a> {
    pub(super) profile: &'a Curve,
    pub(super) path: &'a Curve,
    pub(super) scale: &'a Curve,
    pub(super) twist: &'a Curve,
    pub(super) affine_laws: Option<(&'a Curve, &'a Curve)>,
    pub(super) frame_laws: Option<(&'a Curve, &'a Curve)>,
    pub(super) orientation_guide: Option<&'a Curve>,
    pub(super) contact_point: Option<V>,
    pub(super) options: Options,
    pub(super) next_sections: Option<usize>,
}

pub(super) fn unit(a: V) -> Result<V> {
    let largest = a.iter().fold(0_f64, |m, x| m.max(x.abs()));
    check(
        largest.is_finite() && largest > 0.,
        "Sweep direction must be finite and nonzero",
    )?;
    let scaled = a.map(|x| x / largest);
    let length = scaled[0].hypot(scaled[1]).hypot(scaled[2]);
    Ok(scaled.map(|x| x / length))
}
pub(super) fn project(normal: V, tangent: V) -> Result<V> {
    let n = unit(normal)?;
    let transverse = sub(n, tangent.map(|x| x * dot(n, tangent)));
    check(
        norm(transverse) > 1e-12,
        "Sweep normal is parallel or numerically inseparable from tangent",
    )?;
    unit(transverse)
}
fn reflect(a: V, direction: V) -> Result<V> {
    let n = unit(direction)?;
    Ok(sub(a, n.map(|x| 2. * dot(a, n) * x)))
}
pub(super) fn transported_normal(normal: V, previous_tangent: V, tangent: V, chord: V) -> Result<V> {
    let r = reflect(normal, chord)?;
    let t = reflect(previous_tangent, chord)?;
    let difference = sub(tangent, t);
    let r = if norm(difference) <= 1e-14 { r } else { reflect(r, difference)? };
    project(r, tangent)
}
pub(super) fn rotate(a: V, axis: V, angle: f64) -> V {
    let (s, c) = angle.sin_cos();
    let perpendicular = cross(axis, a);
    let parallel = dot(axis, a) * (1. - c);
    std::array::from_fn(|k| c * a[k] + s * perpendicular[k] + parallel * axis[k])
}
pub(super) fn law(curve: &Curve, fraction: f64) -> Result<f64> {
    let [a, b] = curve.domain();
    Ok(curve
        .evaluate(if fraction == 1. {
            b
        } else {
            a + (b - a) * fraction
        })?
        .point[0])
}
/// A retained constant 3-vector law without a degenerate geometric line.
pub fn constant_vector_law(value: V) -> Result<Curve> {
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![value.to_vec(); 2],
        weights: vec![1., 1.],
        periodic: false,
    };
    curve.validate()?;
    Ok(curve)
}

pub(super) fn vector_law(curve: &Curve, fraction: f64) -> Result<V> {
    let [a, b] = curve.domain();
    // At clamped endpoints the real rational point is the authored control,
    // independent of its weight; avoid a spurious rounding-induced seam gap.
    if fraction == 0. && curve.knots[..=curve.degree].iter().all(|k| *k == a) {
        let p = &curve.control_points[0];
        return Ok([p[0], p[1], p[2]]);
    }
    if fraction == 1.
        && curve.knots[curve.control_points.len()..]
            .iter()
            .all(|k| *k == b)
    {
        let p = curve.control_points.last().unwrap();
        return Ok([p[0], p[1], p[2]]);
    }
    let p = curve
        .evaluate(if fraction == 1. {
            b
        } else {
            a + (b - a) * fraction
        })?
        .point;
    Ok([p[0], p[1], p[2]])
}

pub(super) fn validate_law(curve: &Curve, positive: bool) -> Result<()> {
    curve.validate()?;
    check(
        curve
            .control_points
            .iter()
            .all(|p| p.len() == 3 && p[1] == 0. && p[2] == 0. && (!positive || p[0] > 0.)),
        "Sweep laws require [value,0,0] controls; scale must be positive",
    )
}
pub(super) fn sample(path: &Curve, t: f64) -> Result<(V, V)> {
    let e = path.evaluate(t)?;
    let derivative = match e.d1 {
        Some(d) => d,
        None => {
            let [a, b] = path.domain();
            let side = if t > a {
                path.trim(a, t)?
            } else {
                path.trim(t, b)?
            };
            side.evaluate(t)?
                .d1
                .ok_or_else(|| crate::input("Sweep path has no unique regular tangent"))?
        }
    };
    Ok((
        [e.point[0], e.point[1], e.point[2]],
        unit([derivative[0], derivative[1], derivative[2]])?,
    ))
}
