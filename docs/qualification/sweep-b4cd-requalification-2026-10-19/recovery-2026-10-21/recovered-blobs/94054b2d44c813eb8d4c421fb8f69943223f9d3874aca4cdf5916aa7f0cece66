use brep_core::watertight::{self, Closure};
use nurbs_core::curve::Curve;
#[test]
fn closed_cube_requires_continuous_agreement_and_does_not_trust_shell_flag() {
    let mut model = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    model.shells[0].closed = false;
    model.bodies.clear();
    model.rebuild_topology_ids();
    let before = model.clone();
    let report = watertight::inspect(&model, 10000).unwrap();
    assert_eq!(report.status, Closure::WithinTolerance);
    assert_eq!(report.agreement.uses.len(), 24);
    assert!(report.shells[0].boundary_edges.is_empty());
    assert!(report.agreement.cells <= 10000);
    assert_eq!(model, before);
    assert_eq!(
        watertight::inspect(&model, 1).unwrap().status,
        Closure::Unresolved
    );
    assert!(watertight::inspect(&model, 0).is_err());
    assert!(watertight::inspect(&model, 1000001).is_err());
}
#[test]
fn full_interval_audit_detects_interior_edge_mismatch() {
    let mut model = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let mut curve = model.edges[0].curve.elevate(3).unwrap();
    curve.control_points[1][2] += 0.1;
    curve.control_points[2][2] += 0.1;
    model.edges[0].curve = curve;
    model.rebuild_topology_ids();
    let report = watertight::inspect(&model, 10000).unwrap();
    assert_eq!(report.status, Closure::Mismatch);
    assert!(
        report
            .agreement
            .uses
            .iter()
            .any(|u| u.status == nurbs_core::curve_surface_agreement::Status::Mismatch)
    );
}
#[test]
fn an_open_planar_patch_reports_every_boundary_edge() {
    let p = [
        vec![0., 0., 0.],
        vec![1., 0., 0.],
        vec![1., 1., 0.],
        vec![0., 1., 0.],
    ];
    let wire: Vec<_> = (0..4)
        .map(|i| Curve::from_polyline(vec![p[i].clone(), p[(i + 1) % 4].clone()]).unwrap())
        .collect();
    let model = brep_core::planar_cap::build_sheet(&[wire], 0., 1e-8, 10000).unwrap();
    let report = watertight::inspect(&model, 10000).unwrap();
    assert_eq!(report.status, Closure::OpenBoundary);
    assert_eq!(report.shells[0].boundary_edges.len(), 4);
    assert!(report.agreement.complete);
}

#[test]
fn empty_or_malformed_patch_sets_never_pass() {
    let empty = brep_core::Model::empty(1e-8).unwrap();
    assert_eq!(
        watertight::inspect(&empty, 100).unwrap().status,
        Closure::Unresolved
    );
    let mut bad = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    bad.edges[0].curve.weights[0] = f64::NAN;
    assert!(watertight::inspect(&bad, 100).is_err());
}

fn square_patch(x: f64) -> brep_core::Model {
    let p = [
        vec![x, 0., 0.],
        vec![x + 1., 0., 0.],
        vec![x + 1., 1., 0.],
        vec![x, 1., 0.],
    ];
    let wire = (0..4)
        .map(|i| Curve::from_polyline(vec![p[i].clone(), p[(i + 1) % 4].clone()]).unwrap())
        .collect();
    brep_core::planar_cap::build_sheet(&[wire], 0., 1e-8, 10000).unwrap()
}
#[test]
fn separate_patches_match_only_unique_opposite_boundaries() {
    let models = vec![square_patch(0.), square_patch(1.)];
    let before = models.clone();
    let r = watertight::inspect_patch_set(&models, 100, 10000).unwrap();
    assert_eq!(r.matches.len(), 1);
    assert_eq!(r.unresolved.len(), 6);
    assert!(!r.all_boundaries_paired);
    assert!(r.cells <= 10000);
    let pair = r.matches[0];
    let a = r.boundaries[pair[0]];
    let b = r.boundaries[pair[1]];
    assert_eq!([a.model, b.model], [0, 1]);
    assert_eq!([a.coedge, b.coedge], [1, 3]);
    assert_eq!(models, before);
    let uncertain = watertight::inspect_patch_set(&models, 1, 10000).unwrap();
    assert!(uncertain.matches.is_empty());
    let duplicate = vec![square_patch(0.), square_patch(1.), square_patch(1.)];
    assert!(
        watertight::inspect_patch_set(&duplicate, 100, 10000)
            .unwrap()
            .matches
            .is_empty()
    );
    assert!(watertight::inspect_patch_set(&[], 100, 10000).is_err());
}

#[test]
fn partial_boundaries_are_verified_on_both_original_surfaces() {
    let source = square_patch(0.);
    let mut target = square_patch(1.);
    for v in &mut target.vertices {
        v.point[1] = 0.25 + 0.5 * v.point[1];
    }
    for e in &mut target.edges {
        for p in &mut e.curve.control_points {
            p[1] = 0.25 + 0.5 * p[1];
        }
    }
    for f in &mut target.faces {
        for p in f.surface.control_points.iter_mut().flatten() {
            p[1] = 0.25 + 0.5 * p[1];
        }
    }
    target.rebuild_topology_ids();
    let models = vec![source, target];
    let before = models.clone();
    let a = watertight::BoundaryAddress {
        model: 0,
        face: 0,
        wire: 0,
        coedge: 1,
    };
    let b = watertight::BoundaryAddress {
        model: 1,
        face: 0,
        wire: 0,
        coedge: 3,
    };
    let proof =
        watertight::inspect_boundary_portions(&models, [a, b], [[0.25, 0.75], [0., 1.]], 10000)
            .unwrap();
    assert!(proof.within_tolerance);
    assert_eq!(proof.agreements.len(), 2);
    assert!(proof.cells <= 10000);
    assert_eq!(proof.curve.control_points[0], vec![1., 0.25, 0.]);
    assert_eq!(proof.curve.control_points[1], vec![1., 0.75, 0.]);
    assert!(
        !watertight::inspect_boundary_portions(&models, [a, b], [[0., 0.5], [0., 1.]], 10000)
            .unwrap()
            .within_tolerance
    );
    assert!(
        !watertight::inspect_boundary_portions(&models, [a, b], [[0.25, 0.75], [0., 1.]], 1)
            .unwrap()
            .within_tolerance
    );
    assert!(
        watertight::inspect_boundary_portions(&models, [a, b], [[0.75, 0.25], [0., 1.]], 10000)
            .is_err()
    );
    assert_eq!(models, before);
}
