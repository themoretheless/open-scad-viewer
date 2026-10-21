use brep_core::{
    Model,
    face_cut::{self, Segment},
};
use nurbs_core::curve::Curve;
fn setup() -> (Model, usize, [usize; 2], Segment) {
    let model = brep_core::cuboid([0.; 3], [8.; 3]).unwrap();
    let face = model
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 8.)
        })
        .unwrap();
    let outer = &model.loops[model.faces[face].outer].coedges;
    let vertex = |i: usize| {
        let c = &outer[i];
        model.edges[c.edge].vertices[usize::from(c.reversed)]
    };
    let endpoints = [vertex(0), vertex(2)];
    let pcpoints = [
        outer[0].pcurve.control_points[0].clone(),
        outer[2].pcurve.control_points[0].clone(),
    ];
    // A bowed rational cubic in the original planar chart, not a straight chord.
    let a = &pcpoints[0];
    let b = &pcpoints[1];
    let pcurve = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: vec![
            a.clone(),
            vec![a[0] * 0.75 + b[0] * 0.25, a[1] * 0.5 + b[1] * 0.5],
            vec![a[0] * 0.5 + b[0] * 0.5, a[1] * 0.75 + b[1] * 0.25],
            b.clone(),
        ],
        weights: vec![1., 0.5, 0.5, 1.],
        periodic: false,
    };
    let curve = Curve {
        control_points: pcurve
            .control_points
            .iter()
            .map(|p| {
                model.faces[face]
                    .surface
                    .evaluate(p[0], p[1])
                    .unwrap()
                    .point
                    .to_vec()
            })
            .collect(),
        ..pcurve.clone()
    };
    (model, face, endpoints, Segment { curve, pcurve })
}
#[test]
fn projected_network_infers_descendants_and_refuses_ambiguous_repeated_cut() {
    let (model, face, endpoints, path) = setup();
    let before = model.clone();
    let ring = &model.loops[model.faces[face].outer].coedges;
    let end = model.edges[ring[1].edge].vertices[usize::from(ring[1].reversed)];
    let a = ring[0].pcurve.control_points[0].clone();
    let b = ring[1].pcurve.control_points[0].clone();
    let midpoint = vec![(a[0] + b[0]) * 0.45 + 0.05, (a[1] + b[1]) * 0.45 + 0.05];
    let curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: [a, midpoint, b]
            .iter()
            .map(|p| {
                model.faces[face]
                    .surface
                    .evaluate(p[0], p[1])
                    .unwrap()
                    .point
                    .to_vec()
            })
            .collect(),
        weights: vec![1.; 3],
        periodic: false,
    };
    let cuts = vec![
        face_cut::OwnedCut {
            endpoints,
            curves: vec![path.curve.clone()],
        },
        face_cut::OwnedCut {
            endpoints: [endpoints[0], end],
            curves: vec![curve],
        },
    ];
    let out =
        face_cut::partition_projected_network(&model, face, &cuts, 1e-8, 100000, 1000000).unwrap();
    assert_eq!(out.faces.len(), 3);
    assert_eq!(out.edges.len(), 2);
    assert!(out.cells <= 100000 && out.exact_work <= 1000000);
    assert!(out.model.persistent_naming_complete());
    for i in 0..model.edges.len() {
        assert_eq!(out.model.1.edges[i], model.1.edges[i]);
    }
    for &f in &out.faces {
        assert_eq!(out.model.faces[f].surface, model.faces[face].surface);
    }
    let retained = face_cut::plan_projected_retention(
        &model,
        face,
        &cuts,
        &[[0.2, 0.8], [0.8, 0.2]],
        1e-8,
        100000,
        1000000,
        1000000,
    )
    .unwrap();
    assert_eq!(retained.retained_faces.len(), 2);
    assert_eq!(retained.discard_faces.len(), 1);
    assert_ne!(retained.seed_faces[0], retained.seed_faces[1]);
    assert!(retained.domain_cells <= 1000000);
    assert_eq!(retained.network.model, out.model);
    for point in [[0., 0.], [-1., -1.], [f64::NAN, 0.]] {
        assert!(
            face_cut::plan_projected_retention(
                &model,
                face,
                &cuts,
                &[point],
                1e-8,
                100000,
                1000000,
                1000000,
            )
            .is_err()
        );
    }
    assert!(
        face_cut::plan_projected_retention(
            &model,
            face,
            &cuts,
            &[[0.2, 0.8]],
            1e-8,
            100000,
            1000000,
            1,
        )
        .is_err()
    );
    assert_eq!(
        brep_core::watertight::inspect_with_exact(&out.model, 100000, 1000000)
            .unwrap()
            .closure
            .status,
        brep_core::watertight::Closure::WithinTolerance
    );
    let duplicate = [
        face_cut::OwnedCut {
            endpoints,
            curves: vec![path.curve.clone()],
        },
        face_cut::OwnedCut {
            endpoints,
            curves: vec![path.curve],
        },
    ];
    assert!(
        face_cut::partition_projected_network(&model, face, &duplicate, 1e-8, 100000, 1000000)
            .is_err()
    );
    let opposite = model.edges[ring[3].edge].vertices[usize::from(ring[3].reversed)];
    let crossing = [
        face_cut::OwnedCut {
            endpoints,
            curves: vec![cuts[0].curves[0].clone()],
        },
        face_cut::OwnedCut {
            endpoints: [end, opposite],
            curves: vec![
                Curve::from_polyline(vec![
                    model.vertices[end].point.to_vec(),
                    model.vertices[opposite].point.to_vec(),
                ])
                .unwrap(),
            ],
        },
    ];
    assert!(
        face_cut::partition_projected_network(&model, face, &crossing, 1e-8, 100000, 1000000)
            .is_err()
    );
    assert!(face_cut::partition_projected_network(&model, face, &cuts, 1e-8, 1, 1000000).is_err());
    assert_eq!(model, before);
}
#[test]
fn projected_rational_cut_builds_owned_pcurves_and_refuses_bad_lifts() {
    let (model, face, endpoints, path) = setup();
    let before = model.clone();
    let out =
        face_cut::partition_projected(&model, face, endpoints, &[path.curve.clone()], 1e-8, 100000)
            .unwrap();
    assert_eq!(out.model.faces.len(), model.faces.len() + 1);
    assert_eq!(out.model.edges[out.edges[0]].curve, path.curve);
    assert_eq!(out.model.edges[out.edges[0]].vertices, endpoints);
    assert!(out.cells <= 100000);
    for &f in &out.faces {
        assert_eq!(out.model.faces[f].surface, model.faces[face].surface);
    }
    assert_eq!(
        brep_core::watertight::inspect_with_exact(&out.model, 100000, 1000000)
            .unwrap()
            .closure
            .status,
        brep_core::watertight::Closure::WithinTolerance
    );
    let mut bad = path.curve.clone();
    bad.control_points[1][2] += 0.1;
    assert!(face_cut::partition_projected(&model, face, endpoints, &[bad], 1e-8, 100000).is_err());
    assert!(
        face_cut::partition_projected(&model, face, endpoints, &[path.curve], 1e-8, 1).is_err()
    );
    assert_eq!(model, before);
}
#[test]
fn rational_cut_partitions_original_face_with_shared_edges_and_split_lineage() {
    let (model, face, endpoints, path) = setup();
    let before = model.clone();
    let out = face_cut::partition(&model, face, endpoints, &[path], 1e-8, 1000000).unwrap();
    assert_eq!(out.model.faces.len(), model.faces.len() + 1);
    assert_eq!(out.model.vertices.len(), model.vertices.len());
    assert_eq!(out.model.edges.len(), model.edges.len() + 1);
    for &f in &out.faces {
        assert_eq!(out.model.faces[f].surface, model.faces[face].surface);
    }
    let edge = out.edges[0];
    let uses = out
        .model
        .loops
        .iter()
        .flat_map(|l| &l.coedges)
        .filter(|c| c.edge == edge)
        .collect::<Vec<_>>();
    assert_eq!(uses.len(), 2);
    assert_ne!(uses[0].reversed, uses[1].reversed);
    assert_eq!(out.model.edges[edge].vertices, endpoints);
    for i in 0..model.edges.len() {
        assert_eq!(out.model.1.edges[i], model.1.edges[i]);
    }
    assert!(out.model.1.lineage.iter().any(|r| r.entity_kind == "face"
        && r.parents == [out.source_identity]
        && r.children == out.face_identities));
    assert!(out.model.persistent_naming_complete());
    assert_eq!(
        brep_core::watertight::inspect_with_exact(&out.model, 100000, 1000000)
            .unwrap()
            .closure
            .status,
        brep_core::watertight::Closure::WithinTolerance
    );
    assert!(out.cells <= 1000000 && out.exact_work <= 1000000 && out.trim_pairs <= 10000);
    assert_eq!(model, before);
}
#[test]
fn broken_lift_bad_endpoints_and_budget_refuse_without_mutation() {
    let (model, face, endpoints, path) = setup();
    let before = model.clone();
    let mut bad = Segment {
        curve: path.curve.clone(),
        pcurve: path.pcurve.clone(),
    };
    bad.curve.control_points[1][2] += 0.1;
    assert!(face_cut::partition(&model, face, endpoints, &[bad], 1e-8, 1000000).is_err());
    assert!(
        face_cut::partition(
            &model,
            face,
            [endpoints[0]; 2],
            &[Segment {
                curve: path.curve.clone(),
                pcurve: path.pcurve.clone()
            }],
            1e-8,
            1000000
        )
        .is_err()
    );
    assert!(face_cut::partition(&model, face, endpoints, &[path], 1e-8, 1).is_err());
    assert_eq!(model, before);
}

