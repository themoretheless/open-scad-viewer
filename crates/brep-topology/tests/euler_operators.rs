use brep_topology::{Body, Coedge, Edge, Face, FaceUse, Loop, Model, Shell, Vertex};

fn tetrahedron() -> Model<(), (), ()> {
    Model {
        vertices: vec![
            Vertex {
                point: [0., 0., 0.],
            },
            Vertex {
                point: [1., 0., 0.],
            },
            Vertex {
                point: [0., 1., 0.],
            },
            Vertex {
                point: [0., 0., 1.],
            },
        ],
        edges: vec![
            Edge {
                degenerate: false,
                vertices: [0, 1],
                curve: (),
            },
            Edge {
                degenerate: false,
                vertices: [1, 2],
                curve: (),
            },
            Edge {
                degenerate: false,
                vertices: [2, 0],
                curve: (),
            },
            Edge {
                degenerate: false,
                vertices: [0, 3],
                curve: (),
            },
            Edge {
                degenerate: false,
                vertices: [1, 3],
                curve: (),
            },
            Edge {
                degenerate: false,
                vertices: [2, 3],
                curve: (),
            },
        ],
        loops: vec![
            Loop {
                coedges: vec![
                    Coedge::new(0, true, ()),
                    Coedge::new(2, true, ()),
                    Coedge::new(1, true, ()),
                ],
            },
            Loop {
                coedges: vec![
                    Coedge::new(0, false, ()),
                    Coedge::new(4, false, ()),
                    Coedge::new(3, true, ()),
                ],
            },
            Loop {
                coedges: vec![
                    Coedge::new(1, false, ()),
                    Coedge::new(5, false, ()),
                    Coedge::new(4, true, ()),
                ],
            },
            Loop {
                coedges: vec![
                    Coedge::new(2, false, ()),
                    Coedge::new(3, false, ()),
                    Coedge::new(5, true, ()),
                ],
            },
        ],
        faces: (0..4)
            .map(|outer| Face {
                surface: (),
                outer,
                holes: vec![],
            })
            .collect(),
        shells: vec![Shell {
            faces: (0..4)
                .map(|face| FaceUse {
                    face,
                    reversed: false,
                })
                .collect(),
            closed: true,
        }],
        bodies: vec![Body {
            outer_shell: 0,
            inner_shells: vec![],
        }],
        tolerance_mm: 1e-7,
    }
}

