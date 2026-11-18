use nurbs_core::{
    curve::Curve,
    intersection::certified_curve_surface::{all_in_window, first_in_window},
    surface::Surface,
};
fn source(tangent: bool) -> (Curve, Surface) {
    let (a, b) = (7. / 16., if tangent { 7. / 16. } else { 13. / 16. });
    let c0 = a * b;
    let c1 = -(a + b);
    (
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![
                vec![0., 0.5, c0],
                vec![0.5, 0.5, c0 + c1 / 2.],
                vec![1., 0.5, c0 + c1 + 1.],
            ],
            weights: vec![1.; 3],
            periodic: false,
        },
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        },
    )
}
#[test]
fn roots_on_artificial_split_planes_are_enumerated_once_with_complete_coverage() {
    let (c, s) = source(false);
    let before = (c.clone(), s.clone());
    let r = all_in_window(&c, &s, [[0., 1.]; 3], 10000).unwrap();
    assert!(r.complete, "{r:?}");
    assert!(r.unresolved.is_empty());
    assert_eq!(r.roots.len(), 2, "{r:?}");
    for t in [7. / 16., 13. / 16.] {
        let roots: Vec<_> = r
            .roots
            .iter()
            .filter(|x| x.parameter_box[0][0] <= t && t <= x.parameter_box[0][1])
            .collect();
        assert_eq!(roots.len(), 1);
        let root = roots[0];
        for (axis, x) in [t, t, 0.5].into_iter().enumerate() {
            assert!(root.parameter_box[axis][0] <= x && x <= root.parameter_box[axis][1]);
            assert!(root.existence_domain[axis][0] < x && x < root.existence_domain[axis][1]);
        }
        assert_eq!(root.crossing_sign, if t < 0.5 { -1 } else { 1 });
    }
    assert!(r.cells <= 10000);
    assert_eq!(before, (c, s));
}
#[test]
fn query_boundaries_tangencies_and_exhausted_budgets_stay_unresolved() {
    let (c, s) = source(false);
    for budget in [1, 2, 16] {
        let r = all_in_window(&c, &s, [[0., 1.]; 3], budget).unwrap();
        assert!(r.cells <= budget);
        assert!(!r.complete && !r.unresolved.is_empty());
    }
    let r = all_in_window(&c, &s, [[7. / 16., 1.], [0., 1.], [0., 1.]], 10000).unwrap();
    assert!(!r.complete && !r.unresolved.is_empty(), "{r:?}");
    let (c, s) = source(true);
    let r = all_in_window(&c, &s, [[0., 1.]; 3], 10000).unwrap();
    assert!(r.roots.is_empty());
    assert!(!r.complete && !r.unresolved.is_empty());
}
#[test]
fn first_root_search_keeps_the_unprocessed_domain_explicit() {
    let (c, s) = source(false);
    let r = first_in_window(&c, &s, [[0., 1.]; 3], 10000).unwrap();
    assert_eq!(r.roots.len(), 1);
    assert!(!r.complete && !r.unresolved.is_empty());
}

#[test]
fn generic_root_api_preserves_natural_existence_domains_and_rational_sources() {
    use num_rational::BigRational as Q;
    use num_traits::FromPrimitive;
    use nurbs_core::intersection::certified_curve_surface::intersections;
    let q = |x| Q::from_f64(x).unwrap();
    for a in [3. / 8., 7. / 16.] {
        for rational in [false, true] {
            let b = 13. / 16.;
            let ab = a * b;
            let sum = a + b;
            let l = if rational { 15. } else { 1. };
            let (mut c, mut s) = source(false);
            if rational {
                c.degree = 3;
                c.knots = vec![2., 2., 2., 2., 6., 6., 6., 6.];
                c.weights = vec![3., 4., 5., 6.];
                c.control_points = vec![
                    vec![0., l / 2., l * ab],
                    vec![l / 4., l / 2., l * (4. * ab - sum) / 4.],
                    vec![3. * l / 5., l / 2., l * (5. * ab + 1. - 3. * sum) / 5.],
                    vec![l, l / 2., l * (ab - sum + 1.)],
                ];
            } else {
                c.knots = vec![2., 2., 2., 6., 6., 6.];
                c.control_points = vec![
                    vec![0., 0.5, ab],
                    vec![0.5, 0.5, ab - sum / 2.],
                    vec![1., 0.5, ab - sum + 1.],
                ];
            }
            s.knots_u = vec![-3., -3., 5., 5.];
            s.knots_v = vec![10., 10., 14., 14.];
            for row in &mut s.control_points {
                for p in row {
                    p[0] *= l;
                    p[1] *= l;
                }
            }
            if rational {
                s.weights = vec![vec![1., 3.], vec![2., 6.]];
            }
            let r = intersections(&c, &s, 10000).unwrap();
            assert!(r.complete, "{r:?}");
            assert!(r.unresolved.is_empty());
            assert_eq!(r.roots.len(), 2);
            assert!(r.cells <= 10000);
            for (root, t) in r.roots.iter().zip([a, b]) {
                let u = if rational {
                    q(t) / (q(2.) - q(t))
                } else {
                    q(t)
                };
                let expected = [
                    q(2.) + q(4.) * q(t),
                    q(-3.) + q(8.) * u,
                    if rational { q(11.) } else { q(12.) },
                ];
                let domain = root.existence_domain.unwrap();
                for (k, x) in expected.into_iter().enumerate() {
                    assert!(
                        q(root.parameter_box[k][0]) <= x && x <= q(root.parameter_box[k][1]),
                        "{root:?}"
                    );
                    assert!(q(domain[k][0]) < x && x < q(domain[k][1]));
                }
                assert_eq!(root.crossing_sign, if t == a { -1 } else { 1 });
            }
        }
    }
}

