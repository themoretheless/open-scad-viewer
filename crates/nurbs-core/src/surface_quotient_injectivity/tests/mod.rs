use super::*;
fn triangular() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.]; 2],
            vec![vec![0.5, 0., 0.]; 2],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
fn polar_surface() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![2., 2., 2., 5., 5., 5.],
        knots_v: vec![-4., -4., 8., 8.],
        control_points: vec![
            vec![vec![3., -7., 5.]; 2],
            vec![vec![3.5, -7., 5.], vec![3.5, -6.5, 5.]],
            vec![vec![4., -7., 5.], vec![4., -6., 5.]],
        ],
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn polar_blowup_preserves_original_nonunit_charts_and_both_pole_ends() {
    let mut s = polar_surface();
    for end in [0, 1] {
        if end == 1 {
            s.control_points.reverse();
            s.weights.reverse();
        }
        let original = s.clone();
        let r = certify_polar_source_frame(&s, end, 8, 64).unwrap();
        assert!(r.proven, "{r:?}");
        assert_eq!(r.cells, 64);
        assert!(r.dominance_margin_lower.unwrap() > 0.);
        assert_eq!(s, original);
    }
}
#[test]
fn polar_blowup_refuses_fold_repeated_direction_nonpole_and_work_limit() {
    let mut s = polar_surface();
    s.control_points[1][0][0] = 2.;
    s.control_points[1][1][0] = 2.;
    assert!(!certify_polar_source_frame(&s, 0, 8, 64).unwrap().proven);
    let mut s = polar_surface();
    for row in &mut s.control_points {
        row[1] = row[0].clone();
    }
    assert!(!certify_polar_source_frame(&s, 0, 8, 64).unwrap().proven);
    let mut s = polar_surface();
    s.control_points[0][1][0] += 1e-12;
    assert!(!certify_polar_source_frame(&s, 0, 8, 64).unwrap().proven);
    let r = certify_polar_source_frame(&polar_surface(), 0, 8, 63).unwrap();
    assert!(!r.proven && r.cells == 0 && r.derivative_bounds.is_none());
    assert!(certify_polar_source_frame(&polar_surface(), 2, 8, 64).is_err());
}
fn ruled_surface() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![2., 2., 5., 5.],
        knots_v: vec![-4., -4., -4., 8., 8., 8.],
        control_points: vec![
            vec![vec![3., -7., 5.]; 3],
            vec![vec![4., -7., 6.], vec![4., -6., 6.], vec![3., -6., 6.]],
        ],
        weights: vec![vec![1., 0.25, 2.], vec![0.3, 1., 0.75]],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn ruled_ray_ratio_covers_nonunit_domains_and_independent_weight_rows() {
    let mut s = ruled_surface();
    for end in [0, 1] {
        if end == 1 {
            s.control_points.reverse();
            s.weights.reverse();
        }
        let original = s.clone();
        let r = certify_ruled_source_frame(&s, end, 16, 16).unwrap();
        assert!(r.proven, "{r:?}");
        assert_eq!(r.cells, 16);
        assert!(r.direction_denominator_lower.unwrap() > 0.);
        assert!(r.angular_derivative_numerator.unwrap()[0] > 0.);
        assert_eq!(s, original);
    }
}
#[test]
fn ruled_fold_repeated_ray_and_nonconstant_pole_remain_unproven() {
    let mut folded = ruled_surface();
    folded.control_points[1][1] = vec![8., -12., 6.];
    assert!(
        !certify_ruled_source_frame(&folded, 0, 16, 16)
            .unwrap()
            .proven
    );
    let mut repeated = ruled_surface();
    repeated.control_points[1][2] = repeated.control_points[1][0].clone();
    assert!(
        !certify_ruled_source_frame(&repeated, 0, 16, 16)
            .unwrap()
            .proven
    );
    let mut broken = ruled_surface();
    broken.control_points[0][1][0] += 1e-12;
    let r = certify_ruled_source_frame(&broken, 0, 16, 16).unwrap();
    assert!(!r.proven && r.reason == "boundary-not-collapsed");
}
#[test]
fn ruled_budget_exhaustion_has_no_direction_certificate() {
    let r = certify_ruled_source_frame(&ruled_surface(), 0, 16, 15).unwrap();
    assert!(!r.proven && r.cells == 0 && r.angular_derivative_numerator.is_none());
    assert!(certify_ruled_source_frame(&ruled_surface(), 0, 0, 16).is_err());
}
#[test]
fn collapsed_triangle_is_injective_only_after_identifying_its_pole_boundary() {
    let mut s = triangular();
    let projection = [[1., 0., 0.], [0., 1., 0.]];
    let before = format!("{s:?}");
    let r = certify(&s, 0, projection, 4, 16).unwrap();
    assert!(r.proven, "{r:?}");
    assert_eq!(r.cells, 16);
    assert!(
        !crate::surface_injectivity::certify(&s, 1000)
            .unwrap()
            .proven
    );
    assert_eq!(format!("{s:?}"), before);
    assert!(!certify(&s, 0, projection, 4, 15).unwrap().proven);
    assert!(certify(&s, 0, projection, 0, 16).is_err());
    assert!(certify(&s, 2, projection, 4, 16).is_err());
    s.control_points.reverse();
    s.weights.reverse();
    assert!(certify(&s, 1, projection, 4, 16).unwrap().proven);
    s.control_points.reverse();
    s.weights = vec![vec![1., 3.], vec![2., 6.], vec![4., 12.]];
    assert!(certify(&s, 0, projection, 8, 64).unwrap().proven);
    s.control_points[1][1][1] = 1e-12;
    let r = certify(&s, 0, projection, 8, 64).unwrap();
    assert!(!r.proven);
    assert_eq!(r.reason, "weighted-order-not-proven");
}
#[test]
fn folding_the_transverse_parameter_is_not_hidden_by_a_collapsed_boundary() {
    let mut s = triangular();
    s.degree_v = 2;
    s.knots_v = vec![0., 0., 0., 1., 1., 1.];
    s.control_points = vec![
        vec![vec![0., 0., 0.]; 3],
        vec![vec![0.5, 0., 0.]; 3],
        vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![1., 0., 0.]],
    ];
    s.weights = vec![vec![1.; 3]; 3];
    assert!(
        !certify(&s, 0, [[1., 0., 0.], [0., 1., 0.]], 8, 64)
            .unwrap()
            .proven
    );
}
