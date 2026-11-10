//! Exact source-template equivalences, without editing the supplied model.
//! Vertex/edge renumbering and exact Bezier U-reflection only. Other
//! gauges/orderings remain unknown. This is not tolerance-based matching.
use super::*;
use crate::exact_affine_source::exact_add;
fn raw_same(a: &Model, b: &Model) -> bool {
    a.vertices == b.vertices
        && a.edges == b.edges
        && a.loops == b.loops
        && a.faces == b.faces
        && a.shells == b.shells
        && a.bodies == b.bodies
}
fn point_key(p: [f64; 3]) -> [u64; 3] {
    p.map(|x| if x == 0. { 0 } else { x.to_bits() })
}
pub(super) fn same(a: &Model, b: &Model, r: &mut FiberedBodyReport, max: usize) -> bool {
    if raw_same(a, b) {
        return true;
    }
    // Shell face orientation may change with cap reflection below; exact
    // final comparison still checks every shell use and all incidence.
    if a.vertices.len() != b.vertices.len()
        || a.edges.len() != b.edges.len()
        || a.loops.len() != b.loops.len()
        || a.faces.len() != b.faces.len()
        || a.shells.len() != 1
        || b.shells.len() != 1
        || a.bodies != b.bodies
    {
        return false;
    }
    let extra = 2 * b.vertices.len()
        + 2 * b.edges.len()
        + b.faces
            .iter()
            .map(|f| {
                let coords = f.surface.control_points.iter().map(Vec::len).sum::<usize>();
                2 * (coords
                    + std::iter::once(f.outer)
                        .chain(f.holes.iter().copied())
                        .map(|wire| {
                            b.loops[wire]
                                .coedges
                                .iter()
                                .map(|e| e.pcurve.control_points.len() + 1)
                                .sum::<usize>()
                        })
                        .sum::<usize>()
                    + 1)
            })
            .sum::<usize>();
    if extra > max - r.binding_rows {
        return false;
    }
    r.binding_rows += extra;
    let mut actual = std::collections::BTreeMap::new();
    for (i, v) in a.vertices.iter().enumerate() {
        if actual.insert(point_key(v.point), i).is_some() {
            return false;
        }
    }
    let mut used = vec![false; a.vertices.len()];
    let mut map = Vec::with_capacity(b.vertices.len());
    for v in &b.vertices {
        let Some(&i) = actual.get(&point_key(v.point)) else {
            return false;
        };
        if used[i] {
            return false;
        }
        used[i] = true;
        map.push(i);
    }
    let mut wanted = b.clone();
    for (old, &new) in map.iter().enumerate() {
        wanted.vertices[new] = b.vertices[old].clone();
    }
    for e in &mut wanted.edges {
        e.vertices = e.vertices.map(|i| map[i]);
    }
    // Directed endpoint incidence is an indexing proposal, then every full
    // original curve is compared exactly below. Ambiguous proposals refuse.
    let mut actual_edges = std::collections::BTreeMap::new();
    for (i, e) in a.edges.iter().enumerate() {
        if actual_edges.insert(e.vertices, i).is_some() {
            return false;
        }
    }
    let mut edge_used = vec![false; a.edges.len()];
    let mut edge_map = Vec::with_capacity(b.edges.len());
    let old_edges = wanted.edges.clone();
    for e in &old_edges {
        let Some(&i) = actual_edges.get(&e.vertices) else {
            return false;
        };
        if edge_used[i] || *e != a.edges[i] {
            return false;
        }
        edge_used[i] = true;
        edge_map.push(i);
    }
    for (old, &new) in edge_map.iter().enumerate() {
        wanted.edges[new] = old_edges[old].clone();
    }
    for wire in &mut wanted.loops {
        for e in &mut wire.coedges {
            e.edge = edge_map[e.edge];
        }
    }
    for face in 0..b.faces.len() {
        if a.faces[face].surface == wanted.faces[face].surface {
            continue;
        }
        let f = &wanted.faces[face];
        let mut reflected = f.surface.clone();
        // Exact Bezier control reversal preserves the ORIGINAL knot interval.
        // No rounded knot remapping or refitting supplies proof credit.
        if reflected.control_points.len() != reflected.degree_u + 1
            || reflected.knots_u[..reflected.degree_u + 1]
                .iter()
                .any(|x| *x != reflected.knots_u[reflected.degree_u])
            || reflected.knots_u[reflected.degree_u + 1..]
                .iter()
                .any(|x| *x != reflected.knots_u[reflected.degree_u + 1])
        {
            return false;
        }
        reflected.control_points.reverse();
        reflected.weights.reverse();
        if a.faces[face].surface != reflected {
            return false;
        }
        let d = domains(&f.surface)[0];
        let Some(sum) = exact_add(d[0], d[1]) else {
            return false;
        };
        let loops = std::iter::once(f.outer)
            .chain(f.holes.iter().copied())
            .collect::<Vec<_>>();
        for wire in loops {
            let w = &mut wanted.loops[wire];
            w.coedges.reverse();
            for e in &mut w.coedges {
                if !bezier(&e.pcurve) {
                    return false;
                }
                for p in &mut e.pcurve.control_points {
                    let Some(x) = exact_add(sum, -p[0]) else {
                        return false;
                    };
                    p[0] = x;
                }
                e.pcurve.control_points.reverse();
                e.pcurve.weights.reverse();
                e.reversed ^= true;
            }
        }
        wanted.faces[face].surface = reflected;
        let Some(use_) = wanted.shells[0].faces.iter_mut().find(|u| u.face == face) else {
            return false;
        };
        use_.reversed ^= true;
    }
    raw_same(a, &wanted)
}
