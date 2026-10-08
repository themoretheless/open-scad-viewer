//! Geodesic tracing on parametrized surfaces.
//!
//! Integrates the geodesic ODE (vanishing geodesic curvature) in parameter
//! space with RK4, using the Christoffel symbols of the first fundamental
//! form computed from surface jets (S_u·S_uu etc.). Steps are arc-length
//! parametrized: the initial direction is normalized in the metric
//! ds² = E du² + 2F du dv + G dv², and the ODE preserves unit metric speed.
//!
//! Honesty limits: the residual geodesic curvature reported is recomputed at
//! sample points from local jets and the ODE acceleration; it measures
//! local ODE/evaluation consistency, not accumulated global drift. Near a
//! degenerate metric (EG−F² → 0, e.g. sphere poles) integration stops with
//! `GeodesicStop::DegenerateMetric` instead of crossing the singularity.
use crate::{Result, check, numeric, surface::Surface};
use math_core::{dot, next_up, norm};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeodesicStop {
    LengthReached,
    BoundaryHit,
    BudgetExhausted,
    DegenerateMetric,
}

#[derive(Clone, Debug)]
pub struct GeodesicReport {
    pub uv_polyline: Vec<[f64; 2]>,
    pub length_achieved: f64,
    pub stop: GeodesicStop,
    /// Max |kg| at up to 8 samples, recomputed from 3D jets as
    /// |a − (a·n)n| where a is the second arc-length derivative. Approximate:
    /// it certifies local ODE consistency, not endpoint accuracy.
    pub max_residual_geodesic_curvature: f64,
}

#[derive(Clone, Debug)]
pub struct GeodesicOffsetReport {
    pub offset_uv: Vec<[f64; 2]>,
    /// Max over samples of ||S(end)−S(sample)| − |distance|| plus any length
    /// shortfall when a shot stopped early. Chord ≤ geodesic length always,
    /// so this is an honest lower-bound discrepancy against the request.
    pub max_deviation: f64,
    pub stops: Vec<GeodesicStop>,
}

fn domains(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}

/// First derivatives at (u,v); when the exact parameter is a C0 query point
/// (knot of full multiplicity), retries with a 1e-7-relative nudge and uses
/// those one-sided jets — a documented approximation confined to knot lines.
fn first_jets(s: &Surface, u: f64, v: f64) -> Result<Option<([f64; 3], [f64; 3])>> {
    let dom = domains(s);
    let u = u.clamp(dom[0][0], dom[0][1]);
    let v = v.clamp(dom[1][0], dom[1][1]);
    let e = s.evaluate_validated(u, v)?;
    if let Some(j) = e.first_derivatives() {
        return Ok(Some(j));
    }
    let dom = domains(s);
    let du = (dom[0][1] - dom[0][0]) * 1e-7;
    let dv = (dom[1][1] - dom[1][0]) * 1e-7;
    for (su, sv) in [
        (du, 0.),
        (-du, 0.),
        (0., dv),
        (0., -dv),
        (du, dv),
        (-du, -dv),
    ] {
        let uu = (u + su).clamp(dom[0][0], dom[0][1]);
        let vv = (v + sv).clamp(dom[1][0], dom[1][1]);
        if let Some(j) = s.evaluate_validated(uu, vv)?.first_derivatives() {
            return Ok(Some(j));
        }
    }
    Ok(None)
}

