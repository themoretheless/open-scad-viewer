//! Authored dyadic hybrid source fixture. Every trace/region/contact is then
//! freshly proved by native admission; fixture evaluation supplies no authority.
use super::*;
use nurbs_core::surface::Surface;
use std::collections::BTreeMap;
fn split(a: &[Vec<f64>]) -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
    let mut a = a.to_vec();
    let mut l = vec![a[0].clone()];
    let mut r = vec![a.last().unwrap().clone()];
    for n in (1..a.len()).rev() {
        for i in 0..n {
            a[i] = (0..3).map(|k| (a[i][k] + a[i + 1][k]) * 0.5).collect();
        }
        l.push(a[0].clone());
        r.push(a[n - 1].clone());
    }
    r.reverse();
    (l, r)
}
fn half(s: &Surface, axis: usize, upper: bool) -> Surface {
    let mut t = s.clone();
    if axis == 0 {
        for j in 0..s.control_points[0].len() {
            let column = s
                .control_points
                .iter()
                .map(|r| r[j].clone())
                .collect::<Vec<_>>();
            let (l, r) = split(&column);
            let a = if upper { r } else { l };
            for i in 0..a.len() {
                t.control_points[i][j] = a[i].clone();
            }
        }
    } else {
        for i in 0..s.control_points.len() {
            let (l, r) = split(&s.control_points[i]);
            t.control_points[i] = if upper { r } else { l };
        }
    }
    t
}
fn elevate_line(a: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = a.len();
    let mut b = vec![a[0].clone()];
    for i in 1..n {
        b.push(
            (0..3)
                .map(|k| (i as f64 * a[i - 1][k] + (n - i) as f64 * a[i][k]) / n as f64)
                .collect(),
        );
    }
    b.push(a.last().unwrap().clone());
    b
}
fn cubic(mut s: Surface) -> Surface {
    while s.degree_u < 3 {
        let mut net =
            vec![vec![vec![0.; 3]; s.control_points[0].len()]; s.control_points.len() + 1];
        for j in 0..s.control_points[0].len() {
            let a = s
                .control_points
                .iter()
                .map(|r| r[j].clone())
                .collect::<Vec<_>>();
            for (i, p) in elevate_line(&a).into_iter().enumerate() {
                net[i][j] = p;
            }
        }
        s.control_points = net;
        s.degree_u += 1;
    }
    while s.degree_v < 3 {
        s.control_points = s.control_points.iter().map(|r| elevate_line(r)).collect();
        s.degree_v += 1;
    }
    s.knots_u = [vec![0.; 4], vec![1.; 4]].concat();
    s.knots_v = s.knots_u.clone();
    s.weights = vec![vec![1.; 4]; 4];
    s
}
fn clean(mut p: Vec<f64>) -> Vec<f64> {
    for x in &mut p {
        if *x == 0. {
            *x = 0.;
        }
    }
    p
}
fn vertex(m: &mut crate::Model, map: &mut BTreeMap<[u64; 3], usize>, p: Vec<f64>) -> usize {
    let p = clean(p);
    let key = std::array::from_fn(|k| p[k].to_bits());
    *map.entry(key).or_insert_with(|| {
        let id = m.vertices.len();
        m.vertices.push(crate::Vertex {
            point: p.as_slice().try_into().unwrap(),
        });
        id
    })
}
pub(super) fn hybrid(jump: bool) -> (crate::Model, Vec<usize>, usize, Option<usize>) {
    hybrid_from(super::super::model(), jump)
}
pub(super) fn hybrid_folded(jump: bool) -> (crate::Model, Vec<usize>, usize, Option<usize>) {
    hybrid_from(super::super::model_with_folded_curvature(true), jump)
}
fn hybrid_from(
    original: crate::Model,
    jump: bool,
) -> (crate::Model, Vec<usize>, usize, Option<usize>) {
    let target = original
        .faces
        .iter()
        .position(|f| f.surface.degree_u == 1)
        .unwrap();
    let mut patches = Vec::new();
    let mut parents = Vec::new();
    let mut signs = Vec::new();
    let mut changed = None;
    for (fi, f) in original.faces.iter().enumerate() {
        let mut s = f.surface.clone();
        assert!(s.weights.iter().flatten().all(|w| *w == 1.));
        s.knots_u = [vec![0.; s.degree_u + 1], vec![1.; s.degree_u + 1]].concat();
        s.knots_v = [vec![0.; s.degree_v + 1], vec![1.; s.degree_v + 1]].concat();
        for p in s.control_points.iter_mut().flatten() {
            for x in p.iter_mut() {
                *x *= 6.;
            }
        }
        let loop_ = &original.loops[f.outer];
        let uv = loop_
            .coedges
            .iter()
            .map(|e| &e.pcurve.control_points[0])
            .collect::<Vec<_>>();
        let area = (0..uv.len())
            .map(|i| {
                let a = uv[i];
                let b = uv[(i + 1) % uv.len()];
                a[0] * b[1] - a[1] * b[0]
            })
            .sum::<f64>();
        let first = patches.len();
        let mut add = |s| {
            patches.push(s);
            parents.push(fi);
            signs.push(area > 0.);
        };
        if fi == target {
            for i in [false, true] {
                for j in [false, true] {
                    let mut p = half(&half(&s, 0, i), 1, j);
                    if jump && !i && !j {
                        p = cubic(p);
                        p.control_points[1][1][0] += 1. / 64.;
                        changed = Some(first);
                    }
                    add(p);
                }
            }
        } else {
            add(s);
        }
    }
    let corners = patches
        .iter()
        .flat_map(|s| {
            let (n, m) = (s.control_points.len() - 1, s.control_points[0].len() - 1);
            [(0, 0), (n, 0), (n, m), (0, m)].map(|(i, j)| clean(s.control_points[i][j].clone()))
        })
        .collect::<Vec<_>>();
    let mut model = original.clone();
    model.vertices.clear();
    model.edges.clear();
    model.loops.clear();
    model.faces.clear();
    let mut vertices = BTreeMap::new();
    let mut edges = BTreeMap::new();
    for (fi, s) in patches.into_iter().enumerate() {
        let (n, m) = (s.control_points.len() - 1, s.control_points[0].len() - 1);
        let uv = if signs[fi] {
            [[0., 0.], [1., 0.], [1., 1.], [0., 1.]]
        } else {
            [[0., 0.], [0., 1.], [1., 1.], [1., 0.]]
        };
        let mut coedges = Vec::new();
        for e in 0..4 {
            let (a, b) = (uv[e], uv[(e + 1) % 4]);
            let fixed = usize::from(a[0] != b[0]);
            let varying = 1 - fixed;
            let mut points = if fixed == 0 {
                s.control_points[if a[0] == 0. { 0 } else { n }].clone()
            } else {
                let j = if a[1] == 0. { 0 } else { m };
                s.control_points.iter().map(|r| r[j].clone()).collect()
            };
            if a[varying] > b[varying] {
                points.reverse();
            }
            let degree = points.len() - 1;
            let (l, r) = split(&points);
            let midpoint = clean(l.last().unwrap().clone());
            let subdivide = midpoint != clean(points[0].clone())
                && midpoint != clean(points.last().unwrap().clone())
                && corners.contains(&midpoint);
            let pieces = if subdivide {
                vec![(0., 0.5, l), (0.5, 1., r)]
            } else {
                vec![(0., 1., points)]
            };
            for (lo, hi, points) in pieces {
                let points = points.into_iter().map(clean).collect::<Vec<_>>();
                let va = vertex(&mut model, &mut vertices, points[0].clone());
                let vb = vertex(&mut model, &mut vertices, points.last().unwrap().clone());
                let degenerate = points.iter().all(|p| p == &points[0]);
                let key = (va.min(vb), va.max(vb));
                let id = if !degenerate {
                    edges.get(&key).copied()
                } else {
                    None
                };
                let id = id.unwrap_or_else(|| {
                    let id = model.edges.len();
                    let mut cp = points;
                    if va > vb {
                        cp.reverse();
                    }
                    model.edges.push(crate::Edge {
                        degenerate,
                        vertices: [key.0, key.1],
                        curve: Curve {
                            degree,
                            knots: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
                            weights: vec![1.; cp.len()],
                            control_points: cp,
                            periodic: false,
                        },
                    });
                    if !degenerate {
                        edges.insert(key, id);
                    }
                    id
                });
                let at = |t| vec![a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];
                let pcurve = Curve {
                    degree: 1,
                    knots: vec![0., 0., 1., 1.],
                    control_points: vec![at(lo), at(hi)],
                    weights: vec![1.; 2],
                    periodic: false,
                };
                coedges.push(crate::Coedge::new(id, va > vb, pcurve));
            }
        }
        let outer = model.loops.len();
        model.loops.push(crate::Loop { coedges });
        model.faces.push(crate::Face {
            surface: s,
            outer,
            holes: Vec::new(),
        });
    }
    model.shells[0].faces = parents
        .iter()
        .enumerate()
        .map(|(face, parent)| {
            let reverse = original.shells[0]
                .faces
                .iter()
                .find(|f| f.face == *parent)
                .unwrap()
                .reversed;
            crate::FaceUse {
                face,
                reversed: reverse,
            }
        })
        .collect();
    model.rebuild_topology_ids();
    model.validate().unwrap();
    (model, parents, target, changed)
}
