use brep_core::{edge_partition, face_crossings, face_cut, feature_ops::freeform};
use nurbs_core::surface::Surface;
fn pair() -> (brep_core::Model, usize, brep_core::Model) {
    let (a, b) = (7. / 16., 13. / 16.);
    let ab = a * b;
    let sum = a + b;
    let z = [ab, ab - sum / 2., ab - sum + 1.];
    let surface = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [0., 0.5, 1.]
            .into_iter()
            .enumerate()
            .map(|(i, x)| vec![vec![x, 0.5, z[i]], vec![x, 1.5, z[i]]])
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let source = freeform::sheet(&surface, 1e-6).unwrap();
    let edge = source
        .edges
        .iter()
        .position(|e| e.curve.degree == 2 && e.curve.control_points.iter().all(|p| p[1] == 0.5))
        .unwrap();
    let plane = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![-0.5, 0., 0.], vec![-0.5, 2., 0.]],
            vec![vec![1.5, 0., 0.], vec![1.5, 2., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    (source, edge, freeform::sheet(&plane, 1e-6).unwrap())
}
#[test]
fn automatic_edge_trim_consumes_both_original_roots_once_and_preserves_lineage() {
    let (source, edge, cutter) = pair();
    let before = (source.clone(), cutter.clone());
    let r = face_crossings::inspect(&source, edge, &cutter, 0, 1e-8, 100000).unwrap();
    assert!(r.native_complete && r.complete, "{r:?}");
    assert_eq!(r.roots.len(), 2);
    assert_eq!(r.eligible.len(), 2);
    assert!(r.roots.iter().all(|r| r.root.existence_domain.is_some()));
    let result =
        edge_partition::partition_all_at_face(&source, edge, &cutter, 0, 1e-8, 100000).unwrap();
    assert_eq!(result.vertices.len(), 2);
    assert_eq!(result.edges.len(), 3);
    result.model.validate().unwrap();
    assert!(result.model.persistent_naming_complete());
    assert_eq!(result.model.edges.len(), source.edges.len() + 2);
    assert_eq!(before, (source, cutter));
}
#[test]
fn automatic_face_trim_builds_both_boundary_connected_branches_atomically() {
    let (source, _, cutter) = pair();
    let before = (source.clone(), cutter.clone());
    let result =
        face_cut::partition_fiber_branches_at_face(&source, 0, &cutter, 0, 1e-8, 100000, 1000000)
            .unwrap();
    assert_eq!(result.branches.len(), 2);
    assert_eq!(result.faces.len(), 3);
    assert!(result.boundary_coverage.iter().all(|(_, r)| r.complete));
    result.model.validate().unwrap();
    assert!(result.model.persistent_naming_complete());
    assert_eq!(before, (source, cutter));
}
#[test]
fn failed_automatic_trim_keeps_both_inputs_unchanged() {
    let (source, edge, cutter) = pair();
    let before = (source.clone(), cutter.clone());
    assert!(edge_partition::partition_all_at_face(&source, edge, &cutter, 0, 1e-8, 1).is_err());
    assert!(
        face_cut::partition_fiber_branches_at_face(&source, 0, &cutter, 0, 1e-8, 32, 1000000)
            .is_err()
    );
    assert_eq!(before, (source, cutter));
}

#[path = "../examples/support/fiber_cut_fixture.rs"]
mod curved_fixture;
#[test]
fn curved_cut_roots_keep_original_local_spatial_precision() {
    let (source, cutter) = curved_fixture::pair();
    let mut roots = 0;
    for edge in &source.edges {
        let r = nurbs_core::intersection::certified_curve_surface::intersections(
            &edge.curve,
            &cutter.faces[0].surface,
            10000,
        )
        .unwrap();
        assert!(r.complete, "{r:?}");
        for root in r.roots {
            roots += 1;
            let width = root
                .position_bounds
                .iter()
                .map(|b| (b[1] - b[0]).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(width < source.tolerance_mm / 2., "{width}: {root:?}");
        }
    }
    assert_eq!(roots, 2);
}