#[path = "../examples/support/face_window_fixture.rs"]
mod fixture;
#[test]
fn hole_is_assigned_to_one_sector_and_a_cut_through_it_is_refused() {
    let (source, face, patch, boundary) = fixture::build().unwrap();
    let window =
        brep_core::face_window::replace(&source, face, &patch, &boundary, 1e-8, 1000000).unwrap();
    let model = window.model;
    let original = model.clone();
    let outer = &model.loops[model.faces[face].outer].coedges;
    let start = |i: usize| model.edges[outer[i].edge].vertices[usize::from(outer[i].reversed)];
    let a = &outer[0].pcurve.control_points[0];
    let b = &outer[1].pcurve.control_points[0];
    let pcurve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            a.clone(),
            vec![
                (a[0] + b[0]) * 0.5 - (b[1] - a[1]) * 0.375,
                (a[1] + b[1]) * 0.5 + (b[0] - a[0]) * 0.375,
            ],
            b.clone(),
        ],
        weights: vec![1.; 3],
        periodic: false,
    };
    let curve = Curve {
        control_points: pcurve
            .control_points
            .iter()
            .map(|p| {
                model.faces[face]
                    .surface
                    .evaluate(p[0], p[1])
                    .unwrap()
                    .point
                    .to_vec()
            })
            .collect(),
        ..pcurve.clone()
    };
    let out = face_cut::partition(
        &model,
        face,
        [start(0), start(1)],
        &[Segment { curve, pcurve }],
        1e-8,
        1000000,
    )
    .unwrap();
    assert_eq!(out.model.faces[out.faces[0]].holes.len(), 1);
    assert_eq!(out.model.faces[out.faces[1]].holes.len(), 0);
    assert_eq!(out.model.faces[out.faces[0]].holes, model.faces[face].holes);
    assert_eq!(
        brep_core::watertight::inspect_with_exact(&out.model, 100000, 1000000)
            .unwrap()
            .closure
            .status,
        brep_core::watertight::Closure::WithinTolerance
    );
    let a = outer[0].pcurve.control_points[0].clone();
    let b = outer[2].pcurve.control_points[0].clone();
    let pcurve = Curve::from_polyline(vec![a, b]).unwrap();
    let curve = Curve {
        control_points: pcurve
            .control_points
            .iter()
            .map(|p| {
                model.faces[face]
                    .surface
                    .evaluate(p[0], p[1])
                    .unwrap()
                    .point
                    .to_vec()
            })
            .collect(),
        ..pcurve.clone()
    };
    assert!(
        face_cut::partition(
            &model,
            face,
            [start(0), start(2)],
            &[Segment { curve, pcurve }],
            1e-8,
            1000000
        )
        .is_err()
    );
    assert_eq!(model, original);
}

