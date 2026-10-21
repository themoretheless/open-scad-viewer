use brep_core::{
    Model,
    face_cut::{self, FaceRetention, OwnedCut},
};
use nurbs_core::curve::Curve;

fn diagonal(model: &Model, face: usize) -> OwnedCut {
    let ring = &model.loops[model.faces[face].outer].coedges;
    let endpoints = [0, 2].map(|i| {
        let c = &ring[i];
        model.edges[c.edge].vertices[usize::from(c.reversed)]
    });
    OwnedCut {
        endpoints,
        curves: vec![Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: endpoints
                .iter()
                .map(|&v| model.vertices[v].point.to_vec())
                .collect(),
            weights: vec![1.; 2],
            periodic: false,
        }],
    }
}

#[test]
fn adjacent_support_faces_plan_atomically_with_original_ownership() {
    let model = brep_core::cuboid([0.; 3], [8.; 3]).unwrap();
    let before = model.clone();
    // Find two faces sharing an original edge, rather than relying on face order.
    let face0 = 0;
    let edge = model.loops[model.faces[face0].outer].coedges[0].edge;
    let face1 = (1..model.faces.len())
        .find(|&f| {
            model.loops[model.faces[f].outer]
                .coedges
                .iter()
                .any(|c| c.edge == edge)
        })
        .unwrap();
    let cuts0 = [diagonal(&model, face0)];
    let cuts1 = [diagonal(&model, face1)];
    let seeds = [[0.2, 0.3]];
    let requests = [
        FaceRetention {
            face: face0,
            cuts: &cuts0,
            keep_uv: &seeds,
        },
        FaceRetention {
            face: face1,
            cuts: &cuts1,
            keep_uv: &seeds,
        },
    ];
    let out =
        face_cut::plan_retention_network(&model, &requests, 1e-8, 100000, 1000000, 100000).unwrap();
    assert_eq!(out.regions.len(), 2);
    assert_eq!(out.model.faces.len(), model.faces.len() + 2);
    assert!(out.cells <= 100000 && out.exact_work <= 1000000 && out.domain_cells <= 100000);
    out.model.validate().unwrap();
    assert!(out.model.persistent_naming_complete());
    for region in &out.regions {
        assert_eq!(region.descendants.len(), 2);
        assert_eq!(region.retained_faces.len(), 1);
        assert_eq!(region.discard_faces.len(), 1);
        for &f in &region.descendants {
            assert_eq!(
                out.model.faces[f].surface,
                model.faces[region.source_face].surface
            );
        }
    }
    for i in 0..model.edges.len() {
        assert_eq!(out.model.1.edges[i], model.1.edges[i]);
    }
    for i in 0..model.vertices.len() {
        assert_eq!(out.model.1.vertices[i], model.1.vertices[i]);
    }
    let outside = [[2., 2.]];
    let late_bad = [
        FaceRetention {
            face: face0,
            cuts: &cuts0,
            keep_uv: &seeds,
        },
        FaceRetention {
            face: face1,
            cuts: &cuts1,
            keep_uv: &outside,
        },
    ];
    assert!(
        face_cut::plan_retention_network(&model, &late_bad, 1e-8, 100000, 1000000, 100000).is_err()
    );
    assert!(
        face_cut::plan_retention_network(&model, &requests, 1e-8, out.cells - 1, 1000000, 100000)
            .is_err()
    );
    assert!(face_cut::plan_retention_network(&model, &requests, 1e-8, 100000, 1000000, 1).is_err());
    let duplicate = [
        FaceRetention {
            face: face0,
            cuts: &cuts0,
            keep_uv: &seeds,
        },
        FaceRetention {
            face: face0,
            cuts: &cuts0,
            keep_uv: &seeds,
        },
    ];
    assert!(
        face_cut::plan_retention_network(&model, &duplicate, 1e-8, 100000, 1000000, 100000)
            .is_err()
    );
    assert_eq!(model, before);
}
