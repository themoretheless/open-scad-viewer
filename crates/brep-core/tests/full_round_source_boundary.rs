use brep_core::{feature_ops::full_round_curved::{build, source_boundary, source_supports}, Model};
use nurbs_core::curve::Curve;
fn source(weight: f64) -> Model {
    let c = |p: [[f64; 2]; 3]| Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: p.map(|p| p.to_vec()).to_vec(),
        weights: vec![1., weight, 1.],
        periodic: false,
    };
    let line =
        |a: [f64; 2], b: [f64; 2]| Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap();
    brep_core::prism::extrude(
        &[vec![
            line([0., 0.], [6., 0.]),
            c([[6., 0.], [9., 6.], [6., 12.]]),
            line([6., 12.], [0., 12.]),
            c([[0., 12.], [-3., 6.], [0., 0.]]),
        ]],
        0.,
        6.,
    )
    .unwrap()
}

#[test]
fn complete_original_boundary_keeps_legacy_uv_receipt_and_all_indexed_sources() {
    for weight in [0.7, 1.] {
        let m = source(weight);
        let before = m.clone();
        let r = build(&m, [3, 2, 1]).unwrap();
        let boundary = r.qualify_source_boundary().unwrap();
        assert_eq!(boundary.source(), &before);
        assert_eq!(boundary.edge_uses().len(), m.edges.len());
        assert_eq!(boundary.profiles().oriented_faces().len(), 4);
        assert_eq!(boundary.vertical_seams().bindings().len(), 4);
        assert_eq!(boundary.cap_regularity().len(), 2);
        assert_eq!(
            boundary.caps().original().exact(),
            brep_core::feature_ops::full_round_curved::source_caps::qualify(&m)
                .unwrap()
                .exact()
        );
        boundary.reverify().unwrap();
        let material = boundary.canonical_material().unwrap();
        assert_eq!(material.boundary().source(), &before);
        assert_eq!(material.solid().model().bodies.len(), 1);
        material.reverify().unwrap();
        assert_eq!(m, before);
    }
}
#[test]
fn tolerance_valid_vertex_drift_does_not_authorize_exact_source_boundary() {
    let m = source(0.7);
    let r = build(&m, [3, 2, 1]).unwrap();
    let mut changed = m.clone();
    changed.vertices[0].point[0] += 1e-10;
    changed.validate().unwrap();
    let supports = source_supports::qualify(&changed, r.raw_profile_sources(), [0., 6.]).unwrap();
    assert!(source_boundary::qualify(&supports).is_err());
}
#[test]
fn original_world_stationarity_is_refused_even_for_unchanged_segment_image() {
    let mut m = source(0.7);
    for v in &mut m.vertices {
        for x in &mut v.point {
            *x *= 0.00390625;
        }
    }
    for e in &mut m.edges {
        for p in &mut e.curve.control_points {
            for x in p {
                *x *= 0.00390625;
            }
        }
    }
    for f in &mut m.faces {
        for p in f.surface.control_points.iter_mut().flatten() {
            for x in p {
                *x *= 0.00390625;
            }
        }
    }
    m.validate().unwrap();
    let r = build(&m, [3, 2, 1]).unwrap();
    let mut changed = m.clone();
    let edge = changed
        .edges
        .iter_mut()
        .find(|e| e.curve.degree == 1 && e.curve.control_points.iter().all(|p| p[2] == 0.))
        .unwrap();
    let a = edge.curve.control_points[0].clone();
    let b = edge.curve.control_points[1].clone();
    edge.curve.control_points = vec![a.clone(), b.clone(), a, b];
    edge.curve.degree = 3;
    edge.curve.knots = vec![0., 0., 0., 0., 1., 1., 1., 1.];
    edge.curve.weights = vec![1.; 4];
    changed.tolerance_mm = 1e-2;
    changed.validate().unwrap();
    let supports =
        source_supports::qualify(&changed, r.raw_profile_sources(), [0., 6. * 0.00390625]).unwrap();
    assert!(source_boundary::qualify(&supports).is_err());
}

#[test]
fn canonical_source_material_preserves_through_hole_boundary_roles() {
    // Recognition supplies only fixture construction data, never proof authority.
    let mut loops = brep_core::prism::recognize(&source(0.7))
        .unwrap()
        .unwrap()
        .loops;
    let points = [[2., 2.], [2., 3.], [3., 3.], [3., 2.], [2., 2.]];
    loops.push(
        points
            .windows(2)
            .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
            .collect(),
    );
    let m = brep_core::prism::extrude(&loops, 0., 6.).unwrap();
    let r = build(&m, [3, 2, 1]).unwrap();
    let boundary = r.qualify_source_boundary().unwrap();
    assert_eq!(boundary.profiles().region().containment().len(), 2);
    assert_eq!(boundary.vertical_seams().bindings().len(), 8);
    assert!(boundary.cap_regularity().iter().all(|c| c.len() == 2));
    let material = boundary.canonical_material().unwrap();
    assert_eq!(material.solid().model().bodies.len(), 1);
    assert!(material.solid().model().bodies[0].inner_shells.is_empty());
    assert_eq!(material.solid().nesting().containment().len(), 2);
    material.reverify().unwrap();
    assert_eq!(material.boundary().source(), &m);
}
