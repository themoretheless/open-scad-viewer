use nurbs_core::{ss_intersection as ss, surface::Surface};
#[path = "support/ss_mixed_fiber.rs"]
mod fixture;
fn source() -> ss::QualifiedFiber {
    let original = fixture::sources();
    let mut a = original.first().clone();
    for row in &mut a.control_points {
        row[2][2] += 0.02;
    }
    let b = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        vec![
                            i as f64 / 2.,
                            j as f64 / 2.,
                            0.25 + if i == 2 { 0.125 } else { 0. } + if j == 2 { 0.02 } else { 0. },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    ss::FiberSources::new(&a, &b)
        .unwrap()
        .certify(
            [[0., 0.75], [0.375, 0.625], [0.2, 0.55], [0.3, 0.7]],
            ss::FiberAxis::FirstU,
            ss::FiberLimits::default(),
        )
        .unwrap()
        .into_fiber()
        .unwrap()
}
#[test]
fn adaptive_curved_curved_trace_qualifies_every_original_root_piece() {
    let fiber = source();
    let a = fiber.source().first().clone();
    let b = fiber.source().second().clone();
    let limits = ss::FiberTraceBuildLimits::default();
    let built = ss::construct_fiber_trace(&fiber, limits).unwrap();
    println!(
        "reason={} cells={} samples={} attempts={} pieces={:?}",
        built.reason,
        built.cells,
        built.evaluations,
        built.attempts,
        built.trace.as_ref().map(Vec::len)
    );
    let pieces = built.trace.as_ref().expect(built.reason);
    assert!(pieces.len() > 1 && pieces.len() <= limits.max_segments);
    assert!(built.cells <= limits.max_cells && built.evaluations <= limits.max_evaluations);
    assert!(built.unresolved_interval.is_none());
    assert_eq!(pieces[0].source().domain()[0][0], 0.);
    assert_eq!(pieces.last().unwrap().source().domain()[0][1], 0.75);
    for pair in pieces.windows(2) {
        assert_eq!(
            pair[0].source().domain()[0][1],
            pair[1].source().domain()[0][0]
        );
        assert_eq!(
            pair[0].curve().control_points.last(),
            pair[1].curve().control_points.first()
        );
        for i in 0..2 {
            assert_eq!(
                pair[0].pcurves()[i].control_points.last(),
                pair[1].pcurves()[i].control_points.first()
            );
        }
    }
    for p in pieces {
        p.reverify().unwrap();
        assert!(p.world_error_upper() <= limits.tolerance_world);
        for t in [0., 0.125, 0.5, 0.875, 1.] {
            let uv = p.pcurves()[0].evaluate(t).unwrap().point;
            let u = uv[0];
            let x = 0.25 + 0.25 * u;
            let y = (0.25 + 0.125 * x * x - 0.0625 * u * u).sqrt();
            let z = 0.25 + 0.125 * x * x + 0.02 * y * y;
            let point = p.curve().evaluate(t).unwrap().point;
            let error = point
                .iter()
                .zip([x, y, z])
                .map(|(a, b)| (a - b).abs())
                .sum::<f64>();
            assert!(
                error <= p.world_error_upper(),
                "{error} > {}",
                p.world_error_upper()
            );
        }
    }
    let same_budget = ss::construct_fiber_trace(
        &fiber,
        ss::FiberTraceBuildLimits {
            max_cells: built.cells,
            ..limits
        },
    )
    .unwrap();
    assert!(same_budget.trace.is_some(), "{}", same_budget.reason);
    assert_eq!(same_budget.cells, built.cells);
    let short = ss::construct_fiber_trace(
        &fiber,
        ss::FiberTraceBuildLimits {
            max_cells: built.cells - 1,
            ..limits
        },
    )
    .unwrap();
    assert!(short.trace.is_none());
    assert!(short.cells <= built.cells - 1);
    assert_eq!(fiber.source().first(), &a);
    assert_eq!(fiber.source().second(), &b);
}
#[test]
fn partial_or_failed_construction_never_returns_a_complete_trace() {
    let fiber = source();
    for limits in [
        ss::FiberTraceBuildLimits {
            max_cells: 1,
            ..Default::default()
        },
        ss::FiberTraceBuildLimits {
            max_evaluations: 1,
            ..Default::default()
        },
        ss::FiberTraceBuildLimits {
            max_segments: 1,
            ..Default::default()
        },
        ss::FiberTraceBuildLimits {
            max_depth: 0,
            ..Default::default()
        },
    ] {
        let out = ss::construct_fiber_trace(&fiber, limits).unwrap();
        assert!(out.trace.is_none() && out.unresolved_interval.is_some());
        assert!(out.cells <= limits.max_cells && out.evaluations <= limits.max_evaluations);
    }
    assert!(
        ss::construct_fiber_trace(
            &fiber,
            ss::FiberTraceBuildLimits {
                tolerance_world: f64::NAN,
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn rotated_original_surfaces_and_second_support_clock_construct_the_same_root() {
    let reference = source();
    let angle = 0.37_f64;
    let pose = |p: &[f64]| {
        vec![
            angle.cos() * p[0] + angle.sin() * p[2] + 2.,
            p[1] - 3.,
            -angle.sin() * p[0] + angle.cos() * p[2] + 1.,
        ]
    };
    let transform = |s: &Surface| {
        let mut out = s.clone();
        for p in out.control_points.iter_mut().flatten() {
            *p = pose(p);
        }
        out
    };
    let a = transform(reference.source().first());
    let b = transform(reference.source().second());
    let sources = ss::FiberSources::new(&a, &b).unwrap();
    for (domain, axis) in [
        (reference.domain(), ss::FiberAxis::FirstU),
        (
            [[0., 0.75], [0.375, 0.625], [0.275, 0.4], [0.3, 0.7]],
            ss::FiberAxis::SecondU,
        ),
    ] {
        let fresh = sources
            .certify(domain, axis, ss::FiberLimits::default())
            .unwrap();
        let used = fresh.report().rectangles;
        if matches!(axis, ss::FiberAxis::SecondU) {
            assert_eq!(used, 8); // Four original rectangles plus four correlated rectangles.
            let short = sources
                .certify(
                    domain,
                    axis,
                    ss::FiberLimits {
                        max_rectangles: used - 1,
                        ..Default::default()
                    },
                )
                .unwrap();
            assert!(short.fiber().is_none() && short.report().rectangles <= used - 1);
        }
        let fiber = fresh.into_fiber().expect("posed root fiber");
        let built = ss::construct_fiber_trace(&fiber, Default::default()).unwrap();
        let pieces = built.trace.as_ref().expect(built.reason);
        for piece in pieces {
            piece.reverify().unwrap();
            for t in [0., 0.5, 1.] {
                let uv = piece.pcurves()[0].evaluate(t).unwrap().point;
                let u = uv[0];
                let x = 0.25 + 0.25 * u;
                let y = (0.25 + 0.125 * x * x - 0.0625 * u * u).sqrt();
                let z = 0.25 + 0.125 * x * x + 0.02 * y * y;
                let expected = pose(&[x, y, z]);
                let p = piece.curve().evaluate(t).unwrap().point;
                let err = p
                    .iter()
                    .zip(expected)
                    .map(|(a, b)| (a - b).abs())
                    .sum::<f64>();
                assert!(err <= piece.world_error_upper());
            }
        }
        assert!(built.cells <= 100000 && built.evaluations <= 4096);
    }
}

#[test]
fn certified_unknown_box_contraction_keeps_every_free_value_and_actual_root() {
    let a = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                vec![
                    vec![i as f64 / 2., 0., -1.],
                    vec![i as f64 / 2., 0., if i == 2 { 0.25 } else { 0. }],
                ]
            })
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let square = [1., -2., 4.];
    let b = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: a.knots_u.clone(),
        knots_v: a.knots_u.clone(),
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        vec![
                            -1. + 1.5 * i as f64,
                            -1. + 1.5 * j as f64,
                            -0.2 + 0.125 * square[i] + 0.02 * square[j],
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let sources = ss::FiberSources::new(&a, &b).unwrap();
    let domain = [[0., 1.], [0.1, 0.95], [0.25, 0.75], [0.25, 0.75]];
    assert!(
        sources
            .certify(domain, ss::FiberAxis::FirstU, Default::default())
            .unwrap()
            .fiber()
            .is_none()
    );
    let out = sources
        .certify_contracted(domain, ss::FiberAxis::FirstU, 100000)
        .unwrap();
    let fiber = out.fiber().expect(out.reason());
    fiber.reverify().unwrap();
    assert!(out.steps() > 1 && out.rectangles() <= 100000);
    assert_eq!(out.domain()[0], domain[0]);
    for axis in 1..4 {
        assert!(
            out.domain()[axis][0] >= domain[axis][0] && out.domain()[axis][1] <= domain[axis][1]
        );
    }
    for i in 0..=128 {
        let u = i as f64 / 128.;
        let root = [
            u,
            (0.8 + 0.125 * u * u) / (1. + 0.25 * u * u),
            (u + 1.) / 3.,
            1. / 3.,
        ];
        for axis in 0..4 {
            assert!(out.domain()[axis][0] <= root[axis] && root[axis] <= out.domain()[axis][1]);
        }
    }
    let same = sources
        .certify_contracted(domain, ss::FiberAxis::FirstU, out.rectangles())
        .unwrap();
    assert!(same.fiber().is_some());
    let short = sources
        .certify_contracted(domain, ss::FiberAxis::FirstU, out.rectangles() - 1)
        .unwrap();
    assert!(short.fiber().is_none() && short.rectangles() < out.rectangles());
    assert_eq!(sources.first(), &a);
    assert_eq!(sources.second(), &b);
}