/// Christoffel symbols of the second kind from the first fundamental form and
/// its parametric derivatives, all derived from the surface jet. Returns None
/// when the metric is degenerate. At C1 knot lines the analytic second
/// derivatives are unavailable; there the metric derivatives fall back to
/// central finite differences of E,F,G (an O(δ²) approximation, documented).
fn christoffel(s: &Surface, u: f64, v: f64) -> Result<Option<[[[f64; 2]; 2]; 2]>> {
    let dom = domains(s);
    let u = u.clamp(dom[0][0], dom[0][1]);
    let v = v.clamp(dom[1][0], dom[1][1]);
    let e = s.evaluate_validated(u, v)?;
    let Some((su, sv)) = first_jets(s, u, v)? else {
        return Ok(None);
    };
    let (fe, ff, fg) = (dot(su, su), dot(su, sv), dot(sv, sv));
    let d = fe * fg - ff * ff;
    // Scale-aware degeneracy: D/(EG) is sin² of the tangent-direction angle.
    if !(d > 1e-14 * fe * fg) || fe <= 0. || fg <= 0. || !d.is_finite() {
        return Ok(None);
    }
    let (eu, ev, fu, fv, gu, gv) = match e.second_derivatives() {
        Some((suu, suv, svv)) => (
            2. * dot(su, suu),
            2. * dot(su, suv),
            dot(suu, sv) + dot(su, suv),
            dot(suv, sv) + dot(su, svv),
            2. * dot(sv, suv),
            2. * dot(sv, svv),
        ),
        None => {
            // Central differences of the metric, clamped inside the domain.
            let dom = domains(s);
            let du = (dom[0][1] - dom[0][0]) * 1e-6;
            let dv = (dom[1][1] - dom[1][0]) * 1e-6;
            let metric_at = |u: f64, v: f64| -> Result<Option<[f64; 3]>> {
                let u = u.clamp(dom[0][0], dom[0][1]);
                let v = v.clamp(dom[1][0], dom[1][1]);
                Ok(first_jets(s, u, v)?.map(|(su, sv)| [dot(su, su), dot(su, sv), dot(sv, sv)]))
            };
            let (mu, mv) = (
                [metric_at(u - du, v)?, metric_at(u + du, v)?],
                [metric_at(u, v - dv)?, metric_at(u, v + dv)?],
            );
            let slope = |m: [Option<[f64; 3]>; 2], k: usize, h: f64| match m {
                [Some(a), Some(b)] => (b[k] - a[k]) / (2. * h),
                _ => f64::NAN,
            };
            (
                slope(mu, 0, du),
                slope(mv, 0, dv),
                slope(mu, 1, du),
                slope(mv, 1, dv),
                slope(mu, 2, du),
                slope(mv, 2, dv),
            )
        }
    };
    let w = 2. * d;
    let out = [
        [
            [(fg * eu - 2. * ff * fu + ff * ev) / w, (fg * ev - ff * gu) / w],
            [(fg * ev - ff * gu) / w, (2. * fg * fv - fg * gu - ff * gv) / w],
        ],
        [
            [(2. * fe * fu - fe * ev - ff * eu) / w, (fe * gu - ff * ev) / w],
            [(fe * gu - ff * ev) / w, (fe * gv - 2. * ff * fv + ff * gu) / w],
        ],
    ];
    numeric(
        out.iter().flatten().flatten().all(|x| x.is_finite()),
        "Geodesic Christoffel symbols exceeded finite numeric bounds.",
    )?;
    Ok(Some(out))
}

/// Metric (E,F,G) at a point; None when degenerate or unavailable.
fn metric(s: &Surface, u: f64, v: f64) -> Result<Option<[f64; 3]>> {
    let Some((su, sv)) = first_jets(s, u, v)? else {
        return Ok(None);
    };
    let (fe, ff, fg) = (dot(su, su), dot(su, sv), dot(sv, sv));
    let d = fe * fg - ff * ff;
    if !(d > 1e-14 * fe * fg) || fe <= 0. || fg <= 0. || !d.is_finite() {
        return Ok(None);
    }
    Ok(Some([fe, ff, fg]))
}

/// Arc-length derivative of the state y=(u,v,p,q). None on degenerate metric.
fn rhs(s: &Surface, y: [f64; 4]) -> Result<Option<[f64; 4]>> {
    let Some(g) = christoffel(s, y[0], y[1])? else {
        return Ok(None);
    };
    let (p, q) = (y[2], y[3]);
    let acc = |k: usize| -(g[k][0][0] * p * p + 2. * g[k][0][1] * p * q + g[k][1][1] * q * q);
    Ok(Some([p, q, acc(0), acc(1)]))
}

fn rk4(s: &Surface, y: [f64; 4], h: f64) -> Result<Option<[f64; 4]>> {
    let stage = |k: [f64; 4], c: f64| std::array::from_fn(|i| y[i] + c * k[i]);
    let Some(k1) = rhs(s, y)? else { return Ok(None) };
    let Some(k2) = rhs(s, stage(k1, h / 2.))? else { return Ok(None) };
    let Some(k3) = rhs(s, stage(k2, h / 2.))? else { return Ok(None) };
    let Some(k4) = rhs(s, stage(k3, h))? else { return Ok(None) };
    Ok(Some(std::array::from_fn(|i| {
        y[i] + h * (k1[i] + 2. * k2[i] + 2. * k3[i] + k4[i]) / 6.
    })))
}

