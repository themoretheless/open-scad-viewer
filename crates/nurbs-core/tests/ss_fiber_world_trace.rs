use nurbs_core::{
    curve::Curve,
    ss_intersection::{
        self as ss, FiberAxis, FiberLimits, FiberWorldTraceLimits, TransitionOrientation,
    },
};
#[path = "support/ss_mixed_fiber.rs"]
mod fixture;
fn data() -> (ss::QualifiedFiber, [Curve; 2], Curve) {
    let source = fixture::sources();
    let end = 16_f64 / 17.;
    let fiber = source
        .certify(
            [[0., end], [0.375, 0.625], [0.2, 0.55], [0.3, 0.7]],
            FiberAxis::FirstU,
            FiberLimits::default(),
        )
        .unwrap()
        .into_fiber()
        .unwrap();
    let p = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.5], vec![0.5, 0.5], vec![end, 15_f64 / 34.]],
        weights: vec![1., 1., 17_f64 / 16.],
        periodic: false,
    };
    let world = Curve {
        control_points: p
            .control_points
            .iter()
            .map(|q| vec![0.25 + 0.25 * q[0], q[1], 0.25])
            .collect(),
        ..p.clone()
    };
    let second = Curve {
        control_points: world
            .control_points
            .iter()
            .map(|p| p[..2].to_vec())
            .collect(),
        ..p.clone()
    };
    (fiber, [p, second], world)
}
#[test]
fn curved_rational_world_trace_has_continuous_original_root_error_and_full_clock() {
    let (fiber, p, world) = data();
    let limits = FiberWorldTraceLimits {
        tolerance_world: 1e-9,
        ..Default::default()
    };
    let r = ss::qualify_fiber_world_trace(&fiber, &p, &world, limits).unwrap();
    let proof = r.trace().expect(r.reason());
    proof.reverify().unwrap();
    assert!(proof.world_error_upper() <= limits.tolerance_world);
    assert_eq!(proof.orientation(), TransitionOrientation::Forward);
    for t in [0_f64, 0.125, 0.25, 0.5, 0.75, 0.875, 1.] {
        let uv = p[0].evaluate(t).unwrap().point;
        let actual = [
            0.25 + 0.25 * uv[0],
            (0.25 - uv[0] * uv[0] / 16.).sqrt(),
            0.25,
        ];
        let candidate = world.evaluate(t).unwrap().point;
        let error = candidate
            .iter()
            .zip(actual)
            .map(|(&a, b)| (a - b).abs())
            .sum::<f64>();
        assert!(
            error <= proof.world_error_upper(),
            "{error} > {}",
            proof.world_error_upper()
        );
    }
    let reverse = |mut c: Curve| {
        c.control_points.reverse();
        c.weights.reverse();
        c
    };
    let reversed = [reverse(p[0].clone()), reverse(p[1].clone())];
    let r = ss::qualify_fiber_world_trace(&fiber, &reversed, &reverse(world), limits).unwrap();
    assert_eq!(
        r.trace().unwrap().orientation(),
        TransitionOrientation::Reverse
    );
    r.trace().unwrap().reverify().unwrap();
}
#[test]
fn actual_coefficients_clock_and_proof_work_cannot_inherit_root_trace() {
    let (fiber, p, world) = data();
    let limits = FiberWorldTraceLimits {
        tolerance_world: 1e-9,
        ..Default::default()
    };
    let good = ss::qualify_fiber_world_trace(&fiber, &p, &world, limits).unwrap();
    let used = good.rectangles();
    ss::qualify_fiber_world_trace(
        &fiber,
        &p,
        &world,
        FiberWorldTraceLimits {
            max_rectangles: used,
            ..limits
        },
    )
    .unwrap()
    .trace()
    .unwrap()
    .reverify()
    .unwrap();
    for changed in [
        FiberWorldTraceLimits {
            max_rectangles: used - 1,
            ..limits
        },
        FiberWorldTraceLimits {
            max_agreement_cells: 1,
            ..limits
        },
    ] {
        let short = ss::qualify_fiber_world_trace(&fiber, &p, &world, changed).unwrap();
        assert!(short.trace().is_none());
        assert!(short.rectangles() <= changed.max_rectangles);
        assert!(short.agreement_cells() <= changed.max_agreement_cells);
    }
    let mut damaged = p.clone();
    damaged[1].control_points[1][1] += 0.02;
    assert!(
        ss::qualify_fiber_world_trace(&fiber, &damaged, &world, limits)
            .unwrap()
            .trace()
            .is_none()
    );
    let mut partial = p.clone();
    partial[0].control_points[0][0] = 0.01;
    let r = ss::qualify_fiber_world_trace(&fiber, &partial, &world, limits).unwrap();
    assert!(r.trace().is_none());
    assert_eq!(r.reason(), "candidate-clock-or-box-unproven");
    let mut wrong = world;
    wrong.control_points[1][2] += 0.01;
    assert!(
        ss::qualify_fiber_world_trace(&fiber, &p, &wrong, limits)
            .unwrap()
            .trace()
            .is_none()
    );
}

