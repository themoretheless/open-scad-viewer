use nurbs_core::{
    curve::Curve,
    offset::{
        freeform::NormalOffset,
        freeform_blend::{ContactChamfer, RollingBall},
    },
    patches,
    surface::Surface,
};
fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).fold(0f64, |n, (a, b)| n.hypot(a - b))
}
fn supports() -> [Surface; 2] {
    let a = patches::bezier(
        (0..3)
            .map(|i| {
                (0..2)
                    .map(|j| vec![i as f64 / 2., j as f64, if i == 2 { 0.3 } else { 0. }])
                    .collect()
            })
            .collect(),
        None,
    )
    .unwrap();
    let b = patches::bezier(
        vec![
            vec![vec![-1., 0., 0.], vec![-1., 0., 2.]],
            vec![vec![2., 0., 0.], vec![2., 0., 2.]],
        ],
        None,
    )
    .unwrap();
    [a, b]
}
#[test]
fn freeform_offset_retains_exact_normal_map_and_analytic_derivatives() {
    let s = patches::bezier(
        vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.2]],
        ],
        None,
    )
    .unwrap();
    let before = s.clone();
    let offset = NormalOffset::new(&s, 0.1).unwrap();
    let qualified = offset.qualify([[0., 1.], [0., 1.]], 5000).unwrap();
    assert!(qualified.minimum_stretch() > 0.);
    assert!(qualified.cells() <= 5000);
    for uv in [[0.1, 0.2], [0.3, 0.5], [0.7, 0.8], [0.9, 0.1]] {
        let e = qualified.evaluate(uv).unwrap();
        let base = s.evaluate(uv[0], uv[1]).unwrap();
        let n = base.unit_normal().unwrap();
        assert!(distance(e.point, std::array::from_fn(|i| base.point[i] + 0.1 * n[i])) < 1e-13);
        for axis in 0..2 {
            let mut a = uv;
            let mut b = uv;
            a[axis] -= 1e-5;
            b[axis] += 1e-5;
            let a = offset.evaluate(a).unwrap().point;
            let b = offset.evaluate(b).unwrap().point;
            let fd = std::array::from_fn(|k| (b[k] - a[k]) / 2e-5);
            assert!(distance(fd, if axis == 0 { e.tangent_u } else { e.tangent_v }) < 1e-8);
        }
    }
    assert_eq!(s, before);
    assert!(qualified.evaluate([-0.1, 0.]).is_err());
    assert!(offset.qualify([[0., 1.], [0., 1.]], 0).is_err());
    assert!(NormalOffset::new(&s, f64::NAN).is_err());
}
#[test]
fn focal_crossing_cannot_receive_a_regular_offset_certificate() {
    let s = patches::bezier(
        (0..3)
            .map(|i| {
                (0..2)
                    .map(|j| vec![i as f64 - 1., j as f64, if i == 1 { -0.5 } else { 0.5 }])
                    .collect()
            })
            .collect(),
        None,
    )
    .unwrap();
    let offset = NormalOffset::new(&s, 1.).unwrap();
    assert!(offset.evaluate([0.5, 0.5]).is_err());
    assert!(offset.qualify([[0., 1.], [0., 1.]], 128).is_err());
}
#[test]
fn rolling_ball_queries_solve_curved_support_contacts_and_exact_circle_sections() {
    let [a, b] = supports();
    let ball = RollingBall::new([&a, &b], 0.2, [1., -1.], 0, 1e-11).unwrap();
    let mut seed = [0.4, 0.2, 0.5, 0.3];
    for u in [0.1, 0.25, 0.5, 0.75, 0.9] {
        let section = ball.section(u, seed).unwrap();
        seed = section.parameters();
        assert!(section.offset_residual() < 1e-11);
        assert!((seed[1] - 0.2).abs() < 1e-11);
        for (i, support) in [&a, &b].into_iter().enumerate() {
            let p = support.evaluate(seed[2 * i], seed[2 * i + 1]).unwrap();
            assert!(distance(p.point, section.contacts()[i]) < 1e-13);
            let (contact, normal) = section.evaluate(i as f64).unwrap();
            assert!(distance(contact, p.point) < 1e-10);
            let n = p.unit_normal().unwrap();
            let dot = (0..3).map(|k| normal[k] * n[k]).sum::<f64>();
            assert!((dot.abs() - 1.).abs() < 1e-10);
        }
        let curve = section.curve().unwrap();
        for i in 0..=20 {
            let t = i as f64 / 20.;
            let (p, _) = section.evaluate(t).unwrap();
            assert!((distance(p, section.center()) - 0.2).abs() < 1e-12);
            let p = curve.evaluate(t).unwrap().point;
            assert!((distance([p[0], p[1], p[2]], section.center()) - 0.2).abs() < 1e-12);
        }
    }
    assert!(ball.section(2., seed).is_err());
    assert!(RollingBall::new([&a, &b], 0., [1., -1.], 0, 1e-11).is_err());
    let singular = RollingBall::new([&a, &b], 0.2, [1., -1.], 1, 1e-11).unwrap();
    assert!(singular.section(0.2, seed).is_err());
}
#[test]
fn freeform_chamfer_keeps_original_contact_rails_and_derivatives() {
    let [a, b] = supports();
    let pa = Curve::from_polyline(vec![vec![0., 0.2], vec![1., 0.2]]).unwrap();
    let pb = Curve::from_polyline(vec![vec![1. / 3., 0.3], vec![2. / 3., 0.4]]).unwrap();
    let chamfer = ContactChamfer::new([&a, &b], [&pa, &pb]).unwrap();
    for u in [0.1, 0.35, 0.6, 0.9] {
        let e = chamfer.evaluate(u, 0.4).unwrap();
        for (side, (s, pc)) in [(&a, &pa), (&b, &pb)].into_iter().enumerate() {
            let uv = pc.evaluate(u).unwrap().point;
            let p = s.evaluate(uv[0], uv[1]).unwrap().point;
            assert!(distance(chamfer.evaluate(u, side as f64).unwrap().point, p) < 1e-12);
        }
        let p = chamfer.evaluate(u - 1e-5, 0.4).unwrap().point;
        let q = chamfer.evaluate(u + 1e-5, 0.4).unwrap().point;
        assert!(distance(std::array::from_fn(|k| (q[k] - p[k]) / 2e-5), e.tangent_u) < 1e-8);
    }
    let outside = Curve::from_polyline(vec![vec![-1., 0.], vec![1., 0.]]).unwrap();
    assert!(ContactChamfer::new([&a, &b], [&outside, &pb]).is_err());
    assert!(chamfer.evaluate(0., f64::NAN).is_err());
}
#[test]
fn extracted_uniform_patch_is_not_reinterpreted_as_clamped_bezier_controls() {
    let controls =
        std::array::from_fn(|i| std::array::from_fn(|j| [i as f64, j as f64, (i * j) as f64]));
    let s = nurbs_core::polyhedral_spline::from_uniform_patch(controls).unwrap();
    assert!(distance(s.evaluate(0., 0.).unwrap().point, [1., 1., 1.]) < 1e-12);
    assert!(distance(s.evaluate(1., 1.).unwrap().point, [2., 2., 4.]) < 1e-12);
}

