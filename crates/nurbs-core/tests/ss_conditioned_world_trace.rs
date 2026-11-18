use nurbs_core::{curve::Curve, ss_intersection as ss, surface::Surface};
fn data(extent: f64, offset: f64) -> (ss::QualifiedFiber, [Curve; 2], Curve) {
    let a = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..2)
                    .map(|j| vec![i as f64 / 2., j as f64, if i == 2 { 1. } else { 0. }])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let mut b = a.clone();
    let z = [-0.5, extent / 2. - 0.5, extent * extent + extent - 0.5];
    for i in 0..3 {
        for j in 0..2 {
            b.control_points[i][j][0] *= extent;
            b.control_points[i][j][2] = z[i];
        }
    }
    let sources = ss::FiberSources::with_exact_graph_shear(&a, &b).unwrap();
    let fiber = sources
        .certify(
            [[0.25, 0.75], [0.25, 0.75], [0., 1. / extent], [0., 1.]],
            ss::FiberAxis::FirstV,
            Default::default(),
        )
        .unwrap()
        .into_fiber()
        .unwrap();
    assert!(
        fiber
            .report()
            .operator
            .as_ref()
            .unwrap()
            .uses_exact_graph_shear()
    );
    let x = 0.5 + offset;
    let first = Curve::from_polyline(vec![vec![x, 0.25], vec![x, 0.75]]).unwrap();
    let second =
        Curve::from_polyline(vec![vec![x / extent, 0.25], vec![x / extent, 0.75]]).unwrap();
    let world = Curve::from_polyline(vec![vec![x, 0.25, x * x], vec![x, 0.75, x * x]]).unwrap();
    (fiber, [first, second], world)
}
#[test]
fn original_world_trace_survives_nonlinear_conditioning_and_larger_second_footprint() {
    for extent in [1., 4.] {
        for offset in [0., 2f64.powi(-24), -2f64.powi(-24)] {
            let (fiber, pcurves, curve) = data(extent, offset);
            let before = (
                pcurves.clone(),
                curve.clone(),
                fiber.source().first().clone(),
                fiber.source().second().clone(),
            );
            let limits = ss::FiberWorldTraceLimits {
                tolerance_world: 1e-4,
                ..Default::default()
            };
            let result = ss::qualify_fiber_world_trace(&fiber, &pcurves, &curve, limits).unwrap();
            let owner = result.trace().expect(result.reason());
            owner.reverify().unwrap();
            assert!(owner.world_error_upper() <= limits.tolerance_world);
            assert_eq!(owner.orientation(), ss::TransitionOrientation::Forward);
            // Independent analytic original intersection is (1/2,v,1/4).
            // Continuous source proofs authorize the trace; these checks verify
            // the reported bound against known geometry rather than fit residuals.
            for t in [0., 0.125, 0.5, 0.875, 1.] {
                let p = curve.evaluate(t).unwrap().point;
                let exact = [0.5, 0.25 + 0.5 * t, 0.25];
                let error = p.iter().zip(exact).map(|(a, b)| (a - b).abs()).sum::<f64>();
                assert!(
                    error <= owner.world_error_upper(),
                    "{extent} {offset} {error} {}",
                    owner.world_error_upper()
                );
            }
            assert_eq!(
                before,
                (
                    pcurves.clone(),
                    curve.clone(),
                    fiber.source().first().clone(),
                    fiber.source().second().clone()
                )
            );
            let reverse = |mut c: Curve| {
                c.control_points.reverse();
                c.weights.reverse();
                c
            };
            let pcurves = [reverse(pcurves[0].clone()), reverse(pcurves[1].clone())];
            let result =
                ss::qualify_fiber_world_trace(&fiber, &pcurves, &reverse(curve), limits).unwrap();
            let owner = result.trace().expect(result.reason());
            assert_eq!(owner.orientation(), ss::TransitionOrientation::Reverse);
            owner.reverify().unwrap();
        }
    }
}
#[test]
fn conditioning_does_not_admit_wrong_world_coefficients_missing_agreement_or_too_tight_error() {
    let (fiber, pcurves, mut curve) = data(4., 2f64.powi(-24));
    let limits = ss::FiberWorldTraceLimits {
        tolerance_world: 1e-4,
        ..Default::default()
    };
    let result = ss::qualify_fiber_world_trace(
        &fiber,
        &pcurves,
        &curve,
        ss::FiberWorldTraceLimits {
            max_agreement_cells: 0,
            ..limits
        },
    )
    .unwrap();
    assert!(result.trace().is_none());
    let result = ss::qualify_fiber_world_trace(
        &fiber,
        &pcurves,
        &curve,
        ss::FiberWorldTraceLimits {
            tolerance_world: 1e-10,
            ..limits
        },
    )
    .unwrap();
    assert!(result.trace().is_none());
    for p in &mut curve.control_points {
        p[2] += 0.01;
    }
    let result = ss::qualify_fiber_world_trace(&fiber, &pcurves, &curve, limits).unwrap();
    assert!(result.trace().is_none());
}

#[test]
fn automatic_trace_builder_owns_the_whole_conditioned_free_interval() {
    let (fiber, _, _) = data(4., 0.);
    let limits = ss::FiberTraceBuildLimits {
        tolerance_world: 1e-6,
        ..Default::default()
    };
    let result = ss::construct_fiber_trace(&fiber, limits).unwrap();
    assert!(result.cells <= limits.max_cells);
    let pieces = result.trace.as_ref().expect(result.reason);
    assert!(!pieces.is_empty());
    assert_eq!(
        pieces.first().unwrap().source().domain()[1][0],
        fiber.domain()[1][0]
    );
    assert_eq!(
        pieces.last().unwrap().source().domain()[1][1],
        fiber.domain()[1][1]
    );
    for piece in pieces {
        assert!(
            piece
                .source()
                .report()
                .operator
                .as_ref()
                .unwrap()
                .uses_exact_graph_shear()
        );
        assert!(piece.world_error_upper() <= limits.tolerance_world);
        piece.reverify().unwrap();
    }
    let short = ss::construct_fiber_trace(
        &fiber,
        ss::FiberTraceBuildLimits {
            max_cells: 1,
            ..limits
        },
    )
    .unwrap();
    assert!(short.trace.is_none() && short.cells <= 1);
}
