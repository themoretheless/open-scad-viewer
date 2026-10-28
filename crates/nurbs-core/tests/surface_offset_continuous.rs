use nurbs_core::{
    surface::Surface,
    surface_offset::{
        self, ContinuousOffsetDecision as D, ContinuousOffsetLimits as L,
        ContinuousOffsetStop as Stop,
    },
};
fn saddle(height: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., height]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn translated(s: &Surface, z: f64) -> Surface {
    let mut c = s.clone();
    for row in &mut c.control_points {
        for p in row {
            p[2] += z;
        }
    }
    c
}
#[test]
fn whole_plane_bound_and_private_original_owner_are_independent_of_mutable_reports() {
    for distance in [-0.25, 0.25] {
        let s = saddle(0.);
        let c = translated(&s, distance);
        let saved = (s.clone(), c.clone());
        let mut result =
            surface_offset::inspect_continuous_offset(&s, &c, distance, 1e-9, L::default())
                .unwrap();
        assert_eq!(
            result.report.decision,
            D::WithinTolerance,
            "{:?}",
            result.report
        );
        assert!(result.report.error_upper.unwrap() <= 1e-9);
        assert!(
            result.report.source_normals_proven
                && result.report.ideal_local_fold_free
                && result.report.candidate_locally_regular
        );
        result.report.error_upper = Some(-1.);
        let owner = result.into_qualified().unwrap();
        assert!(owner.report().error_upper.unwrap() >= 0.);
        owner.reverify().unwrap();
        assert_eq!((s, c), saved);
    }
}
#[test]
fn bump_between_original_sample_sites_is_found_by_the_complete_knot_cover() {
    let s = saddle(0.);
    let c = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 0.5, 0.5001, 0.5002, 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [0., 0.5, 0.5001, 0.5002, 1.]
            .into_iter()
            .map(|u| {
                [0., 1.]
                    .into_iter()
                    .map(|v| vec![u, v, if u == 0.5001 { 0.6 } else { 0.5 }])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 5],
        periodic_u: false,
        periodic_v: false,
    };
    for u in [0., 0.25, 0.5, 0.75, 1.] {
        for v in [0., 0.25, 0.5, 0.75, 1.] {
            assert!((c.evaluate(u, v).unwrap().point[2] - 0.5).abs() < 1e-12);
        }
    }
    assert!((c.evaluate(0.5001, 0.5).unwrap().point[2] - 0.5) > 0.09);
    let result =
        surface_offset::inspect_continuous_offset(&s, &c, 0.5, 0.01, L::default()).unwrap();
    assert_eq!(
        result.report.decision,
        D::ExceedsTolerance,
        "{:?}",
        result.report
    );
    assert!(result.report.error_lower > 0.01);
    assert!(result.report.error_upper.is_none());
    assert_eq!(result.report.stop, Stop::Witness);
    assert!(result.report.witness_uv.is_some());
    assert!(result.into_qualified().is_none());
}
#[test]
fn adaptive_original_rational_nonunit_charts_bound_the_actual_normal_offset() {
    let mut s = saddle(0.2);
    s.weights = vec![vec![1., 0.75], vec![1.25, 1.]];
    s.knots_u = vec![2., 2., 7., 7.];
    s.knots_v = vec![-3., -3., 1., 1.];
    let c = translated(&s, 0.1);
    let result =
        surface_offset::inspect_continuous_offset(&s, &c, 0.1, 0.04, L::default()).unwrap();
    assert_eq!(
        result.report.decision,
        D::WithinTolerance,
        "{:?}",
        result.report
    );
    let upper = result.report.error_upper.unwrap();
    assert!(upper <= 0.04);
    for i in 0..=10 {
        for j in 0..=10 {
            let (u, v) = (2. + 5. * i as f64 / 10., -3. + 4. * j as f64 / 10.);
            let e = s.evaluate(u, v).unwrap();
            let p = c.evaluate(u, v).unwrap().point;
            let n = e.unit_normal().unwrap();
            let error = (0..3)
                .map(|k| (p[k] - e.point[k] - 0.1 * n[k]).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(error <= upper + 1e-13, "{error} > {upper}");
        }
    }
    result.into_qualified().unwrap().reverify().unwrap();
}
#[test]
fn exhausted_or_singular_cells_never_publish_a_partial_upper_bound_or_owner() {
    let s = saddle(0.5);
    let c = translated(&s, 0.1);
    for limits in [
        L {
            max_cells: 0,
            max_depth: 24,
        },
        L {
            max_cells: 1,
            max_depth: 24,
        },
        L {
            max_cells: 100,
            max_depth: 0,
        },
    ] {
        let r = surface_offset::inspect_continuous_offset(&s, &c, 0.1, 0.04, limits).unwrap();
        assert_eq!(r.report.decision, D::Unresolved);
        assert!(r.report.error_upper.is_none());
        assert!(r.report.cells <= limits.max_cells);
        assert!(r.into_qualified().is_none());
    }
    let mut singular = saddle(0.);
    for row in &mut singular.control_points {
        for p in row {
            p[1] = 0.;
        }
    }
    let r = surface_offset::inspect_continuous_offset(
        &singular,
        &singular,
        0.1,
        0.01,
        L {
            max_cells: 7,
            max_depth: 3,
        },
    )
    .unwrap();
    assert_eq!(r.report.decision, D::Unresolved);
    assert!(r.into_qualified().is_none());
    let identity =
        surface_offset::inspect_continuous_offset(&singular, &singular, 0., 1e-12, L::default())
            .unwrap();
    assert_eq!(identity.report.error_upper, Some(0.));
    assert!(!identity.report.source_normals_proven);
}
#[test]
fn invalid_domains_and_saved_sampled_flags_grant_no_geometry_authority() {
    let s = saddle(0.);
    let mut proposal = surface_offset::offset(&s, 0.25, 0.01).unwrap();
    for row in &mut proposal.surface.control_points {
        for p in row {
            p[2] += 0.5;
        }
    }
    proposal.exact = true;
    proposal.max_deviation = 0.;
    let r = proposal
        .inspect_continuous(&s, 0.25, 0.01, L::default())
        .unwrap();
    assert_eq!(r.report.decision, D::ExceedsTolerance);
    assert!(r.into_qualified().is_none());
    let mut c = s.clone();
    c.knots_u = vec![2., 2., 3., 3.];
    assert!(surface_offset::inspect_continuous_offset(&s, &c, 0.1, 0.01, L::default()).is_err());
    for tolerance in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(
            surface_offset::inspect_continuous_offset(&s, &s, 0.1, tolerance, L::default())
                .is_err()
        );
    }
}