#[test]
fn nonplanar_original_surface_is_partitioned_after_shared_boundary_root_cuts() {
    use brep_core::edge_partition::{self, PcurveCut};
    use nurbs_core::surface::Axis;
    let (source, face, patch, boundary) = fixture::build().unwrap();
    let window =
        brep_core::face_window::replace(&source, face, &patch, &boundary, 1e-8, 1000000).unwrap();
    let target = window.replacement_face;
    let mut model = window.model;
    let mut endpoints = Vec::new();
    for edge in [window.edges[0], window.edges[2]] {
        let [a, b] = model.edges[edge].curve.domain();
        let cuts = model
            .loops
            .iter()
            .enumerate()
            .flat_map(|(wire, l)| {
                l.coedges
                    .iter()
                    .enumerate()
                    .filter(move |(_, c)| c.edge == edge)
                    .map(move |(coedge, c)| {
                        let [a, b] = c.pcurve.domain();
                        PcurveCut {
                            wire,
                            coedge,
                            parameter: a + (b - a) * 0.5,
                        }
                    })
            })
            .collect::<Vec<_>>();
        let split =
            edge_partition::partition(&model, edge, a + (b - a) * 0.5, &cuts, 1000000).unwrap();
        endpoints.push(split.vertex);
        model = split.model;
    }
    let before = model.clone();
    let curve = model.faces[target].surface.iso(Axis::U, 0.5).unwrap();
    let automatic = face_cut::partition_projected(
        &model,
        target,
        [endpoints[0], endpoints[1]],
        &[curve.clone()],
        1e-8,
        100000,
    )
    .unwrap();
    assert!(automatic.cells <= 100000);
    for &f in &automatic.faces {
        assert_eq!(
            automatic.model.faces[f].surface,
            model.faces[target].surface
        );
    }
    assert_eq!(
        brep_core::watertight::inspect_with_exact(&automatic.model, 100000, 1000000)
            .unwrap()
            .closure
            .status,
        brep_core::watertight::Closure::WithinTolerance
    );
    let pcurve = Curve::from_polyline(vec![vec![0.5, 0.], vec![0.5, 1.]]).unwrap();
    let out = face_cut::partition(
        &model,
        target,
        [endpoints[0], endpoints[1]],
        &[Segment { curve, pcurve }],
        1e-8,
        1000000,
    )
    .unwrap();
    for &f in &out.faces {
        assert_eq!(out.model.faces[f].surface, model.faces[target].surface);
    }
    assert_eq!(
        brep_core::watertight::inspect_with_exact(&out.model, 100000, 1000000)
            .unwrap()
            .closure
            .status,
        brep_core::watertight::Closure::WithinTolerance
    );
    assert_eq!(model, before);
}

