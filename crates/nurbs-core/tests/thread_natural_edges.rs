use nurbs_core::{curve_surface_agreement, thread};

#[test]
fn diagonal_thread_trim_edges_use_verified_degree_six_traces() {
    let spec = thread::Spec {
        error_budget: 1e-2,
        ..thread::Spec::default()
    };
    let trims = thread::trim_candidates(spec, [1. / 32., 1. / 8.], 1e-10).unwrap();
    let c = trims
        .candidates
        .iter()
        .find(|c| c.patch.quarter == 0 && c.patch.profile_segment == 0)
        .unwrap();
    let edges = c.mapped_edges(1e-12, 1e-8, 1, 100000).unwrap();
    assert!(edges.all_edges_within_tolerance);
    let diagonal = edges
        .edges
        .iter()
        .filter(|e| {
            e.pcurve.control_points[0][0] != e.pcurve.control_points[1][0]
                && e.pcurve.control_points[0][1] != e.pcurve.control_points[1][1]
        })
        .collect::<Vec<_>>();
    assert_eq!(diagonal.len(), 2);
    for e in diagonal {
        assert_eq!(e.curve.degree, 6);
        assert_eq!(e.curve.control_points.len(), 7);
        assert_eq!(
            e.agreement.status,
            curve_surface_agreement::Status::WithinTolerance
        );
    }
    for i in 0..edges.edges.len() {
        assert_eq!(
            edges.edges[i].curve.control_points.last(),
            edges.edges[(i + 1) % edges.edges.len()]
                .curve
                .control_points
                .first()
        );
    }
    assert!(
        !c.mapped_edges(1e-12, 1e-8, 1, 0)
            .unwrap()
            .all_edges_within_tolerance
    );
}

#[test]
fn partial_natural_edges_keep_quintic_degree_and_closed_authored_vertices() {
    let trims =
        thread::trim_candidates(thread::Spec::default(), [1. / 32., 1. / 8.], 1e-10).unwrap();
    let c = trims
        .candidates
        .iter()
        .find(|c| c.patch.quarter == 0 && c.patch.profile_segment == 0)
        .unwrap();
    let result = c.mapped_edges(1e-12, 1e-3, 128, 100000).unwrap();
    assert!(result.all_edges_within_tolerance);
    let iso = result
        .edges
        .iter()
        .filter(|e| {
            e.pcurve
                .control_points
                .iter()
                .all(|p| p[0] == 0. || p[0] == 1.)
                && e.pcurve.control_points[0][0] == e.pcurve.control_points[1][0]
        })
        .collect::<Vec<_>>();
    assert_eq!(iso.len(), 2);
    assert!(
        iso.iter()
            .all(|e| e.curve.degree == c.patch.surface.degree_v)
    );
    assert!(
        iso.iter()
            .all(|e| e.curve.control_points.len() <= c.patch.surface.control_points[0].len())
    );
    for i in 0..result.edges.len() {
        assert_eq!(
            result.edges[i].curve.control_points.last(),
            result.edges[(i + 1) % result.edges.len()]
                .curve
                .control_points
                .first()
        );
    }
}

#[test]
fn clamped_iso_endpoints_copy_original_weighted_rows_and_columns() {
    use nurbs_core::surface::{Axis, Surface};
    let s = Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![vec![0.1, 0., 0.], vec![0.2, 1., 0.], vec![0.3, 2., 0.]],
            vec![vec![1.1, 0., 1.], vec![1.2, 1., 2.], vec![1.3, 2., 3.]],
        ],
        weights: vec![vec![1.3, 2.7, 4.5], vec![7.25, 13.5, 2.25]],
        periodic_u: false,
        periodic_v: false,
    };
    for (parameter, row) in [(0., 0), (1., 1)] {
        let c = s.iso(Axis::U, parameter).unwrap();
        assert_eq!(c.control_points, s.control_points[row]);
        assert_eq!(c.weights, s.weights[row]);
        assert_eq!(c.knots, s.knots_v);
    }
    for (parameter, column) in [(0., 0), (1., 2)] {
        let c = s.iso(Axis::V, parameter).unwrap();
        assert_eq!(
            c.control_points,
            s.control_points
                .iter()
                .map(|r| r[column].clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            c.weights,
            s.weights.iter().map(|r| r[column]).collect::<Vec<_>>()
        );
        assert_eq!(c.knots, s.knots_u);
    }
}
#[test]
fn full_natural_thread_edges_retain_surface_definitions_and_exact_identity() {
    let trims = thread::trim_candidates(
        thread::Spec {
            error_budget: 1e-2,
            ..thread::Spec::default()
        },
        [0., 2.],
        1e-10,
    )
    .unwrap();
    let c = &trims.candidates[0];
    let edges = c.mapped_edges(1e-12, 1e-6, 1, 100000).unwrap();
    assert!(edges.all_edges_within_tolerance);
    assert_eq!(edges.edges.len(), 4);
    assert!(
        edges
            .edges
            .iter()
            .any(|e| e.curve.degree == c.patch.surface.degree_v)
    );
    for e in edges.edges {
        let report = curve_surface_agreement::verify_exact(
            &e.curve,
            &e.pcurve,
            &c.patch.surface,
            false,
            1000000,
        )
        .unwrap()
        .unwrap();
        assert_eq!(report.outcome, cad_predicates::BezierIdentity::Equal);
    }
}
