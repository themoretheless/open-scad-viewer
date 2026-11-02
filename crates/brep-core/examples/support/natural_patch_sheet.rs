//! Actual natural-boundary OPEN sheet fixture for unit-domain surfaces.
use brep_core::{Coedge, Edge, Face, FaceUse, Loop, Model, Shell, Vertex};
use nurbs_core::{
    curve::Curve,
    surface::{Axis, Surface},
};
pub fn unit_sheet(s: &Surface) -> Model {
    assert_eq!(
        [
            s.knots_u[s.degree_u],
            s.knots_u[s.control_points.len()],
            s.knots_v[s.degree_v],
            s.knots_v[s.control_points[0].len()]
        ],
        [0., 1., 0., 1.]
    );
    let curves = [
        s.iso(Axis::V, 0.).unwrap(),
        s.iso(Axis::U, 1.).unwrap(),
        s.iso(Axis::V, 1.).unwrap().reverse().unwrap(),
        s.iso(Axis::U, 0.).unwrap().reverse().unwrap(),
    ];
    let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let mut model = Model::empty(1e-7).unwrap();
    for c in &curves {
        let p = &c.control_points[0];
        model.vertices.push(Vertex {
            point: [p[0], p[1], p[2]],
        });
    }
    for (i, c) in curves.into_iter().enumerate() {
        model.edges.push(Edge {
            vertices: [i, (i + 1) % 4],
            curve: c,
            degenerate: false,
        });
    }
    model.loops.push(Loop {
        coedges: (0..4)
            .map(|i| {
                Coedge::new(
                    i,
                    false,
                    Curve::from_polyline(vec![uv[i].to_vec(), uv[(i + 1) % 4].to_vec()]).unwrap(),
                )
            })
            .collect(),
    });
    model.faces.push(Face {
        surface: s.clone(),
        outer: 0,
        holes: vec![],
    });
    model.shells.push(Shell {
        faces: vec![FaceUse {
            face: 0,
            reversed: false,
        }],
        closed: false,
    });
    model.rebuild_topology_ids();
    model.validate().unwrap();
    model
}