#[test]
fn multi_segment_cut_uses_one_interior_vertex_and_refuses_exterior_sector() {
    let (model, face, endpoints, path) = setup();
    let before = model.clone();
    let curves = path.curve.split(0.5).unwrap();
    let automatic =
        face_cut::partition_projected(&model, face, endpoints, &curves, 1e-8, 100000).unwrap();
    assert_eq!(automatic.edges.len(), 2);
    assert_eq!(
        automatic.model.edges[automatic.edges[0]].vertices[1],
        automatic.model.edges[automatic.edges[1]].vertices[0]
    );
    let pcs = path.pcurve.split(0.5).unwrap();
    let path = curves
        .into_iter()
        .zip(pcs)
        .map(|(curve, pcurve)| Segment { curve, pcurve })
        .collect::<Vec<_>>();
    let out = face_cut::partition(&model, face, endpoints, &path, 1e-8, 1000000).unwrap();
    assert_eq!(out.edges.len(), 2);
    assert_eq!(out.model.vertices.len(), model.vertices.len() + 1);
    assert_eq!(
        out.model.edges[out.edges[0]].vertices[1],
        out.model.edges[out.edges[1]].vertices[0]
    );
    let outer = &model.loops[model.faces[face].outer].coedges;
    let a = outer[0].pcurve.control_points[0].clone();
    let b = outer[1].pcurve.control_points[0].clone();
    let pcurve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            a.clone(),
            vec![
                (a[0] + b[0]) * 0.5 + (b[1] - a[1]) * 0.375,
                (a[1] + b[1]) * 0.5 - (b[0] - a[0]) * 0.375,
            ],
            b,
        ],
        weights: vec![1.; 3],
        periodic: false,
    };
    // Construct the affine-plane lift directly, including out-of-domain UVs.
    let s = &model.faces[face].surface;
    let origin = &s.control_points[0][0];
    let curve = Curve {
        control_points: pcurve
            .control_points
            .iter()
            .map(|p| {
                (0..3)
                    .map(|k| {
                        origin[k]
                            + p[0] * (s.control_points[1][0][k] - origin[k])
                            + p[1] * (s.control_points[0][1][k] - origin[k])
                    })
                    .collect()
            })
            .collect(),
        ..pcurve.clone()
    };
    let end = model.edges[outer[1].edge].vertices[usize::from(outer[1].reversed)];
    assert!(
        face_cut::partition(
            &model,
            face,
            [endpoints[0], end],
            &[Segment { curve, pcurve }],
            1e-8,
            1000000
        )
        .is_err()
    );
    assert_eq!(model, before);
}