#[test]
fn a_curve_on_both_supports_cannot_claim_the_wrong_original_root_branch() {
    let source = fixture::sources();
    let mut a = source.first().clone();
    let mut b = source.second().clone();
    for row in &mut a.control_points {
        for (j, p) in row.iter_mut().enumerate() {
            p[2] = [0.1875, -0.3125, 0.1875][j];
        }
    }
    for row in &mut b.control_points {
        for p in row {
            p[2] = 0.;
        }
    }
    let sources = ss::FiberSources::new(&a, &b).unwrap();
    let lower = sources
        .certify(
            [[0., 1.], [0.2, 0.3], [0.2, 0.55], [0.1, 0.4]],
            FiberAxis::FirstU,
            FiberLimits::default(),
        )
        .unwrap()
        .into_fiber()
        .unwrap();
    let upper = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.75], vec![1., 0.75]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let other = Curve {
        control_points: vec![vec![0.25, 0.75], vec![0.5, 0.75]],
        ..upper.clone()
    };
    let world = Curve {
        control_points: vec![vec![0.25, 0.75, 0.], vec![0.5, 0.75, 0.]],
        ..upper.clone()
    };
    for (p, s) in [(&upper, &a), (&other, &b)] {
        let r =
            nurbs_core::curve_surface_agreement::verify(&world, p, s, false, 1e-9, 1024).unwrap();
        assert_eq!(
            r.status,
            nurbs_core::curve_surface_agreement::Status::WithinTolerance
        );
    }
    let r = ss::qualify_fiber_world_trace(
        &lower,
        &[upper, other],
        &world,
        FiberWorldTraceLimits::default(),
    )
    .unwrap();
    assert!(r.trace().is_none());
    assert_eq!(r.reason(), "candidate-clock-or-box-unproven");
}

