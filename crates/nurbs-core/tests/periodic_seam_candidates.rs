use nurbs_core::{
    curve::Curve,
    periodic_seam,
    surface::{self, Axis},
};
fn periodic(p: usize, active: Vec<f64>, period: f64) -> Curve {
    let n = active.len();
    let knots = (0..n + 2 * p + 1)
        .map(|i| {
            let j = i as isize - p as isize;
            active[j.rem_euclid(n as isize) as usize] + j.div_euclid(n as isize) as f64 * period
        })
        .collect();
    let mut controls: Vec<_> = (0..n)
        .map(|i| vec![i as f64, (i * i % 7) as f64, 0.])
        .collect();
    let mut weights: Vec<_> = (0..n).map(|i| 1. + (i % 3) as f64 * 0.5).collect();
    for i in 0..p {
        controls.push(controls[i].clone());
        weights.push(weights[i]);
    }
    Curve {
        degree: p,
        knots,
        control_points: controls,
        weights,
        periodic: true,
    }
}
fn basis(k: &[f64], i: usize, p: usize, t: f64) -> f64 {
    if p == 0 {
        return if k[i] <= t && t < k[i + 1] { 1. } else { 0. };
    }
    let a = if k[i + p] > k[i] {
        (t - k[i]) / (k[i + p] - k[i]) * basis(k, i, p - 1, t)
    } else {
        0.
    };
    let b = if k[i + p + 1] > k[i + 1] {
        (k[i + p + 1] - t) / (k[i + p + 1] - k[i + 1]) * basis(k, i + 1, p - 1, t)
    } else {
        0.
    };
    a + b
}
fn oracle(c: &Curve, t: f64) -> Vec<f64> {
    let [a, b] = c.domain();
    let t = a + (t - a).rem_euclid(b - a);
    let mut point = vec![0.; 3];
    let mut denominator = 0.;
    for i in 0..c.control_points.len() {
        let w = basis(&c.knots, i, c.degree, t) * c.weights[i];
        denominator += w;
        for k in 0..3 {
            point[k] += w * c.control_points[i][k];
        }
    }
    for x in &mut point {
        *x /= denominator;
    }
    point
}
#[test]
fn arbitrary_seams_preserve_independently_evaluated_periodic_geometry() {
    for c in [
        periodic(1, vec![0., 1., 3.], 4.),
        periodic(2, vec![2., 3., 4., 5., 6., 7.], 6.),
        periodic(3, vec![-2., -1., 0., 2., 3.], 6.),
        periodic(2, vec![0., 0., 1., 2., 3., 4.], 5.),
    ] {
        c.validate().unwrap();
        let [a, b] = c.domain();
        let before = c.clone();
        for fraction in [0.013, 0.125, 0.333, 0.75, 0.987] {
            let seam = a + (b - a) * fraction;
            let q = periodic_seam::curve_candidate(&c, seam).unwrap();
            if fraction == 0.333 {
                let proof = periodic_seam::verify_curve(&c, &q, 0.1, 10000).unwrap();
                assert!(proof.accepted, "degree {}: {proof:?}", c.degree);
                assert!(proof.error_upper <= 0.1);
            }
            assert!(q.periodic);
            assert_eq!(q.degree, c.degree);
            assert_eq!(q.domain()[0], seam);
            assert_eq!(
                q.control_points.len(),
                c.control_points.len() + usize::from(!c.knots.contains(&seam))
            );
            for i in 0..=240 {
                let t = q.domain()[0] + (q.domain()[1] - q.domain()[0]) * i as f64 / 240.;
                let actual = q.evaluate(t).unwrap().point;
                let expected = oracle(&c, t);
                assert!(
                    actual
                        .iter()
                        .zip(expected)
                        .all(|(x, y)| (*x - y).abs() < 1e-10),
                    "degree {}, seam {seam}, t {t}",
                    c.degree
                );
            }
            let n = q.control_points.len() - q.degree;
            for i in 0..q.degree {
                assert_eq!(q.control_points[i], q.control_points[n + i]);
                assert_eq!(q.weights[i], q.weights[n + i]);
            }
        }
        assert_eq!(c, before);
    }
}
#[test]
fn arbitrary_surface_seam_works_in_both_axes_and_preserves_other_basis() {
    let c = periodic(2, vec![2., 3., 4., 5., 6., 7.], 6.);
    let s = surface::extrude(&c, [0., 0., 3.]).unwrap();
    for (source, axis) in [
        (s.clone(), Axis::U),
        (nurbs_core::surface_edit::transpose(&s).unwrap(), Axis::V),
    ] {
        let before = source.clone();
        let q = periodic_seam::surface_candidate(&source, axis, 3.375).unwrap();
        for i in 0..=100 {
            let t = 3.375 + 6. * i as f64 / 100.;
            for v in [0., 0.375, 1.] {
                let expected = oracle(&c, t);
                let uv = match axis {
                    Axis::U => [t, v],
                    Axis::V => [v, t],
                };
                let point = q.evaluate(uv[0], uv[1]).unwrap().point;
                assert!(
                    (point[0] - expected[0]).abs() < 1e-10
                        && (point[1] - expected[1]).abs() < 1e-10
                        && (point[2] - 3. * v).abs() < 1e-10
                );
            }
        }
        match axis {
            Axis::U => {
                assert_eq!(q.knots_v, source.knots_v);
                assert!(q.periodic_u);
            }
            Axis::V => {
                assert_eq!(q.knots_u, source.knots_u);
                assert!(q.periodic_v);
            }
        }
        assert_eq!(source, before);
    }
}
#[test]
fn invalid_seams_and_control_limits_are_atomic() {
    let c = periodic(2, vec![2., 3., 4., 5., 6., 7.], 6.);
    let before = c.clone();
    for seam in [1., 8., f64::NAN] {
        assert!(periodic_seam::curve_candidate(&c, seam).is_err());
    }
    assert_eq!(c, before);
    let mut q = c;
    q.periodic = false;
    assert!(periodic_seam::curve_candidate(&q, 3.375).is_err());
    let c = periodic(1, (0..255).map(|i| i as f64).collect(), 255.);
    assert!(periodic_seam::curve_candidate(&c, 0.375).is_err());
    assert!(periodic_seam::curve_candidate(&c, 0.).is_ok());
    let s = surface::extrude(&c, [0., 0., 1.]).unwrap();
    let before = s.clone();
    assert!(periodic_seam::surface_candidate(&s, Axis::U, 0.375).is_err());
    assert_eq!(s, before);
}

