use num_rational::BigRational as Q;
use nurbs_core::ss_intersection as ss;
#[allow(dead_code)]
#[path = "support/ss_original_ideal.rs"]
mod fixture;
fn q(x: f64) -> Q {
    Q::from_float(x).unwrap()
}
fn node() -> ss::QualifiedSsRootNode {
    let mut saddle = fixture::tangent(false);
    for i in 0..3 {
        for j in 0..3 {
            saddle.control_points[i][j][2] = (i as f64 / 2. - 0.5) * (j as f64 / 2. - 0.5);
        }
    }
    let source = ss::FiberSources::new(&fixture::plane(0.), &saddle).unwrap();
    let original =
        ss::qualify_original_ss_ideal(&source, [1, 1, 2, 2], Default::default()).unwrap();
    let basis = ss::qualify_ss_groebner_basis(&original, Default::default()).unwrap();
    let reduced = ss::qualify_ss_deflated_system(&basis, Default::default()).unwrap();
    ss::qualify_ss_root_node(
        &reduced,
        std::array::from_fn(|_| q(0.5)),
        Default::default(),
    )
    .unwrap()
}
#[test]
fn strict_transforms_separate_both_original_crossing_directions() {
    let node = node();
    node.reverify().unwrap();
    for radial in [2, 3] {
        let chart =
            ss::qualify_ss_blowup_chart(&node, q(0.125), radial, Default::default()).unwrap();
        chart.reverify().unwrap();
        let mut d = [[0.45, 0.55]; 4];
        d[radial] = [0.35, 0.65];
        d[radial - 2] = [0.7, 0.8];
        let axis = if radial == 2 {
            ss::FiberAxis::SecondU
        } else {
            ss::FiberAxis::SecondV
        };
        let arc = ss::qualify_ss_blowup_root_arc(&chart, d, axis, Default::default()).unwrap();
        arc.reverify().unwrap();
        assert_eq!(arc.report().status, ss::FiberStatus::UniformUniqueRoot);
        for step in [-0.0625, 0.0625] {
            let mut p = std::array::from_fn(|_| q(0.5));
            p[radial] += q(step);
            p[radial - 2] += q(step);
            let lift = chart.lift_dominant(&p).unwrap();
            for k in 0..4 {
                assert_eq!(chart.original_coordinate_map()[k].evaluate(&lift), p[k]);
            }
            for f in chart.equations() {
                assert_eq!(f.evaluate(&lift), q(0.));
            }
        }
    }
}
#[test]
fn atlas_maps_cover_arbitrary_original_directions_with_exact_inverse() {
    let node = node();
    let delta = [0.03125, -0.0625, 0.015625, -0.046875];
    let point = std::array::from_fn(|k| q(0.5) + q(delta[k]));
    let dominant = ss::qualify_ss_blowup_chart(&node, q(0.125), 1, Default::default()).unwrap();
    let lift = dominant.lift_dominant(&point).unwrap();
    for k in 0..4 {
        assert!(q(0.) <= lift[k] && lift[k] <= q(1.));
        assert_eq!(
            dominant.original_coordinate_map()[k].evaluate(&lift),
            point[k]
        );
    }
    let wrong = ss::qualify_ss_blowup_chart(&node, q(0.125), 0, Default::default()).unwrap();
    assert!(wrong.lift_dominant(&point).is_err());
    assert!(dominant.lift_dominant(node.center()).is_err());
    assert!(
        ss::qualify_ss_root_node(
            node.source(),
            std::array::from_fn(|_| q(0.6)),
            Default::default()
        )
        .is_err()
    );
    assert!(ss::qualify_ss_blowup_chart(&node, q(0.), 0, Default::default()).is_err());
    assert!(
        ss::qualify_ss_blowup_chart(
            &node,
            q(0.125),
            0,
            ss::ExactSsGroebnerLimits {
                algebra: ss::ExactSsAlgebraLimits {
                    max_work: 1,
                    ..Default::default()
                },
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn original_world_root_curves_share_the_exact_crossing_node() {
    use std::sync::Arc;
    let n = node();
    let mut curves = Vec::new();
    for radial in [2, 3] {
        let chart = ss::qualify_ss_blowup_chart(&n, q(0.125), radial, Default::default()).unwrap();
        let mut d = [[0.45, 0.55]; 4];
        d[radial] = [0.35, 0.65];
        d[radial - 2] = [0.7, 0.8];
        let axis = if radial == 2 {
            ss::FiberAxis::SecondU
        } else {
            ss::FiberAxis::SecondV
        };
        let arc = ss::qualify_ss_blowup_root_arc(&chart, d, axis, Default::default()).unwrap();
        let c = Arc::new(ss::OriginalSsRootCurve::from_blowup(arc));
        let b = c.enclose([0.5; 2], Default::default()).unwrap();
        for k in 0..3 {
            let expected = if k < 2 { 0.5 } else { 0. };
            assert!(b.world[k][0] <= expected && expected <= b.world[k][1]);
        }
        let inside = c.enclose([0.6; 2], Default::default()).unwrap();
        let expected = if radial == 2 {
            [0.525, 0.5, 0.]
        } else {
            [0.5, 0.525, 0.]
        };
        for k in 0..3 {
            assert!(
                inside.world[k][0] <= expected[k] && expected[k] <= inside.world[k][1],
                "{inside:?}"
            );
        }
        curves.push(c);
    }
    let point = Arc::new(ss::OriginalSsRootPoint::new(curves[0].clone(), 0.5).unwrap());
    let joined =
        ss::bind_original_ss_port(point, curves[1].clone(), [0; 4], Default::default()).unwrap();
    assert_eq!(joined.kind(), ss::OriginalSsPortKind::OriginalSourceNode);
    joined.reverify().unwrap();
    // The algebraic isolator is not a point and need not refine to binary 0.5.
    let clock=Arc::new(ss::QualifiedSsAlgebraicClock::from_binary(
        &[0.25,-1.,1.], [0.49,0.52], Default::default()).unwrap());
    let algebraic=ss::OriginalSsAlgebraicPoint::new(curves[0].clone(),clock).unwrap();
    let policy=ss::SsAlgebraicPointLimits {bisections:0,..Default::default()};
    let selected=Arc::new(ss::OriginalSsRootPoint::from_algebraic(algebraic.clone(),policy).unwrap());
    assert_ne!(selected.clock_bounds().unwrap()[0],selected.clock_bounds().unwrap()[1]);
    assert!(selected.equals_binary_parameter(0.5).unwrap());
    assert!(!selected.equals_binary_parameter(0.5000000000000001).unwrap());
    let joined=ss::bind_original_ss_port(selected,curves[1].clone(),[0;4],Default::default()).unwrap();
    assert_eq!(joined.kind(),ss::OriginalSsPortKind::OriginalSourceNode);
    joined.reverify().unwrap();
    let exhausted=Arc::new(ss::OriginalSsRootPoint::from_algebraic(algebraic,
        ss::SsAlgebraicPointLimits {clock:ss::SsAlgebraicClockLimits {
            max_operations:7,..Default::default()
        },..policy}).unwrap());
    assert!(ss::bind_original_ss_port(exhausted,curves[1].clone(),[0;4],Default::default()).is_err());

    let away = Arc::new(ss::OriginalSsRootPoint::new(curves[0].clone(), 0.6).unwrap());
    assert!(
        ss::bind_original_ss_port(away, curves[1].clone(), [0; 4], Default::default()).is_err()
    );
}
#[test]
fn cusp_uses_angular_clock_and_keeps_both_arms_at_the_exact_node() {
    use nurbs_core::surface::Surface;
    let cx = [-0.125, 0.125, -0.125, 0.125];
    let cy = [0.25, -0.25, 0.25];
    let cusp = Surface {
        degree_u: 3,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..4)
            .map(|i| {
                (0..3)
                    .map(|j| vec![i as f64 / 3., j as f64 / 2., cy[j] - cx[i]])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 4],
        periodic_u: false,
        periodic_v: false,
    };
    let sources = ss::FiberSources::new(&fixture::plane(0.), &cusp).unwrap();
    let ideal = ss::qualify_original_ss_ideal(&sources, [1, 1, 3, 2], Default::default()).unwrap();
    let mut center = std::array::from_fn(|_| q(0.5));
    center[0] =
        ideal.homogeneous()[1][0].evaluate(&center) / ideal.homogeneous()[1][3].evaluate(&center);
    let basis = ss::qualify_ss_groebner_basis(&ideal, Default::default()).unwrap();
    let system = ss::qualify_ss_deflated_system(&basis, Default::default()).unwrap();
    let n = ss::qualify_ss_root_node(&system, center, Default::default()).unwrap();
    let chart = ss::qualify_ss_blowup_chart(&n, q(0.125), 2, Default::default()).unwrap();
    let arc = ss::qualify_ss_blowup_root_arc(
        &chart,
        [[0.70, 0.80], [0.45, 0.55], [0.45, 0.60], [0.475, 0.525]],
        ss::FiberAxis::SecondV,
        Default::default(),
    )
    .unwrap();
    // Exact original horizontal derivative fixes the exceptional first-U slope.
    let p = std::array::from_fn(|_| q(0.5));
    let slope = ideal.homogeneous()[1][0]
        .derivative(2)
        .unwrap()
        .evaluate(&p);
    let node_lift = [(slope + q(2.)) / q(4.), q(0.5), q(0.5), q(0.5)];
    let arc = arc
        .bind_node_preimage(node_lift, Default::default())
        .unwrap();
    assert_eq!(arc.node_parameter(), Some(0.5));
    let curve = ss::OriginalSsRootCurve::from_blowup(arc);
    curve.reverify().unwrap();
    assert!(curve.node_at(0.5).is_some());
    let a = curve.enclose([0.48; 2], Default::default()).unwrap();
    let b = curve.enclose([0.52; 2], Default::default()).unwrap();
    assert!(a.uv[2][0] > 0.5 && b.uv[2][0] > 0.5);
    assert!(a.uv[3][1] < 0.5 && 0.5 < b.uv[3][0]);
}
#[test]
fn all_jacobian_minors_separate_regular_arms_from_the_shared_singular_locus() {
    let n = node();
    let rank = ss::qualify_ss_rank_strata(n.source(), Default::default()).unwrap();
    rank.reverify().unwrap();
    assert!(!rank.proves_no_singular_roots());
    assert_eq!(rank.considered_minors(), 4);
    assert_eq!(
        rank.classify_exact(std::array::from_fn(|_| q(0.5)), Default::default())
            .unwrap(),
        ss::ExactSsRankLocation::Singular
    );
    assert_eq!(
        rank.classify_exact([q(0.625), q(0.5), q(0.625), q(0.5)], Default::default())
            .unwrap(),
        ss::ExactSsRankLocation::Regular
    );
    assert_eq!(
        rank.classify_exact(std::array::from_fn(|_| q(0.6)), Default::default())
            .unwrap(),
        ss::ExactSsRankLocation::OffSource
    );
    assert!(
        ss::qualify_ss_rank_strata(
            n.source(),
            ss::SsRankStrataLimits {
                max_minors: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
}
