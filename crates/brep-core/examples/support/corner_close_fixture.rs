use brep_core::{
    Coedge, Edge, Face, FaceUse, Loop, Model, Shell, Vertex, corner_close::BoundaryUse,
};
use nurbs_core::{
    corner_blend::Options,
    surface::{Axis, Surface},
    surface_blend::Boundary,
};
fn height(x: f64, y: f64) -> f64 {
    x * x / 4. + y * y / 8.
}
fn top(x: [f64; 2], y: [f64; 2]) -> Surface {
    let square = |a: [f64; 2]| [a[0] * a[0], a[0] * a[1], a[1] * a[1]];
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        vec![
                            x[0] + (x[1] - x[0]) * i as f64 / 2.,
                            y[0] + (y[1] - y[0]) * j as f64 / 2.,
                            square(x)[i] / 4. + square(y)[j] / 8.,
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
fn add(model: &mut Model, s: Surface, vertices: [usize; 4], reversed: bool) -> usize {
    let curves = [
        s.iso(Axis::V, 0.).unwrap(),
        s.iso(Axis::U, 1.).unwrap(),
        s.iso(Axis::V, 1.).unwrap(),
        s.iso(Axis::U, 0.).unwrap(),
    ];
    let pairs = [
        [vertices[0], vertices[1]],
        [vertices[1], vertices[2]],
        [vertices[3], vertices[2]],
        [vertices[0], vertices[3]],
    ];
    let pcs = [
        vec![vec![0., 0.], vec![1., 0.]],
        vec![vec![1., 0.], vec![1., 1.]],
        vec![vec![1., 1.], vec![0., 1.]],
        vec![vec![0., 1.], vec![0., 0.]],
    ];
    let mut coedges = Vec::new();
    for i in 0..4 {
        let pair = pairs[i];
        let edge = model
            .edges
            .iter()
            .position(|e| e.vertices == pair || e.vertices == [pair[1], pair[0]])
            .unwrap_or_else(|| {
                let n = model.edges.len();
                model.edges.push(Edge {
                    curve: curves[i].clone(),
                    vertices: pair,
                    degenerate: false,
                });
                n
            });
        let traversal = if i >= 2 { [pair[1], pair[0]] } else { pair };
        coedges.push(Coedge::new(
            edge,
            model.edges[edge].vertices != traversal,
            nurbs_core::curve::Curve::from_polyline(pcs[i].clone()).unwrap(),
        ));
    }
    let face = model.faces.len();
    let outer = model.loops.len();
    model.loops.push(Loop { coedges });
    model.faces.push(Face {
        surface: s,
        outer,
        holes: vec![],
    });
    model.shells[0].faces.push(FaceUse { face, reversed });
    face
}
pub fn fixture() -> (Model, [BoundaryUse; 4], Options) {
    let grid = [-1., 0., 1., 2.];
    let id = |x: usize, y: usize, z: usize| z * 16 + x * 4 + y;
    let mut m = Model::empty(1e-7).unwrap();
    for z in 0..2 {
        for &x in &grid {
            for &y in &grid {
                m.vertices.push(Vertex {
                    point: [x, y, if z == 0 { -3. } else { height(x, y) }],
                });
            }
        }
    }
    m.shells.push(Shell {
        faces: vec![],
        closed: false,
    });
    let mut supports = [0usize; 4];
    for x in 0..3 {
        for y in 0..3 {
            let corners = |z| {
                [
                    id(x, y, z),
                    id(x + 1, y, z),
                    id(x + 1, y + 1, z),
                    id(x, y + 1, z),
                ]
            };
            let mut bottom = top([grid[x], grid[x + 1]], [grid[y], grid[y + 1]]);
            for p in bottom.control_points.iter_mut().flatten() {
                p[2] = -3.;
            }
            add(&mut m, bottom, corners(0), true);
            if x != 1 || y != 1 {
                let f = add(
                    &mut m,
                    top([grid[x], grid[x + 1]], [grid[y], grid[y + 1]]),
                    corners(1),
                    false,
                );
                match (x, y) {
                    (0, 1) => supports[0] = f,
                    (2, 1) => supports[1] = f,
                    (1, 0) => supports[2] = f,
                    (1, 2) => supports[3] = f,
                    _ => {}
                }
            }
        }
    }
    for axis in 0..2 {
        for end in [0usize, 3] {
            for k in 0..3 {
                let (a, b) = if axis == 0 {
                    ([end, k], [end, k + 1])
                } else {
                    ([k, end], [k + 1, end])
                };
                let corners = [
                    id(a[0], a[1], 0),
                    id(b[0], b[1], 0),
                    id(b[0], b[1], 1),
                    id(a[0], a[1], 1),
                ];
                let s = top([grid[a[0]], grid[b[0]]], [grid[a[1]], grid[b[1]]]);
                let (curved, degree, knots) = if axis == 0 {
                    (s.iso(Axis::U, 0.).unwrap(), s.degree_v, s.knots_v)
                } else {
                    (s.iso(Axis::V, 0.).unwrap(), s.degree_u, s.knots_u)
                };
                let wall = Surface {
                    degree_u: degree,
                    degree_v: 1,
                    knots_u: knots,
                    knots_v: vec![0., 0., 1., 1.],
                    control_points: curved
                        .control_points
                        .iter()
                        .map(|p| vec![vec![p[0], p[1], -3.], p.clone()])
                        .collect(),
                    weights: vec![vec![1.; 2]; curved.control_points.len()],
                    periodic_u: false,
                    periodic_v: false,
                };
                add(
                    &mut m,
                    wall,
                    corners,
                    (axis == 0 && end == 0) || (axis == 1 && end == 3),
                );
            }
        }
    }
    m.rebuild_topology_ids();
    m.validate().unwrap();
    let options = Options {
        boundaries: [
            Boundary::UEnd,
            Boundary::UStart,
            Boundary::VEnd,
            Boundary::VStart,
        ],
        reverse_along: [false; 4],
        gains: [1.; 4],
        compatibility_tolerance: 1e-9,
        max_controls: 4096,
    };
    let uses = std::array::from_fn(|side| {
        let face = supports[side];
        let fixed = usize::from(side >= 2);
        let value = if side == 0 || side == 2 { 1. } else { 0. };
        let edge = m.loops[m.faces[face].outer]
            .coedges
            .iter()
            .find(|c| c.pcurve.control_points.iter().all(|p| p[fixed] == value))
            .unwrap()
            .edge;
        BoundaryUse { face, edge }
    });
    (m, uses, options)
}
/// Same owned body boundary, with top faces trimmed from one larger immutable
/// source surface. Rim edges now lie inside its chart, not on natural sides.
pub fn fixture_extended() -> (Model, [BoundaryUse; 4], Options) {
    let (mut m, uses, mut options) = fixture();
    let mut original = top([-1., 2.], [-1., 2.]);
    for k in original.knots_u.iter_mut().chain(&mut original.knots_v) {
        *k = 2. + 6. * *k;
    }
    let faces = m
        .faces
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            f.surface.degree_u == 2
                && f.surface.degree_v == 2
                && f.surface
                    .control_points
                    .iter()
                    .flatten()
                    .any(|p| p[2] != -3.)
        })
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    for face in faces {
        let origin = m.faces[face].surface.control_points[0][0].clone();
        let wire = m.faces[face].outer;
        for c in &mut m.loops[wire].coedges {
            for p in &mut c.pcurve.control_points {
                p[0] = 2. * p[0] + 2. * origin[0] + 4.;
                p[1] = 2. * p[1] + 2. * origin[1] + 4.;
            }
        }
        m.faces[face].surface = original.clone();
    }
    options.gains = [2.; 4];
    m.rebuild_topology_ids();
    m.validate().unwrap();
    (m, uses, options)
}