#[test]
fn continuous_report_covers_normalized_wrap_and_only_returns_accepted_curve() {
    let c = periodic(2, vec![2., 3., 4., 5., 6., 7.], 6.);
    for seam in [2.125, 3.333, 7.875] {
        let report = periodic_seam::curve_report(&c, seam, 0.05, 10000).unwrap();
        assert!(
            report.verification.accepted,
            "seam {seam}: {:?}",
            report.verification
        );
        assert!(report.verification.error_upper <= 0.05);
        let q = report.curve.unwrap();
        let [start, end] = q.domain();
        for i in 0..=1000 {
            let phase = i as f64 / 1000.;
            let t = start + (end - start) * phase;
            let point = q.evaluate(t).unwrap().point;
            let expected = oracle(&c, seam + 6. * phase);
            let error = point
                .iter()
                .zip(expected)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(error <= report.verification.error_upper + 1e-12);
        }
    }
    let report = periodic_seam::curve_report(&c, 3.375, 1e-12, 7).unwrap();
    assert!(!report.verification.accepted);
    assert!(report.curve.is_none());
}

#[test]
fn phase_certificate_does_not_accept_a_translated_periodic_curve() {
    let c = periodic(2, vec![2., 3., 4., 5., 6., 7.], 6.);
    let mut translated = c.clone();
    for p in &mut translated.control_points {
        p[1] += 2.;
    }
    let report = periodic_seam::verify_curve(&c, &translated, 0.01, 100).unwrap();
    assert!(!report.accepted);
    assert!(report.error_upper >= 2.);
    assert!(periodic_seam::verify_curve(&c, &translated, 0., 100).is_err());
    assert!(periodic_seam::verify_curve(&c, &translated, 0.1, 0).is_err());
    assert!(periodic_seam::verify_curve(&c, &translated, 0.1, 1).is_err());
    let mut nonperiodic = translated;
    nonperiodic.periodic = false;
    assert!(periodic_seam::verify_curve(&c, &nonperiodic, 0.1, 100).is_err());
}
