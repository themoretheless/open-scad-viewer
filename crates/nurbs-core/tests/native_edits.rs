//! Independent rational equations, original non-unit domains, seams and refusals.
use nurbs_core::{
    curve::Curve,
    surface::{Axis, Surface},
    surface_edit as edit,
};
fn curve() -> Curve {
    let c = Curve {
        degree: 2,
        knots: vec![2., 2., 2., 5., 5., 5.],
        control_points: vec![vec![0., 0., 1.], vec![2., 3., 2.], vec![4., 0., 3.]],
        weights: vec![1., 2., 1.],
        periodic: false,
    };
    c.validate().unwrap();
    c
}
fn curve_equation(u: f64) -> [f64; 3] {
    let t = (u - 2.) / 3.;
    let a = (1. - t) * (1. - t);
    let b = 4. * t * (1. - t);
    let c = t * t;
    let w = a + b + c;
    [(2. * b + 4. * c) / w, 3. * b / w, (a + 2. * b + 3. * c) / w]
}
fn near(a: &[f64], b: &[f64]) {
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b) {
        assert!((x - y).abs() < 2e-10, "{a:?} != {b:?}");
    }
}
fn verify_curve(c: &Curve, reverse: bool) {
    let [a, b] = c.domain();
    for i in 0..=300 {
        let u = a + (b - a) * i as f64 / 300.;
        near(
            &c.evaluate(u).unwrap().point,
            &curve_equation(if reverse { 7. - u } else { u }),
        );
    }
}
#[test]
fn native_curve_exact_edits_preserve_independent_weighted_equation() {
    let c = curve();
    verify_curve(&c.insert(3., 2).unwrap(), false);
    verify_curve(&c.elevate(7).unwrap(), false);
    let pieces = c.split(3.25).unwrap();
    assert_eq!(pieces[0].domain(), [2., 3.25]);
    assert_eq!(pieces[1].domain(), [3.25, 5.]);
    for p in &pieces {
        verify_curve(p, false);
    }
    verify_curve(&c.trim(2.25, 4.75).unwrap(), false);
    verify_curve(&c.reverse().unwrap(), true);
    let refined = c.insert(3., 2).unwrap().insert(4., 1).unwrap();
    let spans = refined.decompose().unwrap();
    assert_eq!(spans.len(), 3);
    for span in spans {
        let p = span.definition();
        assert_eq!(p.control_points.len(), p.degree + 1);
        verify_curve(p, false);
    }
}
#[test]
fn native_curve_edits_refuse_invalid_requests() {
    let c = curve();
    assert!(c.insert(f64::NAN, 1).is_err());
    assert!(c.insert(3., 0).is_err());
    assert!(c.insert(3., 3).is_err());
    assert!(c.insert(6., 1).is_err());
    assert!(c.elevate(1).is_err());
    assert!(c.elevate(26).is_err());
    assert!(c.split(2.).is_err());
    assert!(c.split(5.).is_err());
    assert!(c.trim(4., 3.).is_err());
    assert!(c.trim(1., 4.).is_err());
    let mut broken = c;
    broken.weights.clear();
    assert!(broken.reverse().is_err());
    assert!(broken.decompose().is_err());
}
fn surface() -> Surface {
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 5., 5.],
        knots_v: vec![-4., -4., 2., 2.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 2., 1.]],
            vec![vec![3., 0., 1.], vec![3., 2., 4.]],
        ],
        weights: vec![vec![1., 2.], vec![3., 4.]],
        periodic_u: false,
        periodic_v: false,
    };
    s.validate().unwrap();
    s
}
fn surface_equation(u: f64, v: f64) -> [f64; 3] {
    let u = (u - 2.) / 3.;
    let v = (v + 4.) / 6.;
    let a = (1. - u) * (1. - v);
    let b = 2. * (1. - u) * v;
    let c = 3. * u * (1. - v);
    let d = 4. * u * v;
    let w = a + b + c + d;
    [3. * (c + d) / w, 2. * (b + d) / w, (b + c + 4. * d) / w]
}
fn verify_surface(s: &Surface, domain: [f64; 4], map: impl Fn(f64, f64) -> (f64, f64)) {
    for i in 0..=30 {
        for j in 0..=30 {
            let u = domain[0] + (domain[1] - domain[0]) * i as f64 / 30.;
            let v = domain[2] + (domain[3] - domain[2]) * j as f64 / 30.;
            let (a, b) = map(u, v);
            near(&s.evaluate(u, v).unwrap().point, &surface_equation(a, b));
        }
    }
}
#[test]
fn surface_refinement_and_elevation_preserve_rational_tensor_equation() {
    let s = surface();
    for axis in [Axis::U, Axis::V] {
        let mid = match axis {
            Axis::U => 3.,
            Axis::V => -1.,
        };
        verify_surface(
            &edit::insert(&s, axis, mid, 1).unwrap(),
            [2., 5., -4., 2.],
            |u, v| (u, v),
        );
        verify_surface(
            &edit::elevate(&s, axis, 5).unwrap(),
            [2., 5., -4., 2.],
            |u, v| (u, v),
        );
    }
}
#[test]
fn surface_reorientation_preserves_chart_and_normal_contract() {
    let s = surface();
    let t = edit::transpose(&s).unwrap();
    verify_surface(&t, [-4., 2., 2., 5.], |u, v| (v, u));
    assert_eq!(edit::transpose(&t).unwrap(), s);
    verify_surface(
        &edit::reverse(&s, Axis::U).unwrap(),
        [2., 5., -4., 2.],
        |u, v| (7. - u, v),
    );
    verify_surface(
        &edit::reverse(&s, Axis::V).unwrap(),
        [2., 5., -4., 2.],
        |u, v| (u, -2. - v),
    );
    let a = s.evaluate(3., -1.).unwrap().unit_normal().unwrap();
    let b = t.evaluate(-1., 3.).unwrap().unit_normal().unwrap();
    near(&a, &b.map(|v| -v));
}
#[test]
fn surface_split_trim_and_decompose_keep_domains_and_shared_seams() {
    let s = surface();
    let pieces = edit::split(&s, Axis::U, 3.).unwrap();
    verify_surface(&pieces[0], [2., 3., -4., 2.], |u, v| (u, v));
    verify_surface(&pieces[1], [3., 5., -4., 2.], |u, v| (u, v));
    for j in 0..=100 {
        let v = -4. + 6. * j as f64 / 100.;
        near(
            &pieces[0].evaluate(3., v).unwrap().point,
            &pieces[1].evaluate(3., v).unwrap().point,
        );
    }
    let pieces = edit::split(&s, Axis::V, -1.).unwrap();
    verify_surface(&pieces[0], [2., 5., -4., -1.], |u, v| (u, v));
    verify_surface(&pieces[1], [2., 5., -1., 2.], |u, v| (u, v));
    verify_surface(
        &s.trim([2.5, 4., -3., 1.]).unwrap(),
        [2.5, 4., -3., 1.],
        |u, v| (u, v),
    );
    let refined =
        edit::insert(&edit::insert(&s, Axis::U, 3., 1).unwrap(), Axis::V, -1., 1).unwrap();
    let p = edit::decompose(&refined).unwrap();
    assert_eq!(p.len(), 4);
    for p in p {
        assert_eq!(p.control_points.len(), p.degree_u + 1);
        assert_eq!(p.control_points[0].len(), p.degree_v + 1);
        let (u, v) = p
            .evaluate(p.knots_u[p.degree_u], p.knots_v[p.degree_v])
            .unwrap()
            .domains();
        verify_surface(&p, [u[0], u[1], v[0], v[1]], |u, v| (u, v));
    }
}
#[test]
fn surface_iso_curves_follow_independent_equation() {
    let s = surface();
    for axis in [Axis::U, Axis::V] {
        let fixed = match axis {
            Axis::U => 3.,
            Axis::V => -1.,
        };
        let c = s.iso(axis, fixed).unwrap();
        let [a, b] = c.domain();
        for i in 0..=200 {
            let t = a + (b - a) * i as f64 / 200.;
            let (u, v) = match axis {
                Axis::U => (fixed, t),
                Axis::V => (t, fixed),
            };
            near(&c.evaluate(t).unwrap().point, &surface_equation(u, v));
        }
    }
    assert!(s.iso(Axis::U, 1.).is_err());
    assert!(s.iso(Axis::V, f64::NAN).is_err());
}
#[test]
fn surface_edits_refuse_invalid_input_without_panicking() {
    let s = surface();
    for axis in [Axis::U, Axis::V] {
        assert!(edit::split(&s, axis, f64::NAN).is_err());
        assert!(edit::split(&s, axis, 20.).is_err());
        assert!(edit::insert(&s, axis, 3., 0).is_err());
        assert!(edit::elevate(&s, axis, 26).is_err());
    }
    assert!(s.trim([2., 5., 2., -4.]).is_err());
    assert!(s.trim([1., 5., -4., 2.]).is_err());
    let mut broken = s;
    broken.control_points.clear();
    assert!(edit::transpose(&broken).is_err());
    assert!(edit::decompose(&broken).is_err());
    assert!(edit::reverse(&broken, Axis::U).is_err());
}

