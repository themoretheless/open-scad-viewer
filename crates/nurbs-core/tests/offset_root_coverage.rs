use nurbs_core::{
    offset_contact_path as path,
    surface::Surface,
    surface_offset::coverage::{self, Limits, PathRequest},
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
fn request(drive: [f64; 2]) -> PathRequest {
    PathRequest {
        axis: 0,
        drive,
        seeds: drive.map(|u| [[u, 0.2], [(u + 1.) / 3., 0.2]]),
        limits: path::Limits {
            cells: 32,
            spans: 16,
            iterations: 8,
            numerical_tolerance_mm: 1e-9,
            padding_fraction: 0.1,
        },
    }
}
fn limits(boxes: usize) -> Limits {
    Limits {
        boxes,
        spans_per_bound: 16,
        path_cells: 128,
    }
}
#[test]
fn all_natural_parameter_pairs_are_excluded_or_owned_by_a_fresh_path() {
    let a = plane();
    let b = cross(true);
    let saved = (a.clone(), b.clone());
    let r = coverage::certify([&a, &b], [0.2, -0.2], &[request([0., 1.])], limits(10000)).unwrap();
    assert!(r.paths[0].continuous_path_proven);
    assert!(
        r.root_coverage_proven,
        "{} pending={} visited={}",
        r.reason,
        r.pending.len(),
        r.visited_boxes
    );
    assert!(!r.absence_proven);
    assert!(r.pending.is_empty());
    assert!(!r.covered.is_empty());
    assert!(r.excluded_boxes > 0);
    assert!(
        r.covered
            .iter()
            .all(|c| c.path == 0 && c.segment < r.paths[0].cells.len())
    );
    assert_eq!((a, b), saved);
}
#[test]
fn missing_path_and_work_exhaustion_preserve_the_full_unexcluded_product() {
    let a = plane();
    let b = cross(true);
    let r = coverage::certify([&a, &b], [0.2, -0.2], &[], limits(1)).unwrap();
    assert!(!r.root_coverage_proven && !r.absence_proven);
    assert_eq!(r.pending.len(), 2);
    let volume: f64 = r
        .pending
        .iter()
        .map(|c| c.iter().flatten().map(|d| d[1] - d[0]).product::<f64>())
        .sum();
    assert_eq!(volume, 1.);
    assert_eq!(r.visited_boxes, 1);
}
#[test]
fn a_proved_middle_segment_does_not_cover_unsearched_branch_ends() {
    let a = plane();
    let b = cross(true);
    let r = coverage::certify(
        [&a, &b],
        [0.2, -0.2],
        &[request([0.25, 0.75])],
        limits(2048),
    )
    .unwrap();
    assert!(r.paths[0].continuous_path_proven);
    assert!(!r.root_coverage_proven && !r.absence_proven);
    assert!(!r.pending.is_empty());
}
#[test]
fn complete_separation_proves_absence_without_a_positive_path_receipt() {
    let a = plane();
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p[2] = 3.;
    }
    let r = coverage::certify([&a, &b], [0.2; 2], &[], limits(1)).unwrap();
    assert!(r.absence_proven && r.root_coverage_proven);
    assert_eq!(r.excluded_boxes, 1);
    assert!(r.covered.is_empty() && r.pending.is_empty());
}
#[test]
fn natural_boundary_roots_and_nonisolated_contacts_are_not_dropped() {
    let a = plane();
    let b = cross(false);
    let mut req = request([0., 1.]);
    req.seeds = [[[0., 0.2], [0., 0.2]], [[1., 0.2], [1., 0.2]]];
    let r = coverage::certify([&a, &b], [0.2, -0.2], &[req], limits(256)).unwrap();
    assert!(!r.root_coverage_proven && !r.pending.is_empty());
    let coincident = coverage::certify([&a, &a], [0.2; 2], &[], limits(32)).unwrap();
    assert!(
        !coincident.root_coverage_proven
            && !coincident.absence_proven
            && !coincident.pending.is_empty()
    );
}
#[test]
fn path_work_is_shared_and_invalid_later_seeds_cannot_hide_behind_exhaustion() {
    let a = plane();
    let b = cross(true);
    let req = request([0., 1.]);
    let r = coverage::certify(
        [&a, &b],
        [0.2, -0.2],
        &[req, req],
        Limits {
            path_cells: 2,
            ..limits(8)
        },
    )
    .unwrap();
    assert_eq!(r.path_cells, 2);
    assert_eq!(r.paths.len(), 2);
    let mut bad = req;
    bad.seeds[0][1][0] = -1.;
    assert!(
        coverage::certify(
            [&a, &b],
            [0.2, -0.2],
            &[req, bad],
            Limits {
                path_cells: 2,
                ..limits(8)
            }
        )
        .is_err()
    );
}