fn inside(d: &[[f64; 2]; 2], y: [f64; 4], slack: f64) -> bool {
    let wu = (d[0][1] - d[0][0]) * slack;
    let wv = (d[1][1] - d[1][0]) * slack;
    y[0] >= d[0][0] - wu && y[0] <= d[0][1] + wu && y[1] >= d[1][0] - wv && y[1] <= d[1][1] + wv
}

/// Residual geodesic curvature at a state: |a − (a·n)n| with
/// a = S_uu p² + 2S_uv pq + S_vv q² + S_u p' + S_v q'.
fn residual_kg(s: &Surface, y: [f64; 4]) -> Result<f64> {
    let e = s.evaluate_validated(y[0], y[1])?;
    let (Some((su, sv)), Some((suu, suv, svv)), Some(n)) = (
        e.first_derivatives(),
        e.second_derivatives(),
        e.unit_normal(),
    ) else {
        return Ok(0.);
    };
    let Some(g) = christoffel(s, y[0], y[1])? else {
        return Ok(0.);
    };
    let (p, q) = (y[2], y[3]);
    let acc = |k: usize| -(g[k][0][0] * p * p + 2. * g[k][0][1] * p * q + g[k][1][1] * q * q);
    let a: [f64; 3] = std::array::from_fn(|i| {
        suu[i] * p * p + 2. * suv[i] * p * q + svv[i] * q * q + su[i] * acc(0) + sv[i] * acc(1)
    });
    let kn = dot(a, n);
    let kg: [f64; 3] = std::array::from_fn(|i| a[i] - kn * n[i]);
    Ok(norm(kg))
}

fn finish(
    surface: &Surface,
    states: Vec<[f64; 4]>,
    length_achieved: f64,
    stop: GeodesicStop,
) -> Result<GeodesicReport> {
    let mut max_kg = 0_f64;
    let n = states.len();
    let checks = 8.min(n);
    for c in 0..checks {
        let i = c * (n - 1) / checks.max(1);
        max_kg = max_kg.max(residual_kg(surface, states[i.min(n - 1)])?);
    }
    Ok(GeodesicReport {
        uv_polyline: states.iter().map(|y| [y[0], y[1]]).collect(),
        length_achieved,
        stop,
        max_residual_geodesic_curvature: next_up(max_kg),
    })
}

