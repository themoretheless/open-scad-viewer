use nurbs_core::{
    offset_contact_path as path,
    surface::Surface,
    surface_offset::{
        boundary_contact::{
            self as boundary, Boundary, Limits as B,
            attachment::{self, Limits},
        },
        coverage::{self, PathRequest},
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
fn planes() -> [Surface; 2] {
    let a = plane();
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p.swap(1, 2);
        p[0] = 3. * p[0] - 1.;
    }
    [a, b]
}
fn work() -> Limits {
    Limits {
        coverage: coverage::Limits {
            boxes: 128,
            spans_per_bound: 128,
            path_cells: 1024,
        },
        connection_steps: 10000,
        section_queries: 10000,
        spans_per_query: 128,
    }
}
fn path_limits() -> path::Limits {
    path::Limits {
        cells: 128,
        spans: 128,
        iterations: 16,
        numerical_tolerance_mm: 1e-9,
        padding_fraction: 0.002,
    }
}
fn request(drive: [f64; 2]) -> PathRequest {
    PathRequest {
        axis: 0,
        drive,
        seeds: drive.map(|t| [[t, 0.2], [(t + 1.) / 3., 0.2]]),
        limits: path_limits(),
    }
}
fn bound() -> Boundary {
    Boundary {
        source: 0,
        axis: 0,
        high: true,
    }
}
fn tube() -> [[[f64; 2]; 2]; 2] {
    [[[1.; 2], [0.1, 0.3]], [[0.5, 0.8], [0.1, 0.3]]]
}
#[test]
fn actual_common_root_connects_literal_endpoint_to_entire_fresh_path() {
    let s = planes();
    let saved = s.clone();
    let l = work();
    let r = attachment::certify(
        s.each_ref(),
        [0.2, -0.2],
        bound(),
        tube(),
        B::default(),
        &[request([0.98, 1.])],
        l,
    )
    .unwrap();
    assert!(r.endpoint.regular_inward_branch_proven());
    assert!(r.coverage.paths[0].continuous_path_proven);
    assert_eq!(r.connections.len(), 1);
    assert_eq!(r.connections[0].path, 0);
    assert!(r.unresolved_paths.is_empty() && r.disjoint_from_collar.is_empty());
    assert!(r.connections[0].witness.first_uv[0][0] >= 0.99);
    assert!(r.connection_steps <= l.connection_steps && r.section_queries <= l.section_queries);
    assert_eq!(s, saved);
}
#[test]
fn second_source_boundary_keeps_original_uv_identity() {
    let s = planes();
    let drive = [0.6635, 0.665];
    let req = PathRequest {
        axis: 0,
        drive,
        seeds: drive.map(|t| [[t, 0.2], [3. * t - 1., 0.2]]),
        limits: path_limits(),
    };
    let b = Boundary {
        source: 1,
        axis: 0,
        high: true,
    };
    let t = tube();
    let r = attachment::certify(
        [&s[1], &s[0]],
        [-0.2, 0.2],
        b,
        [t[1], t[0]],
        B::default(),
        &[req],
        work(),
    )
    .unwrap();
    assert_eq!(r.connections.len(), 1);
    let w = &r.connections[0].witness;
    assert!(w.second_uv[0][0] >= 0.99 && w.second_uv[0][1] <= 1.);
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
fn nonlinear_end_collar_attaches_interior_original_contact_path() {
    let s = curved();
    let drive = [0.986, 0.99];
    let req = PathRequest {
        axis: 1,
        drive,
        seeds: drive.map(|t| [[0.905, t], [0.134, t + 0.006]]),
        limits: path_limits(),
    };
    let r = attachment::certify(
        s.each_ref(),
        [0.1; 2],
        Boundary {
            source: 1,
            axis: 1,
            high: true,
        },
        [[[0.89, 0.92], [0.975, 0.999]], [[0.12, 0.15], [1.; 2]]],
        B::default(),
        &[req],
        work(),
    )
    .unwrap();
    assert!(r.endpoint.regular_inward_branch_proven());
    assert!(r.coverage.paths[0].continuous_path_proven);
    assert_eq!(r.connections.len(), 1);
    assert!(r.connections[0].witness.second_uv[1][0] >= 0.99);
}
#[test]
fn missing_middle_arc_and_shared_work_exhaustion_do_not_forge_a_link() {
    let s = planes();
    let r = attachment::certify(
        s.each_ref(),
        [0.2, -0.2],
        bound(),
        tube(),
        B::default(),
        &[request([0.1, 0.2])],
        work(),
    )
    .unwrap();
    assert!(r.connections.is_empty());
    assert_eq!(r.disjoint_from_collar, vec![0]);
    assert!(!r.coverage.root_coverage_proven && !r.coverage.pending.is_empty());
    let mut l = work();
    l.connection_steps = 1;
    let r = attachment::certify(
        s.each_ref(),
        [0.2, -0.2],
        bound(),
        tube(),
        B::default(),
        &[request([0.98, 1.])],
        l,
    )
    .unwrap();
    assert!(r.connections.is_empty());
    assert_eq!(r.unresolved_paths, vec![0]);
    assert_eq!(r.connection_steps, 1);
    let mut l = work();
    l.section_queries = 1;
    let req = request([0.98, 1.]);
    let r = attachment::certify(
        s.each_ref(),
        [0.2, -0.2],
        bound(),
        tube(),
        B::default(),
        &[req, req],
        l,
    )
    .unwrap();
    assert_eq!(r.connections.len(), 1);
    assert_eq!(r.unresolved_paths, vec![1]);
    assert_eq!(r.section_queries, 1);
}
#[test]
fn coincident_world_branches_on_different_original_uv_sheets_are_not_welded() {
    let mut a = plane();
    for p in a.control_points.iter_mut().flatten() {
        p[0] *= 0.6;
    }
    let mut b = plane();
    b.degree_u = 3;
    b.knots_u = vec![0., 0., 0., 0., 1., 1., 1., 1.];
    b.control_points = [0., 2., -1., 1.]
        .map(|x| vec![vec![x, 0.5, 0.], vec![x, 0.5, 1.]])
        .to_vec();
    b.weights = vec![vec![1.; 2]; 4];
    let drive = [0.995, 1.];
    let req = PathRequest {
        axis: 0,
        drive,
        seeds: drive.map(|t| [[t, 0.3], [0.918, 0.2]]),
        limits: path_limits(),
    };
    let r = attachment::certify(
        [&a, &b],
        [0.2; 2],
        bound(),
        [[[1.; 2], [0.28, 0.32]], [[0.145, 0.155], [0.19, 0.21]]],
        B::default(),
        &[req],
        work(),
    )
    .unwrap();
    assert!(r.endpoint.regular_inward_branch_proven());
    assert!(r.coverage.paths[0].continuous_path_proven);
    assert!(r.connections.is_empty());
    assert_eq!(r.disjoint_from_collar, vec![0]);
    let mut centers = vec![];
    for u in [0.152, 0.918] {
        let p = nurbs_core::offset_contact_predictor::propose(
            [&a, &b],
            [0.2; 2],
            [[0.997, 0.3], [u, 0.2]],
            0,
            1e-10,
            16,
            0.001,
            128,
        )
        .unwrap();
        assert!(p.residual_mm <= 1e-10);
        centers.push(p.center);
    }
    assert!(
        centers[0]
            .iter()
            .zip(centers[1])
            .all(|(a, b)| (a - b).abs() < 1e-9)
    );
    let bridges = r
        .endpoint
        .bridge_paths(&[req], attachment::bridge::Limits::default())
        .unwrap();
    assert!(bridges.attempts.iter().all(|a| !a.connected()));
}

#[test]
fn automatic_bridge_fills_a_missing_arc_only_after_both_root_links() {
    let s = planes();
    let req = request([0.1, 0.2]);
    let p = boundary::certify(s.each_ref(), [0.2, -0.2], bound(), tube(), B::default()).unwrap();
    let mut l = attachment::bridge::Limits::default();
    l.max_attempts = 1;
    let r = p.bridge_paths(&[req, req], l).unwrap();
    assert_eq!(r.attempts.len(), 1);
    let a = &r.attempts[0];
    assert!(a.connected(), "{}", a.reason());
    assert!(a.request().unwrap().drive[0] < 0.2 && a.request().unwrap().drive[1] > 0.99);
    assert!(a.path_relation().unwrap().connections.len() > 0);
    assert!(a.endpoint_relation().unwrap().connections.len() > 0);
    assert!(r.anchor_queries <= l.anchor_refinements + 1);
    r.reverify().unwrap();
    l.path_links.connection_steps = 1;
    let r = p.bridge_paths(&[req], l).unwrap();
    assert!(r.attempts.iter().all(|a| !a.connected()));
    l.max_attempts = 0;
    assert!(p.bridge_paths(&[req], l).is_err());
}

#[test]
fn automatic_nonlinear_bridge_reaches_original_interior_without_authored_anchor() {
    let s = curved();
    let drive = [0.35, 0.55];
    let req = PathRequest {
        axis: 1,
        drive,
        seeds: drive.map(|t| [[0.92, t], [0.12, t + 0.005]]),
        limits: path_limits(),
    };
    let p = boundary::certify(
        s.each_ref(),
        [0.1; 2],
        Boundary {
            source: 1,
            axis: 1,
            high: true,
        },
        [[[0.89, 0.92], [0.975, 0.999]], [[0.12, 0.15], [1.; 2]]],
        B::default(),
    )
    .unwrap();
    let r = p
        .bridge_paths(&[req], attachment::bridge::Limits::default())
        .unwrap();
    assert!(
        r.attempts.iter().any(|a| a.connected()),
        "{:?}",
        r.attempts.iter().map(|a| a.reason()).collect::<Vec<_>>()
    );
    assert!(!r.initial_paths.root_coverage_proven);
    r.reverify().unwrap();
}
