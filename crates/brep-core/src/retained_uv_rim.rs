//! Linear retained-rim subdivision used by same-surface face replacement.
//! Every split is propagated to all neighboring coedges before the new rim is
//! sewn; a closed shell therefore never acquires T-junctions.
use crate::{Coedge, Model, Result};
use std::collections::BTreeMap;

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt()
}

struct Piece {
    edge: usize,
    vertices: [usize; 2],
    interval: [f64; 2],
}

pub(crate) fn subdivide_and_sew_linear_rims(
    model: &mut Model,
    source_edge_count: usize,
    new_edge_range: std::ops::Range<usize>,
    new_vertex_start: usize,
) -> Result<()> {
    let mut cuts = BTreeMap::<usize, Vec<(f64, usize)>>::new();
    for old in 0..source_edge_count {
        let edge = &model.edges[old];
        if edge.degenerate || edge.curve.degree != 1 || edge.curve.control_points.len() != 2 {
            continue;
        }
        let a = model.vertices[edge.vertices[0]].point;
        let b = model.vertices[edge.vertices[1]].point;
        let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let dd = d.iter().map(|v| v * v).sum::<f64>();
        if dd <= model.tolerance_mm.powi(2) { continue; }
        for vertex in new_vertex_start..model.vertices.len() {
            let p = model.vertices[vertex].point;
            let t = (0..3).map(|i| (p[i] - a[i]) * d[i]).sum::<f64>() / dd;
            if t <= model.tolerance_mm || t >= 1. - model.tolerance_mm { continue; }
            let projection = std::array::from_fn(|i| a[i] + t * d[i]);
            if distance(p, projection) <= model.tolerance_mm {
                cuts.entry(old).or_default().push((t, vertex));
            }
        }
    }
    let mut pieces = BTreeMap::<usize, Vec<Piece>>::new();
    for (old, mut cuts) in cuts {
        cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
        cuts.dedup_by(|a, b| (a.0 - b.0).abs() <= model.tolerance_mm);
        let original = model.edges[old].clone();
        let domain = original.curve.domain();
        let mut list = Vec::new();
        let mut previous = 0.;
        let mut start = original.vertices[0];
        for (t, vertex) in cuts.into_iter().chain([(1., original.vertices[1])]) {
            let edge = if previous == 0. && t == 1. {
                old
            } else if previous == 0. {
                old
            } else {
                model.edges.len()
            };
            let interval = [domain[0] + previous * (domain[1] - domain[0]), domain[0] + t * (domain[1] - domain[0])];
            let curve = original.curve.trim(interval[0], interval[1])?;
            let piece = Piece { edge, vertices: [start, vertex], interval };
            if edge == old {
                model.edges[old].vertices = piece.vertices;
                model.edges[old].curve = curve;
            } else {
                model.edges.push(crate::Edge { degenerate: false, vertices: piece.vertices, curve });
            }
            list.push(piece);
            previous = t;
            start = vertex;
        }
        pieces.insert(old, list);
    }
    if pieces.is_empty() { return Ok(()); }
    for wire in &mut model.loops {
        let mut rebuilt = Vec::new();
        for coedge in &wire.coedges {
            let Some(list) = pieces.get(&coedge.edge) else {
                rebuilt.push(coedge.clone());
                continue;
            };
            let iter: Box<dyn Iterator<Item = &Piece>> = if coedge.reversed {
                Box::new(list.iter().rev())
            } else {
                Box::new(list.iter())
            };
            for piece in iter {
                let mut pcurve = coedge.pcurve.clone();
                let pd = pcurve.domain();
                let parameter = |t: f64| pd[0] + t * (pd[1] - pd[0]);
                pcurve = pcurve.trim(parameter(piece.interval[0]), parameter(piece.interval[1]))?;
                if coedge.reversed { pcurve = pcurve.reverse()?; }
                rebuilt.push(Coedge::new(piece.edge, coedge.reversed, pcurve));
            }
        }
        wire.coedges = rebuilt;
    }
    // Replace each new rim edge by the unique old piece with the same retained
    // endpoints. Interior arrangement edges have no such counterpart.
    let mut replacements = BTreeMap::<usize, (usize, bool)>::new();
    for new in new_edge_range {
        let vertices = model.edges[new].vertices;
        let matches = pieces.values().flatten().filter(|piece| {
            (piece.vertices == vertices) || (piece.vertices == [vertices[1], vertices[0]])
        }).collect::<Vec<_>>();
        if matches.len() > 1 {
            return Err(crate::invalid("Replacement rim edge matching is ambiguous"));
        }
        if let Some(piece) = matches.first() {
            replacements.insert(new, (piece.edge, piece.vertices != vertices));
        }
    }
    for coedge in model.loops.iter_mut().flat_map(|w| &mut w.coedges) {
        if let Some(&(old, flip)) = replacements.get(&coedge.edge) {
            coedge.edge = old;
            coedge.reversed ^= flip;
        }
    }
    Ok(())
}
