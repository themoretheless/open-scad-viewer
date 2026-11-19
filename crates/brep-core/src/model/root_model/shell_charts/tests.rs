use super::*;

#[test]
fn pinched_nonperiodic_trim_repeats_a_vertex() {
    assert!(require_distinct_trim_vertices([0, 1, 2, 3]).is_ok());
    assert!(require_distinct_trim_vertices([0, 1, 2, 0, 3, 4]).is_err());
}

#[test]
fn census_clock_images_preserve_both_shore_aliases() {
    let q = |a: i32, b: i32| num_rational::BigRational::new(a.into(), b.into());
    let period = q(1, 1);
    assert_eq!(
        intersect_clock_images([0., 1.], [-0.125, 0.125], Some(&period)).unwrap(),
        vec![[q(-1, 8), q(0, 1)], [q(0, 1), q(1, 8)]]
    );
    assert_eq!(
        intersect_clock_images([1., 1.], [-0.125, 0.125], Some(&period)).unwrap(),
        vec![[q(0, 1), q(0, 1)]]
    );
    assert!(
        intersect_clock_images([0.25, 0.5], [-0.125, 0.125], Some(&period))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn contact_window_accepts_a_different_free_axis_only_for_full_coverage() {
    let mut a =
        nurbs_core::polynomial::graph([0., 1., 0., 1.], &[vec![0., 0.], vec![0., 0.]]).unwrap();
    assert_eq!([a.degree_u, a.degree_v], [1, 1]);
    let mut b = a.clone();
    for point in b.control_points.iter_mut().flatten() {
        point[2] = point[1] - 0.5;
        point[1] = 0.5;
    }
    // Non-unit native clocks distinguish the two U coordinates.
    a.knots_u = vec![0., 0., 2., 2.];
    let sources = ss::FiberSources::new(&a, &b).unwrap();
    let fiber = sources
        .certify(
            [[0.25, 1.75], [0.25, 0.75], [0.25, 0.75], [0.25, 0.75]],
            ss::FiberAxis::SecondU,
            Default::default(),
        )
        .unwrap()
        .into_fiber()
        .unwrap();
    let curve = Arc::new(ss::OriginalSsRootCurve::from_native(fiber));
    let port = |clock| {
        let point = Arc::new(ss::OriginalSsRootPoint::new(curve.clone(), clock).unwrap());
        ss::bind_original_ss_port(point, curve.clone(), [0; 4], Default::default()).unwrap()
    };
    let edge = ss::OriginalSsRootEdge::new(port(0.375), port(0.625)).unwrap();
    let left = ss::OriginalSsRootEdge::new(port(0.375), port(0.5)).unwrap();
    let right = ss::OriginalSsRootEdge::new(port(0.5), port(0.625)).unwrap();
    let retraced = ss::OriginalSsRootEdge::new(port(0.5), port(0.4375)).unwrap();
    assert!(adjacent_trim_arcs_overlap(&left, &retraced, Default::default()).unwrap());
    assert!(!adjacent_trim_arcs_overlap(&left, &right, Default::default()).unwrap());
    let alternate = sources
        .certify(
            [[0.125, 1.875], [0.125, 0.875], [0.25, 0.75], [0.125, 0.875]],
            ss::FiberAxis::SecondU,
            Default::default(),
        )
        .unwrap()
        .into_fiber()
        .unwrap();
    let alternate = Arc::new(ss::OriginalSsRootCurve::from_native(alternate));
    let alternate_port = |clock| {
        let point = Arc::new(ss::OriginalSsRootPoint::new(alternate.clone(), clock).unwrap());
        ss::bind_original_ss_port(point, alternate.clone(), [0; 4], Default::default()).unwrap()
    };
    let retraced_other_chart =
        ss::OriginalSsRootEdge::new(alternate_port(0.5), alternate_port(0.4375)).unwrap();
    assert!(!left.curve().same_definition(&alternate).unwrap());
    assert!(
        adjacent_trim_arcs_overlap(&left, &retraced_other_chart, Default::default()).unwrap()
    );
    let different_axis = sources
        .certify(
            [[0.75, 1.25], [0.25, 0.75], [0.25, 0.75], [0.25, 0.75]],
            ss::FiberAxis::FirstU,
            Default::default(),
        )
        .unwrap()
        .into_fiber()
        .unwrap();
    let different_axis = Arc::new(ss::OriginalSsRootCurve::from_native(different_axis));
    let first_u_port = |clock| {
        let point =
            Arc::new(ss::OriginalSsRootPoint::new(different_axis.clone(), clock).unwrap());
        ss::bind_original_ss_port(point, different_axis.clone(), [0; 4], Default::default())
            .unwrap()
    };
    let reverse_first_u =
        ss::OriginalSsRootEdge::new(first_u_port(1.), first_u_port(0.75)).unwrap();
    let forward_first_u =
        ss::OriginalSsRootEdge::new(first_u_port(1.), first_u_port(1.25)).unwrap();
    assert!(adjacent_trim_arcs_overlap(&left, &reverse_first_u, Default::default()).unwrap());
    assert!(!adjacent_trim_arcs_overlap(&left, &forward_first_u, Default::default()).unwrap());
    let short_retrace =
        ss::OriginalSsRootEdge::new(first_u_port(1.), first_u_port(0.984375)).unwrap();
    assert!(adjacent_trim_arcs_overlap(&left, &short_retrace, Default::default()).unwrap());
    // Two opposed uses of the same segment share both endpoints.
    let return_edge = ss::OriginalSsRootEdge::new(port(0.5), port(0.375)).unwrap();
    assert!(adjacent_trim_arcs_overlap(&left, &return_edge, Default::default()).unwrap());
    assert!(adjacent_trim_arcs_overlap(&return_edge, &left, Default::default()).unwrap());
    let gapped = ss::OriginalSsRootEdge::new(port(0.53125), port(0.625)).unwrap();
    let window = |lo, hi| {
        [[0.75, 1.25], [0.5; 2], [lo, hi], [0.5; 2]].map(|bounds| {
            bounds.map(|value| num_rational::BigRational::from_float(value).unwrap())
        })
    };
    assert!(covers_contact_window(
        &[&edge],
        &sources,
        0,
        window(0.375, 0.625),
        vec![],
        [None; 4]
    ));
    assert!(!covers_contact_window(
        &[&edge],
        &sources,
        0,
        window(0.3125, 0.625),
        vec![],
        [None; 4]
    ));
    assert!(covers_contact_window(
        &[&left, &right],
        &sources,
        0,
        window(0.375, 0.625),
        vec![],
        [None; 4],
    ));
    assert!(!covers_contact_window(
        &[&left, &gapped],
        &sources,
        0,
        window(0.375, 0.625),
        vec![],
        [None; 4],
    ));

    let chart = sources
        .certify_quotient(
            [[0.25, 1.75], [0.25, 0.75], [0.25, 0.75], [0.25, 0.75]],
            ss::FiberAxis::SecondU,
            Default::default(),
        )
        .unwrap()
        .into_fiber()
        .unwrap();
    let mut remaining = 8;
    let outside = [[0., 2.], [0.25, 0.75], [0.375, 0.625], [0.25, 0.75]];
    assert!(!refine_contact_clock(&[&edge], &chart, outside, &mut remaining, &Default::default(), |_| false).unwrap());
    assert_eq!(remaining, 8);
}

#[test]
fn periodic_clock_strips_join_in_the_census_chart() {
    let q = |n: i32| num_rational::BigRational::new(n.into(), 8.into());
    let period = q(8);
    let window = [q(7), q(9)];
    let left = contact_clock_strips(&window, &[q(7), q(8)], Some(&period));
    let right = contact_clock_strips(&window, &[q(0), q(1)], Some(&period));
    assert!(left.contains(&([q(7), q(8)], q(0))));
    assert!(right.contains(&([q(8), q(9)], q(-8))));
    assert!(covers_clock(
        left.into_iter()
            .chain(right)
            .map(|(range, _)| range)
            .collect(),
        window
    ));
}

#[test]
fn contact_window_lift_requires_one_shift_for_the_entire_interval() {
    let q = |n: i32| num_rational::BigRational::new(n.into(), 8.into());
    let period = q(8);
    let mut window = std::array::from_fn(|_| [q(0), q(1)]);
    let mut target = window.clone();
    window[1] = [q(7), q(9)];
    target[1] = [q(-1), q(1)];
    let periods = [None, Some(&period), None, None];
    assert_eq!(
        lift_contact_window(&window, &target, periods),
        Some(target.clone())
    );
    target[1] = [q(0), q(1)];
    assert!(lift_contact_window(&window, &target, periods).is_none());
}

#[test]
fn contact_leaf_clips_nonperiodic_bounds_and_preserves_periodic_aliases() {
    let period = num_rational::BigRational::from_integer(1.into());
    let faces = [[[0.25, 0.75], [-0.125, 0.125]], [[0., 1.]; 2]];
    let mut root = [[0., 1.], [0.875, 1.], [0., 1.], [1., 2.]];
    assert!(
        clip_contact_to_face_bounds(&mut root, faces, [None, Some(&period), None, None])
            .unwrap()
    );
    assert_eq!(root, [[0.25, 0.75], [0.875, 1.], [0., 1.], [1., 1.]]);
    root[1] = [0.25, 0.5];
    assert!(
        !clip_contact_to_face_bounds(&mut root, faces, [None, Some(&period), None, None])
            .unwrap()
    );
}

#[test]
fn contact_boxes_cover_a_window_with_different_split_axes() {
    let q = |n: i32| num_rational::BigRational::from_integer(n.into());
    let window = std::array::from_fn(|_| [q(0), q(2)]);
    let mut left = window.clone();
    left[0][1] = q(1);
    let mut lower_right = window.clone();
    lower_right[0][0] = q(1);
    lower_right[1][1] = q(1);
    let mut upper_right = lower_right.clone();
    upper_right[1] = [q(1), q(2)];
    assert!(!covers_contact_boxes(window.clone(), &[left.clone(), lower_right.clone()]));
    assert!(covers_contact_boxes(window, &[left, lower_right, upper_right]));
}

#[test]
fn shared_clock_coverage_requires_endpoints_and_no_gaps() {
    let q = |n: i32| num_rational::BigRational::from_integer(n.into());
    assert!(covers_clock(vec![[q(1), q(2)], [q(0), q(1)]], [q(0), q(2)]));
    assert!(!covers_clock(
        vec![[q(0), q(1)], [q(2), q(3)]],
        [q(0), q(3)]
    ));
    assert!(!covers_clock(vec![[q(1), q(3)]], [q(0), q(3)]));
    assert!(!covers_clock(vec![[q(0), q(2)]], [q(0), q(3)]));
}

#[test]
fn chart_separation_subdivides_overlapping_world_enclosures() {
    let plane = |shift: f64| Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [0., 1.]
            .map(|u| [0., 1.].map(|v| vec![u, v, u + shift]).to_vec())
            .to_vec(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let a = plane(0.);
    let b = plane(0.5);
    let rectangles = [[[0., 1.]; 2]; 2];
    separate_charts([&a, &b], rectangles, &mut 4096).unwrap();
    assert!(separate_charts([&a, &b], rectangles, &mut 1).is_err());
    assert!(separate_charts([&a, &a], rectangles, &mut 128).is_err());
}
