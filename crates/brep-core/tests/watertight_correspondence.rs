use brep_core::{
    Model,
    watertight::{self, BoundaryAddress},
};
use nurbs_core::{curve::Curve, surface::Surface};
fn patch(x: f64) -> Model {
    let points = [
        vec![x, 0., 0.],
        vec![x + 1., 0., 0.],
        vec![x + 1., 1., 0.],
        vec![x, 1., 0.],
    ];
    let wire = (0..4)
        .map(|i| {
            Curve::from_polyline(vec![points[i].clone(), points[(i + 1) % 4].clone()]).unwrap()
        })
        .collect();
    brep_core::planar_cap::build_sheet(&[wire], 0., 1e-4, 10000).unwrap()
}
fn pair() -> [BoundaryAddress; 2] {
    [
        BoundaryAddress {
            model: 0,
            face: 0,
            wire: 0,
            coedge: 1,
        },
        BoundaryAddress {
            model: 1,
            face: 0,
            wire: 0,
            coedge: 3,
        },
    ]
}
fn nonlinear_target() -> Model {
    let mut m = patch(1.);
    // Same rectangle image, original target face has y(v)=.25v+.75v^2.
    // Bottom/right/top pcurves are adjusted only by their natural normalized
    // planar coordinates; the matched left boundary is v=1-t.
    let f = &mut m.faces[0];
    let old = f.surface.clone();
    f.surface = Surface {
        degree_u: old.degree_u,
        degree_v: 2,
        knots_u: old.knots_u.clone(),
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: old
            .control_points
            .iter()
            .map(|row| {
                let a = &row[0];
                let b = &row[1];
                vec![
                    a.clone(),
                    vec![a[0], a[1] + 0.125 * (b[1] - a[1]), a[2]],
                    b.clone(),
                ]
            })
            .collect(),
        weights: vec![vec![1.; 3]; old.control_points.len()],
        periodic_u: false,
        periodic_v: false,
    };
    // build_sheet's UV domain is unit. Matched left pcurve traverses v=1..0.
    m.rebuild_topology_ids();
    m
}
#[test]
fn inferred_nonlinear_seam_is_verified_on_original_surfaces() {
    let models = vec![patch(0.), nonlinear_target()];
    let before = models.clone();
    assert!(
        !watertight::infer_boundary_correspondence(&models, pair(), 128, 10000)
            .unwrap()
            .within_tolerance
    );
    let r = watertight::infer_boundary_correspondence(&models, pair(), 256, 10000).unwrap();
    assert!(r.within_tolerance, "{r:?}");
    assert_eq!(r.agreements.len(), 2);
    assert!(r.cells <= 10000);
    assert_eq!(r.nodes[0], [0., 1.]);
    assert_eq!(*r.nodes.last().unwrap(), [1., 0.]);
    assert!(r.nodes.iter().any(|n| (n[0] + n[1] - 1.).abs() > 0.1));
    for n in &r.nodes {
        let v = 1. - n[1];
        assert!((n[0] - (0.25 * v + 0.75 * v * v)).abs() < 1e-8);
    }
    assert_eq!(models, before);
    assert!(
        !watertight::inspect_boundary_correspondence(&models, pair(), &[[0., 1.], [1., 0.]], 10000)
            .unwrap()
            .within_tolerance
    );
    assert!(
        !watertight::infer_boundary_correspondence(&models, pair(), 256, 1)
            .unwrap()
            .within_tolerance
    );
}
#[test]
fn reversed_source_coedge_uses_original_domain_without_reverse_knot_rounding() {
    let mut a = patch(0.);
    let edge = a.loops[0].coedges[1].edge;
    a.edges[edge].curve = a.edges[edge].curve.reverse().unwrap();
    a.edges[edge].vertices.swap(0, 1);
    a.loops[0].coedges[1].reversed = true;
    a.rebuild_topology_ids();
    let models = vec![a, nonlinear_target()];
    let r = watertight::infer_boundary_correspondence(&models, pair(), 256, 10000).unwrap();
    assert!(r.within_tolerance, "{r:?}");
}
#[test]
fn mismatch_limits_and_invalid_addresses_never_admit() {
    let models = vec![patch(0.), patch(2.)];
    assert!(
        !watertight::infer_boundary_correspondence(&models, pair(), 16, 10000)
            .unwrap()
            .within_tolerance
    );
    for (segments, work) in [(0, 100), (1025, 100), (16, 0), (16, 1000001)] {
        assert!(
            watertight::infer_boundary_correspondence(&models, pair(), segments, work).is_err()
        );
    }
    let mut address = pair();
    address[1].model = 0;
    assert!(watertight::infer_boundary_correspondence(&models, address, 16, 100).is_err());
    assert!(
        watertight::inspect_boundary_correspondence(&models, pair(), &[[0.1, 1.], [1., 0.]], 100)
            .is_err()
    );
}
