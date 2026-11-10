use brep_core::{Coedge, Edge, Face, FaceUse, Loop, Model, Shell, Vertex, face_curvature as c, region_replacement as r};
use nurbs_core::{
    curve::Curve,
    surface::{Axis, Surface},
};
fn edge(s: &Surface, e: usize) -> Curve {
    match e {
        0 => s.iso(Axis::V, 0.),
        1 => s.iso(Axis::U, 1.),
        2 => s.iso(Axis::V, 1.).and_then(|c| c.reverse()),
        _ => s.iso(Axis::U, 0.).and_then(|c| c.reverse()),
    }
    .unwrap()
}
fn sheet(s: &Surface) -> Model {
    let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let curves = (0..4).map(|e| edge(s, e)).collect::<Vec<_>>();
    let mut m = Model(
        brep_topology::Model {
            vertices: curves
                .iter()
                .map(|c| Vertex {
                    point: c.control_points[0].clone().try_into().unwrap(),
                })
                .collect(),
            edges: curves
                .into_iter()
                .enumerate()
                .map(|(i, curve)| Edge {
                    vertices: [i, (i + 1) % 4],
                    curve,
                    degenerate: false,
                })
                .collect(),
            loops: vec![Loop {
                coedges: (0..4)
                    .map(|i| {
                        Coedge::new(
                            i,
                            false,
                            Curve::from_polyline(vec![uv[i].to_vec(), uv[(i + 1) % 4].to_vec()])
                                .unwrap(),
                        )
                    })
                    .collect(),
            }],
            faces: vec![Face {
                surface: s.clone(),
                outer: 0,
                holes: vec![],
            }],
            shells: vec![Shell {
                faces: vec![FaceUse {
                    face: 0,
                    reversed: false,
                }],
                closed: false,
            }],
            bodies: vec![],
            tolerance_mm: 1e-7,
        },
        Default::default(),
    );
    m.rebuild_topology_ids();
    m
}

fn spine() -> Curve {
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0., 0.], vec![0.125, 0.25, 5.], vec![0.25, 0., 10.]],
        weights: vec![1., 0.875, 1.],
        periodic: false,
    }
}
#[test]
fn rational_spine_has_whole_g2_contacts_and_bounded_curvature() {
    qualify(spine(), [[1., 0., 0.], [0., 1., 0.]]);
}
#[test]
fn rational_spine_oblique_supports_have_whole_g2_and_bounded_curvature() {
    qualify(spine(), [[1., 0., 0.], [0.5, 0.875, 0.]]);
}
fn qualify(source: Curve, directions: [[f64; 3]; 2]) {
    let extents = vec![1.; source.control_points.len()];
    qualify_extents(source, directions, extents);
}
fn qualify_extents(source: Curve, directions: [[f64; 3]; 2], extents: Vec<f64>) {
    let before = source.clone();
    let t = r::propose_nurbs_edge_transition(&source, &extents, directions, 0.214).unwrap();
    assert_eq!(source, before);
    assert_eq!(t.blend.knots_v, source.knots);
    assert_eq!(t.blend.weights[0], source.weights);
    for i in 0..2 {
        let u = i as f64;
        let v = (1 - i) as f64;
        let edge = t.blend.iso(Axis::U, u).unwrap();
        let a = Curve::from_polyline(vec![vec![u, 0.], vec![u, 1.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![v, 0.], vec![v, 1.]]).unwrap();
        let p = nurbs_core::surface_g2_pcurves::inspect_owned(
            &edge,
            [&a, &b],
            [&t.blend, &t.supports[i]],
            [false; 2],
            1e-7,
            1e-5,
            1e-4,
            100000,
            1000000,
        )
        .unwrap();
        assert!(p.report.positions.proven);
        assert_eq!(
            p.report.geometric.decision,
            nurbs_core::surface_join::Decision::WithinTolerance
        );
    }
    for s in std::iter::once(&t.blend).chain(&t.supports) {
        let m = sheet(s);
        let p = c::inspect(
            &m,
            0,
            10.,
            1e-8,
            c::Limits {
                cells: 50000,
                domain_cells: 1000000,
                shape_cells: 1000000,
            },
            &[],
        )
        .unwrap();
        assert_eq!(
            p.decision,
            c::Decision::Within,
            "upper={:?}",
            p.absolute_curvature_upper
        );
    }
}
#[test]
fn preserves_multispan_clock_and_refuses_c1_or_invalid_input() {
    let mut s = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 0.375, 0.625, 1., 1., 1., 1.],
        control_points: (0..6)
            .map(|i| vec![i as f64 / 32., 0., i as f64 * 2.])
            .collect(),
        weights: vec![1., 0.875, 1.125, 1., 0.875, 1.],
        periodic: false,
    };
    let t = r::propose_nurbs_edge_transition(&s, &[1.; 6], [[1., 0., 0.], [0., 1., 0.]], 0.214)
        .unwrap();
    assert_eq!(t.blend.knots_v, s.knots);
    assert_eq!(t.blend.degree_v, 3);
    s.knots[5] = 0.375;
    assert!(
        r::propose_nurbs_edge_transition(&s, &[1.; 6], [[1., 0., 0.], [0., 1., 0.]], 0.214)
            .is_err()
    );
    assert!(
        r::propose_nurbs_edge_transition(&spine(), &[1.; 2], [[1., 0., 0.], [0., 1., 0.]], 0.214)
            .is_err()
    );
    assert!(
        r::propose_nurbs_edge_transition(&spine(), &[1.; 3], [[1., 0., 0.], [2., 0., 0.]], 0.214)
            .is_err()
    );
    assert!(
        r::propose_nurbs_edge_transition(
            &spine(),
            &[1., -1., 1.],
            [[1., 0., 0.], [0., 1., 0.]],
            0.214
        )
        .is_err()
    );
}

#[test]
fn proposal_does_not_admit_an_excessively_tight_blend() {
    let t =
        r::propose_nurbs_edge_transition(&spine(), &[0.01; 3], [[1., 0., 0.], [0., 1., 0.]], 0.214)
            .unwrap();
    let report = c::inspect(
        &sheet(&t.blend),
        0,
        10.,
        1e-8,
        c::Limits {
            cells: 50000,
            domain_cells: 1000000,
            shape_cells: 1000000,
        },
        &[],
    )
    .unwrap();
    assert_eq!(report.decision, c::Decision::Exceeds);
}

#[test]
fn spatial_multispan_rational_edge_has_whole_g2_and_bounded_curvature() {
    let s = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 0.375, 0.625, 1., 1., 1., 1.],
        control_points: vec![
            vec![0., 0., 0.],
            vec![0.125, 0.125, 2.],
            vec![0., 0.25, 4.],
            vec![0.25, 0.125, 6.],
            vec![0.125, -0.125, 8.],
            vec![0.375, 0., 10.],
        ],
        weights: vec![1., 0.875, 1.125, 1., 0.875, 1.],
        periodic: false,
    };
    qualify(s, [[1., 0., 0.], [0.5, 0.875, 0.]]);
}

#[test]
fn varying_contact_extent_keeps_whole_g2_and_bounded_curvature() {
    qualify_extents(
        spine(),
        [[1., 0., 0.], [0.5, 0.875, 0.]],
        vec![1., 1.125, 0.875],
    );
}
