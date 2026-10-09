use nurbs_core::{
    curve::Curve,
    foundation::approximate_edits::rebuild_surface_report,
    surface::{self, Axis, Surface},
};
fn panel() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 7., 7.],
        knots_v: vec![-3., -3., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 4., 0.]],
            vec![vec![4., 0., 0.], vec![4., 4., 0.]],
        ],
        weights: vec![vec![1., 2.], vec![3., 6.]],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn rational_panel_rebuild_bounds_independent_formula_in_both_axes() {
    for axis in [Axis::U, Axis::V] {
        let s = panel();
        let r = rebuild_surface_report(&s, axis, 2, 4, 0.2, None).unwrap();
        assert!(r.certificate.accepted);
        for i in 0..=20 {
            for j in 0..=20 {
                let u = i as f64 / 20.;
                let v = j as f64 / 20.;
                let p = r.surface.evaluate(2. + 5. * u, -3. + 4. * v).unwrap().point;
                let e = [12. * u / (1. + 2. * u), 8. * v / (1. + v), 0.];
                let d = p
                    .iter()
                    .zip(e)
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>()
                    .sqrt();
                assert!(d <= r.certificate.error_upper + 1e-12);
            }
        }
        assert_eq!(s, panel());
    }
}
#[test]
fn insufficient_representation_rolls_back() {
    let s = nurbs_core::polynomial::graph(
        [0., 1., 0., 1.],
        &[vec![0., 0., 1.], vec![0., 0., 0.], vec![1., 0., 0.]],
    )
    .unwrap();
    for axis in [Axis::U, Axis::V] {
        let r = rebuild_surface_report(&s, axis, 1, 2, 1e-6, None).unwrap();
        assert!(!r.certificate.accepted);
        assert_eq!(r.surface, s);
    }
}
#[test]
fn periodic_axis_rebuild_preserves_wrap_and_analytic_profile() {
    let c = Curve {
        degree: 2,
        knots: (0..9).map(|i| i as f64).collect(),
        control_points: vec![
            vec![1., 0., 0.],
            vec![0., 1., 0.],
            vec![-1., 0., 0.],
            vec![0., -1., 0.],
            vec![1., 0., 0.],
            vec![0., 1., 0.],
        ],
        weights: vec![1., 0.8, 1.2, 1., 1., 0.8],
        periodic: true,
    };
    let s = surface::extrude(&c, [0., 0., 3.]).unwrap();
    let r = rebuild_surface_report(&s, Axis::U, 2, 14, 0.5, None).unwrap();
    assert!(r.certificate.accepted);
    assert!(r.surface.periodic_u);
    let n = r.surface.control_points.len();
    assert_eq!(
        &r.surface.control_points[..2],
        &r.surface.control_points[n - 2..]
    );
    for i in 0..80 {
        let u = 2. + 4. * i as f64 / 80.;
        let span = u.floor() as usize;
        let t = u - span as f64;
        let basis = [0.5 * (1. - t).powi(2), 0.5 + t - t * t, 0.5 * t * t];
        let mut e = [0.; 3];
        let mut denominator = 0.;
        for j in 0..3 {
            let w = basis[j] * c.weights[span - 2 + j];
            denominator += w;
            for k in 0..2 {
                e[k] += w * c.control_points[span - 2 + j][k];
            }
        }
        for k in 0..2 {
            e[k] /= denominator;
        }
        for v in [0., 0.3, 1.] {
            e[2] = 3. * v;
            let p = r.surface.evaluate(u, v).unwrap().point;
            let d = p
                .iter()
                .zip(e)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(d <= r.certificate.error_upper + 1e-12);
        }
    }
}
#[test]
fn invalid_requests_fail() {
    let s = panel();
    for (degree, count) in [(0, 2), (26, 30), (2, 2), (1, 257)] {
        assert!(rebuild_surface_report(&s, Axis::U, degree, count, 0.1, None).is_err());
    }
    for budget in [-1., f64::NAN, f64::INFINITY] {
        assert!(rebuild_surface_report(&s, Axis::U, 1, 2, budget, None).is_err());
    }
}
