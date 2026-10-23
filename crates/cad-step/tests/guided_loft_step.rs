use brep_core::{Coedge, Edge, Face, FaceUse, Loop, Model, Shell, TopologyIds, Vertex};
use nurbs_core::{
    curve::Curve,
    surface::{Axis, Surface},
};

fn line2(a: [f64; 2], b: [f64; 2]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    }
}
fn open_face(surface: Surface) -> Model {
    let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let curves = [
        surface.iso(Axis::V, 0.).unwrap(),
        surface.iso(Axis::U, 1.).unwrap(),
        surface.iso(Axis::V, 1.).unwrap().reverse().unwrap(),
        surface.iso(Axis::U, 0.).unwrap().reverse().unwrap(),
    ];
    let mut model = Model(
        brep_topology::Model {
            vertices: uv
                .iter()
                .map(|p| Vertex {
                    point: surface.evaluate(p[0], p[1]).unwrap().point,
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
                    .map(|i| Coedge {
                        edge: i,
                        reversed: false,
                        pcurve: line2(uv[i], uv[(i + 1) % 4]),
                    })
                    .collect(),
            }],
            faces: vec![Face {
                surface,
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
            tolerance_mm: 1e-6,
        },
        TopologyIds::default(),
    );
    model.rebuild_topology_ids();
    model.validate().unwrap();
    model
}

#[test]
fn guided_loft_retains_curved_boundaries_and_jets_through_step_v9() {
    let sections: Vec<_> = [0., 2.]
        .into_iter()
        .map(|z| nurbs_core::primitives::line([0., 0., z], [1., 0., z]).unwrap())
        .collect();
    let guide = nurbs_core::paths::bezier(
        vec![vec![0.5, 0., 0.], vec![0.5, 1., 1.], vec![0.5, 0., 2.]],
        None,
    )
    .unwrap();
    let s = nurbs_core::guided_loft::interpolate(
        &sections,
        &[0., 1.],
        &[guide],
        &[0.5],
        Some([vec![[0., 2., 2.]; 2], vec![[0., -2., 2.]; 2]]),
    )
    .unwrap();
    let model = open_face(s);
    let (text, _, _) = cad_step::export_step_v9(&model).unwrap();
    assert!(text.contains("SHELL_BASED_SURFACE_MODEL"));
    if let Ok(path) = std::env::var("LOFT_STEP_OUTPUT") {
        std::fs::write(path, &text).unwrap();
    }
    let (back, _, _) = cad_step::import_step_v9(&text).unwrap();
    assert_eq!(back.faces.len(), 1);
    assert_eq!(back.edges.len(), 4);
    assert!(!back.shells[0].closed);
    assert!(back.bodies.is_empty());
    for u in [0., 0.13, 0.5, 0.87, 1.] {
        for v in [0., 0.17, 0.5, 0.83, 1.] {
            let p = back.faces[0].surface.evaluate(u, v).unwrap();
            let expected = [u, 2. * v * (1. - v), 2. * v];
            for k in 0..3 {
                assert!((p.point[k] - expected[k]).abs() < 1e-9);
            }
            let (_, dv) = p.first_derivatives().unwrap();
            assert!((dv[1] - (2. - 4. * v)).abs() < 1e-9 && (dv[2] - 2.).abs() < 1e-9);
        }
    }
    for (edge, original) in back.edges.iter().zip(&model.edges) {
        for t in [0., 0.17, 0.5, 0.83, 1.] {
            let p = edge.curve.evaluate(t).unwrap().point;
            let q = original.curve.evaluate(t).unwrap().point;
            for k in 0..3 {
                assert!((p[k] - q[k]).abs() < 1e-9);
            }
        }
    }
}

#[test]
fn rational_and_multiple_guide_step_matrix() {
    let arc = nurbs_core::primitives::circle_arc([0., 0., 0.], [0., 0., 1.], 2., 0., 90.).unwrap();
    let sections: Vec<_> = [0., 1., 2.]
        .into_iter()
        .map(|z| {
            let mut c = arc.clone();
            for p in &mut c.control_points {
                p[2] = z;
            }
            c
        })
        .collect();
    let guides: Vec<_> = [0, arc.control_points.len() - 1]
        .into_iter()
        .map(|i| {
            let p = &arc.control_points[i];
            nurbs_core::primitives::line([p[0], p[1], 0.], [p[0], p[1], 2.]).unwrap()
        })
        .collect();
    let lines: Vec<_> = [0., 2.]
        .into_iter()
        .map(|z| nurbs_core::primitives::line([0., 0., z], [1., 0., z]).unwrap())
        .collect();
    let interior: Vec<_> = [0.25, 0.75]
        .into_iter()
        .map(|u| {
            nurbs_core::paths::bezier(
                vec![vec![u, 0., 0.], vec![u, 1., 1.], vec![u, 0., 2.]],
                None,
            )
            .unwrap()
        })
        .collect();
    let cases = [
        (
            "rational-guided",
            nurbs_core::guided_loft::interpolate(
                &sections,
                &[0., 0.5, 1.],
                &guides,
                &[0., 1.],
                None,
            )
            .unwrap(),
        ),
        (
            "rational-natural",
            nurbs_core::natural_loft::interpolate(&sections, &[0., 0.5, 1.]).unwrap(),
        ),
        (
            "rational-clamped",
            nurbs_core::natural_loft::clamped(
                &sections,
                &[0., 0.5, 1.],
                [0., 0., 2.],
                [0., 0., 2.],
            )
            .unwrap(),
        ),
        (
            "multiple-interior-guides",
            nurbs_core::guided_loft::interpolate(&lines, &[0., 1.], &interior, &[0.25, 0.75], None)
                .unwrap(),
        ),
    ];
    for (name, surface) in cases {
        let model = open_face(surface.clone());
        let (text, _, _) = cad_step::export_step_v9(&model).unwrap();
        let (back, _, _) = cad_step::import_step_v9(&text).unwrap();
        let mut samples = Vec::new();
        for u in [0., 0.13, 0.37, 0.83, 1.] {
            for v in [0., 0.17, 0.37, 0.83, 1.] {
                let expected = surface.evaluate(u, v).unwrap().point;
                let actual = back.faces[0].surface.evaluate(u, v).unwrap().point;
                for k in 0..3 {
                    assert!((actual[k] - expected[k]).abs() < 1e-9);
                }
                samples.push(value_codec::json!({"u":u,"v":v,"point":expected}));
            }
        }
        if let Ok(dir) = std::env::var("LOFT_STEP_MATRIX_OUTPUT") {
            std::fs::write(format!("{dir}/{name}.step"), text).unwrap();
            std::fs::write(
                format!("{dir}/{name}.samples.json"),
                value_codec::to_string(&samples).unwrap(),
            )
            .unwrap();
        }
    }
}

#[test]
fn closed_loft_step_retains_shared_seam_and_cyclic_jets() {
    let sections: Vec<_> = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.], [1., 0.]]
        .into_iter()
        .map(|p| nurbs_core::primitives::line([p[0], p[1], 0.], [p[0], p[1], 1.]).unwrap())
        .collect();
    let surface = nurbs_core::natural_loft::closed(&sections, &[0., 1., 2., 3., 4.]).unwrap();
    let vertices = [
        surface.evaluate(0., 0.).unwrap().point,
        surface.evaluate(1., 0.).unwrap().point,
    ];
    let mut model = Model(
        brep_topology::Model {
            vertices: vertices.into_iter().map(|point| Vertex { point }).collect(),
            edges: vec![
                Edge {
                    vertices: [0, 1],
                    curve: surface.iso(Axis::V, 0.).unwrap(),
                    degenerate: false,
                },
                Edge {
                    vertices: [1, 1],
                    curve: surface.iso(Axis::U, 1.).unwrap(),
                    degenerate: false,
                },
                Edge {
                    vertices: [0, 0],
                    curve: surface.iso(Axis::U, 0.).unwrap(),
                    degenerate: false,
                },
            ],
            loops: vec![Loop {
                coedges: vec![
                    Coedge {
                        edge: 0,
                        reversed: false,
                        pcurve: line2([0., 0.], [1., 0.]),
                    },
                    Coedge {
                        edge: 1,
                        reversed: false,
                        pcurve: line2([1., 0.], [1., 1.]),
                    },
                    Coedge {
                        edge: 0,
                        reversed: true,
                        pcurve: line2([1., 1.], [0., 1.]),
                    },
                    Coedge {
                        edge: 2,
                        reversed: true,
                        pcurve: line2([0., 1.], [0., 0.]),
                    },
                ],
            }],
            faces: vec![Face {
                surface: surface.clone(),
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
            tolerance_mm: 1e-6,
        },
        TopologyIds::default(),
    );
    model.rebuild_topology_ids();
    model.validate().unwrap();
    let (text, _, _) = cad_step::export_step_v9(&model).unwrap();
    let (back, _, _) = cad_step::import_step_v9(&text).unwrap();
    assert_eq!(back.edges.len(), 3);
    let mut samples = Vec::new();
    for u in [0., 0.13, 0.37, 0.83, 1.] {
        for v in [0., 0.17, 0.37, 0.83, 1.] {
            let expected = surface.evaluate(u, v).unwrap();
            let actual = back.faces[0].surface.evaluate(u, v).unwrap();
            for k in 0..3 {
                assert!((actual.point[k] - expected.point[k]).abs() < 1e-9);
            }
            let (_, dv) = expected.first_derivatives().unwrap();
            samples.push(value_codec::json!({"u":u,"v":v,"point":expected.point,"dv":dv}));
        }
    }
    for u in [0., 0.13, 0.37, 0.83, 1.] {
        let a = back.faces[0].surface.evaluate(u, 0.).unwrap();
        let b = back.faces[0].surface.evaluate(u, 1.).unwrap();
        let (_, av) = a.first_derivatives().unwrap();
        let (_, bv) = b.first_derivatives().unwrap();
        let (_, _, avv) = a.second_derivatives().unwrap();
        let (_, _, bvv) = b.second_derivatives().unwrap();
        for k in 0..3 {
            assert!((a.point[k] - b.point[k]).abs() < 1e-9);
            assert!((av[k] - bv[k]).abs() < 1e-9);
            assert!((avv[k] - bvv[k]).abs() < 1e-9);
        }
    }
    if let Ok(dir) = std::env::var("LOFT_STEP_MATRIX_OUTPUT") {
        std::fs::write(format!("{dir}/closed-shared-seam.step"), text).unwrap();
        std::fs::write(
            format!("{dir}/closed-shared-seam.samples.json"),
            value_codec::to_string(&samples).unwrap(),
        )
        .unwrap();
    }
}
