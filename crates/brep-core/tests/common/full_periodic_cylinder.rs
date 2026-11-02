use brep_core::{Model, TopologyIds};
use brep_topology::{Body, Coedge, Edge, Face, FaceUse, Loop, Shell, Vertex};
use nurbs_core::{curve::Curve, surface::Surface};
pub fn source() -> Model {
    let points = [
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
    let bottom = Curve {
        degree: 2,
        knots: vec![-1., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 5., 5.],
        control_points: points.iter().map(|p| vec![p[0], p[1], 0.]).collect(),
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
    let mut top = bottom.clone();
    for p in &mut top.control_points {
        p[2] = 2.;
    }
    let cap = |z| Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-1., -1., 1., 1.],
        knots_v: vec![-1., -1., 1., 1.],
        control_points: vec![
            vec![vec![-1., -1., z], vec![-1., 1., z]],
            vec![vec![1., -1., z], vec![1., 1., z]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let pc = Curve {
        control_points: points.iter().map(|p| p.to_vec()).collect(),
        ..bottom.clone()
    };
    let line =
        |a: [f64; 2], b: [f64; 2]| Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap();
    let mut s = Model(
        brep_topology::Model {
            vertices: vec![
                Vertex {
                    point: [1., 0., 0.],
                },
                Vertex {
                    point: [1., 0., 2.],
                },
            ],
            edges: vec![
                Edge {
                    curve: bottom.clone(),
                    vertices: [0, 0],
                    degenerate: false,
                },
                Edge {
                    curve: top,
                    vertices: [1, 1],
                    degenerate: false,
                },
                Edge {
                    curve: Curve::from_polyline(vec![vec![1., 0., 0.], vec![1., 0., 2.]]).unwrap(),
                    vertices: [0, 1],
                    degenerate: false,
                },
            ],
            loops: vec![
                Loop {
                    coedges: vec![
                        Coedge::new(0, false, line([0., 0.], [4., 0.])),
                        Coedge::new(2, false, line([4., 0.], [4., 1.])),
                        Coedge::new(1, true, line([4., 1.], [0., 1.])),
                        Coedge::new(2, true, line([0., 1.], [0., 0.])),
                    ],
                },
                Loop {
                    coedges: vec![Coedge::new(0, false, pc.clone())],
                },
                Loop {
                    coedges: vec![Coedge::new(1, false, pc)],
                },
            ],
            faces: vec![
                Face {
                    surface: nurbs_core::surface::extrude(&bottom, [0., 0., 2.]).unwrap(),
                    outer: 0,
                    holes: vec![],
                },
                Face {
                    surface: cap(0.),
                    outer: 1,
                    holes: vec![],
                },
                Face {
                    surface: cap(2.),
                    outer: 2,
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
                    FaceUse {
                        face: 2,
                        reversed: false,
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
