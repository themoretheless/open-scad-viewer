use nurbs_core::{surface::Surface, surface_correspondence::*};
fn patch() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| vec![i as f64, j as f64, (i * j) as f64 / 8.])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1., 2., 1.], vec![2., 4., 2.], vec![1., 2., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}
fn limits() -> Limits {
    Limits {
        cells: 10000,
        products: 1000000,
    }
}
#[test]
fn rational_saddle_interior_is_bounded_and_sources_unchanged() {
    let a = patch();
    let mut b = a.clone();
    b.control_points[1][1][2] += 0.125;
    let before = (a.clone(), b.clone());
    let r = inspect(&a, &b, 0.13, limits()).unwrap();
    assert!(r.within_budget && r.error_upper.unwrap() >= 0.125 && r.error_upper.unwrap() < 0.13);
    // All four boundary curves are unchanged; the interior is not.
    assert!(b.evaluate(0.5, 0.5).unwrap().point[2] > a.evaluate(0.5, 0.5).unwrap().point[2]);
    assert!(!inspect(&a, &b, 0.01, limits()).unwrap().within_budget);
    assert_eq!((a, b), before);
}
#[test]
fn unequal_weights_and_knot_partitions_use_original_homogeneous_nets() {
    let a = patch();
    let mut b = a.clone();
    for w in b.weights.iter_mut().flatten() {
        *w *= 2.;
    }
    let r = inspect(&a, &b, 1e-9, limits()).unwrap();
    assert!(r.within_budget, "{r:?}");
    assert!(r.products > 27);
    let b = nurbs_core::surface_edit::insert(&b, nurbs_core::surface::Axis::U, 0.5, 1).unwrap();
    let r = inspect(&a, &b, 1e-8, limits()).unwrap();
    assert!(r.within_budget, "{r:?}");
    assert!(r.cells >= 2);
    for cells in [0, 1] {
        let r = inspect(&a, &b, 1e-8, Limits { cells, ..limits() }).unwrap();
        assert!(r.error_upper.is_none() && !r.within_budget && r.cells <= cells);
    }
    let r = inspect(
        &a,
        &b,
        1e-8,
        Limits {
            products: 1,
            ..limits()
        },
    )
    .unwrap();
    assert!(r.error_upper.is_none() && r.products <= 1);
}
#[test]
fn invalid_weights_and_different_parameter_domains_cannot_supply_a_bound() {
    let a = patch();
    let mut b = a.clone();
    b.knots_u = vec![0., 0., 0., 2., 2., 2.];
    assert_eq!(
        inspect(&a, &b, 1., limits()).unwrap().reason,
        Some("parameter-domain-mismatch")
    );
    b.weights[1][1] = 0.;
    assert!(inspect(&a, &b, 1., limits()).is_err());
}

#[test]
fn degree_elevation_and_weight_rescaling_bound_a_known_translation() {
    let a = patch();
    let mut b = nurbs_core::surface_edit::elevate(&a, nurbs_core::surface::Axis::U, 3).unwrap();
    for point in b.control_points.iter_mut().flatten() {
        point[2] += 0.125;
    }
    for w in b.weights.iter_mut().flatten() {
        *w *= 2.;
    }
    let r = inspect(&a, &b, 0.126, limits()).unwrap();
    assert!(r.within_budget, "{r:?}");
    assert!(r.error_upper.unwrap() >= 0.125 && r.error_upper.unwrap() < 0.126);
    for u in [0., 0.25, 0.75, 1.] {
        for v in [0., 0.25, 0.75, 1.] {
            let delta = b.evaluate(u, v).unwrap().point[2] - a.evaluate(u, v).unwrap().point[2];
            assert!((delta - 0.125).abs() < 1e-12);
        }
    }
}

#[test]
fn glued_charts_cannot_borrow_a_p_matrix_as_a_common_strong_metric() {
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![4., 1., 0.]],
            vec![vec![1., -1., 0.], vec![5., 0., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let p = [[1, 0, 0], [0, 1, 0]];
    assert!(
        nurbs_core::surface_linear_monotonicity::inspect(&s, p, 16)
            .unwrap()
            .certified
    );
    assert!(
        !nurbs_core::surface_linear_monotonicity::inspect_strong_metric(&s, p, [1., 1.], 16)
            .unwrap()
            .certified
    );
}

#[test]
fn affine_parameter_rates_keep_non_dyadic_original_knots_and_refuse_zero_rates() {
    let stations = [0., 0.2, 0.4, 0.6, 0.8, 1.];
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0.2, 0.4, 0.6, 0.8, 1., 1.],
        control_points: (0..2)
            .map(|u| stations.iter().map(|&v| vec![u as f64, v, 0.]).collect())
            .collect(),
        weights: vec![vec![1.; 6]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let original = s.clone();
    let p = [[-1, 0, 0], [0, 1, 0]];
    assert!(
        !nurbs_core::surface_linear_monotonicity::inspect_strong_metric(&s, p, [1., 1.], 32)
            .unwrap()
            .certified
    );
    let proof = nurbs_core::surface_linear_monotonicity::inspect_strong_reparameterized(
        &s,
        p,
        [1., 1.],
        [[-2., -2.], [0.5, 0.5]],
        32,
    )
    .unwrap();
    assert!(proof.certified && proof.cells == 5);
    assert!(
        !nurbs_core::surface_linear_monotonicity::inspect_strong_reparameterized(
            &s,
            p,
            [1., 1.],
            [[-2., -2.], [0.5, 0.5]],
            4
        )
        .unwrap()
        .certified
    );
    assert!(
        nurbs_core::surface_linear_monotonicity::inspect_strong_reparameterized(
            &s,
            p,
            [1., 1.],
            [[0., 1.], [1., 1.]],
            32
        )
        .is_err()
    );
    assert_eq!(s, original);
}

#[test]
fn polynomial_natural_edges_keep_nondyadic_parameters_under_constant_weight_scaling() {
    let stations = [0., 0.2, 0.4, 0.6, 0.8, 1.];
    let knots = vec![0., 0., 0.2, 0.4, 0.6, 0.8, 1., 1.];
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: knots.clone(),
        control_points: (0..2)
            .map(|u| stations.iter().map(|&v| vec![u as f64, v, v * v]).collect())
            .collect(),
        weights: vec![vec![7.; 6], vec![11.; 6]],
        periodic_u: false,
        periodic_v: false,
    };
    let mut edge = nurbs_core::curve::Curve {
        degree: 1,
        knots,
        control_points: s.control_points[0].clone(),
        weights: vec![5.; 6],
        periodic: false,
    };
    let uv = nurbs_core::curve::Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![0., 1.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let proof = nurbs_core::curve_surface_agreement::verify_exact(&edge, &uv, &s, false, 1000000)
        .unwrap()
        .unwrap();
    assert_eq!(proof.outcome, cad_predicates::BezierIdentity::Equal);
    let short =
        nurbs_core::curve_surface_agreement::verify_exact(&edge, &uv, &s, false, 1).unwrap();
    assert!(!short.is_some_and(|p| p.outcome == cad_predicates::BezierIdentity::Equal));
    edge.control_points[2][2] = edge.control_points[2][2].next_up();
    assert!(
        !nurbs_core::curve_surface_agreement::verify_exact(&edge, &uv, &s, false, 1000000)
            .unwrap()
            .is_some_and(|p| p.outcome == cad_predicates::BezierIdentity::Equal)
    );
}