fn quad() -> Model<(), (), ()> {
    Model {
        vertices: vec![
            Vertex {
                point: [0., 0., 0.],
            },
            Vertex {
                point: [1., 0., 0.],
            },
            Vertex {
                point: [1., 1., 0.],
            },
            Vertex {
                point: [0., 1., 0.],
            },
        ],
        edges: vec![
            Edge {
                degenerate: false,
                vertices: [0, 1],
                curve: (),
            },
            Edge {
                degenerate: false,
                vertices: [1, 2],
                curve: (),
            },
            Edge {
                degenerate: false,
                vertices: [2, 3],
                curve: (),
            },
            Edge {
                degenerate: false,
                vertices: [3, 0],
                curve: (),
            },
        ],
        loops: vec![Loop {
            coedges: (0..4).map(|edge| Coedge::new(edge, false, ())).collect(),
        }],
        faces: vec![Face {
            surface: (),
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
    }
}

fn assert_counts(model: &Model<(), (), ()>, expected: [usize; 3], chi: i64) {
    assert_eq!(
        [model.vertices.len(), model.edges.len(), model.faces.len()],
        expected
    );
    assert_eq!(model.euler_characteristic(), chi);
    model.validate_topology().unwrap();
}

#[test]
fn mev_kev_round_trip_every_tetrahedron_edge_with_invariants() {
    let original = tetrahedron();
    for edge in 0..original.edges.len() {
        let split = original
            .make_edge_vertex(edge, [0.25, 0.25, 0.25], (), ())
            .unwrap();
        assert_counts(&split, [5, 7, 4], 2);
        assert_eq!(split.shell_genus(0), Some(0));
        let restored = split
            .kill_edge_vertex(original.edges.len(), original.vertices.len(), ())
            .unwrap();
        assert_eq!(restored, original);
        assert_counts(&restored, [4, 6, 4], 2);
    }
}

#[test]
fn mef_kef_round_trip_quad_and_composed_edits_preserve_chi() {
    let original = quad();
    let split = original.make_edge_face(0, 0, 2, (), ()).unwrap();
    assert_counts(&split, [4, 5, 2], 1);
    assert_eq!(split.boundary_edges().len(), 4);
    let restored = split.kill_edge_face(original.edges.len()).unwrap();
    assert_eq!(restored, original);

    let tetra = tetrahedron();
    let split_vertex = tetra.make_edge_vertex(0, [0.5, 0., 0.], (), ()).unwrap();
    let split_face = split_vertex.make_edge_face(0, 0, 2, (), ()).unwrap();
    assert_counts(&split_face, [5, 8, 5], 2);
    let joined_face = split_face
        .kill_edge_face(split_face.edges.len() - 1)
        .unwrap();
    let restored = joined_face.kill_edge_vertex(6, 4, ()).unwrap();
    assert_eq!(restored, tetra);
}

#[test]
fn mev_rejects_nan_atomically_and_kev_refuses_non_degree_two_vertices() {
    let original = tetrahedron();
    let error = original
        .make_edge_vertex(0, [f64::NAN, 0., 0.], (), ())
        .unwrap_err();
    assert_eq!(error.code, "BREP_INVALID_TOPOLOGY");
    assert_eq!(original, tetrahedron());

    let error = original.kill_edge_vertex(0, 0, ()).unwrap_err();
    assert_eq!(error.code, brep_topology::EULER_INVALID_OPERATION);
}

#[test]
fn non_manifold_input_is_refused_before_any_euler_edit() {
    let mut invalid = quad();
    invalid.loops.push(invalid.loops[0].clone());
    invalid.loops.push(invalid.loops[0].clone());
    invalid.faces.push(Face {
        surface: (),
        outer: 1,
        holes: vec![],
    });
    invalid.faces.push(Face {
        surface: (),
        outer: 2,
        holes: vec![],
    });
    invalid.shells[0].faces.extend([
        FaceUse {
            face: 1,
            reversed: false,
        },
        FaceUse {
            face: 2,
            reversed: false,
        },
    ]);
    assert!(
        invalid
            .validate_topology()
            .unwrap_err()
            .message
            .contains("Non-manifold edge")
    );
    let before = invalid.clone();
    let error = invalid
        .make_edge_vertex(0, [0.5, 0., 0.], (), ())
        .unwrap_err();
    assert_eq!(error.code, "BREP_INVALID_TOPOLOGY");
    assert_eq!(invalid, before);
}

#[test]
fn trimmed_split_and_hole_redistribution_fail_closed_with_typed_errors() {
    let mut trimmed = quad();
    let trim = brep_topology::CoedgeTrim {
        curve_parameter: [0., 1.],
        pcurve_parameter: [0., 1.],
        periodic_lift: [[0, 0]; 2],
    };
    trimmed.loops[0].coedges[0] = Coedge::new(0, false, ()).with_trim(trim).unwrap();
    let error = trimmed
        .make_edge_vertex(0, [0.5, 0., 0.], (), ())
        .unwrap_err();
    assert_eq!(error.code, brep_topology::EULER_UNSUPPORTED);

    let mut holed = quad();
    holed.faces[0].holes.push(0);
    // Validate the operator precondition directly: the deliberately reused
    // loop is not a valid model and is refused by the existing admission pass.
    let error = holed.make_edge_face(0, 0, 2, (), ()).unwrap_err();
    assert_eq!(error.code, "BREP_INVALID_TOPOLOGY");
}
