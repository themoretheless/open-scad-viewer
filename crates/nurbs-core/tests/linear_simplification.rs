use nurbs_core::{curve::Curve, linear_simplification::simplify};
fn poly(points: &[[f64; 2]]) -> Curve {
    Curve::from_polyline(points.iter().map(|p| p.to_vec()).collect()).unwrap()
}
fn same_samples(a: &Curve, b: &Curve) {
    assert_eq!(a.domain(), b.domain());
    let [lo, hi] = a.domain();
    for i in 0..=128 {
        let t = lo + (hi - lo) * i as f64 / 128.;
        let p = a.evaluate(t).unwrap().point;
        let q = b.evaluate(t).unwrap().point;
        for (x, y) in p.iter().zip(q) {
            assert!((x - y).abs() < 1e-11, "changed point at {t}: {p:?}");
        }
    }
}

#[test]
fn removes_linear_controls_by_copying_original_values_and_parameters() {
    let mut source = Curve::from_polyline(
        (0..6)
            .map(|i| vec![i as f64, 2. * i as f64, -3. * i as f64])
            .collect(),
    )
    .unwrap();
    source.weights.fill(3.);
    let original = (
        source.control_points.clone(),
        source.weights.clone(),
        source.knots.clone(),
    );
    let report = simplify(&source, 1_000_000).unwrap();
    assert!(report.complete);
    assert_eq!(report.retained_source_controls, vec![0, 5]);
    assert_eq!(report.removed_source_controls, vec![1, 2, 3, 4]);
    assert_eq!(report.curve.weights, vec![3., 3.]);
    assert_eq!(report.curve.knots, vec![0., 0., 5., 5.]);
    assert_eq!(
        report.curve.control_points,
        vec![
            source.control_points[0].clone(),
            source.control_points[5].clone()
        ]
    );
    same_samples(&source, &report.curve);
    assert_eq!(
        (source.control_points, source.weights, source.knots),
        original
    );
}

#[test]
fn preserves_real_corners_while_reducing_each_straight_run() {
    let source = poly(&[[0., 0.], [1., 0.], [2., 0.], [2., 1.], [2., 2.]]);
    let report = simplify(&source, 1_000_000).unwrap();
    assert!(report.complete);
    assert_eq!(report.retained_source_controls, vec![0, 2, 4]);
    same_samples(&source, &report.curve);
}

#[test]
fn world_collinearity_does_not_authorize_a_change_of_traversal_speed() {
    let source = poly(&[[0., 0.], [1., 0.], [3., 0.]]);
    let report = simplify(&source, 1_000_000).unwrap();
    assert!(report.complete);
    assert!(report.removed_source_controls.is_empty());
    assert_eq!(report.curve.knots, source.knots);
    assert_eq!(report.curve.control_points, source.control_points);
    assert_eq!(report.curve.evaluate(1.).unwrap().point, vec![1., 0.]);
}

#[test]
fn one_ulp_of_nominal_geometry_is_preserved() {
    let next = f64::from_bits(1_f64.to_bits() + 1);
    let source = poly(&[[0., 0.], [1., next], [2., 2.]]);
    let report = simplify(&source, 1_000_000).unwrap();
    assert!(report.complete);
    assert!(report.removed_source_controls.is_empty());
    assert_eq!(report.curve.control_points, source.control_points);
}

#[test]
fn closed_nonperiodic_profiles_keep_the_authored_seam_and_domain() {
    let source = poly(&[
        [0., 0.],
        [1., 0.],
        [2., 0.],
        [2., 1.],
        [2., 2.],
        [1., 2.],
        [0., 2.],
        [0., 1.],
        [0., 0.],
    ]);
    let report = simplify(&source, 1_000_000).unwrap();
    assert!(report.complete);
    assert_eq!(report.retained_source_controls, vec![0, 2, 4, 6, 8]);
    assert_eq!(
        report.curve.control_points.first(),
        source.control_points.first()
    );
    assert_eq!(
        report.curve.control_points.last(),
        source.control_points.last()
    );
    same_samples(&source, &report.curve);
}

