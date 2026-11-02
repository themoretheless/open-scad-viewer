//! Rational curved stock and original-chart contact data for trimming tests.
use brep_core::{Model, edge_partition, face_cut, region_replacement as r};
use nurbs_core::{
    curve::Curve,
    surface::{Axis, Surface},
};
pub struct Cut {
    pub face: usize,
    pub endpoints: [usize; 2],
    pub segments: Vec<face_cut::Segment>,
    pub replacement: Vec<Curve>,
    pub anchor: r::RemovalAnchor,
    pub smooth: bool,
}
pub struct Fixture {
    pub source: Model,
    pub prepared: Model,
    pub blend: Surface,
    pub cuts: Vec<Cut>,
}
fn chart(old: &Surface, p: &[f64]) -> Vec<f64> {
    let a = &old.control_points[0][0];
    let b = &old.control_points[1][0];
    let c = &old.control_points[0][1];
    [b, c]
        .iter()
        .map(|q| {
            let k = (0..3).find(|k| q[*k] != a[*k]).unwrap();
            (p[k] - a[k]) / (q[k] - a[k])
        })
        .collect()
}
pub fn build() -> Fixture {
    let cube = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let spine = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0., 0.], vec![0.5, 0.25, 5.], vec![0.25, 0., 10.]],
        weights: vec![1., 0.875, 1.],
        periodic: false,
    };
    let mut source = cube.clone();
    for (v, old) in source.vertices.iter_mut().zip(&cube.vertices) {
        let c = &spine.control_points[if old.point[2] == 0. { 0 } else { 2 }];
        v.point = [c[0] + 4. * old.point[0], c[1] + 4. * old.point[1], c[2]];
    }
    for (face, old) in source.faces.iter_mut().zip(&cube.faces) {
        let s = &old.surface;
        let a = &s.control_points[0][0];
        let b = &s.control_points[1][0];
        let c = &s.control_points[0][1];
        let axis = if b[2] != a[2] {
            Some(0)
        } else if c[2] != a[2] {
            Some(1)
        } else {
            None
        };
        let du = if axis == Some(0) { 2 } else { 1 };
        let dv = if axis == Some(1) { 2 } else { 1 };
        let mut points = vec![vec![vec![0.; 3]; dv + 1]; du + 1];
        let mut weights = vec![vec![1.; dv + 1]; du + 1];
        for i in 0..=du {
            for j in 0..=dv {
                let pos = s
                    .evaluate(i as f64 / du as f64, j as f64 / dv as f64)
                    .unwrap()
                    .point;
                let index = if let Some(axis) = axis {
                    let n = if axis == 0 { i } else { j };
                    let forward = if axis == 0 { b[2] > a[2] } else { c[2] > a[2] };
                    if forward { n } else { 2 - n }
                } else if a[2] == 0. {
                    0
                } else {
                    2
                };
                let cp = &spine.control_points[index];
                points[i][j] = vec![cp[0] + 4. * pos[0], cp[1] + 4. * pos[1], cp[2]];
                if axis.is_some() {
                    weights[i][j] = spine.weights[index];
                }
            }
        }
        face.surface = Surface {
            degree_u: du,
            degree_v: dv,
            knots_u: [vec![0.; du + 1], vec![1.; du + 1]].concat(),
            knots_v: [vec![0.; dv + 1], vec![1.; dv + 1]].concat(),
            control_points: points,
            weights,
            periodic_u: false,
            periodic_v: false,
        };
    }
    for e in 0..source.edges.len() {
        let (f, c) = source
            .faces
            .iter()
            .find_map(|f| {
                source.loops[f.outer]
                    .coedges
                    .iter()
                    .find(|c| c.edge == e)
                    .map(|c| (f, c))
            })
            .unwrap();
        let a = &c.pcurve.control_points[0];
        let b = c.pcurve.control_points.last().unwrap();
        let mut curve = if a[0] == b[0] {
            f.surface.iso(Axis::U, a[0]).unwrap()
        } else {
            f.surface.iso(Axis::V, a[1]).unwrap()
        };
        if curve.control_points[0].as_slice() != source.vertices[source.edges[e].vertices[0]].point
        {
            curve = curve.reverse().unwrap();
        }
        assert_eq!(
            curve.control_points[0].as_slice(),
            source.vertices[source.edges[e].vertices[0]].point
        );
        source.edges[e].curve = curve;
    }
    source.rebuild_topology_ids();
    source.validate().unwrap();
    let face = |axis: usize, value: f64| {
        cube.faces
            .iter()
            .position(|f| {
                f.surface
                    .control_points
                    .iter()
                    .flatten()
                    .all(|p| p[axis] == value)
            })
            .unwrap()
    };
    let sharp = cube
        .edges
        .iter()
        .position(|e| {
            e.vertices
                .iter()
                .all(|v| cube.vertices[*v].point[0] == 0. && cube.vertices[*v].point[1] == 0.)
        })
        .unwrap();
    let ends = [0., 1.].map(|z| {
        cube.edges[sharp]
            .vertices
            .into_iter()
            .find(|v| cube.vertices[*v].point[2] == z)
            .unwrap()
    });
    let mut prepared = source.clone();
    let mut roots = [[0; 2]; 2];
    for end in 0..2 {
        for axis in 0..2 {
            let e = cube
                .edges
                .iter()
                .position(|e| {
                    let p = e.vertices.map(|v| cube.vertices[v].point);
                    p[0][2] == end as f64
                        && p[1][2] == end as f64
                        && p[0][1 - axis] == 0.
                        && p[1][1 - axis] == 0.
                        && p[0][axis] != p[1][axis]
                })
                .unwrap();
            let t = if cube.vertices[cube.edges[e].vertices[0]].point[axis] == 0. {
                0.25
            } else {
                0.75
            };
            let uses = prepared
                .loops
                .iter()
                .enumerate()
                .flat_map(|(w, l)| {
                    l.coedges
                        .iter()
                        .enumerate()
                        .filter(move |(_, c)| c.edge == e)
                        .map(move |(i, c)| edge_partition::PcurveCut {
                            wire: w,
                            coedge: i,
                            parameter: if c.reversed { 1. - t } else { t },
                        })
                })
                .collect::<Vec<_>>();
            let split = edge_partition::partition(&prepared, e, t, &uses, 100_000).unwrap();
            roots[end][axis] = split.vertex;
            prepared = split.model;
        }
    }
    let blend = r::propose_nurbs_edge_transition(
        &spine,
        &[1.; 3],
        [[1., 0., 0.], [0., 1., 0.]],
        0.3125 / 2f64.sqrt(),
    )
    .unwrap()
    .blend;
    let curves = [
        blend.iso(Axis::V, 0.).unwrap(),
        blend.iso(Axis::U, 1.).unwrap(),
        blend.iso(Axis::V, 1.).unwrap().reverse().unwrap(),
        blend.iso(Axis::U, 0.).unwrap().reverse().unwrap(),
    ];
    let faces = [face(2, 0.), face(0, 0.), face(2, 1.), face(1, 0.)];
    let endpoints = [
        [roots[0][0], roots[0][1]],
        [roots[0][1], roots[1][1]],
        [roots[1][1], roots[1][0]],
        [roots[1][0], roots[0][0]],
    ];
    let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let cuts = curves
        .into_iter()
        .enumerate()
        .map(|(i, curve)| {
            let pc = if i % 2 == 0 {
                let end = i / 2;
                let o = &spine.control_points[if end == 0 { 0 } else { 2 }];
                Curve {
                    control_points: curve
                        .control_points
                        .iter()
                        .map(|p| {
                            chart(
                                &cube.faces[faces[i]].surface,
                                &[(p[0] - o[0]) / 4., (p[1] - o[1]) / 4., end as f64],
                            )
                        })
                        .collect(),
                    degree: curve.degree,
                    knots: curve.knots.clone(),
                    weights: curve.weights.clone(),
                    periodic: false,
                }
            } else {
                let positions = if i == 1 {
                    [[0., 0.25, 0.], [0., 0.25, 1.]]
                } else {
                    [[0.25, 0., 1.], [0.25, 0., 0.]]
                };
                Curve::from_polyline(
                    positions
                        .iter()
                        .map(|p| chart(&cube.faces[faces[i]].surface, p))
                        .collect(),
                )
                .unwrap()
            };
            Cut {
                face: faces[i],
                endpoints: endpoints[i],
                segments: vec![face_cut::Segment { curve, pcurve: pc }],
                replacement: vec![
                    Curve::from_polyline(vec![uv[i].to_vec(), uv[(i + 1) % 4].to_vec()]).unwrap(),
                ],
                anchor: if i % 2 == 0 {
                    r::RemovalAnchor::Vertex(ends[i / 2])
                } else {
                    r::RemovalAnchor::Edge(sharp)
                },
                smooth: i % 2 == 1,
            }
        })
        .collect();
    Fixture {
        source,
        prepared,
        blend,
        cuts,
    }
}
