//! Discrete rotation-minimizing sweep, using Wang et al. (2008), Table I.
//! The loft is piecewise linear along the path; it is not an exact RMF surface.
use crate::{
    Result, check,
    curve::Curve,
    surface::{Surface, loft},
};
type V = [f64; 3];
use math_core::{cross, dot, sub};
fn unit(a: V) -> Result<V> {
    let n = dot(a, a).sqrt();
    check(
        n.is_finite() && n > 1e-12,
        "Sweep frame has a zero or nonfinite direction",
    )?;
    Ok(a.map(|x| x / n))
}
fn reflect(a: V, v: V) -> Result<V> {
    let d = dot(v, v);
    check(
        d.is_finite() && d > 1e-24,
        "Sweep frame reflection is degenerate; refine or split the path",
    )?;
    let s = 2. * dot(a, v) / d;
    Ok(std::array::from_fn(|i| a[i] - s * v[i]))
}
#[inline(never)]
fn one_sided_derivative(path: &Curve, t: f64) -> Result<Vec<f64>> {
    let [a, b] = path.domain();
    let side = if t > a {
        path.trim(a, t)?
    } else {
        path.trim(t, b)?
    };
    side.evaluate(t)?.d1.ok_or_else(|| {
        crate::Error::new(
            crate::INVALID_INPUT,
            "Sweep requires a regular differentiable path",
        )
    })
}
pub(crate) fn sample(path: &Curve, t: f64) -> Result<(V, V)> {
    let e = path.evaluate(t)?;
    let d = match e.d1 {
        Some(d) => d,
        None => one_sided_derivative(path, t)?,
    };
    Ok((
        [e.point[0], e.point[1], e.point[2]],
        unit([d[0], d[1], d[2]])?,
    ))
}
/// Origin is the path start. Initial normal selects a frame, but the first profile
/// stays at its original world position. Subsequent sections are rigidly transported.
/// Closed paths distribute frame holonomy along sampled chord length; the seam is C0.
pub(crate) fn frames(profile: &Curve, path: &Curve, normal: V, sections: usize) -> Result<(Vec<Curve>, bool)> {
    profile.validate()?;
    path.validate()?;
    check(
        (2..=125).contains(&sections),
        "Framed sweep frame count exceeds 125",
    )?;
    check(
        profile.control_points[0].len() == 3 && path.control_points[0].len() == 3,
        "Framed sweep requires 3D curves",
    )?;

    let [a, b] = path.domain();
    let (start, t0) = sample(path, a)?;
    let (end, t1) = sample(path, b)?;
    let closed = dot(sub(end, start), sub(end, start)) <= 1e-24;
    check(
        !path.periodic || closed,
        "Periodic sweep path endpoints do not coincide",
    )?;
    if closed {
        check(
            sections >= 4,
            "Closed sweep requires at least four sections",
        )?;
        check(
            dot(sub(t0, t1), sub(t0, t1)) < 1e-20,
            "Closed sweep seam tangents do not agree; split the path",
        )?;
    }
    // Reject internal knots without a unique tangent, including ones not sampled.
    for &k in &path.knots {
        if k > a && k < b && path.knots.iter().filter(|&&v| v == k).count() >= path.degree {
            let left = path.trim(a, k)?;
            let right = path.trim(k, b)?;
            let l = sample(&left, k)?.1;
            let r = sample(&right, k)?.1;
            check(
                dot(sub(l, r), sub(l, r)) < 1e-20,
                "Split the sweep path at tangent discontinuities",
            )?;
        }
    }
    let n = unit(normal)?;
    let r0 = unit(sub(n, t0.map(|x| x * dot(n, t0))))?;
    let s0 = cross(t0, r0);
    let coordinates: Vec<V> = profile
        .control_points
        .iter()
        .map(|p| {
            let q = sub([p[0], p[1], p[2]], start);
            [dot(q, r0), dot(q, s0), dot(q, t0)]
        })
        .collect();
    let (mut position, mut tangent, mut r) = (start, t0, r0);
    let mut stations = Vec::new();
    let mut lengths = vec![0.];
    for i in 0..sections {
        let (next, t) = sample(path, a + (b - a) * i as f64 / (sections - 1) as f64)?;
        if i > 0 {
            let v = sub(next, position);
            let reflected_r = reflect(r, v)?;
            let reflected_t = reflect(tangent, v)?;
            r = unit(reflect(reflected_r, sub(t, reflected_t))?)?;
            r = unit(sub(r, t.map(|x| x * dot(r, t))))?;
        }
        if i > 0 {
            lengths.push(lengths[i - 1] + dot(sub(next, position), sub(next, position)).sqrt());
        }
        stations.push((next, t, r));
        position = next;
        tangent = t;
    }
    let twist = if closed {
        dot(t0, cross(r, r0)).atan2(dot(r, r0))
    } else {
        0.
    };
    let total = *lengths.last().unwrap();
    let mut curves = Vec::new();
    for (i, (next, t, r)) in stations.into_iter().enumerate() {
        let angle = if closed {
            twist * lengths[i] / total
        } else {
            0.
        };
        let r = std::array::from_fn(|k| r[k] * angle.cos() + cross(t, r)[k] * angle.sin());
        let s = cross(t, r);
        let mut c = profile.clone();
        c.control_points = coordinates
            .iter()
            .map(|q| {
                (0..3)
                    .map(|k| next[k] + q[0] * r[k] + q[1] * s[k] + q[2] * t[k])
                    .collect()
            })
            .collect();
        curves.push(c);
    }
    if closed {
        let first = curves[0].clone();
        *curves.last_mut().unwrap() = first;
    }
    Ok((curves, closed))
}
pub fn sweep(profile: &Curve, path: &Curve, normal: V, sections: usize) -> Result<Surface> {
    check(
        (2..=32).contains(&sections),
        "Framed sweep requires 2..32 sections",
    )?;
    let (curves, closed) = frames(profile, path, normal, sections)?;
    let mut result = loft(&curves)?;
    if closed {
        result.periodic_v = true;
        result.knots_v = (0..sections + 2)
            .map(|i| (i as f64 - 1.) / (sections - 1) as f64)
            .collect();
    } else {
        for k in &mut result.knots_v {
            *k /= (sections - 1) as f64;
        }
    }
    result.validate()?;
    Ok(result)
}
#[derive(Clone, Debug)]
pub struct CheckedSweep {
    pub surface: Option<Surface>,
    pub report: SweepReport,
}