#[test]
fn nonuniform_sites_are_checked_in_their_original_parameters() {
    let mut source = poly(&[[-3., 6.], [-1., 2.], [2., -4.], [5., -10.]]);
    source.knots = vec![-3., -3., -1., 2., 5., 5.];
    source.validate().unwrap();
    let report = simplify(&source, 1_000_000).unwrap();
    assert!(report.complete);
    assert_eq!(report.retained_source_controls, vec![0, 3]);
    assert_eq!(report.curve.knots, vec![-3., -3., 5., 5.]);
    same_samples(&source, &report.curve);
}

#[test]
fn incomplete_search_still_preserves_every_returned_segment() {
    let source = Curve::from_polyline(
        (0..20)
            .map(|i| vec![i as f64, 2. * i as f64, -i as f64])
            .collect(),
    )
    .unwrap();
    let full = simplify(&source, 1_000_000).unwrap();
    assert!(full.complete && full.work_used > 1);
    let budget = full.work_used - 1;
    let partial = simplify(&source, budget).unwrap();
    assert!(!partial.complete);
    assert_eq!(partial.reason, Some(cad_predicates::Reason::ResourceLimit));
    assert!(partial.work_used <= budget);
    assert!(!partial.removed_source_controls.is_empty());
    same_samples(&source, &partial.curve);
    let refused = simplify(&source, 1).unwrap();
    assert!(!refused.complete);
    assert!(refused.removed_source_controls.is_empty());
    assert_eq!(refused.curve.control_points, source.control_points);
}

#[test]
fn unsupported_storage_and_invalid_inputs_are_rejected() {
    let source = poly(&[[0., 0.], [1., 1.], [2., 2.]]);
    assert!(simplify(&source, 0).is_err());
    assert!(simplify(&source, 1_000_001).is_err());
    assert!(simplify(&source.elevate(2).unwrap(), 1_000_000).is_err());
    let mut bad = source.clone();
    bad.weights[1] = 2.;
    assert!(simplify(&bad, 1_000_000).is_err());
    let mut bad = source.clone();
    bad.periodic = true;
    assert!(simplify(&bad, 1_000_000).is_err());
    let mut bad = source.clone();
    bad.knots = vec![-1., 0., 1., 2., 3.];
    bad.validate().unwrap();
    assert!(simplify(&bad, 1_000_000).is_err());
    let mut bad = source.clone();
    bad.control_points[1][0] = f64::NAN;
    assert!(simplify(&bad, 1_000_000).is_err());
    let mut bad = source.clone();
    bad.control_points[1][0] = 1e10;
    assert!(simplify(&bad, 1_000_000).is_err());
    let mut discontinuous = poly(&[[0., 0.], [1., 0.], [1., 1.], [2., 1.]]);
    discontinuous.knots = vec![0., 0., 1., 1., 2., 2.];
    assert!(discontinuous.validate().is_err());
    assert!(simplify(&discontinuous, 1_000_000).is_err());
}

#[test]
fn degenerate_constant_controls_and_large_translations_are_safe() {
    let constant = Curve::from_polyline(vec![vec![7., -9., 3.]; 8]).unwrap();
    let report = simplify(&constant, 1_000_000).unwrap();
    assert!(report.complete);
    assert_eq!(report.curve.control_points.len(), 2);
    same_samples(&constant, &report.curve);
    let shift = (1_u64 << 29) as f64;
    let source = poly(&[
        [shift, shift],
        [shift + 1., shift + 2.],
        [shift + 2., shift + 4.],
    ]);
    let report = simplify(&source, 1_000_000).unwrap();
    assert!(report.complete);
    assert_eq!(report.removed_source_controls, vec![1]);
    assert_eq!(
        report.curve.control_points,
        vec![
            source.control_points[0].clone(),
            source.control_points[2].clone()
        ]
    );
}
