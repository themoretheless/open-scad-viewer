use nurbs_core::{
    periodic_seam,
    surface::{Axis, Surface},
    surface_edit,
};
fn source() -> Surface {
    let mut points: Vec<_> = (0..6)
        .map(|i| {
            vec![
                vec![i as f64, (i * i % 7) as f64, 0.],
                vec![i as f64 + 0.25 * (i % 2) as f64, (i * i % 7) as f64, 3.],
            ]
        })
        .collect();
    let mut weights: Vec<_> = (0..6)
        .map(|i| vec![1. + (i % 3) as f64 * 0.5, 1. + (i % 2) as f64])
        .collect();
    for i in 0..2 {
        points.push(points[i].clone());
        weights.push(weights[i].clone());
    }
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: (0..11).map(|i| i as f64).collect(),
        knots_v: vec![-3., -3., 1., 1.],
        control_points: points,
        weights,
        periodic_u: true,
        periodic_v: false,
    }
}
fn basis(k: &[f64], i: usize, p: usize, t: f64, end: f64) -> f64 {
    if p == 0 {
        return if (k[i] <= t && t < k[i + 1]) || (t == end && k[i] < t && k[i + 1] == t) {
            1.
        } else {
            0.
        };
    }
    let a = if k[i + p] > k[i] {
        (t - k[i]) / (k[i + p] - k[i]) * basis(k, i, p - 1, t, end)
    } else {
        0.
    };
    let b = if k[i + p + 1] > k[i + 1] {
        (k[i + p + 1] - t) / (k[i + p + 1] - k[i + 1]) * basis(k, i + 1, p - 1, t, end)
    } else {
        0.
    };
    a + b
}
fn oracle(s: &Surface, uv: [f64; 2]) -> [f64; 3] {
    let mut out = [0.; 3];
    let mut denominator = 0.;
    for i in 0..s.control_points.len() {
        for j in 0..s.control_points[0].len() {
            let w = basis(
                &s.knots_u,
                i,
                s.degree_u,
                uv[0],
                s.knots_u[s.control_points.len()],
            ) * basis(
                &s.knots_v,
                j,
                s.degree_v,
                uv[1],
                s.knots_v[s.control_points[0].len()],
            ) * s.weights[i][j];
            denominator += w;
            for k in 0..3 {
                out[k] += w * s.control_points[i][j][k];
            }
        }
    }
    for x in &mut out {
        *x /= denominator;
    }
    out
}
#[test]
fn whole_tensor_product_report_accepts_both_axes_and_bounds_independent_formula() {
    let original = source();
    for (s, axis, index) in [
        (original.clone(), Axis::U, 0),
        (surface_edit::transpose(&original).unwrap(), Axis::V, 1),
    ] {
        let before = s.clone();
        let seam = 3.333;
        let r = periodic_seam::surface_report(&s, axis, seam, 0.75, 20000).unwrap();
        assert!(r.verification.accepted, "{:?}", r.verification);
        assert!(r.verification.error_upper <= 0.75);
        let q = r.surface.unwrap();
        let knots = if index == 0 { &q.knots_u } else { &q.knots_v };
        let degree = if index == 0 { q.degree_u } else { q.degree_v };
        let n = if index == 0 {
            q.control_points.len()
        } else {
            q.control_points[0].len()
        };
        let end = knots[n];
        for i in 0..=50 {
            let phase = i as f64 / 50.;
            let t = seam + (end - seam) * phase;
            let old = 2. + (seam + 6. * phase - 2.).rem_euclid(6.);
            for j in 0..=20 {
                let v = -3. + 4. * j as f64 / 20.;
                let uv = if index == 0 { [t, v] } else { [v, t] };
                let olduv = if index == 0 { [old, v] } else { [v, old] };
                let p = q.evaluate(uv[0], uv[1]).unwrap().point;
                let expected = oracle(&s, olduv);
                let error = p
                    .into_iter()
                    .zip(expected)
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>()
                    .sqrt();
                assert!(error <= r.verification.error_upper + 1e-11);
            }
        }
        assert_eq!(knots[degree], seam);
        assert_eq!(s, before);
    }
}
#[test]
fn changed_surface_and_small_budget_do_not_return_a_certified_edit() {
    let s = source();
    let mut q = s.clone();
    for row in &mut q.control_points {
        for p in row {
            p[2] += 2.;
        }
    }
    let r = periodic_seam::verify_surface(&s, &q, Axis::U, 0.01, 50).unwrap();
    assert!(!r.accepted);
    assert!(r.error_upper >= 2.);
    let r = periodic_seam::surface_report(&s, Axis::U, 3.375, 1e-12, 7).unwrap();
    assert!(!r.verification.accepted);
    assert!(r.surface.is_none());
    assert!(periodic_seam::surface_report(&s, Axis::U, 3.375, 0.1, 1).is_err());
}
#[test]
fn invalid_axes_domains_tolerances_and_budgets_are_refused() {
    let s = source();
    let q = periodic_seam::surface_candidate(&s, Axis::U, 3.375).unwrap();
    for tol in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(periodic_seam::verify_surface(&s, &q, Axis::U, tol, 100).is_err());
    }
    for budget in [0, 100001] {
        assert!(periodic_seam::verify_surface(&s, &q, Axis::U, 0.1, budget).is_err());
    }
    assert!(periodic_seam::verify_surface(&s, &q, Axis::V, 0.1, 100).is_err());
    let mut q = q;
    q.knots_v = vec![-4., -4., 1., 1.];
    assert!(periodic_seam::verify_surface(&s, &q, Axis::U, 0.1, 100).is_err());
}