/// RK4 integration of the geodesic ODE (kg = 0) in parameter space using the
/// Christoffel symbols of the first fundamental form (from surface jets).
/// Step size adapts to local curvature via full-vs-two-half RK4 comparison;
/// the budget max_steps ≤ 4096 bounds accepted steps; stops at the domain
/// boundary or when the metric degenerates. Never panics at poles.
pub fn trace_geodesic_report(
    surface: &Surface,
    start_uv: [f64; 2],
    direction_uv: [f64; 2],
    length: f64,
    max_steps: usize,
) -> Result<GeodesicReport> {
    surface.validate()?;
    check(
        start_uv
            .iter()
            .chain(direction_uv.iter())
            .chain([length].iter())
            .all(|x| x.is_finite()),
        "Geodesic start, direction and length must be finite.",
    )?;
    check(length > 0., "Geodesic length must be positive.")?;
    check(
        (1..=4096).contains(&max_steps),
        "Geodesic step budget must be 1..4096.",
    )?;
    let d = domains(surface);
    check(
        inside(&d, [start_uv[0], start_uv[1], 0., 0.], 0.),
        "Geodesic start must lie inside the surface domain.",
    )?;
    let Some(m) = metric(surface, start_uv[0], start_uv[1])? else {
        return finish(surface, vec![[start_uv[0], start_uv[1], 0., 0.]], 0., GeodesicStop::DegenerateMetric);
    };
    let speed2 = m[0] * direction_uv[0] * direction_uv[0]
        + 2. * m[1] * direction_uv[0] * direction_uv[1]
        + m[2] * direction_uv[1] * direction_uv[1];
    check(
        speed2 > 0.,
        "Geodesic direction must be nonzero in the surface metric.",
    )?;
    let scale = speed2.sqrt();
    let mut y = [
        start_uv[0],
        start_uv[1],
        direction_uv[0] / scale,
        direction_uv[1] / scale,
    ];
    let mut states = vec![y];
    let tolerance = 1e-9;
    let h_min = length * 1e-10;
    let mut h = length / 64.;
    let mut s = 0.;
    let mut steps = 0;
    loop {
        if s >= length * (1. - 1e-12) {
            return finish(surface, states, s, GeodesicStop::LengthReached);
        }
        if steps == max_steps {
            return finish(surface, states, s, GeodesicStop::BudgetExhausted);
        }
        let mut step = h.min(length - s);
        // Error control: one full RK4 step vs two half steps, halving until
        // the defect is within tolerance. Degenerate evaluation inside a step
        // below h_min reports DegenerateMetric.
        let accepted = loop {
            match (rk4(surface, y, step)?, rk4(surface, y, step / 2.)?) {
                (Some(full), Some(mid)) => {
                    let Some(half) = rk4(surface, mid, step / 2.)? else {
                        if step / 2. <= h_min {
                            return finish(surface, states, s, GeodesicStop::DegenerateMetric);
                        }
                        step /= 2.;
                        continue;
                    };
                    let err = (0..4)
                        .map(|i| (full[i] - half[i]).abs())
                        .fold(0., f64::max);
                    if err <= tolerance || step / 2. <= h_min {
                        break half;
                    }
                    step /= 2.;
                }
                _ => {
                    if step / 2. <= h_min {
                        return finish(surface, states, s, GeodesicStop::DegenerateMetric);
                    }
                    step /= 2.;
                }
            }
        };
        if !inside(&d, accepted, 0.) {
            // Bisect the segment y → accepted for the last inside point, then
            // clamp onto the boundary and stop.
            let (mut a, mut b) = (y, accepted);
            for _ in 0..64 {
                let mid = std::array::from_fn(|i| (a[i] + b[i]) / 2.);
                if inside(&d, mid, 0.) {
                    a = mid;
                } else {
                    b = mid;
                }
            }
            a[0] = a[0].clamp(d[0][0], d[0][1]);
            a[1] = a[1].clamp(d[1][0], d[1][1]);
            states.push(a);
            return finish(surface, states, s + step, GeodesicStop::BoundaryHit);
        }
        // Grow the step when the defect was far below tolerance.
        let err_free = step;
        y = accepted;
        s += step;
        steps += 1;
        states.push(y);
        h = (err_free * 2.).min(length / 8.);
    }
}

