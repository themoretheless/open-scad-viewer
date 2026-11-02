use nurbs_core::{
    offset_contact_path as path,
    surface::Surface,
    surface_offset::coverage::{self, DiscoveryLimits, Limits},
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
fn cross(wide: bool) -> Surface {
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p.swap(1, 2);
        if wide {
            p[0] = 3. * p[0] - 1.;
        }
    }
    b
}
fn limits() -> DiscoveryLimits {
    DiscoveryLimits {
        candidate_boxes: 4096,
        parameter_tolerance: 0.25,
        predictor_queries: 64,
        max_paths: 8,
        path: path::Limits {
            cells: 64,
            spans: 32,
            iterations: 16,
            numerical_tolerance_mm: 1e-9,
            padding_fraction: 0.1,
        },
        qualification: Limits {
            boxes: 20000,
            spans_per_bound: 32,
            path_cells: 8192,
        },
    }
}
#[test]
fn transverse_paths_are_found_without_caller_seeds_then_independently_replayed() {
    let a = plane();
    let b = cross(true);
    let saved = (a.clone(), b.clone());
    let work = limits();
    let r = coverage::discover([&a, &b], [0.2, -0.2], work).unwrap();
    assert!(
        r.coverage.root_coverage_proven,
        "{} pending={}",
        r.coverage.reason,
        r.coverage.pending.len()
    );
    assert!(!r.coverage.absence_proven);
    assert!(!r.requests.is_empty());
    assert!(
        r.predictor_queries <= work.predictor_queries
            && r.path_cells() <= work.qualification.path_cells
    );
    let replay = coverage::certify([&a, &b], [0.2, -0.2], &r.requests, work.qualification).unwrap();
    assert!(replay.root_coverage_proven);
    assert_eq!((a, b), saved);
}
#[test]
fn separation_needs_no_seed_search_or_numerical_positive_receipt() {
    let a = plane();
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p[2] = 3.;
    }
    let r = coverage::discover([&a, &b], [0.1; 2], limits()).unwrap();
    assert!(r.coverage.absence_proven && r.coverage.root_coverage_proven);
    assert_eq!(r.predictor_queries, 0);
    assert!(r.requests.is_empty() && r.search.is_none());
}
fn two_branches() -> [Surface; 2] {
    let mut a = plane();
    for p in a.control_points.iter_mut().flatten() {
        p[1] = 3. * p[1] - 1.;
    }
    let mut b = plane();
    b.degree_v = 2;
    b.knots_v = vec![0., 0., 0., 1., 1., 1.];
    b.control_points = [-1., 2.]
        .map(|x| vec![vec![x, 0., 0.25], vec![x, 0.5, -0.25], vec![x, 1., 0.25]])
        .to_vec();
    b.weights = vec![vec![1.; 3]; 2];
    [a, b]
}
#[test]
fn both_disconnected_contact_sets_are_found_and_full_root_coverage_is_proved() {
    let s = two_branches();
    let r = coverage::discover(s.each_ref(), [0.1, -0.1], limits()).unwrap();
    assert!(r.requests.len() >= 2);
    assert!(
        r.coverage.root_coverage_proven,
        "{} pending={} requests={}",
        r.coverage.reason,
        r.coverage.pending.len(),
        r.requests.len()
    );
    assert!(!r.coverage.absence_proven);
}
#[test]
fn finding_only_one_of_two_contacts_never_authorizes_complete_coverage() {
    let s = two_branches();
    let mut work = limits();
    work.max_paths = 1;
    let r = coverage::discover(s.each_ref(), [0.1, -0.1], work).unwrap();
    assert_eq!(r.requests.len(), 1);
    assert!(!r.coverage.root_coverage_proven && !r.coverage.absence_proven);
    assert!(!r.coverage.pending.is_empty());
}
#[test]
fn numerical_limits_nonisolated_and_natural_boundary_roots_remain_unresolved() {
    let a = plane();
    let mut work = limits();
    work.predictor_queries = 1;
    work.qualification.boxes = 32;
    let same = coverage::discover([&a, &a], [0.2; 2], work).unwrap();
    assert!(
        !same.coverage.root_coverage_proven
            && !same.coverage.absence_proven
            && !same.coverage.pending.is_empty()
    );
    let b = cross(false);
    work = limits();
    work.predictor_queries = 8;
    work.qualification.boxes = 256;
    let boundary = coverage::discover([&a, &b], [0.2, -0.2], work).unwrap();
    assert!(!boundary.coverage.root_coverage_proven && !boundary.coverage.pending.is_empty());
}
#[test]
fn invalid_discovery_limits_are_not_hidden_by_easy_separation() {
    let a = plane();
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p[2] = 3.;
    }
    let mut work = limits();
    work.predictor_queries = 0;
    assert!(coverage::discover([&a, &b], [0.1; 2], work).is_err());
    work = limits();
    work.qualification.boxes = 0;
    assert!(coverage::discover([&a, &b], [0.1; 2], work).is_err());
}

#[test]
fn nonlinear_root_with_inconclusive_wide_proposal_is_found_within_shared_work() {
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
    let work = limits();
    let r = coverage::discover([&a, &b], [0.1; 2], work).unwrap();
    assert!(!r.requests.is_empty());
    assert!(
        r.requests
            .iter()
            .any(|p| p.limits.padding_fraction < work.path.padding_fraction)
    );
    assert!(r.predictor_queries <= work.predictor_queries);
    assert!(r.path_cells() <= work.qualification.path_cells);
    assert!(r.coverage.paths.iter().all(|p| p.continuous_path_proven));
    assert!(!r.coverage.root_coverage_proven && !r.coverage.pending.is_empty());
}