#[test]
fn offset_fitting_retains_nonunit_original_uv_clocks() {
    let mut source = supports()[0].clone();
    source.knots_u = vec![2., 2., 2., 7., 7., 7.];
    source.knots_v = vec![-3., -3., 1., 1.];
    let offset = NormalOffset::new(&source, 0.05).unwrap();
    let fit = offset.approximate(1e-3).unwrap();
    assert!(!fit.exact);
    assert!(fit.max_deviation <= 1e-3);
    assert_eq!(fit.surface.knots_u[fit.surface.degree_u], 2.);
    assert_eq!(fit.surface.knots_u[fit.surface.control_points.len()], 7.);
    assert_eq!(fit.surface.knots_v[fit.surface.degree_v], -3.);
    assert_eq!(fit.surface.knots_v[fit.surface.control_points[0].len()], 1.);
    for uv in [[2.23, -2.71], [3.76, -0.41], [6.89, 0.79]] {
        let a = offset.evaluate(uv).unwrap().point;
        let b = fit.surface.evaluate(uv[0], uv[1]).unwrap().point;
        assert!(distance(a, b) < 1e-3);
    }
}

#[test]
fn rational_source_offset_derivatives_use_original_weights() {
    let mut source = supports()[0].clone();
    source.weights[1][0] = 0.7;
    source.weights[2][1] = 1.3;
    let before = source.clone();
    let offset = NormalOffset::new(&source, 0.02).unwrap();
    let uv = [0.31, 0.47];
    let e = offset.evaluate(uv).unwrap();
    for axis in 0..2 {
        let mut a = uv;
        let mut b = uv;
        a[axis] -= 1e-5;
        b[axis] += 1e-5;
        let a = offset.evaluate(a).unwrap().point;
        let b = offset.evaluate(b).unwrap().point;
        assert!(
            distance(
                std::array::from_fn(|k| (b[k] - a[k]) / 2e-5),
                if axis == 0 { e.tangent_u } else { e.tangent_v }
            ) < 1e-7
        );
    }
    assert_eq!(source, before);
    assert_eq!(offset.source().weights, source.weights);
}
