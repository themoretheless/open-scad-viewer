use nurbs_core::{
    curve::Curve,
    offset_contact_pcurve::FitLimits,
    surface::Surface,
    surface_offset::regions,
    trim_domain::{Location, TrimDomain},
};
fn plane() -> Surface {
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
    }
}
fn trace() -> Curve {
    Curve {
        degree: 2,
        knots: vec![2., 2., 2., 5., 5., 5.],
        control_points: vec![vec![0.2, 0.5], vec![0.5, 0.6], vec![0.8, 0.5]],
        weights: vec![1., 0.8, 1.],
        periodic: false,
    }
}
#[test]
fn rational_trace_gives_an_exact_closed_original_uv_loop_and_injective_map() {
    let s = plane();
    let t = trace();
    let before = (s.clone(), t.clone());
    let r = regions::build_strip(&s, &t, 0).unwrap();
    assert_eq!(r.contact_boundary(), 2);
    assert!(!r.clock_reversed());
    assert_eq!(r.map().knots_u, t.knots);
    let mut reversed = t.control_points.clone();
    reversed.reverse();
    assert_eq!(r.boundary()[2].control_points, reversed);
    assert_eq!(
        nurbs_core::trim_domain::exact_loop_joins(r.boundary()).unwrap(),
        Some(true)
    );
    let domain = TrimDomain::new(&[r.boundary().to_vec()], 1e-10).unwrap();
    assert_eq!(
        domain
            .classify([[0.4, 0.5], [0.1, 0.2]], 4096)
            .unwrap()
            .location,
        Location::Inside
    );
    assert_eq!(
        domain
            .classify([[0.9, 1.], [0.1, 0.2]], 4096)
            .unwrap()
            .location,
        Location::Outside
    );
    r.reverify().unwrap();
    assert_eq!((s, t), before);
}
#[test]
fn reversed_original_trace_keeps_its_domain_and_records_the_clock_direction() {
    let s = plane();
    let mut t = trace();
    t.control_points.reverse();
    t.weights.reverse();
    let r = regions::build_strip(&s, &t, 0).unwrap();
    assert!(r.clock_reversed());
    assert_eq!(r.contact_trace(), &t);
    assert_eq!(r.map().knots_u, t.knots);
    r.reverify().unwrap();
}
#[test]
fn folded_collapsed_outside_or_invalid_sides_never_create_a_region() {
    let s = plane();
    let mut fold = trace();
    fold.control_points[2][0] = 0.1;
    let mut outside = trace();
    outside.control_points[1][0] = 1.1;
    let mut collapsed = trace();
    for p in &mut collapsed.control_points {
        p[1] = 0.;
    }
    for t in [fold, outside, collapsed] {
        assert!(regions::build_strip(&s, &t, 0).is_err());
    }
    assert!(regions::build_strip(&s, &trace(), 4).is_err());
}
fn curved() -> [Surface; 2] {
    let mut a = plane();
    a.degree_u = 2;
    a.knots_u = vec![0., 0., 0., 1., 1., 1.];
    a.control_points = [[1., 0.], [1., 1.], [0., 1.]]
        .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]])
        .to_vec();
    a.weights = [1., std::f64::consts::FRAC_1_SQRT_2, 1.]
        .map(|w| vec![w; 2])
        .to_vec();
    let mut b = plane();
    b.control_points = vec![
        vec![vec![0., 1., 0.], vec![0., 1., 1.]],
        vec![vec![0., 2., 0.], vec![0.5, 2., 1.]],
    ];
    [a, b]
}
#[test]
fn nonlinear_contact_pair_builds_both_closed_regions_with_source_axis_sides() {
    let s = curved();
    let saved = s.clone();
    let r = regions::fit_pair(
        s.each_ref(),
        [0.1; 2],
        1,
        [0.35, 0.55],
        [0.88, 0.96],
        [[0.07, 0.22], [0.3, 0.6]],
        [3, 1],
        1e-7,
        FitLimits::default(),
    )
    .unwrap();
    assert_eq!(r.strips()[0].contact_boundary(), 1);
    assert_eq!(r.strips()[1].contact_boundary(), 3);
    for strip in r.strips() {
        assert_eq!(strip.map().knots_v, strip.contact_trace().knots);
        assert_eq!(strip.contact_trace().domain(), [0.35, 0.55]);
        assert_eq!(
            nurbs_core::trim_domain::exact_loop_joins(strip.boundary()).unwrap(),
            Some(true)
        );
    }
    assert!(
        r.fitting()
            .attempts
            .last()
            .unwrap()
            .reports
            .iter()
            .all(|p| p.correspondence_proven)
    );
    r.reverify().unwrap();
    assert_eq!(s, saved);
}
#[test]
fn independent_nonunit_original_uv_charts_retain_the_common_contact_clock() {
    let mut a = plane();
    a.knots_u = vec![2., 2., 7., 7.];
    let mut b = plane();
    b.knots_u = vec![-3., -3., 4., 4.];
    for p in b.control_points.iter_mut().flatten() {
        p.swap(1, 2);
        p[0] = 3. * p[0] - 1.;
    }
    let r = regions::fit_pair(
        [&a, &b],
        [0.2, -0.2],
        0,
        [3., 6.],
        [0.1, 0.3],
        [[-0.6, 1.6], [0.1, 0.3]],
        [0, 0],
        1e-8,
        FitLimits::default(),
    )
    .unwrap();
    for strip in r.strips() {
        assert_eq!(strip.map().knots_u, strip.contact_trace().knots);
        assert_eq!(strip.contact_trace().domain(), [3., 6.]);
        assert_eq!(strip.contact_boundary(), 2);
    }
    r.reverify().unwrap();
}
#[test]
fn unproved_contact_or_insufficient_fit_work_publishes_no_partial_region_pair() {
    let s = curved();
    let saved = s.clone();
    assert!(
        regions::fit_pair(
            s.each_ref(),
            [0.1; 2],
            1,
            [0.35, 0.55],
            [0.88, 0.96],
            [[0.07, 0.22], [0.3, 0.6]],
            [3, 1],
            1e-7,
            FitLimits {
                proof_cells: 1,
                ..FitLimits::default()
            }
        )
        .is_err()
    );
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p[2] = 3.;
    }
    assert!(
        regions::fit_pair(
            [&plane(), &b],
            [0.1; 2],
            0,
            [0.2, 0.8],
            [0.1, 0.9],
            [[0.1, 0.9]; 2],
            [0, 0],
            1e-7,
            FitLimits::default()
        )
        .is_err()
    );
    assert_eq!(s, saved);
}
