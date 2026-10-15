//! Canal surface of a moving sphere. Pointwise envelope equations and sampled
//! NURBS approximation; no union boundary, global regularity or solid claim.
use crate::{Result, check, curve::Curve, surface::Surface};
type V = [f64; 3];
fn norm(v: V) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}
fn dot(a: V, b: V) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn unit(v: V) -> Result<V> {
    let scale = v.iter().map(|x| x.abs()).fold(0_f64, f64::max);
    check(
        v.iter().all(|x| x.is_finite()) && scale > 0.,
        "Envelope frame or tangent is singular",
    )?;
    let scaled = v.map(|x| x / scale);
    let length = norm(scaled);
    Ok(scaled.map(|x| x / length))
}

fn validate(path: &Curve, radius: &Curve, normal: V) -> Result<()> {
    path.validate()?;
    radius.validate()?;
    check(
        path.control_points[0].len() == 3 && radius.control_points[0].len() == 3,
        "Envelope needs 3D path and scalar radius curve",
    )?;
    check(
        path.weights.iter().chain(&radius.weights).all(|w| *w > 0.)
            && radius
                .control_points
                .iter()
                .all(|p| p[0] > 0. && p[1] == 0. && p[2] == 0.),
        "Envelope radius law needs positive controls and weights, encoded [r,0,0]",
    )?;
    check(
        normal.iter().all(|x| x.is_finite()),
        "Envelope reference normal must be finite",
    )?;
    unit(normal)?;
    Ok(())
}
#[derive(Clone, Debug)]
pub struct Circle {
    pub sphere_center: V,
    pub sphere_radius: f64,
    pub tangent: V,
    /// dr/ds, with s the centerline arc length.
    pub radius_slope: f64,
    pub center: V,
    pub radius: f64,
    pub normal: V,
    pub binormal: V,
}
impl Circle {
    /// Angle in radians. Evaluates the analytic envelope circle in binary64.
    pub fn point(&self, angle: f64) -> Result<V> {
        check(angle.is_finite(), "Envelope angle must be finite")?;
        let (s, c) = angle.sin_cos();
        let p: V = std::array::from_fn(|i| {
            self.center[i] + self.radius * (c * self.normal[i] + s * self.binormal[i])
        });
        check(p.iter().all(|x| x.is_finite()), "Envelope point overflow")?;
        Ok(p)
    }
    fn controls(&self) -> Vec<Vec<f64>> {
        [
            [1., 0.],
            [1., 1.],
            [0., 1.],
            [-1., 1.],
            [-1., 0.],
            [-1., -1.],
            [0., -1.],
            [1., -1.],
            [1., 0.],
        ]
        .iter()
        .map(|q| {
            (0..3)
                .map(|i| {
                    self.center[i] + self.radius * (q[0] * self.normal[i] + q[1] * self.binormal[i])
                })
                .collect()
        })
        .collect()
    }
}
// A repeated knot may hide a numerically smooth authored join. Compare both
// one-sided jets rather than silently choosing one side of an actual corner.
fn derivative(curve: &Curve, t: f64, available: Option<Vec<f64>>) -> Result<Vec<f64>> {
    if let Some(d) = available {
        return Ok(d);
    }
    let [a, b] = curve.domain();
    check(
        a < t && t < b,
        "Envelope endpoint derivative is unavailable",
    )?;
    let left = curve.trim(a, t)?.evaluate(t)?.d1;
    let right = curve.trim(t, b)?.evaluate(t)?.d1;
    let (Some(left), Some(right)) = (left, right) else {
        return Err(crate::Error::new(
            crate::INVALID_INPUT,
            "Envelope one-sided derivative is unavailable",
        ));
    };
    let l = [left[0], left[1], left[2]];
    let r = [right[0], right[1], right[2]];
    let scale = norm(l).max(norm(r));
    check(
        scale.is_finite()
            && norm(std::array::from_fn(|k| l[k] - r[k])) <= 64. * f64::EPSILON * scale,
        "Envelope requires matching one-sided derivatives at repeated knots",
    )?;
    Ok(left)
}
fn evaluate(path: &Curve, radius: &Curve, normal: V, u: f64) -> Result<Circle> {
    check(
        u.is_finite() && (0. ..=1.).contains(&u),
        "Envelope parameter must be in [0,1]",
    )?;
    let pd = path.domain();
    let rd = radius.domain();
    let parameter = |d: [f64; 2]| {
        if u == 0. {
            d[0]
        } else if u == 1. {
            d[1]
        } else {
            (1. - u) * d[0] + u * d[1]
        }
    };
    let p = path.evaluate(parameter(pd))?;
    let r = radius.evaluate(parameter(rd))?;
    let d = derivative(path, parameter(pd), p.d1)?;
    let dr = derivative(radius, parameter(rd), r.d1)?;
    let velocity = [d[0], d[1], d[2]];
    let tangent = unit(velocity)?;
    let velocity_scale = velocity.iter().map(|x| x.abs()).fold(0_f64, f64::max);
    let scaled_speed = norm(velocity.map(|x| x / velocity_scale));
    let slope = ((dr[0] / velocity_scale) / scaled_speed) * ((rd[1] - rd[0]) / (pd[1] - pd[0]));
    check(
        slope.is_finite() && slope.abs() < 1. && r.point[0] > 0.,
        "Sphere envelope requires positive radius and |dr/ds| < 1",
    )?;
    let n = unit(normal)?;
    let projection = dot(n, tangent);
    let n = unit(std::array::from_fn(|i| n[i] - projection * tangent[i]))?;
    let b = cross(tangent, n);
    let sphere_radius = r.point[0];
    let sphere_center = [p.point[0], p.point[1], p.point[2]];
    let center: V = std::array::from_fn(|i| sphere_center[i] - sphere_radius * slope * tangent[i]);
    let circle_radius = sphere_radius * ((1. - slope) * (1. + slope)).sqrt();
    check(
        center.iter().all(|x| x.is_finite()) && circle_radius.is_finite() && circle_radius > 0.,
        "Envelope circle overflows or collapses",
    )?;
    Ok(Circle {
        sphere_center,
        sphere_radius,
        tangent,
        radius_slope: slope,
        center,
        radius: circle_radius,
        normal: n,
        binormal: b,
    })
}
/// Characteristic circle satisfying |X-C|²=r² and (X-C)·T=-r dr/ds.
/// Path and radius domains normalize independently; reference normal chooses
/// an angular chart and must not be parallel to the local centerline tangent.
pub fn characteristic_circle(path: &Curve, radius: &Curve, normal: V, u: f64) -> Result<Circle> {
    validate(path, radius, normal)?;
    evaluate(path, radius, normal, u)
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub sampled_tolerance: f64,
    pub max_patches: usize,
}
#[derive(Clone, Debug)]
pub struct Patch {
    pub surface: Surface,
    pub path_interval: [f64; 2],
    pub sampled_control_deviation: f64,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub patches: Vec<Patch>,
    pub within_sampled_tolerance: bool,
    pub sampled_control_deviation: f64,
    pub continuous_bound: bool,
    pub rounding_certified: bool,
}
fn validate_limits(limits: Limits) -> Result<()> {
    check(
        limits.sampled_tolerance.is_finite()
            && limits.sampled_tolerance > 0.
            && (1..=256).contains(&limits.max_patches),
        "Envelope needs positive sampled tolerance and 1..256 patches",
    )?;
    Ok(())
}
/// Adaptive linear interpolation of exact rational characteristic circles.
/// Quarter/midpoint comparisons bound the entire angular circle ONLY at the
/// checked path stations; they do not prove continuous pathwise accuracy.
/// Budget exhaustion returns all intervals with acceptance=false, never a hole.
pub fn approximate(path: &Curve, radius: &Curve, normal: V, limits: Limits) -> Result<Report> {
    validate(path, radius, normal)?;
    validate_limits(limits)?;
    let controls = |u| -> Result<Vec<Vec<f64>>> {
        let points = evaluate(path, radius, normal, u)?.controls();
        check(
            points[0] != points[4] && points[2] != points[6],
            "Envelope circle collapses at coordinate precision",
        )?;
        Ok(points)
    };
    // Every source knot is a mandatory boundary: a corner must not hide
    // between the three diagnostic stations of a large initial interval.
    let mut breaks = vec![0., 1.];
    for curve in [path, radius] {
        let [a, b] = curve.domain();
        for &k in &curve.knots {
            if a < k && k < b {
                let u = (k - a) / (b - a);
                check(
                    u.is_finite() && 0. < u && u < 1.,
                    "Envelope knot normalization collapses",
                )?;
                breaks.push(u);
            }
        }
    }
    breaks.sort_by(f64::total_cmp);
    breaks.dedup();
    if breaks.len() - 1 > limits.max_patches {
        return Err(crate::Error::new(
            crate::RESOURCE_LIMIT,
            "Envelope source spans exceed patch budget",
        ));
    }
    let mut rings = Vec::with_capacity(breaks.len());
    for &u in &breaks {
        rings.push(controls(u)?);
    }
    let mut pending = Vec::with_capacity(limits.max_patches);
    for i in (0..breaks.len() - 1).rev() {
        pending.push((
            breaks[i],
            breaks[i + 1],
            rings[i].clone(),
            rings[i + 1].clone(),
        ));
    }
    let mut patches = Vec::new();
    let mut maximum = 0_f64;
    while let Some((a, b, left, right)) = pending.pop() {
        let mid = a + (b - a) * 0.5;
        check(
            a < mid && mid < b,
            "Envelope path subdivision reached numeric resolution",
        )?;
        let mut error = 0_f64;
        let middle = controls(mid)?;
        for (q, sample) in [
            (0.25, controls(a + (b - a) * 0.25)?),
            (0.5, middle.clone()),
            (0.75, controls(a + (b - a) * 0.75)?),
        ] {
            for i in 0..9 {
                error = error.max(norm(std::array::from_fn(|k| {
                    sample[i][k] - ((1. - q) * left[i][k] + q * right[i][k])
                })));
            }
        }
        check(error.is_finite(), "Envelope deviation overflow")?;
        if error > limits.sampled_tolerance
            && patches.len() + pending.len() + 2 <= limits.max_patches
        {
            pending.push((mid, b, middle.clone(), right));
            pending.push((a, mid, left, middle));
            continue;
        }
        let weights = vec![
            1.,
            std::f64::consts::FRAC_1_SQRT_2,
            1.,
            std::f64::consts::FRAC_1_SQRT_2,
            1.,
            std::f64::consts::FRAC_1_SQRT_2,
            1.,
            std::f64::consts::FRAC_1_SQRT_2,
            1.,
        ];
        let surface = Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![a, a, b, b],
            knots_v: vec![0., 0., 0., 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1., 1., 1.],
            control_points: vec![left, right],
            weights: vec![weights; 2],
            periodic_u: false,
            periodic_v: false,
        };
        surface.validate()?;
        maximum = maximum.max(error);
        patches.push(Patch {
            surface,
            path_interval: [a, b],
            sampled_control_deviation: error,
        });
    }
    Ok(Report {
        patches,
        within_sampled_tolerance: maximum <= limits.sampled_tolerance,
        sampled_control_deviation: maximum,
        continuous_bound: false,
        rounding_certified: false,
    })
}

mod domain;
pub use domain::{AuditedEnvelope, DomainReport, approximate_with_domain_audit, inspect_domain};
