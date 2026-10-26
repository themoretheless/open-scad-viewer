//! Closed analytic control: 0<=x,y<=1, -1<=z<=y+x²/4.
use brep_core::Model;
use nurbs_core::{curve_surface_agreement, surface::Surface};
pub fn build() -> Model {
    let original = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let mut model = original.clone();
    let warp = |p: [f64; 3]| [p[0], p[1], -1. + p[2] * (1. + p[1] + p[0] * p[0] / 4.)];
    for vertex in &mut model.vertices {
        vertex.point = warp(vertex.point);
    }
    for (face, old) in model.faces.iter_mut().zip(&original.faces) {
        let s = &old.surface;
        let a = s.evaluate(0., 0.).unwrap().point;
        let b = s.evaluate(1., 0.).unwrap().point;
        let c = s.evaluate(0., 1.).unwrap().point;
        let xu = b[0] != a[0];
        let xv = c[0] != a[0];
        let du = if xu { 2 } else { 1 };
        let dv = if xv { 2 } else { 1 };
        let square = |start: f64, end: f64, k: usize| [start * start, start * end, end * end][k];
        let points = (0..=du)
            .map(|i| {
                (0..=dv)
                    .map(|j| {
                        let p = s
                            .evaluate(i as f64 / du as f64, j as f64 / dv as f64)
                            .unwrap()
                            .point;
                        let x2 = if xu {
                            square(a[0], b[0], i)
                        } else if xv {
                            square(a[0], c[0], j)
                        } else {
                            a[0] * a[0]
                        };
                        vec![p[0], p[1], -1. + p[2] * (1. + p[1] + x2 / 4.)]
                    })
                    .collect()
            })
            .collect();
        face.surface = Surface {
            degree_u: du,
            degree_v: dv,
            knots_u: [vec![0.; du + 1], vec![1.; du + 1]].concat(),
            knots_v: [vec![0.; dv + 1], vec![1.; dv + 1]].concat(),
            control_points: points,
            weights: vec![vec![1.; dv + 1]; du + 1],
            periodic_u: false,
            periodic_v: false,
        };
    }
    for edge in 0..model.edges.len() {
        let (face, use_) = model
            .faces
            .iter()
            .find_map(|f| {
                model.loops[f.outer]
                    .coedges
                    .iter()
                    .find(|c| c.edge == edge)
                    .map(|c| (f, c))
            })
            .unwrap();
        let mut ends = model.edges[edge].vertices.map(|v| model.vertices[v].point);
        if use_.reversed {
            ends.reverse();
        }
        let lift = curve_surface_agreement::lift(
            &use_.pcurve,
            &face.surface,
            Some(ends),
            model.tolerance_mm / 2.,
            100000,
        )
        .unwrap();
        let curve = lift.curve.unwrap();
        model.edges[edge].curve = if use_.reversed {
            curve.reverse().unwrap()
        } else {
            curve
        };
    }
    model.rebuild_topology_ids();
    model.validate().unwrap();
    model
}

/// Exact chart-axis permutation; world geometry and shell orientation stay fixed.
#[allow(dead_code)]
pub fn transpose_charts(source: &mut Model) {
    for index in 0..source.faces.len() {
        let face = &mut source.faces[index];
        let old = face.surface.clone();
        face.surface.degree_u = old.degree_v;
        face.surface.degree_v = old.degree_u;
        face.surface.knots_u = old.knots_v;
        face.surface.knots_v = old.knots_u;
        face.surface.control_points = (0..old.control_points[0].len())
            .map(|j| {
                old.control_points
                    .iter()
                    .map(|row| row[j].clone())
                    .collect()
            })
            .collect();
        face.surface.weights = (0..old.weights[0].len())
            .map(|j| old.weights.iter().map(|row| row[j]).collect())
            .collect();
        // Keep a positive local trim winding after reversing the chart.
        let wires = std::iter::once(face.outer)
            .chain(face.holes.iter().copied())
            .collect::<Vec<_>>();
        for wire in wires {
            let ring = &mut source.loops[wire].coedges;
            ring.reverse();
            for use_ in ring {
                use_.reversed = !use_.reversed;
                use_.pcurve = use_.pcurve.reverse().unwrap();
                for p in &mut use_.pcurve.control_points {
                    p.swap(0, 1);
                }
            }
        }
    }
    for shell in &mut source.shells {
        for use_ in &mut shell.faces {
            use_.reversed = !use_.reversed;
        }
    }
    source.rebuild_topology_ids();
    source.validate().unwrap();
}
