use brep_core::{Coedge, Edge, Face, FaceUse, Loop, Model, Shell, Vertex};
use nurbs_core::{
    curve::Curve,
    surface::{Axis, Surface},
};
pub fn sheet(surface: Surface) -> Model {
    let mut m = Model::empty(1e-7).unwrap();
    let corners = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    m.vertices = corners
        .iter()
        .map(|p| Vertex {
            point: surface.evaluate(p[0], p[1]).unwrap().point,
        })
        .collect();
    let curves = [
        surface.iso(Axis::V, 0.).unwrap(),
        surface.iso(Axis::U, 1.).unwrap(),
        surface.iso(Axis::V, 1.).unwrap(),
        surface.iso(Axis::U, 0.).unwrap(),
    ];
    let pairs = [[0, 1], [1, 2], [3, 2], [0, 3]];
    m.edges = curves
        .into_iter()
        .zip(pairs)
        .map(|(curve, vertices)| Edge {
            curve,
            vertices,
            degenerate: false,
        })
        .collect();
    m.loops.push(Loop {
        coedges: (0..4)
            .map(|i| {
                Coedge::new(
                    i,
                    i >= 2,
                    Curve::from_polyline(vec![corners[i].to_vec(), corners[(i + 1) % 4].to_vec()])
                        .unwrap(),
                )
            })
            .collect(),
    });
    m.faces.push(Face {
        surface,
        outer: 0,
        holes: vec![],
    });
    m.shells.push(Shell {
        closed: false,
        faces: vec![FaceUse {
            face: 0,
            reversed: false,
        }],
    });
    m.rebuild_topology_ids();
    m.validate().unwrap();
    m
}
pub fn pair() -> (Model, Model) {
    let a = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        vec![
                            0.25 + 0.125 * i as f64,
                            j as f64 / 2.,
                            if i == 2 { 0.0625 } else { 0. } + if j == 2 { 1.02 } else { 0. },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let b = Surface {
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        vec![
                            i as f64 / 2.,
                            j as f64 / 2.,
                            0.25 + if i == 2 { 0.125 } else { 0. } + if j == 2 { 0.02 } else { 0. },
                        ]
                    })
                    .collect()
            })
            .collect(),
        ..a.clone()
    };
    (sheet(a), sheet(b))
}