#[test]
fn original_multispan_surface_and_authored_world_knots_preserve_full_error_cover() {
    let (old, p, world) = data();
    let original = old.source().first();
    let mut s = original.clone();
    s.knots_u = vec![0., 0., 0., 0.5, 1., 1., 1.];
    s.control_points = vec![
        original.control_points[0].clone(),
        original.control_points[0]
            .iter()
            .zip(&original.control_points[1])
            .map(|(a, b)| a.iter().zip(b).map(|(a, b)| (a + b) * 0.5).collect())
            .collect(),
        original.control_points[1]
            .iter()
            .zip(&original.control_points[2])
            .map(|(a, b)| a.iter().zip(b).map(|(a, b)| (a + b) * 0.5).collect())
            .collect(),
        original.control_points[2].clone(),
    ];
    s.weights = vec![vec![1.; 3]; 4];
    s.validate().unwrap();
    let source = ss::FiberSources::new(&s, old.source().second()).unwrap();
    let f = source
        .certify(old.domain(), old.free_axis(), FiberLimits::default())
        .unwrap()
        .into_fiber()
        .unwrap();
    let split = world.insert(0.5, 1).unwrap();
    assert_eq!(split.control_points.len(), 4);
    let limits = FiberWorldTraceLimits {
        tolerance_world: 1e-9,
        ..Default::default()
    };
    let r = ss::qualify_fiber_world_trace(&f, &p, &split, limits).unwrap();
    let proof = r.trace().expect(r.reason());
    proof.reverify().unwrap();
    assert!(proof.agreement_cells() > 2);
    assert!(proof.world_error_upper() <= limits.tolerance_world);
    for t in [0.499999, 0.5, 0.500001] {
        let uv = p[0].evaluate(t).unwrap().point;
        let expected = [
            0.25 + 0.25 * uv[0],
            (0.25 - uv[0] * uv[0] / 16.).sqrt(),
            0.25,
        ];
        let point = split.evaluate(t).unwrap().point;
        assert!(
            point
                .iter()
                .zip(expected)
                .map(|(&a, b)| (a - b).abs())
                .sum::<f64>()
                <= proof.world_error_upper()
        );
    }
    let short = ss::qualify_fiber_world_trace(
        &f,
        &p,
        &split,
        FiberWorldTraceLimits {
            max_agreement_cells: 1,
            ..limits
        },
    )
    .unwrap();
    assert!(short.trace().is_none());
    assert!(short.agreement_cells() <= 1);
}

#[test]
fn genuine_piecewise_source_curvature_has_complete_original_world_trace_cover() {
    use nurbs_core::surface::Surface;
    let x = [0.25, 0.3125, 0.4375, 0.5];
    let z = [0.25, 0.25, 0.28125, 0.375];
    let a = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 0.5, 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..4)
            .map(|i| vec![vec![x[i], 0., z[i]], vec![x[i], 1., z[i]]])
            .collect(),
        weights: vec![vec![1.; 2]; 4],
        periodic_u: false,
        periodic_v: false,
    };
    let b = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0.5, 0.], vec![0., 0.5, 1.]],
            vec![vec![1., 0.5, 0.], vec![1., 0.5, 1.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let source = ss::FiberSources::new(&a, &b).unwrap();
    let fiber = source
        .certify(
            [[0., 1.], [0.25, 0.75], [0.125, 0.625], [0.125, 0.5]],
            FiberAxis::FirstU,
            FiberLimits::default(),
        )
        .unwrap()
        .into_fiber()
        .unwrap();
    let first = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.5], vec![1., 0.5]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let second = Curve {
        degree: 2,
        knots: a.knots_u.clone(),
        control_points: (0..4).map(|i| vec![x[i], z[i]]).collect(),
        weights: vec![1.; 4],
        periodic: false,
    };
    let world = Curve {
        control_points: (0..4).map(|i| vec![x[i], 0.5, z[i]]).collect(),
        ..second.clone()
    };
    let limits = FiberWorldTraceLimits {
        tolerance_world: 1e-9,
        ..Default::default()
    };
    let r = ss::qualify_fiber_world_trace(&fiber, &[first, second], &world, limits).unwrap();
    let proof = r.trace().expect(r.reason());
    proof.reverify().unwrap();
    assert!(proof.world_error_upper() <= limits.tolerance_world);
    assert!(proof.agreement_cells() > 2);
    // C1 at u=.5, with genuine second-derivative change from .125 to .625.
    for u in [0_f64, 0.25, 0.499999, 0.5, 0.500001, 0.75, 1.] {
        let g = if u <= 0.5 {
            u * u / 16.
        } else {
            let d = u - 0.5;
            0.015625 + 0.0625 * d + 0.3125 * d * d
        };
        let actual = [0.25 + 0.25 * u, 0.5, 0.25 + g];
        let point = world.evaluate(u).unwrap().point;
        assert!(
            point
                .iter()
                .zip(actual)
                .map(|(&a, b)| (a - b).abs())
                .sum::<f64>()
                <= proof.world_error_upper()
        );
    }
}

