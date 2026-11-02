use brep_core::Model;
use brep_topology::{Coedge, Edge, Face, FaceUse, Loop, Vertex};
use nurbs_core::curve::Curve;
#[path = "full_periodic_cylinder.rs"]
mod cylinder;
/// A closed cylinder partitioned into a full-period face with a trim hole and
/// the original curved patch occupying that hole. No through-cut is inferred.
pub fn source(u: [f64; 2], v: [f64; 2]) -> Model {
    let mut s = cylinder::source();
    let uv = [[u[0], v[0]], [u[0], v[1]], [u[1], v[1]], [u[1], v[0]]];
    let vertex = s.vertices.len();
    for p in uv {
        let e = s.faces[0].surface.evaluate(p[0], p[1]).unwrap();
        s.vertices.push(Vertex {
            point: [e.point[0], e.point[1], e.point[2]],
        });
    }
    let left = Curve::from_polyline(vec![
        s.vertices[vertex].point.to_vec(),
        s.vertices[vertex + 1].point.to_vec(),
    ])
    .unwrap();
    let right = Curve::from_polyline(vec![
        s.vertices[vertex + 3].point.to_vec(),
        s.vertices[vertex + 2].point.to_vec(),
    ])
    .unwrap();
    let arc = |z: f64| {
        let mut c = s.edges[0].curve.trim(u[0], u[1]).unwrap();
        for p in &mut c.control_points {
            p[2] = z;
        }
        c
    };
    let top = arc(2. * v[1]);
    let bottom = arc(2. * v[0]);
    let edge = s.edges.len();
    s.edges.extend([
        Edge {
            curve: left,
            vertices: [vertex, vertex + 1],
            degenerate: false,
        },
        Edge {
            curve: top,
            vertices: [vertex + 1, vertex + 2],
            degenerate: false,
        },
        Edge {
            curve: right,
            vertices: [vertex + 3, vertex + 2],
            degenerate: false,
        },
        Edge {
            curve: bottom,
            vertices: [vertex, vertex + 3],
            degenerate: false,
        },
    ]);
    let line =
        |a: usize, b: usize| Curve::from_polyline(vec![uv[a].to_vec(), uv[b].to_vec()]).unwrap();
    let hole = s.loops.len();
    s.loops.push(Loop {
        coedges: vec![
            Coedge::new(edge, false, line(0, 1)),
            Coedge::new(edge + 1, false, line(1, 2)),
            Coedge::new(edge + 2, true, line(2, 3)),
            Coedge::new(edge + 3, true, line(3, 0)),
        ],
    });
    s.faces[0].holes.push(hole);
    let patch = s.loops.len();
    s.loops.push(Loop {
        coedges: vec![
            Coedge::new(edge + 3, false, line(0, 3)),
            Coedge::new(edge + 2, false, line(3, 2)),
            Coedge::new(edge + 1, true, line(2, 1)),
            Coedge::new(edge, true, line(1, 0)),
        ],
    });
    let face = s.faces.len();
    let surface = s.faces[0].surface.clone();
    s.faces.push(Face {
        surface,
        outer: patch,
        holes: vec![],
    });
    s.shells[0].faces.push(FaceUse {
        face,
        reversed: false,
    });
    s.rebuild_topology_ids();
    s.validate().unwrap();
    s
}
