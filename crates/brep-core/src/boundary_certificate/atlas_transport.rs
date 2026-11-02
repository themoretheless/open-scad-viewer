//! Exact proof-only reversal of the U charts used by oriented STEP faces.
//! This is a proposal, not a successful bound: complete region correspondence,
//! original/retained regularity and independent volume gates remain mandatory.
use super::*;
use crate::exact_affine_source::exact_add;
use nurbs_core::curve::Curve;
fn spend(used: &mut usize, max: usize) -> Option<()> {
    if *used == max {
        return None;
    }
    *used += 1;
    Some(())
}
fn reflected(x: f64, sum: f64, used: &mut usize, max: usize) -> Option<f64> {
    spend(used, max)?;
    exact_add(sum, -x)
}
fn reverse_curve(c: &mut Curve, used: &mut usize, max: usize) -> Option<()> {
    spend(used, max)?;
    let [a, b] = c.domain();
    let sum = exact_add(a, b)?;
    let knots = c
        .knots
        .iter()
        .rev()
        .map(|&x| reflected(x, sum, used, max))
        .collect::<Option<Vec<_>>>()?;
    c.control_points.reverse();
    c.weights.reverse();
    c.knots = knots;
    Some(())
}
// STEP may enumerate vertices, edges and wires in first-use order. Admit only
// a unique exact geometric bijection; coincident/ambiguous entities refuse.
fn reindex(a: &Model, b: &Model, used: &mut usize, max: usize) -> Option<Model> {
    if a.vertices.len() != b.vertices.len()
        || a.edges.len() != b.edges.len()
        || a.loops.len() != b.loops.len()
        || a.faces.len() != b.faces.len()
    {
        return None;
    }
    let mut view = a.clone();
    let mut vertices = Vec::new();
    let mut taken = std::collections::BTreeSet::new();
    for v in &a.vertices {
        let mut matches = Vec::new();
        for (i, w) in b.vertices.iter().enumerate() {
            spend(used, max)?;
            if v.point == w.point {
                matches.push(i);
            }
        }
        if matches.len() != 1 || !taken.insert(matches[0]) {
            return None;
        }
        vertices.push(matches[0]);
    }
    for (i, &j) in vertices.iter().enumerate() {
        view.0.vertices[j] = a.vertices[i].clone();
    }
    let mut edges = Vec::new();
    taken.clear();
    for e in &a.edges {
        let endpoints = e.vertices.map(|i| vertices[i]);
        let mut matches = Vec::new();
        for (j, f) in b.edges.iter().enumerate() {
            spend(used, max)?;
            if endpoints == f.vertices && e.degenerate == f.degenerate && e.curve == f.curve {
                matches.push(j);
            }
        }
        if matches.len() != 1 || !taken.insert(matches[0]) {
            return None;
        }
        edges.push(matches[0]);
    }
    for (i, &j) in edges.iter().enumerate() {
        view.0.edges[j] = a.edges[i].clone();
        view.0.edges[j].vertices = a.edges[i].vertices.map(|i| vertices[i]);
    }
    let mut wires = vec![None; a.loops.len()];
    taken.clear();
    for (f, g) in a.faces.iter().zip(&b.faces) {
        if f.holes.len() != g.holes.len() {
            return None;
        }
        for (i, j) in std::iter::once(f.outer)
            .chain(f.holes.iter().copied())
            .zip(std::iter::once(g.outer).chain(g.holes.iter().copied()))
        {
            spend(used, max)?;
            if wires[i].replace(j).is_some() || !taken.insert(j) {
                return None;
            }
        }
    }
    for (i, j) in wires.iter().enumerate() {
        let j = (*j)?;
        view.0.loops[j] = a.loops[i].clone();
        for c in &mut view.0.loops[j].coedges {
            spend(used, max)?;
            c.edge = edges[c.edge];
        }
    }
    for f in &mut view.0.faces {
        f.outer = wires[f.outer]?;
        for h in &mut f.holes {
            *h = wires[*h]?;
        }
    }
    Some(view)
}
pub(super) fn prepare(a: &Model, b: &Model, max: usize) -> (Option<(Model, Vec<usize>)>, usize) {
    let mut used = 0;
    let Some(indexed) = reindex(a, b, &mut used, max) else {
        return (None, used);
    };
    let a = &indexed;
    // Face, shell and body order is unchanged; only exact unique incidence
    // renumbering above is admitted.
    if a.vertices.len() != b.vertices.len()
        || a.edges.len() != b.edges.len()
        || a.loops.len() != b.loops.len()
        || a.faces.len() != b.faces.len()
        || a.shells.len() != b.shells.len()
        || a.bodies != b.bodies
        || a.edges
            .iter()
            .zip(&b.edges)
            .any(|(x, y)| x.vertices != y.vertices || x.degenerate != y.degenerate)
        || a.faces
            .iter()
            .zip(&b.faces)
            .any(|(x, y)| x.outer != y.outer || x.holes != y.holes)
    {
        return (None, used);
    }
    let mut flips = vec![false; a.faces.len()];
    for (x, y) in a.shells.iter().zip(&b.shells) {
        if x.closed != y.closed || x.faces.len() != y.faces.len() {
            return (None, used);
        }
        for (x, y) in x.faces.iter().zip(&y.faces) {
            if x.face != y.face {
                return (None, used);
            }
            flips[x.face] = x.reversed != y.reversed;
        }
    }
    if !flips.iter().any(|f| *f) {
        return (None, used);
    }
    let result = (|| -> Option<(Model, Vec<usize>)> {
        spend(&mut used, max)?;
        let mut view = a.clone();
        let mut seen = std::collections::BTreeSet::new();
        for f in &a.faces {
            for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
                spend(&mut used, max)?;
                if !seen.insert(wire) {
                    return None;
                }
            }
        }
        let mut changed = Vec::new();
        for (face, &flip) in flips.iter().enumerate() {
            if !flip {
                continue;
            }
            spend(&mut used, max)?;
            let s = &mut view.0.faces[face].surface;
            let sum = exact_add(s.knots_u[s.degree_u], s.knots_u[s.control_points.len()])?;
            let knots = s
                .knots_u
                .iter()
                .rev()
                .map(|&x| reflected(x, sum, &mut used, max))
                .collect::<Option<Vec<_>>>()?;
            s.control_points.reverse();
            s.weights.reverse();
            s.knots_u = knots;
            let wires = std::iter::once(view.faces[face].outer)
                .chain(view.faces[face].holes.iter().copied())
                .collect::<Vec<_>>();
            for wire in wires {
                view.0.loops[wire].coedges.reverse();
                for c in &mut view.0.loops[wire].coedges {
                    spend(&mut used, max)?;
                    c.reversed = !c.reversed;
                    reverse_curve(&mut c.pcurve, &mut used, max)?;
                    for p in &mut c.pcurve.control_points {
                        p[0] = reflected(p[0], sum, &mut used, max)?;
                    }
                }
            }
            changed.push(face);
        }
        for shell in &mut view.0.shells {
            for f in &mut shell.faces {
                f.reversed ^= flips[f.face];
            }
        }
        super::common_regions(&view, b).then_some((view, changed))
    })();
    (result, used)
}
