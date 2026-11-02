use nurbs_core::{
    surface::Surface,
    surface_offset::boundary_contact::{self as boundary, Boundary, DiscoveryLimits, Limits},
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
fn planes() -> [Surface; 2] {
    let a = plane();
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p.swap(1, 2);
        p[0] = 3. * p[0] - 1.;
    }
    [a, b]
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
fn literal_boundary_root_has_regular_inward_branch_and_full_replay() {
    let s = planes();
    let saved = s.clone();
    for high in [false, true] {
        let coordinate = if high { 1. } else { 0. };
        let p = boundary::certify(
            s.each_ref(),
            [0.2, -0.2],
            Boundary {
                source: 0,
                axis: 0,
                high,
            },
            [[[coordinate; 2], [0.1, 0.3]], [[0.2, 0.8], [0.1, 0.3]]],
            Limits::default(),
        )
        .unwrap();
        assert!(p.regular_inward_branch_proven());
        assert_eq!(p.witness().unwrap().first_uv[0], [coordinate; 2]);
        assert!(
            p.witness().unwrap().point[0][0] <= coordinate
                && p.witness().unwrap().point[0][1] >= coordinate
        );
        p.reverify().unwrap();
    }
    assert_eq!(s, saved);
}
#[test]
fn nonlinear_second_chart_endpoint_preserves_original_uv_order() {
    let s = curved();
    let tube = [[[0.89, 0.92], [0.975, 0.999]], [[0.12, 0.15], [1.; 2]]];
    let p = boundary::certify(
        s.each_ref(),
        [0.1; 2],
        Boundary {
            source: 1,
            axis: 1,
            high: true,
        },
        tube,
        Limits::default(),
    )
    .unwrap();
    assert!(p.witness().is_some());
    assert!(p.regular_inward_branch_proven());
    let w = p.witness().unwrap();
    assert_eq!(w.second_uv[1], [1.; 2]);
    assert!(w.first_uv[1][1] < 1. && w.first_uv[1][0] > 0.9);
    p.reverify().unwrap();
    let q = boundary::certify(
        [&s[1], &s[0]],
        [0.1; 2],
        Boundary {
            source: 0,
            axis: 1,
            high: true,
        },
        [tube[1], tube[0]],
        Limits::default(),
    )
    .unwrap();
    assert_eq!(q.witness().unwrap().second_uv, w.first_uv);
    assert_eq!(q.witness().unwrap().first_uv, w.second_uv);
    assert_eq!(q.witness().unwrap().point, w.point);
}
#[test]
fn all_faces_are_searched_without_seeds_and_roots_are_not_component_counts() {
    let s = planes();
    let l = DiscoveryLimits::default();
    let r = boundary::discover(s.each_ref(), [0.2, -0.2], l).unwrap();
    assert_eq!(r.faces.len(), 8);
    assert!(r.boundary_coverage_proven());
    for high in [false, true] {
        assert!(r.endpoints.iter().any(|p| p.boundary()
            == Boundary {
                source: 0,
                axis: 0,
                high
            }
            && p.regular_inward_branch_proven()));
    }
    assert!(r.predictor_queries <= l.predictor_queries);
    assert!(r.qualifications <= r.predictor_queries);
    assert!(
        r.faces
            .iter()
            .all(|f| f.candidates.visited_boxes <= l.boxes_per_boundary)
    );
    assert!(r.endpoints.len() <= l.max_endpoints);
    assert!(
        r.faces
            .iter()
            .all(|f| f.coverage.visited <= l.qualification_boxes_per_boundary)
    );
}
#[test]
fn automatic_nonlinear_endpoint_is_found_on_second_natural_boundary() {
    let s = curved();
    let r = boundary::discover(s.each_ref(), [0.1; 2], DiscoveryLimits::default()).unwrap();
    assert!(
        r.endpoints.iter().any(|p| p.boundary()
            == Boundary {
                source: 1,
                axis: 1,
                high: true
            }
            && p.regular_inward_branch_proven()),
        "endpoints={} queries={} errors={:?}",
        r.endpoints.len(),
        r.predictor_queries,
        r.numerical_failures
    );
    assert!(
        r.boundary_coverage_proven(),
        "pending={:?} owners={:?}",
        r.faces
            .iter()
            .map(|f| (f.boundary, f.coverage.pending.len()))
            .collect::<Vec<_>>(),
        r.endpoints.iter().map(|p| p.boundary()).collect::<Vec<_>>()
    );
}
#[test]
fn limits_nonisolated_contacts_and_nearby_samples_do_not_authorize_endpoints() {
    let s = planes();
    let b = Boundary {
        source: 0,
        axis: 0,
        high: true,
    };
    assert!(
        boundary::certify(
            s.each_ref(),
            [0.2, -0.2],
            b,
            [[[0.999; 2], [0.1, 0.3]], [[0.2, 0.8], [0.1, 0.3]]],
            Limits::default()
        )
        .is_err()
    );
    let a = plane();
    let p = boundary::certify(
        [&a, &a],
        [0.1; 2],
        b,
        [[[1.; 2], [0.1, 0.9]], [[0.1, 1.], [0.1, 0.9]]],
        Limits::default(),
    )
    .unwrap();
    assert!(p.witness().is_none() && !p.regular_inward_branch_proven());
    let mut cross = plane();
    for p in cross.control_points.iter_mut().flatten() {
        p.swap(1, 2);
    }
    let corner = boundary::certify(
        [&a, &cross],
        [0.2, -0.2],
        Boundary {
            source: 0,
            axis: 0,
            high: false,
        },
        [[[0.; 2], [0.1, 0.3]], [[0., 0.2], [0.1, 0.3]]],
        Limits::default(),
    )
    .unwrap();
    assert!(corner.witness().is_none());
    let mut l = DiscoveryLimits::default();
    l.predictor_queries = 1;
    l.boxes_per_boundary = 1;
    l.qualification_boxes_per_boundary = 1;
    let r = boundary::discover(s.each_ref(), [0.2, -0.2], l).unwrap();
    assert_eq!(r.faces.len(), 8);
    assert!(r.predictor_queries <= 1);
    assert!(r.faces.iter().any(|f| !f.candidates.pending.is_empty()));
    assert!(!r.boundary_coverage_proven());
    assert!(r.faces.iter().any(|f| !f.coverage.pending.is_empty()));
    l.max_endpoints = 0;
    assert!(boundary::discover(s.each_ref(), [0.2, -0.2], l).is_err());
}
#[test]
fn nonunit_clocks_and_transpose_keep_literal_boundary_and_signed_normal() {
    let mut s = curved();
    s[0] = nurbs_core::surface_edit::transpose(&s[0]).unwrap();
    s[1].knots_v = vec![5., 5., 9., 9.];
    let p = boundary::certify(
        s.each_ref(),
        [-0.1, 0.1],
        Boundary {
            source: 1,
            axis: 1,
            high: true,
        },
        [[[0.975, 0.999], [0.89, 0.92]], [[0.12, 0.15], [9.; 2]]],
        Limits::default(),
    )
    .unwrap();
    assert!(p.regular_inward_branch_proven());
    assert_eq!(p.witness().unwrap().second_uv[1], [9.; 2]);
    p.reverify().unwrap();
}

#[test]
fn full_boundary_absence_is_distinct_from_finding_endpoints_and_validates_limits() {
    let a = plane();
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p[2] = 3.;
    }
    let r = boundary::discover([&a, &b], [0.1; 2], DiscoveryLimits::default()).unwrap();
    assert!(r.boundary_coverage_proven());
    assert!(r.faces.iter().all(|f| f.absence_proven()));
    assert!(r.endpoints.is_empty());
    assert_eq!(r.predictor_queries, 0);
    let mut l = DiscoveryLimits::default();
    l.qualification_boxes_per_boundary = 0;
    assert!(boundary::discover([&a, &b], [0.1; 2], l).is_err());
}