#[test]
fn signed_cylinder_offsets_separate_error_accuracy_from_focal_crossing_and_orientation() {
    let s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [[1., 0.], [1., 1.], [0., 1.]]
            .into_iter()
            .map(|xy| {
                [0., 1.]
                    .into_iter()
                    .map(|z| vec![xy[0], xy[1], z])
                    .collect()
            })
            .collect(),
        weights: vec![
            vec![1.; 2],
            vec![std::f64::consts::FRAC_1_SQRT_2; 2],
            vec![1.; 2],
        ],
        periodic_u: false,
        periodic_v: false,
    };
    for distance in [0.25, -2.] {
        let mut c = s.clone();
        for row in &mut c.control_points {
            for p in row {
                p[0] *= 1. + distance;
                p[1] *= 1. + distance;
            }
        }
        let r = surface_offset::inspect_continuous_offset(&s, &c, distance, 0.01, L::default())
            .unwrap();
        assert_eq!(r.report.decision, D::WithinTolerance, "{:?}", r.report);
        assert!(r.report.error_upper.unwrap() <= 0.01 && r.report.source_normals_proven);
        assert_eq!(
            r.report.ideal_local_fold_free,
            distance > 0.,
            "{:?}",
            r.report
        );
        assert_eq!(
            r.report.candidate_locally_regular,
            distance > 0.,
            "{:?}",
            r.report
        );
        for u in [0., 0.125, 0.5, 0.875, 1.] {
            let e = s.evaluate(u, 0.5).unwrap();
            let p = c.evaluate(u, 0.5).unwrap().point;
            let n = e.unit_normal().unwrap();
            let error = (0..3)
                .map(|k| (p[k] - e.point[k] - distance * n[k]).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(error < 1e-12);
        }
        r.into_qualified().unwrap().reverify().unwrap();
    }
}

#[test]
#[cfg(feature = "transport")]
fn json_boundary_recomputes_coefficients_and_preserves_unresolved_state() {
    use value_codec::{Value, json};
    let source = saddle(0.);
    let mut request = json!({"op":"surface_offset_continuous_deviation", "source":source,
        "candidate":translated(&source,0.25), "distance":0.25,"tolerance":1e-8,
        "maxCells":1000,"maxDepth":20,"exact":true,"errorUpper":0.0});
    let run = |v: &Value| -> Value {
        value_codec::from_str(&nurbs_core::execute(&v.to_string())).unwrap()
    };
    let accepted = run(&request);
    assert_eq!(accepted["ok"], true);
    assert_eq!(accepted["value"]["decision"], "within_tolerance");
    assert_eq!(accepted["value"]["continuousBound"], true);
    assert_eq!(accepted["value"]["topologyAuthority"], false);
    assert_eq!(accepted["value"]["globalEmbeddingCertified"], false);
    request["maxCells"] = json!(0);
    let limited = run(&request);
    assert_eq!(limited["value"]["decision"], "unresolved");
    assert!(limited["value"]["errorUpper"].is_null());
    assert_eq!(limited["value"]["continuousBound"], false);
    request["maxCells"] = json!(1000);
    request["candidate"] = json!(translated(&source, 0.5));
    assert_eq!(run(&request)["value"]["decision"], "exceeds_tolerance");
    request["maxDepth"] = json!(41);
    assert_eq!(run(&request)["ok"], false);
}
