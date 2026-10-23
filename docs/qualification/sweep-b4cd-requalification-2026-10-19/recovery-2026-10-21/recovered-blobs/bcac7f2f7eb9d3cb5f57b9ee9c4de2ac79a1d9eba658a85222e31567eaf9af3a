use nurbs_core::{curve::Curve, periodic_seam};
fn source() -> Curve {
    Curve {
        degree: 2,
        knots: (0..11).map(|i| i as f64).collect(),
        control_points: vec![
            vec![1., 0.],
            vec![0.5, 1.],
            vec![-0.5, 1.],
            vec![-1., 0.],
            vec![-0.5, -1.],
            vec![0.5, -1.],
            vec![1., 0.],
            vec![0.5, 1.],
        ],
        weights: vec![1., 2., 1., 2., 1., 2., 1., 2.],
        periodic: true,
    }
}
#[test]
fn every_existing_knot_preserves_wrapped_geometry_and_periodic_storage() {
    let c = source();
    for seam in 2..8 {
        let seam = seam as f64;
        let q = periodic_seam::curve_at_knot(&c, seam).unwrap();
        assert_eq!(q.domain(), [seam, seam + 6.]);
        assert!(q.periodic);
        for i in 0..=300 {
            let t = seam + 6. * i as f64 / 300.;
            let old = 2. + (t - 2.).rem_euclid(6.);
            let a = c.evaluate(old).unwrap().point;
            let b = q.evaluate(t).unwrap().point;
            assert!(a.iter().zip(b).all(|(x, y)| (*x - y).abs() < 1e-11));
        }
    }
}
#[test]
fn non_knot_nonperiodic_and_out_of_domain_requests_are_refused() {
    let c = source();
    for seam in [1., 8., 2.5, f64::NAN] {
        assert!(periodic_seam::curve_at_knot(&c, seam).is_err());
    }
    let mut q = c;
    q.periodic = false;
    assert!(periodic_seam::curve_at_knot(&q, 3.).is_err());
}

#[test]
fn surface_axis_rotation_preserves_periodic_extrusion() {
    let mut c = source();
    for p in &mut c.control_points {
        p.push(0.);
    }
    let s = nurbs_core::surface::extrude(&c, [0., 0., 3.]).unwrap();
    let q = periodic_seam::surface_at_knot(&s, nurbs_core::surface::Axis::U, 4.).unwrap();
    assert!(q.periodic_u);
    assert_eq!(q.knots_v, s.knots_v);
    for i in 0..=100 {
        for v in [0., 0.25, 0.75, 1.] {
            let u = 4. + 6. * i as f64 / 100.;
            let old = 2. + (u - 2.).rem_euclid(6.);
            let a = s.evaluate(old, v).unwrap().point;
            let b = q.evaluate(u, v).unwrap().point;
            assert!(a.iter().zip(b).all(|(x, y)| (*x - y).abs() < 1e-11));
        }
    }
    assert!(periodic_seam::surface_at_knot(&s, nurbs_core::surface::Axis::V, 0.5).is_err());
}
