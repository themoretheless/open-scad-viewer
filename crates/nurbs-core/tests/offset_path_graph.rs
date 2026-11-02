use nurbs_core::{
    offset_contact_path as path,
    surface::Surface,
    surface_offset::coverage::{
        self, DiscoveryLimits, Limits as CoverageLimits, PathRequest,
        graph::{self, Limits},
    },
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
fn cross() -> Surface {
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p.swap(1, 2);
        p[0] = 3. * p[0] - 1.;
    }
    b
}
fn path_limits() -> path::Limits {
    path::Limits {
        cells: 64,
        spans: 32,
        iterations: 16,
        numerical_tolerance_mm: 1e-9,
        padding_fraction: 0.1,
    }
}
fn limits() -> Limits {
    Limits {
        coverage: CoverageLimits {
            boxes: 20000,
            spans_per_bound: 32,
            path_cells: 8192,
        },
        connection_steps: 10000,
        section_queries: 10000,
        spans_per_query: 32,
    }
}
fn request(drive: [f64; 2]) -> PathRequest {
    PathRequest {
        axis: 0,
        drive,
        seeds: drive.map(|u| [[u, 0.2], [(u + 1.) / 3., 0.2]]),
        limits: path_limits(),
    }
}
#[test]
fn adjacent_closed_drive_intervals_join_by_a_certified_common_endpoint_root() {
    let a = plane();
    let b = cross();
    let requests = [request([0., 0.5]), request([0.5, 1.])];
    let r = graph::certify([&a, &b], [0.2, -0.2], &requests, limits()).unwrap();
    assert!(r.coverage.root_coverage_proven);
    assert_eq!(r.component_count, Some(1));
    assert_eq!(r.connections.len(), 1);
    assert_eq!(r.connections[0].witness.first_uv[0], [0.5; 2]);
}
#[test]
fn duplicate_paths_are_not_counted_as_distinct_components() {
    let a = plane();
    let b = cross();
    let saved = (a.clone(), b.clone());
    let q = request([0., 1.]);
    let r = graph::certify([&a, &b], [0.2, -0.2], &[q, q, q], limits()).unwrap();
    assert_eq!(r.component_count, Some(1));
    assert_eq!(r.connected_groups.len(), 1);
    assert_eq!(r.connected_groups[0].len(), 3);
    assert_eq!(r.connections.len(), 2);
    assert_eq!((a, b), saved);
}
#[test]
fn overlapping_paths_with_different_driving_axes_share_a_fresh_root() {
    let mut a = plane();
    for p in a.control_points.iter_mut().flatten() {
        p[1] = 3. * p[1] - 1.;
    }
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p[0] = 3. * p[0] - 1.;
        p[1] = 3. * p[1] - 1.;
        p[2] = p[1] - p[0];
    }
    let first = PathRequest {
        axis: 0,
        drive: [0., 1.],
        seeds: [[[0., 0.5], [0.5; 2]], [[1., 0.5], [0.5; 2]]],
        limits: path_limits(),
    };
    let second = PathRequest {
        axis: 1,
        drive: [0.4, 0.55],
        seeds: [[[0.5, 0.4], [0.5; 2]], [[0.5, 0.55], [0.5; 2]]],
        limits: path_limits(),
    };
    let r = graph::certify([&a, &b], [0.1; 2], &[first, second], limits()).unwrap();
    assert_eq!(r.component_count, Some(1));
    assert_eq!(r.connections.len(), 1);
    assert!(
        r.connections[0].witness.first_uv[1][0] >= 0.4
            && r.connections[0].witness.first_uv[1][1] <= 0.55
    );
}
#[test]
fn automatically_discovered_disjoint_contact_sets_have_two_parameter_components() {
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
    let found = coverage::discover(
        [&a, &b],
        [0.1, -0.1],
        DiscoveryLimits {
            candidate_boxes: 4096,
            parameter_tolerance: 0.25,
            predictor_queries: 64,
            max_paths: 8,
            path: path_limits(),
            qualification: limits().coverage,
        },
    )
    .unwrap();
    let r = graph::certify([&a, &b], [0.1, -0.1], &found.requests, limits()).unwrap();
    assert_eq!(r.component_count, Some(2));
    assert_eq!(r.connected_groups.len(), 2);
    assert!(r.unresolved_pairs.is_empty());
}
#[test]
fn a_missing_middle_arc_cannot_turn_two_partial_groups_into_a_component_count() {
    let a = plane();
    let b = cross();
    let r = graph::certify(
        [&a, &b],
        [0.2, -0.2],
        &[request([0., 0.4]), request([0.6, 1.])],
        limits(),
    )
    .unwrap();
    assert!(!r.coverage.root_coverage_proven);
    assert_eq!(r.component_count, None);
    assert_eq!(r.connected_groups.len(), 2);
    assert!(!r.coverage.pending.is_empty());
}
#[test]
fn exhausted_connectivity_work_keeps_ambiguity_even_when_root_coverage_is_complete() {
    let a = plane();
    let b = cross();
    let q = request([0., 1.]);
    let mut work = limits();
    work.connection_steps = 1;
    let r = graph::certify([&a, &b], [0.2, -0.2], &[q, q, q], work).unwrap();
    assert!(r.coverage.root_coverage_proven);
    assert_eq!(r.component_count, None);
    assert!(!r.unresolved_pairs.is_empty());
    assert!(
        r.connection_steps <= work.connection_steps && r.section_queries <= work.section_queries
    );
}
#[test]
fn absence_nonisolated_and_invalid_work_do_not_create_fake_components() {
    let a = plane();
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p[2] = 3.;
    }
    assert_eq!(
        graph::certify([&a, &b], [0.1; 2], &[], limits())
            .unwrap()
            .component_count,
        Some(0)
    );
    let mut work = limits();
    work.coverage.boxes = 32;
    assert_eq!(
        graph::certify([&a, &a], [0.1; 2], &[], work)
            .unwrap()
            .component_count,
        None
    );
    work = limits();
    work.section_queries = 0;
    assert!(graph::certify([&a, &b], [0.1; 2], &[], work).is_err());
}