/// Sampled refinement evidence; it does not establish a continuous error bound.
#[derive(Clone, Debug, PartialEq)]
pub struct SweepReport {
    pub accepted: bool,
    pub sampled_control_deviation: f64,
    pub budget: f64,
    pub stations: usize,
    pub sections: usize,
    pub closed_path: bool,
}

#[path = "framed_sweep/serialization.rs"]
#[cfg(feature = "codec")]
mod serialization;

/// A refinement diagnostic, NOT a continuous Hausdorff or exact RMF certificate.
/// All profile controls share the same positive rational weights in every section,
/// so their maximum displacement bounds the whole profile at each checked station.
pub fn checked_sweep(
    profile: &Curve,
    path: &Curve,
    normal: V,
    sections: usize,
    max_deviation: f64,
) -> Result<CheckedSweep> {
    check(
        max_deviation.is_finite() && max_deviation >= 0.,
        "Sweep deviation budget must be finite and nonnegative",
    )?;
    let surface = sweep(profile, path, normal, sections)?;
    let (fine, _) = frames(profile, path, normal, 4 * (sections - 1) + 1)?;
    let mut maximum = 0_f64;
    for (i, section) in fine.iter().enumerate() {
        let interval = (i / 4).min(sections - 2);
        let fraction = (i - 4 * interval) as f64 / 4.;
        for (k, p) in section.control_points.iter().enumerate() {
            let delta: V = std::array::from_fn(|axis| {
                p[axis]
                    - ((1. - fraction) * surface.control_points[k][interval][axis]
                        + fraction * surface.control_points[k][interval + 1][axis])
            });
            let error = dot(delta, delta).sqrt();
            check(error.is_finite(), "Sweep refinement error overflowed")?;
            maximum = maximum.max(error);
        }
    }
    let accepted = maximum <= max_deviation;
    let closed = surface.periodic_v;
    Ok(CheckedSweep {
        surface: if accepted { Some(surface) } else { None },
        report: SweepReport {
            accepted,
            sampled_control_deviation: maximum,
            budget: max_deviation,
            stations: fine.len(),
            sections,
            closed_path: closed,
        },
    })
}
pub(crate) fn checked_scaled_profile(
    profile: &Curve,
    path: &Curve,
    radius: &Curve,
    normal: [f64; 3],
    sections: usize,
    budget: f64,
) -> Result<CheckedSweep> {
    path.validate()?;
    radius.validate()?;
    check(
        path.control_points[0].len() == 3,
        "Scaled sweep path must be 3D",
    )?;
    check(
        (2..=32).contains(&sections),
        "Scaled sweep requires 2..32 sections",
    )?;
    check(
        budget.is_finite() && budget > 0.,
        "Scaled sweep budget must be positive and finite",
    )?;
    check(
        radius
            .control_points
            .iter()
            .all(|p| p.len() == 3 && p[0] > 0. && p[1] == 0. && p[2] == 0.),
        "Section scale law requires positive [value,0,0] controls",
    )?;
    let scaled = |count: usize| -> Result<(Vec<Curve>, bool)> {
        let (mut curves, closed) = crate::framed_sweep::frames(profile, path, normal, count)?;
        let [a, b] = radius.domain();
        let [c, d] = path.domain();
        if closed {
            check(
                radius.evaluate(a)?.point[0] == radius.evaluate(b)?.point[0],
                "Closed section scale endpoints must agree",
            )?;
        }
        for (i, curve) in curves.iter_mut().enumerate() {
            let v = i as f64 / (count - 1) as f64;
            let center = path.evaluate(c + (d - c) * v)?.point;
            let r = radius.evaluate(a + (b - a) * v)?.point[0];
            check(
                r.is_finite() && r > 0.,
                "Evaluated section scale must be positive and finite",
            )?;
            for p in &mut curve.control_points {
                for k in 0..3 {
                    p[k] = center[k] + r * (p[k] - center[k]);
                }
            }
            curve.validate()?;
        }
        if closed {
            let first = curves[0].clone();
            *curves.last_mut().unwrap() = first;
        }
        Ok((curves, closed))
    };
    let (coarse, closed) = scaled(sections)?;
    let (fine, _) = scaled(4 * (sections - 1) + 1)?;
    let mut surface = crate::surface::loft(&coarse)?;
    if closed {
        surface.periodic_v = true;
        surface.knots_v = (0..sections + 2)
            .map(|i| (i as f64 - 1.) / (sections - 1) as f64)
            .collect();
    } else {
        for k in &mut surface.knots_v {
            *k /= (sections - 1) as f64;
        }
    }
    surface.validate()?;
    let mut maximum = 0_f64;
    for (i, curve) in fine.iter().enumerate() {
        let j = (i / 4).min(sections - 2);
        let f = (i - 4 * j) as f64 / 4.;
        for (k, p) in curve.control_points.iter().enumerate() {
            let delta: [f64; 3] = std::array::from_fn(|axis| {
                p[axis]
                    - ((1. - f) * surface.control_points[k][j][axis]
                        + f * surface.control_points[k][j + 1][axis])
            });
            let error = delta[0].hypot(delta[1]).hypot(delta[2]);
            check(error.is_finite(), "Scaled section refinement overflowed")?;
            maximum = maximum.max(error);
        }
    }
    let accepted = maximum <= budget;
    Ok(CheckedSweep {
        surface: if accepted { Some(surface) } else { None },
        report: crate::framed_sweep::SweepReport {
            accepted,
            sampled_control_deviation: maximum,
            budget,
            stations: fine.len(),
            sections,
            closed_path: closed,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn line(a: V, b: V) -> Curve {
        Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![a.to_vec(), b.to_vec()],
            weights: vec![1., 1.],
            periodic: false,
        }
    }
    #[test]
    fn straight_path_preserves_profile_and_translates() {
        let p = line([1., 0., 0.], [2., 0., 0.]);
        let q = line([0., 0., 0.], [0., 0., 5.]);
        let s = sweep(&p, &q, [1., 0., 0.], 8).unwrap();
        for i in 0..=10 {
            let t = i as f64 / 10.;
            let x = s.evaluate(0.3, t).unwrap().point;
            assert!((x[0] - 1.3).abs() < 1e-12);
            assert!((x[2] - 5. * t).abs() < 1e-12);
        }
    }
    #[test]
    fn circular_path_turns_profile_without_changing_section_length() {
        let p = line([1., 0., 0.], [1.2, 0., 0.]);
        let q = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        };
        let s = sweep(&p, &q, [1., 0., 0.], 17).unwrap();
        for i in 0..17 {
            let t = i as f64 / 16.;
            let a = s.evaluate(0., t).unwrap().point;
            let b = s.evaluate(1., t).unwrap().point;
            let center = q.evaluate(t).unwrap().point;
            for k in 0..3 {
                assert!((a[k] - center[k]).abs() < 1e-12);
            }
            assert!(((b[0] * b[0] + b[1] * b[1]).sqrt() - 1.2).abs() < 1e-12);
        }
    }
    #[test]
    fn rejects_ambiguous_initial_frame_and_invalid_counts() {
        let p = line([1., 0., 0.], [2., 0., 0.]);
        let q = line([0., 0., 0.], [0., 0., 5.]);
        assert!(sweep(&p, &q, [0., 0., 1.], 8).is_err());
        assert!(sweep(&p, &q, [1., 0., 0.], 33).is_err());
    }
    #[test]
    fn refinement_detects_arc_error_and_improves_with_sections() {
        let p = line([1., 0., 0.], [1.2, 0., 0.]);
        let q = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        };
        let coarse = checked_sweep(&p, &q, [1., 0., 0.], 3, 0.001).unwrap();
        assert!(!coarse.report.accepted);
        assert!(coarse.surface.is_none());
        let fine = checked_sweep(&p, &q, [1., 0., 0.], 32, 0.001).unwrap();
        assert!(fine.report.accepted);
        assert!(fine.surface.is_some());
        assert!(
            fine.report.sampled_control_deviation < coarse.report.sampled_control_deviation / 100.
        );
        assert_eq!(fine.report.stations, 125);
        assert!(checked_sweep(&p, &q, [1., 0., 0.], 32, f64::NAN).is_err());
    }

    fn circle() -> Curve {
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1., 1., 1.],
            control_points: vec![
                [1., 0., 0.],
                [1., 1., 0.],
                [0., 1., 0.],
                [-1., 1., 0.],
                [-1., 0., 0.],
                [-1., -1., 0.],
                [0., -1., 0.],
                [1., -1., 0.],
                [1., 0., 0.],
            ]
            .into_iter()
            .map(|p| p.to_vec())
            .collect(),
            weights: (0..9)
                .map(|i| {
                    if i % 2 == 0 {
                        1.
                    } else {
                        std::f64::consts::FRAC_1_SQRT_2
                    }
                })
                .collect(),
            periodic: false,
        }
    }
    #[test]
    #[cfg(feature = "codec")]
    fn closed_circle_has_periodic_storage_and_exact_position_seam() {
        let p = line([1., 0., 0.], [1.2, 0., 0.]);
        let q = circle();
        let before = q.clone();
        let s = sweep(&p, &q, [1., 0., 0.], 17).unwrap();
        assert!(s.periodic_v);
        s.validate().unwrap();
        if let Ok(path) = std::env::var("CAD_CLOSED_SWEEP_NATIVE_OUTPUT") {
            std::fs::write(
                path,
                value_codec::to_string(&value_codec::json!({"surfaces":[{"surface":s.clone()}]}))
                    .unwrap(),
            )
            .unwrap();
        }
        for row in &s.control_points {
            assert_eq!(row.first(), row.last());
        }
        for i in 0..=16 {
            let v = i as f64 / 16.;
            let a = s.evaluate(0., v).unwrap().point;
            let b = s.evaluate(1., v).unwrap().point;
            let center = q.evaluate(v).unwrap().point;
            for k in 0..3 {
                assert!((a[k] - center[k]).abs() < 1e-12);
            }
            assert!(((b[0] * b[0] + b[1] * b[1]).sqrt() - 1.2).abs() < 1e-12);
        }
        assert_eq!(q.control_points, before.control_points);
        assert_eq!(q.weights, before.weights);
        let report = checked_sweep(&p, &q, [1., 0., 0.], 17, 1.).unwrap();
        assert!(report.report.closed_path);
        assert!(sweep(&p, &q, [1., 0., 0.], 3).is_err());
    }
    #[test]
    fn nonplanar_periodic_path_distributes_holonomy_and_closes() {
        let mut points: Vec<Vec<f64>> = (0..6)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU / 6.;
                vec![a.cos(), a.sin(), 0.3 * (2. * a).sin()]
            })
            .collect();
        points.extend_from_within(0..3);
        let q = Curve {
            degree: 3,
            knots: (0..13).map(|i| (i as f64 - 3.) / 6.).collect(),
            weights: vec![1.; 9],
            control_points: points,
            periodic: true,
        };
        q.validate().unwrap();
        let a = sample(&q, 0.).unwrap().0;
        let p = line([a[0], a[1], a[2] + 0.1], [a[0], a[1], a[2] + 0.2]);
        let s = sweep(&p, &q, [0., 0., 1.], 25).unwrap();
        assert!(s.periodic_v);
        s.validate().unwrap();
        for row in &s.control_points {
            assert_eq!(row.first(), row.last());
        }
        for j in 0..25 {
            let x = s.evaluate(0., j as f64 / 24.).unwrap().point;
            let y = s.evaluate(1., j as f64 / 24.).unwrap().point;
            assert!(
                (dot(
                    sub([x[0], x[1], x[2]], [y[0], y[1], y[2]]),
                    sub([x[0], x[1], x[2]], [y[0], y[1], y[2]])
                )
                .sqrt()
                    - 0.1)
                    .abs()
                    < 1e-12
            );
        }
    }
    #[test]
    fn refuses_closed_cusps_and_internal_corners() {
        let p = line([0., 0., 0.], [0., 0., 1.]);
        let q = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.], vec![0., 0., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        assert!(sweep(&p, &q, [0., 0., 1.], 17).is_err());
        let q = Curve {
            degree: 1,
            knots: vec![0., 0., 0.5, 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.], vec![1., 1., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        assert!(sweep(&p, &q, [0., 0., 1.], 17).is_err());
    }

    #[test]
    fn coincident_sections_do_not_make_an_open_path_periodic() {
        let p = line([0., 0., 0.], [0., 0., 1.]);
        let q = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        };
        let s = sweep(&p, &q, [0., 0., 1.], 17).unwrap();
        assert!(!s.periodic_v);
        let r = checked_sweep(&p, &q, [0., 0., 1.], 17, 1.).unwrap();
        assert!(!r.report.closed_path);
    }
}