/// Offsets a curve-on-surface (given as uv polyline) by geodesic distance:
/// at each of `samples` resampled points, shoots a metric-perpendicular
/// geodesic of `distance` and connects the endpoints. The report carries an
/// honest max deviation versus the requested distance.
pub fn geodesic_offset_report(
    surface: &Surface,
    curve_uv: &[[f64; 2]],
    distance: f64,
    samples: usize,
) -> Result<GeodesicOffsetReport> {
    surface.validate()?;
    check(
        curve_uv.len() >= 2
            && curve_uv.iter().flatten().all(|x| x.is_finite())
            && distance.is_finite(),
        "Geodesic offset needs a finite uv polyline of at least two points.",
    )?;
    check(samples >= 2, "Geodesic offset needs at least two samples.")?;
    check(distance != 0., "Geodesic offset distance must be nonzero.")?;
    let d = domains(surface);
    for p in curve_uv {
        check(
            p[0] >= d[0][0] && p[0] <= d[0][1] && p[1] >= d[1][0] && p[1] <= d[1][1],
            "Geodesic offset curve must lie inside the surface domain.",
        )?;
    }
    // Resample the polyline uniformly by uv chord length.
    let mut cumulative = vec![0.];
    for w in curve_uv.windows(2) {
        cumulative.push(
            cumulative.last().unwrap() + ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt(),
        );
    }
    let total = *cumulative.last().unwrap();
    check(total > 0., "Geodesic offset curve must have nonzero length.")?;
    let sample_at = |t: f64| -> [f64; 2] {
        let i = cumulative.partition_point(|&c| c < t).saturating_sub(1).min(curve_uv.len() - 2);
        let span = cumulative[i + 1] - cumulative[i];
        let r = if span > 0. { (t - cumulative[i]) / span } else { 0. };
        [
            curve_uv[i][0] + r * (curve_uv[i + 1][0] - curve_uv[i][0]),
            curve_uv[i][1] + r * (curve_uv[i + 1][1] - curve_uv[i][1]),
        ]
    };
    let mut offset_uv = Vec::with_capacity(samples);
    let mut stops = Vec::with_capacity(samples);
    let mut max_deviation = 0_f64;
    let mut previous_normal: Option<[f64; 2]> = None;
    for k in 0..samples {
        let t = total * k as f64 / (samples - 1) as f64;
        let p = sample_at(t);
        // Tangent by central uv difference, then metric-perpendicular:
        // for G·t = (A,B) the unit perpendicular is (−B, A)/√(EG−F²).
        let e = total * 1e-6;
        let a = sample_at((t - e).max(0.));
        let b = sample_at((t + e).min(total));
        let tangent = [b[0] - a[0], b[1] - a[1]];
        let Some(m) = metric(surface, p[0], p[1])? else {
            stops.push(GeodesicStop::DegenerateMetric);
            offset_uv.push(p);
            max_deviation = max_deviation.max(distance.abs());
            continue;
        };
        let ga = m[0] * tangent[0] + m[1] * tangent[1];
        let gb = m[1] * tangent[0] + m[2] * tangent[1];
        let det = (m[0] * m[2] - m[1] * m[1]).sqrt();
        let tangent_norm2 = m[0] * tangent[0] * tangent[0]
            + 2. * m[1] * tangent[0] * tangent[1]
            + m[2] * tangent[1] * tangent[1];
        if !(tangent_norm2 > 0.) || !(det > 0.) {
            stops.push(GeodesicStop::DegenerateMetric);
            offset_uv.push(p);
            max_deviation = max_deviation.max(distance.abs());
            continue;
        }
        let mut normal = [-gb / det, ga / det];
        // Normalize to unit metric length and orient by continuity.
        let nscale = (m[0] * normal[0] * normal[0]
            + 2. * m[1] * normal[0] * normal[1]
            + m[2] * normal[1] * normal[1])
            .sqrt();
        normal = [normal[0] / nscale, normal[1] / nscale];
        if let Some(prev) = previous_normal {
            let dot_m = m[0] * normal[0] * prev[0]
                + m[1] * (normal[0] * prev[1] + normal[1] * prev[0])
                + m[2] * normal[1] * prev[1];
            if dot_m < 0. {
                normal = [-normal[0], -normal[1]];
            }
        }
        previous_normal = Some(normal);
        let direction = [normal[0] * distance.signum(), normal[1] * distance.signum()];
        let report = trace_geodesic_report(surface, p, direction, distance.abs(), 256)?;
        let end = *report.uv_polyline.last().unwrap();
        stops.push(report.stop);
        offset_uv.push(end);
        let chord = norm({
            let pa = surface.evaluate_validated(p[0], p[1])?.point;
            let pb = surface.evaluate_validated(end[0], end[1])?.point;
            [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]]
        });
        let shortfall = (distance.abs() - report.length_achieved).max(0.);
        max_deviation = max_deviation.max((chord - distance.abs()).abs() + shortfall);
    }
    Ok(GeodesicOffsetReport {
        offset_uv,
        max_deviation: next_up(max_deviation),
        stops,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::Curve;
    use crate::surface::revolve;

    fn cylinder() -> Surface {
        // Radius 1, height 1, axis z; u along height, v around (knots 0..4).
        let profile = Curve::from_polyline(vec![vec![1., 0., 0.], vec![1., 0., 1.]]).unwrap();
        revolve(&profile, [0.; 3], [0., 0., 1.], 360.).unwrap()
    }
    fn sphere() -> Surface {
        // Unit sphere; poles at u=0 and u=1 of the profile domain 0..2.
        let profile = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 2., 2., 2.],
            control_points: vec![
                vec![0., 0., -1.],
                vec![1., 0., -1.],
                vec![1., 0., 0.],
                vec![1., 0., 1.],
                vec![0., 0., 1.],
            ],
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        };
        revolve(&profile, [0.; 3], [0., 0., 1.], 360.).unwrap()
    }
    fn plane() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1., 1.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        }
    }

    #[test]
    fn geodesic_on_cylinder_is_analytic_helix() {
        let s = cylinder();
        // The rational circle has non-constant speed in v, so the test is
        // parameterization-independent: a geodesic unrolls to a straight line
        // in (x = R·θ, z) with constant dz/ds = √E/√(E+G) and dx/dz = √(G/E).
        let start = [0.1, 0.5];
        let (su, sv) = s.evaluate(start[0], start[1]).unwrap().first_derivatives().unwrap();
        let e0: f64 = su.iter().map(|x| x * x).sum();
        let g0: f64 = sv.iter().map(|x| x * x).sum();
        let length = 1.2;
        let report = trace_geodesic_report(&s, start, [1., 1.], length, 4096).unwrap();
        assert_eq!(report.stop, GeodesicStop::LengthReached);
        assert!((report.length_achieved - length).abs() < 1e-9);
        let end = *report.uv_polyline.last().unwrap();
        let actual = s.evaluate(end[0], end[1]).unwrap().point;
        // Analytic helix endpoint: z advances at √E/√(E+G), θ at Δz·√(G/E)/R.
        let z_end = 0.1 + length * (e0 / (e0 + g0)).sqrt();
        let theta_end = start[1] * std::f64::consts::PI / 2.
            + (z_end - 0.1) * (g0 / e0).sqrt();
        let analytic = [theta_end.cos(), theta_end.sin(), z_end];
        let deviation: f64 = (0..3).map(|i| (actual[i] - analytic[i]).powi(2)).sum::<f64>().sqrt();
        assert!(deviation < 1e-6, "helix deviation {deviation}");
        assert!(report.max_residual_geodesic_curvature < 1e-6);
    }

    #[test]
    fn geodesic_on_sphere_is_great_circle_antipodal() {
        let s = sphere();
        // Start on the equator (u=1 of profile domain 0..2); tilted direction
        // keeps the great circle clear of the poles. Any geodesic of length
        // π·R from P ends at the antipode of P.
        let start = [1.0, 0.0];
        let p0 = s.evaluate(start[0], start[1]).unwrap().point;
        let report = trace_geodesic_report(
            &s,
            start,
            [1., 1.],
            std::f64::consts::PI,
            4096,
        )
        .unwrap();
        assert_eq!(report.stop, GeodesicStop::LengthReached);
        let end = *report.uv_polyline.last().unwrap();
        let p1 = s.evaluate(end[0], end[1]).unwrap().point;
        let distance: f64 = (0..3).map(|i| (p1[i] + p0[i]).powi(2)).sum::<f64>().sqrt();
        assert!(distance < 1e-6, "antipodal deviation {distance}");
        assert!(report.max_residual_geodesic_curvature < 1e-5);
    }

    #[test]
    fn geodesic_offset_on_plane_equals_planar_offset() {
        let s = plane();
        let curve = [[0.2, 0.5], [0.8, 0.5]];
        let distance = 0.1;
        let report = geodesic_offset_report(&s, &curve, distance, 8).unwrap();
        assert!(report.max_deviation < 1e-8, "deviation {}", report.max_deviation);
        assert!(report.stops.iter().all(|&s| s == GeodesicStop::LengthReached));
        // Perpendicular of the +u direction is ±v; offset polyline is parallel.
        for (k, p) in report.offset_uv.iter().enumerate() {
            let t = k as f64 / 7.;
            let base = 0.2 + 0.6 * t;
            assert!((p[0] - base).abs() < 1e-9, "{p:?}");
            assert!((p[1] - 0.5).abs() - distance < 1e-9, "{p:?}");
        }
    }

    #[test]
    fn sphere_pole_degeneracy_stops_without_panic() {
        let s = sphere();
        // From the equator straight at the north pole.
        let report = trace_geodesic_report(
            &s,
            [1.0, 0.0],
            [1., 0.],
            std::f64::consts::PI,
            4096,
        )
        .unwrap();
        assert!(
            matches!(report.stop, GeodesicStop::DegenerateMetric | GeodesicStop::BoundaryHit),
            "stop {:?}",
            report.stop
        );
        assert!(report.length_achieved < std::f64::consts::PI);
    }

    #[test]
    fn budget_exhaustion_is_reported() {
        let s = cylinder();
        let report = trace_geodesic_report(&s, [0.1, 1.0], [1., 1.], 1.5, 1).unwrap();
        assert_eq!(report.stop, GeodesicStop::BudgetExhausted);
        assert!(report.length_achieved < 1.5);
    }
}