#[test]
fn different_parameter_roots_are_not_merged_by_equal_world_positions() {
    let (mut c, s) = source(false);
    for p in &mut c.control_points {
        p[0] = 0.5;
    }
    let r = all_in_window(&c, &s, [[0., 1.]; 3], 10000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.roots.len(), 2);
    let mut ranges: Vec<_> = r.roots.iter().map(|r| r.parameter_box[0]).collect();
    ranges.sort_by(|a, b| a[0].total_cmp(&b[0]));
    assert!(ranges[0][1] < ranges[1][0]);
    for root in r.roots {
        for (k, x) in [0.5, 0.5, 0.].into_iter().enumerate() {
            assert!(root.position_bounds[k][0] <= x && x <= root.position_bounds[k][1]);
        }
    }
}
#[test]
fn different_surface_preimages_remain_distinct_at_the_same_curve_parameter() {
    let (_, mut s) = source(false);
    s.degree_u = 2;
    s.knots_u = vec![0., 0., 0., 1., 1., 1.];
    s.control_points = [0.25, -0.25, 0.25]
        .into_iter()
        .map(|x| vec![vec![x, 0., 0.], vec![x, 1., 0.]])
        .collect();
    s.weights = vec![vec![1.; 2]; 3];
    let c = Curve::from_polyline(vec![vec![0.0625, 0.5, -1.], vec![0.0625, 0.5, 1.]]).unwrap();
    let r =
        nurbs_core::intersection::certified_curve_surface::intersections(&c, &s, 10000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.roots.len(), 2);
    for (root, u) in r.roots.iter().zip([0.25, 0.75]) {
        for (k, x) in [0.5, u, 0.5].into_iter().enumerate() {
            assert!(root.parameter_box[k][0] <= x && x <= root.parameter_box[k][1]);
        }
        assert_eq!(root.crossing_sign, if u < 0.5 { -1 } else { 1 });
    }
}

#[test]
fn repair_budget_exhaustion_never_drops_pending_cells_or_claims_partial_completeness() {
    let (c, s) = source(false);
    let full = all_in_window(&c, &s, [[0., 1.]; 3], 10000).unwrap();
    assert!(full.complete);
    for budget in [
        400,
        407,
        408,
        409,
        410,
        full.cells.saturating_sub(1),
        full.cells,
    ] {
        let r = all_in_window(&c, &s, [[0., 1.]; 3], budget).unwrap();
        assert!(r.cells <= budget);
        assert_eq!(r.complete, r.unresolved.is_empty());
        if r.complete {
            assert_eq!(r.roots.len(), 2);
        } else {
            assert!(!r.unresolved.is_empty());
        }
    }
}
#[test]
fn repair_does_not_import_a_root_from_outside_the_requested_window() {
    let (c, s) = source(false);
    let window = [[0., 0.6], [0., 1.], [0., 1.]];
    let r = all_in_window(&c, &s, window, 10000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.roots.len(), 1);
    let root = &r.roots[0];
    assert!(root.parameter_box[0][0] <= 7. / 16. && 7. / 16. <= root.parameter_box[0][1]);
    for k in 0..3 {
        assert!(
            window[k][0] <= root.existence_domain[k][0]
                && root.existence_domain[k][1] <= window[k][1]
        );
    }
}
