use nurbs_core::ss_intersection::{
    self as ss, FiberAxis, FiberLimits, FiberSources, FiberStatus, TransitionOrientation,
};
#[path = "support/ss_mixed_fiber.rs"]
mod fixture;
fn charts() -> (ss::QualifiedFiber, ss::QualifiedFiber, [[f64; 2]; 4]) {
    let src = fixture::sources();
    let first = src
        .certify(
            [[0.5, 0.75], [0.4, 0.6], [0.3, 0.5], [0.4, 0.6]],
            FiberAxis::FirstU,
            FiberLimits::default(),
        )
        .unwrap()
        .into_fiber()
        .unwrap();
    let second = src
        .certify(
            [
                [0.5, 0.8],
                [0.465, 0.48],
                [0.35, 0.45],
                [0.46875, 0.4765625],
            ],
            FiberAxis::SecondV,
            FiberLimits::default(),
        )
        .unwrap()
        .into_fiber()
        .unwrap();
    (
        first,
        second,
        [
            [0.640625, 0.671875],
            [0.465, 0.48],
            [0.35, 0.45],
            [0.46875, 0.4765625],
        ],
    )
}
#[test]
fn nonlinear_original_chart_transition_has_owned_reverse_coordinate_orientation() {
    let (a, b, d) = charts();
    let r = ss::qualify_fiber_transition(&a, &b, d, 100000).unwrap();
    let proof = r.transition().expect(r.reason());
    proof.reverify().unwrap();
    assert_eq!(proof.orientation(), TransitionOrientation::Reverse);
    let images = proof.endpoint_images();
    assert!(images[1][1] < images[0][0]);
    for (i, u) in d[0].into_iter().enumerate() {
        let exact = (0.25 - u * u / 16.).sqrt();
        assert!(images[i][0] <= exact && exact <= images[i][1]);
        let p = a.source().first().evaluate(u, exact).unwrap().point;
        let q = b
            .source()
            .second()
            .evaluate(0.25 + 0.25 * u, exact)
            .unwrap()
            .point;
        assert!(p.iter().zip(q).all(|(&p, q)| (p - q).abs() < 1e-12));
    }
    let same = ss::qualify_fiber_transition(&a, &a, d, 100000).unwrap();
    assert_eq!(
        same.transition().unwrap().orientation(),
        TransitionOrientation::Forward
    );
}
#[test]
fn overlapping_boxes_absent_roots_and_shared_work_cannot_inherit_a_transition() {
    let (a, b, d) = charts();
    let full = ss::qualify_fiber_transition(&a, &b, d, 100000).unwrap();
    let work = full.rectangles();
    ss::qualify_fiber_transition(&a, &b, d, work)
        .unwrap()
        .transition()
        .unwrap()
        .reverify()
        .unwrap();
    for budget in [1, work - 1] {
        let r = ss::qualify_fiber_transition(&a, &b, d, budget).unwrap();
        assert!(r.transition().is_none());
        assert!(r.rectangles() <= budget);
    }
    let mut missing = d;
    missing[0] = [0.5, 0.55];
    let r = ss::qualify_fiber_transition(&a, &b, missing, 100000).unwrap();
    assert!(r.transition().is_none());
    let mut point = d;
    point[0] = [0.65; 2];
    assert!(ss::qualify_fiber_transition(&a, &b, point, 100000).is_err());
}
#[test]
fn closed_source_endpoint_basis_must_not_be_replaced_by_one_sided_interior_limits() {
    let source = fixture::sources();
    let mut a = source.first().clone();
    a.degree_u = 1;
    a.knots_u = vec![-1., 0., 0., 1., 2.];
    a.validate().unwrap();
    let sources = FiberSources::new(&a, source.second()).unwrap();
    let full = sources
        .certify(
            [[0., 1.], [0.375, 0.625], [0.2, 0.55], [0.3, 0.7]],
            FiberAxis::FirstU,
            FiberLimits::default(),
        )
        .unwrap();
    assert_eq!(full.report().status, FiberStatus::Unresolved);
    assert_eq!(full.report().reason, "original-endpoint-basis-unproven");
    assert!(full.fiber().is_none());
    let mut upper = a.clone();
    upper.knots_u = vec![-1., 0., 1., 1., 2.];
    let end = FiberSources::new(&upper, source.second())
        .unwrap()
        .certify(
            [[0., 1.], [0.375, 0.625], [0.2, 0.55], [0.3, 0.7]],
            FiberAxis::FirstU,
            FiberLimits::default(),
        )
        .unwrap();
    assert_eq!(end.report().reason, "original-endpoint-basis-unproven");
    assert!(end.fiber().is_none());
    let inside = sources
        .certify(
            [[0.125, 0.875], [0.375, 0.625], [0.2, 0.55], [0.3, 0.7]],
            FiberAxis::FirstU,
            FiberLimits::default(),
        )
        .unwrap();
    assert_eq!(
        inside.report().status,
        FiberStatus::UniformUniqueRoot,
        "{:?}",
        inside.report()
    );
}
