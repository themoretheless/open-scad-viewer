//! Shared exact stages of the opposite-row support proofs: the signed control
//! net against the planar carrier, strip coverage per axis, complete boundary
//! edge recognition and the pole enclosure of recognised edges.
use super::*;

/// Largest control count per direction accepted for a single Bezier patch.
const MAX_CONTROLS: usize = 33;
/// Smallest control count per direction; a patch needs two rows.
const MIN_CONTROLS: usize = 2;

/// Plane-side signs of every control point of the non-planar surface.
pub(super) struct SignedNet {
    pub n: [usize; 2],
    pub signs: Vec<Vec<Sign>>,
}

impl SignedNet {
    fn at(&self, axis: usize, i: usize, j: usize) -> Sign {
        if axis == 0 {
            self.signs[i][j]
        } else {
            self.signs[j][i]
        }
    }

    /// Both end rows vanish and every free-coordinate control column has a
    /// strict interior coefficient. Positive rational Bernstein bases make the
    /// ENTIRE open strip strictly sided, even at its other boundaries.
    pub fn covered(&self, axis: usize, w: &mut Work) -> Option<bool> {
        let n = self.n;
        let mut covered = true;
        for j in 0..n[1 - axis] {
            w.charge()?;
            covered &= self.at(axis, 0, j) == Sign::Zero
                && self.at(axis, n[axis] - 1, j) == Sign::Zero
                && (1..n[axis] - 1).any(|i| self.at(axis, i, j) != Sign::Zero);
        }
        Some(covered)
    }
}

fn single_clamped_patch(s: &Surface, n: [usize; 2], w: &mut Work) -> Option<()> {
    let degree = [s.degree_u, s.degree_v];
    let knots = [&s.knots_u, &s.knots_v];
    if s.periodic_u
        || s.periodic_v
        || n != [degree[0] + 1, degree[1] + 1]
        || n.iter()
            .any(|&n| !(MIN_CONTROLS..=MAX_CONTROLS).contains(&n))
    {
        return None;
    }
    for axis in 0..2 {
        for (i, &k) in knots[axis].iter().enumerate() {
            w.charge()?;
            let anchor = if i <= degree[axis] {
                degree[axis]
            } else {
                n[axis]
            };
            if k != *knots[axis].get(anchor)? {
                return None;
            }
        }
    }
    Some(())
}

fn support_plane<'a>(p: &'a Surface, w: &mut Work) -> Option<[&'a [f64]; 3]> {
    let first = p.control_points.first()?;
    let plane = [
        first.first()?.as_slice(),
        p.control_points.last()?.first()?.as_slice(),
        first.last()?.as_slice(),
    ];
    for axes in [[0, 1], [0, 2], [1, 2]] {
        w.charge()?;
        if orient(&plane, Some(axes))? != Sign::Zero {
            return Some(plane);
        }
    }
    None
}

fn planar_carrier(p: &Surface, plane: [&[f64]; 3], w: &mut Work) -> Option<()> {
    for (point, &weight) in p
        .control_points
        .iter()
        .flatten()
        .zip(p.weights.iter().flatten())
    {
        w.charge()?;
        if !weight.is_finite() || weight <= 0. || w.side(plane, point)? != Sign::Zero {
            return None;
        }
    }
    Some(())
}

fn one_sided_signs(
    s: &Surface,
    plane: [&[f64]; 3],
    n: [usize; 2],
    w: &mut Work,
) -> Option<Vec<Vec<Sign>>> {
    let mut side = None;
    let mut signs = vec![vec![Sign::Zero; n[1]]; n[0]];
    for (u, row) in signs.iter_mut().enumerate() {
        for (v, slot) in row.iter_mut().enumerate() {
            w.charge()?;
            let weight = *s.weights.get(u)?.get(v)?;
            if !weight.is_finite() || weight <= 0. {
                return None;
            }
            let sign = w.side(plane, s.control_points.get(u)?.get(v)?)?;
            *slot = sign;
            if sign != Sign::Zero {
                if side.is_some_and(|old| old != sign) {
                    return None;
                }
                side = Some(sign);
            }
        }
    }
    side?; // Coplanar surfaces require independent region/overlap proofs.
    Some(signs)
}

/// Single clamped Bezier patch `s` signed against the exact plane of `p`.
pub(super) fn signed_net(s: &Surface, p: &Surface, w: &mut Work) -> Option<SignedNet> {
    let n = [s.control_points.len(), s.control_points.first()?.len()];
    single_clamped_patch(s, n, w)?;
    let plane = support_plane(p, w)?;
    planar_carrier(p, plane, w)?;
    let signs = one_sided_signs(s, plane, n, w)?;
    Some(SignedNet { n, signs })
}

/// Untrimmed coedge of `face` whose edge is in `allowed` and is exactly the
/// complete rational boundary row `(axis, row)` of `s`.
pub(super) fn boundary_edge(
    model: &Model,
    face: &brep_topology::Face<Surface>,
    s: &Surface,
    allowed: &BTreeSet<usize>,
    (axis, row): (usize, usize),
    w: &mut Work,
) -> Option<Option<usize>> {
    for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
        for c in &model.loops[wire].coedges {
            w.charge()?;
            if !allowed.contains(&c.edge) || c.trim().is_some() {
                continue;
            }
            let e = &model.edges[c.edge];
            let curve = &e.curve;
            if curve.control_points.first()?.as_slice() != model.vertices[e.vertices[0]].point
                || curve.control_points.last()?.as_slice() != model.vertices[e.vertices[1]].point
            {
                continue;
            }
            // Complete rational boundary identity, never endpoint-only
            // recognition. Failed exact predicate work is charged too.
            w.charge()?;
            if boundary(s, &c.pcurve, curve) == Some((axis, row)) {
                return Some(Some(c.edge));
            }
        }
    }
    Some(None)
}

/// Axis-aligned box of every pole of the given edges.
pub(super) fn edge_enclosure(
    model: &Model,
    edges: &[usize],
    w: &mut Work,
) -> Option<[[f64; 2]; 3]> {
    let mut enclosure = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for &e in edges {
        for pole in &model.edges[e].curve.control_points {
            w.charge()?;
            for (k, range) in enclosure.iter_mut().enumerate() {
                range[0] = range[0].min(pole[k]);
                range[1] = range[1].max(pole[k]);
            }
        }
    }
    Some(enclosure)
}
