use nurbs_core::curve::Curve;
fn curve(points: Vec<Vec<f64>>) -> Curve {
    let d = points.len() - 1;
    Curve {
        degree: d,
        knots: [vec![0.; d + 1], vec![1.; d + 1]].concat(),
        weights: vec![1.; d + 1],
        control_points: points,
        periodic: false,
    }
}
fn line(a: [f64; 2], b: [f64; 2]) -> Curve {
    Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap()
}
fn wire(c: Curve) -> Vec<Curve> {
    vec![
        c,
        line([60., 1.], [60., 10.]),
        line([60., 10.], [0., 10.]),
        line([0., 10.], [0., 0.]),
    ]
}
#[test]
fn algebraic_speed_equivalence_is_used_by_boolean_and_even_odd_cancellation() {
    let a = wire(curve(vec![
        vec![0., 0.],
        vec![0., 0.],
        vec![10., 0.],
        vec![30., 0.],
        vec![60., 1.],
    ]));
    let b = wire(curve(vec![
        vec![0., 0.],
        vec![0., 0.],
        vec![0., 0.],
        vec![3., 0.],
        vec![12., 0.],
        vec![30., 0.],
        vec![60., 1.],
    ]));
    let area = brep_core::bezier_profile::validate_profile(&[a.clone()], 1e-7).unwrap();
    let union =
        brep_core::bezier_profile::boolean(&[a.clone()], &[b.clone()], "union", 1e-7).unwrap();
    assert!(
        (brep_core::bezier_profile::validate_profile(&union, 1e-7).unwrap() - area).abs() < 1e-6
    );
    assert!(
        brep_core::bezier_profile::boolean(&[a.clone()], &[b.clone()], "difference", 1e-7)
            .unwrap()
            .is_empty()
    );
    assert!(
        brep_core::bezier_profile::normalize(&[a, b], 1e-7)
            .unwrap()
            .is_empty()
    );
}
#[test]
fn degree_eight_self_loop_is_split_from_an_exact_parent_census() {
    let points = (0..=8)
        .map(|i| {
            let i = i as f64;
            let c2 = i * (i - 1.) / 2.;
            let c3 = i * (i - 1.) * (i - 2.) / 6.;
            vec![10.5 * i - 3. * c2, 10.5 * i - 16. * c2 + 8. * c3]
        })
        .collect();
    let c = curve(points);
    let proof = brep_core::profile_certificate::certify_curve_intersections(&c, &c, true).unwrap();
    assert_eq!(proof.events, vec![[0.25, 0.75]]);
    let normalized =
        brep_core::bezier_profile::normalize(&[vec![c, line([0., 84.], [0., 0.])]], 1e-7).unwrap();
    assert!(normalized.len() >= 2);
    assert!(normalized.iter().flatten().any(|c| c.degree == 8));
    for wire in normalized {
        assert!(
            brep_core::bezier_profile::signed_area(&wire, 1e-7)
                .unwrap()
                .abs()
                > 1e-3
        );
    }
}