#[test]
fn second_surface_clock_and_rotated_world_frame_use_the_actual_source_operator() {
    use nurbs_core::surface::Surface;
    let mut a = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0.25, 0., 0.], vec![0.25, 1., 0.]],
            vec![vec![0.5, 0., 0.], vec![0.5, 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let mut b = Surface {
        control_points: vec![
            vec![vec![0., 0.5, -1.], vec![0., 0.5, 1.]],
            vec![vec![1., 0.5, -1.], vec![1., 0.5, 1.]],
        ],
        ..a.clone()
    };
    let rotate = |p: &mut Vec<f64>| {
        let q = p.clone();
        *p = vec![-q[1] + 3., q[2] - 5., q[0] + 7.];
    };
    for s in [&mut a, &mut b] {
        for p in s.control_points.iter_mut().flatten() {
            rotate(p);
        }
    }
    let source = ss::FiberSources::new(&a, &b).unwrap();
    let f = source
        .certify(
            [[0.125, 0.875], [0.25, 0.75], [0.3125, 0.4375], [0.25, 0.75]],
            FiberAxis::SecondU,
            FiberLimits::default(),
        )
        .unwrap()
        .into_fiber()
        .unwrap();
    let p = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0.25, 0.5], vec![0.75, 0.5]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let q = Curve {
        control_points: vec![vec![0.3125, 0.5], vec![0.4375, 0.5]],
        ..p.clone()
    };
    let mut c = Curve {
        control_points: vec![vec![0.3125, 0.5, 0.], vec![0.4375, 0.5, 0.]],
        ..p.clone()
    };
    for point in &mut c.control_points {
        rotate(point);
    }
    let limits = FiberWorldTraceLimits {
        tolerance_world: 1e-9,
        ..Default::default()
    };
    let r = ss::qualify_fiber_world_trace(&f, &[p, q], &c, limits).unwrap();
    let proof = r.trace().expect(r.reason());
    proof.reverify().unwrap();
    assert!(proof.world_error_upper() <= limits.tolerance_world);
    assert_eq!(proof.source().free_axis(), FiberAxis::SecondU);
}

#[test]
fn original_multispan_clock_covers_the_whole_root_and_flat_spans_are_refused() {
    let (fiber, mut p, world) = data();
    p[0] = p[0].insert(0.5, 1).unwrap();
    let limits = FiberWorldTraceLimits {
        tolerance_world: 1e-9,
        ..Default::default()
    };
    let r = ss::qualify_fiber_world_trace(&fiber, &p, &world, limits).unwrap();
    let proof = r.trace().expect(r.reason());
    proof.reverify().unwrap();
    assert_eq!(proof.orientation(), TransitionOrientation::Forward);
    assert!(proof.world_error_upper() <= limits.tolerance_world);
    for t in [0., 0.25, 0.499999, 0.5, 0.500001, 0.75, 1.] {
        let uv = p[0].evaluate(t).unwrap().point;
        let point = world.evaluate(t).unwrap().point;
        let actual = [
            0.25 + 0.25 * uv[0],
            (0.25 - uv[0] * uv[0] / 16.).sqrt(),
            0.25,
        ];
        assert!(
            point
                .iter()
                .zip(actual)
                .map(|(&a, b)| (a - b).abs())
                .sum::<f64>()
                <= proof.world_error_upper()
        );
    }
    let mut flat = p;
    flat[0] = Curve {
        degree: 1,
        knots: vec![0., 0., 0.5, 1., 1.],
        control_points: vec![
            vec![0., 0.5],
            vec![0., 0.5],
            vec![16_f64 / 17., 15_f64 / 34.],
        ],
        weights: vec![1.; 3],
        periodic: false,
    };
    let r = ss::qualify_fiber_world_trace(&fiber, &flat, &world, limits).unwrap();
    assert!(r.trace().is_none());
    assert_eq!(r.reason(), "candidate-clock-or-box-unproven");
    assert_eq!(r.agreement_cells(), 0);
}