#[test]
fn affine_reparameterization_matches_independent_equation() {
    let c = curve().insert(3., 1).unwrap();
    let report = nurbs_core::foundation::reparameterize_curve_report(&c, [-8., 4.], None).unwrap();
    assert_eq!(report.old_domain, [2., 5.]);
    assert_eq!(report.new_domain, [-8., 4.]);
    assert_eq!(report.curve.domain(), [-8., 4.]);
    assert_eq!(report.curve.control_points, c.control_points);
    assert_eq!(report.curve.weights, c.weights);
    for i in 0..=300 {
        let t = -8. + 12. * i as f64 / 300.;
        near(
            &report.curve.evaluate(t).unwrap().point,
            &curve_equation(2. + (t + 8.) / 4.),
        );
    }
    assert!(nurbs_core::foundation::reparameterize_curve_report(&c, [1., 1.], None).is_err());
    assert!(
        nurbs_core::foundation::reparameterize_curve_report(&c, [0., f64::INFINITY], None).is_err()
    );
}
#[test]
fn affine_reparameterization_refuses_collapsed_distinct_knots() {
    let c = Curve {
        degree: 2,
        knots: vec![2., 2., 2., 3., 3. + 2e-9, 5., 5., 5.],
        control_points: vec![
            vec![0., 0.],
            vec![1., 0.],
            vec![1., 1.],
            vec![2., 1.],
            vec![3., 0.],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    c.validate().unwrap();
    assert!(
        nurbs_core::foundation::reparameterize_curve_report(&c, [1e8, 1e8 + 1.], None).is_err(),
        "Distinct source knots must not merge under rounding"
    );
}

#[test]
fn periodic_surface_reorientation_preserves_seam_and_storage() {
    let p = vec![
        vec![1., 0., 0.],
        vec![0.5, 1., 0.],
        vec![-0.5, 1., 0.],
        vec![-1., 0., 0.],
        vec![-0.5, -1., 0.],
        vec![0.5, -1., 0.],
        vec![1., 0., 0.],
        vec![0.5, 1., 0.],
    ];
    let c = Curve {
        degree: 2,
        knots: (0..11).map(|i| i as f64).collect(),
        control_points: p,
        weights: vec![1., 2., 1., 2., 1., 2., 1., 2.],
        periodic: true,
    };
    c.validate().unwrap();
    let s = nurbs_core::surface::extrude(&c, [0., 0., 3.]).unwrap();
    let t = edit::transpose(&s).unwrap();
    assert!(t.periodic_v);
    assert!(!t.periodic_u);
    assert_eq!(edit::transpose(&t).unwrap(), s);
    let r = edit::reverse(&s, Axis::U).unwrap();
    assert!(r.periodic_u);
    for i in 0..=120 {
        let u = 2. + 6. * i as f64 / 120.;
        for j in 0..=10 {
            let v = j as f64 / 10.;
            near(
                &t.evaluate(v, u).unwrap().point,
                &s.evaluate(u, v).unwrap().point,
            );
            near(
                &r.evaluate(u, v).unwrap().point,
                &s.evaluate(10. - u, v).unwrap().point,
            );
        }
    }
    let mapped = nurbs_core::foundation::reparameterize_curve_report(&c, [-3., 9.], None).unwrap();
    assert!(mapped.period_preserved);
    assert!(mapped.curve.periodic);
    for i in 0..=120 {
        let t = -3. + 12. * i as f64 / 120.;
        near(
            &mapped.curve.evaluate(t).unwrap().point,
            &c.evaluate(2. + (t + 3.) / 2.).unwrap().point,
        );
    }
}
#[test]
fn native_edits_honor_control_net_budgets() {
    let c =
        Curve::from_polyline((0..128).map(|i| vec![i as f64, (i % 2) as f64]).collect()).unwrap();
    assert!(c.elevate(3).is_err());
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: std::iter::once(0.)
            .chain((0..32).map(|i| i as f64))
            .chain(std::iter::once(31.))
            .collect(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..32)
            .map(|i| vec![vec![i as f64, 0., 0.], vec![i as f64, 1., 0.]])
            .collect(),
        weights: vec![vec![1., 1.]; 32],
        periodic_u: false,
        periodic_v: false,
    };
    s.validate().unwrap();
    assert!(edit::insert(&s, Axis::U, 0.5, 1).is_err());
    assert!(edit::elevate(&s, Axis::U, 2).is_err());
    assert!(edit::transpose(&s).is_ok());
}

#[test]
fn reversal_refuses_collapsed_distinct_knots() {
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1. + 2e-9, 1e8, 1e8, 1e8],
        control_points: vec![
            vec![0., 0.],
            vec![1., 0.],
            vec![1., 1.],
            vec![2., 1.],
            vec![3., 0.],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    c.validate().unwrap();
    assert!(
        c.reverse().is_err(),
        "Distinct source knots must not merge when reversing"
    );
}
