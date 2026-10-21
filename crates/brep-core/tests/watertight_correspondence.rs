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

#[test]
fn original_subranges_preserve_fractional_ends_and_do_not_imply_full_coverage() {
    let models = vec![patch(0.), patch(1.)];
    let before = models.clone();
    let nodes = [[0.1, 0.1, 0.9, 0.9], [0.7, 0.7, 0.3, 0.3]];
    let r = watertight::inspect_boundary_subranges(&models, pair(), &nodes, 10000).unwrap();
    assert!(r.within_tolerance, "{r:?}");
    assert_eq!(r.nodes, nodes);
    assert_eq!(r.agreements.len(), 3);
    assert!(r.edge_pair_distance_upper.unwrap() <= 2. * models[0].tolerance_mm);
    assert_eq!(models, before);
    // Partial admission must not weaken the original full-boundary contract.
    assert!(
        watertight::inspect_boundary_correspondence(
            &models,
            pair(),
            &[[0.1, 0.9], [0.7, 0.3]],
            10000,
        )
        .is_err()
    );
}

#[test]
fn original_subranges_check_independent_target_edge_not_only_its_face_lift() {
    let mut target = patch(1.);
    let edge = target.loops[0].coedges[3].edge;
    // Preserve both endpoints while moving the original curve's interior off
    // its support. Endpoint-only or source-to-two-lifts checks miss this.
    let old = target.edges[edge].curve.clone();
    let a = old.evaluate(old.domain()[0]).unwrap().point;
    let b = old.evaluate(old.domain()[1]).unwrap().point;
    target.edges[edge].curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![a.clone(), vec![a[0] + 0.2, (a[1] + b[1]) / 2., a[2]], b],
        weights: vec![1.; 3],
        periodic: false,
    };
    target.rebuild_topology_ids();
    let models = vec![patch(0.), target];
    let r = watertight::inspect_boundary_subranges(
        &models,
        pair(),
        &[[0.1, 0.1, 0.9, 0.9], [0.7, 0.7, 0.3, 0.3]],
        10000,
    )
    .unwrap();
    assert!(!r.within_tolerance);
    assert_eq!(r.agreements.len(), 3);
    assert!(r.edge_pair_distance_upper.is_none());
}

#[test]
fn original_subranges_allow_independent_edge_and_pcurve_parameterizations() {
    let mut target = patch(1.);
    let edge = target.loops[0].coedges[3].edge;
    let old = target.edges[edge].curve.clone();
    let a = old.evaluate(old.domain()[0]).unwrap().point;
    let b = old.evaluate(old.domain()[1]).unwrap().point;
    target.edges[edge].curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            a.clone(),
            (0..3).map(|k| a[k] + 0.125 * (b[k] - a[k])).collect(),
            b,
        ],
        weights: vec![1.; 3],
        periodic: false,
    };
    target.rebuild_topology_ids();
    let models = vec![patch(0.), target];
    let nodes: Vec<_> = (0..=128)
        .map(|i| {
            let q = 0.75 - 0.5 * i as f64 / 128.;
            let p = 0.25 * q + 0.75 * q * q;
            [1. - p, 1. - p, q, p]
        })
        .collect();
    let r = watertight::inspect_boundary_subranges(&models, pair(), &nodes, 10000).unwrap();
    assert!(r.within_tolerance, "{r:?}");
    assert!(r.cells <= 10000);
    let identity: Vec<_> = nodes.iter().map(|n| [n[0], n[1], n[3], n[3]]).collect();
    assert!(
        !watertight::inspect_boundary_subranges(&models, pair(), &identity, 10000)
            .unwrap()
            .within_tolerance
    );
}

#[test]
fn original_subranges_reject_invalid_maps_and_exhaustion_without_authoring() {
    let models = vec![patch(0.), patch(1.)];
    let nodes = [[0.1, 0.1, 0.9, 0.9], [0.7, 0.7, 0.3, 0.3]];
    let r = watertight::inspect_boundary_subranges(&models, pair(), &nodes, 1).unwrap();
    assert!(!r.within_tolerance);
    assert!(r.cells <= 1);
    assert!(r.edge_pair_distance_upper.is_none());
    for bad in [
        [[0.1, 0.1, 0.3, 0.3], [0.7, 0.7, 0.9, 0.9]],
        [[0.1, 0.1, 0.9, 0.9], [0.7, 0.1, 0.3, 0.3]],
        [[0.1, f64::NAN, 0.9, 0.9], [0.7, 0.7, 0.3, 0.3]],
    ] {
        assert!(watertight::inspect_boundary_subranges(&models, pair(), &bad, 10000).is_err());
    }
}

#[test]
fn original_subranges_keep_reversed_original_curves_and_nonunit_domains() {
    let mut models = vec![patch(0.), patch(1.)];
    for (i, model) in models.iter_mut().enumerate() {
        let coedge = if i == 0 { 1 } else { 3 };
        let edge = model.loops[0].coedges[coedge].edge;
        model.edges[edge].curve = model.edges[edge].curve.reverse().unwrap();
        for knot in &mut model.edges[edge].curve.knots {
            *knot = if *knot == 0. { 0.3 } else { 1.7 };
        }
        model.edges[edge].vertices.swap(0, 1);
        model.loops[0].coedges[coedge].reversed = true;
        model.rebuild_topology_ids();
    }
    let before = models.clone();
    let r = watertight::inspect_boundary_subranges(
        &models,
        pair(),
        &[[0.1, 0.1, 0.9, 0.9], [0.7, 0.7, 0.3, 0.3]],
        10000,
    )
    .unwrap();
    assert!(r.within_tolerance, "{r:?}");
    assert_eq!(models, before);
}
