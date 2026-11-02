use brep_core::{Model, TopologyIds};
use brep_topology::{Body, Coedge, Edge, Face, FaceUse, Loop, Shell, Vertex};
use nurbs_core::{curve::Curve, surface::Surface};
pub fn source() -> Model {
    let p = [
        [1., 0.],
        [1., 1.],
        [0., 1.],
        [-1., 1.],
        [-1., 0.],
        [-1., -1.],
        [0., -1.],
        [1., -1.],
        [1., 0.],
        [1., 1.],
    ];
    let curve = Curve {
        degree: 2,
        knots: vec![-1., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 5., 5.],
        control_points: p.iter().map(|p| vec![p[0], p[1], 0.]).collect(),
        weights: (0..10)
            .map(|i| {
                if i % 2 == 0 {
                    1.
                } else {
                    std::f64::consts::FRAC_1_SQRT_2
                }
            })
            .collect(),
        periodic: true,
    };
    let pc = Curve {
        control_points: p.iter().map(|p| p.to_vec()).collect(),
        ..curve.clone()
    };
    let graph = |sign: f64| {
        let x = [-1., 0., 1.];
        let square = [1., -1., 1.];
        Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![-1., -1., -1., 1., 1., 1.],
            knots_v: vec![-1., -1., -1., 1., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| vec![x[i], x[j], sign * (1. - square[i] - square[j])])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    };
    let mut s = Model(
        brep_topology::Model {
            vertices: vec![Vertex {
                point: [1., 0., 0.],
            }],
            edges: vec![Edge {
                vertices: [0, 0],
                curve,
                degenerate: false,
            }],
            loops: vec![
                Loop {
                    coedges: vec![Coedge::new(0, false, pc.clone())],
                },
                Loop {
                    coedges: vec![Coedge::new(0, false, pc)],
                },
            ],
            faces: vec![
                Face {
                    surface: graph(1.),
                    outer: 0,
                    holes: vec![],
                },
                Face {
                    surface: graph(-1.),
                    outer: 1,
                    holes: vec![],
                },
            ],
            shells: vec![Shell {
                faces: vec![
                    FaceUse {
                        face: 0,
                        reversed: false,
                    },
                    FaceUse {
                        face: 1,
                        reversed: true,
                    },
                ],
                closed: true,
            }],
            bodies: vec![Body {
                outer_shell: 0,
                inner_shells: vec![],
            }],
            tolerance_mm: 1e-7,
        },
        TopologyIds::default(),
    );
    s.rebuild_topology_ids();
    s.validate().unwrap();
    s
}
