//! Exact coordinate halfspaces. Identity-owned boundaries are the only
//! permitted contacts; adjacent faces are never exempted from this check.
use super::*;
use std::collections::BTreeSet;
#[derive(Debug)]
pub struct ContactReport {
    pub certified_pairs: Vec<[usize; 2]>,
    pub unresolved_pairs: Vec<[usize; 2]>,
    pub cells: usize,
    pub all_pairs_classified: bool,
}
fn spans(k: &[f64], p: usize, n: usize) -> Option<Vec<usize>> {
    if p == 0
        || p > 16
        || n > 256
        || k[..=p].iter().any(|&x| x != k[p])
        || k[n..].iter().any(|&x| x != k[n])
    {
        return None;
    }
    let result = (p..n)
        .filter(|&i| k[i] < k[i + 1])
        .map(|i| i - p)
        .collect::<Vec<_>>();
    if result.iter().enumerate().any(|(j, &i)| i != j * p) || result.len() * p + 1 != n {
        return None;
    }
    Some(result)
}
// Positive Bernstein bases: every open cell, interior grid edge and grid
// vertex must have a strict coefficient. Zeros can then occur only on the
// natural rectangle boundary. All comparisons use original finite scalars.
fn interior_strict(s: &Surface, axis: usize, plane: f64, work: &mut Work) -> bool {
    let Some(us) = spans(&s.knots_u, s.degree_u, s.control_points.len()) else {
        return false;
    };
    let Some(vs) = spans(&s.knots_v, s.degree_v, s.control_points[0].len()) else {
        return false;
    };
    let mut strict = |u: usize, v: usize| work.spend(1) && s.control_points[u][v][axis] != plane;
    for &u in &us {
        for &v in &vs {
            if !(u..=u + s.degree_u).any(|i| (v..=v + s.degree_v).any(|j| strict(i, j))) {
                return false;
            }
        }
    }
    for &u in &us[1..] {
        for &v in &vs {
            if !(v..=v + s.degree_v).any(|j| strict(u, j)) {
                return false;
            }
        }
    }
    for &v in &vs[1..] {
        for &u in &us {
            if !(u..=u + s.degree_u).any(|i| strict(i, v)) {
                return false;
            }
        }
    }
    for &u in &us[1..] {
        for &v in &vs[1..] {
            if !strict(u, v) {
                return false;
            }
        }
    }
    true
}
fn edges(m: &Model, f: usize) -> BTreeSet<usize> {
    let f = &m.faces[f];
    std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .flat_map(|l| m.loops[l].coedges.iter().map(|c| c.edge))
        .collect()
}
fn owned_zero_set(
    m: &Model,
    face: usize,
    other: usize,
    axis: usize,
    plane: f64,
    work: &mut Work,
) -> bool {
    if !interior_strict(&m.faces[face].surface, axis, plane, work) {
        return false;
    }
    let mine = edges(m, face);
    let theirs = edges(m, other);
    if !work.spend(
        mine.len()
            + theirs.len()
            + m.faces[face].surface.knots_u.len()
            + m.faces[face].surface.knots_v.len(),
    ) {
        return false;
    }
    let vertices = theirs
        .iter()
        .flat_map(|&e| m.edges[e].vertices)
        .collect::<BTreeSet<_>>();
    for id in mine {
        let e = &m.edges[id];
        if !work.spend(4 * e.curve.control_points.len() + e.curve.knots.len()) {
            return false;
        }
        if theirs.contains(&id) {
            continue;
        }
        if e.degenerate {
            if !e
                .curve
                .control_points
                .iter()
                .all(|p| p.as_slice() == m.vertices[e.vertices[0]].point.as_slice())
            {
                return false;
            }
            if m.vertices[e.vertices[0]].point[axis] == plane
                && (!vertices.contains(&e.vertices[0]) || e.vertices[0] != e.vertices[1])
            {
                return false;
            }
            continue;
        }
        let c = &e.curve;
        if !c.control_points.iter().all(|p| p[axis] >= plane)
            && !c.control_points.iter().all(|p| p[axis] <= plane)
        {
            return false;
        }
        let Some(pieces) = spans(&c.knots, c.degree, c.control_points.len()) else {
            return false;
        };
        for i in pieces {
            if c.control_points[i..=i + c.degree]
                .iter()
                .all(|p| p[axis] == plane)
            {
                return false;
            }
        }
        for i in (0..c.control_points.len()).step_by(c.degree) {
            if c.control_points[i][axis] != plane {
                continue;
            }
            let vertex = if i == 0 {
                e.vertices[0]
            } else if i + 1 == c.control_points.len() {
                e.vertices[1]
            } else {
                return false;
            };
            if !vertices.contains(&vertex) {
                return false;
            }
        }
    }
    true
}
fn hull(s: &Surface, k: usize) -> [f64; 2] {
    let mut b = [f64::INFINITY, f64::NEG_INFINITY];
    for p in s.control_points.iter().flatten() {
        b[0] = b[0].min(p[k]);
        b[1] = b[1].max(p[k]);
    }
    b
}
struct Work {
    used: usize,
    max: usize,
    exhausted: bool,
}
impl Work {
    fn spend(&mut self, n: usize) -> bool {
        if n > self.max.saturating_sub(self.used) {
            self.exhausted = true;
            return false;
        }
        self.used += n;
        true
    }
}
fn certify(m: &Model, pair: [usize; 2], work: &mut Work) -> bool {
    let size = pair
        .iter()
        .map(|&f| {
            let s = &m.faces[f].surface;
            s.control_points.len() * s.control_points[0].len()
        })
        .sum::<usize>();
    for axis in 0..3 {
        if !work.spend(size) {
            return false;
        }
        let a = hull(&m.faces[pair[0]].surface, axis);
        let b = hull(&m.faces[pair[1]].surface, axis);
        if a[1] < b[0] || b[1] < a[0] {
            return true;
        }
        for plane in [a[1], a[0]] {
            if !(a[1] == plane && b[0] == plane || b[1] == plane && a[0] == plane) {
                continue;
            }
            if owned_zero_set(m, pair[0], pair[1], axis, plane, work)
                || owned_zero_set(m, pair[1], pair[0], axis, plane, work)
            {
                return true;
            }
        }
    }
    false
}
pub(super) fn inspect(
    m: &Model,
    pairs: &crate::face_contacts::Report,
    exact_boundary: bool,
    trims_inside: bool,
    max_cells: usize,
) -> ContactReport {
    let mut work = Work {
        used: 0,
        max: max_cells.min(100000),
        exhausted: false,
    };
    let mut out = ContactReport {
        certified_pairs: vec![],
        unresolved_pairs: vec![],
        cells: 0,
        all_pairs_classified: false,
    };
    // This additional proof is admitted only for the same immutable model
    // after exact edge/UV agreement, exact joins and contained simple trims.
    for p in &pairs.pairs {
        if p.boundary.is_some()
            || p.hull_disjoint
            || p.volume_separation.is_some()
            || p.result.as_ref().is_some_and(|r| r.absence_proven)
        {
            continue;
        }
        if exact_boundary && trims_inside && certify(m, p.faces, &mut work) && !work.exhausted {
            out.certified_pairs.push(p.faces);
            continue;
        }
        out.unresolved_pairs.push(p.faces);
    }
    out.cells = work.used;
    out.all_pairs_classified = pairs.next_pair.is_none()
        && pairs.pairs.len() + pairs.grouped_pairs == pairs.total_pairs
        && out.unresolved_pairs.is_empty();
    out
}

#[cfg(test)]
#[path = "contact_tests.rs"]
mod tests;
