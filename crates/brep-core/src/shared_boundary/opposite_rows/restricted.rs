//! Exact strip support with row-wise restriction to the second carrier.
//! Every zero natural row must be a full shared edge, strictly outside the
//! second original hull, or confined to one actual shared vertex. No epsilon,
//! sampled clipping or coordinate-only vertex ownership is used.
use super::*;
fn hull<'a>(points: impl Iterator<Item = &'a [f64]>, w: &mut Work) -> Option<[[f64; 2]; 3]> {
    let mut out = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for p in points {
        w.charge()?;
        for k in 0..3 {
            out[k][0] = out[k][0].min(p[k]);
            out[k][1] = out[k][1].max(p[k]);
        }
    }
    Some(out)
}
fn owners(model: &Model, face: usize, w: &mut Work) -> Option<(BTreeSet<usize>, BTreeSet<usize>)> {
    let f = &model.faces[face];
    let mut edges = BTreeSet::new();
    let mut vertices = BTreeSet::new();
    for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
        for c in &model.loops[wire].coedges {
            w.charge()?;
            if c.trim().is_some() {
                continue;
            }
            edges.insert(c.edge);
            let e = &model.edges[c.edge];
            let curve = &e.curve;
            let count = curve.control_points.len();
            if curve.periodic || curve.degree == 0 {
                continue;
            }
            let mut clamped = true;
            for &k in &curve.knots[..=curve.degree] {
                w.charge()?;
                clamped &= k == curve.knots[curve.degree];
            }
            for &k in &curve.knots[count..] {
                w.charge()?;
                clamped &= k == curve.knots[count];
            }
            if !clamped {
                continue;
            }
            for (i, index) in [0, count - 1].into_iter().enumerate() {
                w.charge()?;
                if curve.control_points[index].as_slice() == model.vertices[e.vertices[i]].point {
                    vertices.insert(e.vertices[i]);
                }
            }
        }
    }
    Some((edges, vertices))
}
pub(super) fn prove(
    model: &Model,
    faces: [usize; 2],
    planar: usize,
    w: &mut Work,
) -> Option<crate::boundary_hull_contact::Certificate> {
    let a = &model.faces[faces[1 - planar]];
    let b = &model.faces[faces[planar]];
    let s = &a.surface;
    let p = &b.surface;
    let n = [s.control_points.len(), s.control_points[0].len()];
    let degree = [s.degree_u, s.degree_v];
    let knots = [&s.knots_u, &s.knots_v];
    if s.periodic_u
        || s.periodic_v
        || n != [degree[0] + 1, degree[1] + 1]
        || n.iter().any(|&n| n < 2 || n > 33)
    {
        return None;
    }
    for axis in 0..2 {
        for (i, &k) in knots[axis].iter().enumerate() {
            w.charge()?;
            if k != knots[axis][if i <= degree[axis] {
                degree[axis]
            } else {
                n[axis]
            }] {
                return None;
            }
        }
    }
    let plane = [
        p.control_points[0][0].as_slice(),
        p.control_points[p.control_points.len() - 1][0].as_slice(),
        p.control_points[0][p.control_points[0].len() - 1].as_slice(),
    ];
    let mut noncollinear = false;
    for axes in [[0, 1], [0, 2], [1, 2]] {
        w.charge()?;
        match orient(&plane, Some(axes))? {
            Sign::Zero => {}
            _ => {
                noncollinear = true;
                break;
            }
        }
    }
    if !noncollinear {
        return None;
    }
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
    let mut side = None;
    let mut signs = vec![vec![Sign::Zero; n[1]]; n[0]];
    for u in 0..n[0] {
        for v in 0..n[1] {
            w.charge()?;
            if !s.weights[u][v].is_finite() || s.weights[u][v] <= 0. {
                return None;
            }
            let sign = w.side(plane, &s.control_points[u][v])?;
            signs[u][v] = sign;
            if sign != Sign::Zero {
                if side.is_some_and(|old| old != sign) {
                    return None;
                }
                side = Some(sign);
            }
        }
    }
    side?; // Coplanar surfaces require independent region/overlap proofs.
    for axis in 0..2 {
        if n[axis] < 3 {
            continue;
        }
        let at = |i: usize, j: usize| if axis == 0 { signs[i][j] } else { signs[j][i] };
        // Both end rows vanish. Every free-coordinate control column has a
        // strict interior coefficient. Positive rational Bernstein bases make
        // the ENTIRE open strip strictly sided, even at its other boundaries.
        let mut covered = true;
        for j in 0..n[1 - axis] {
            w.charge()?;
            covered &= at(0, j) == Sign::Zero
                && at(n[axis] - 1, j) == Sign::Zero
                && (1..n[axis] - 1).any(|i| at(i, j) != Sign::Zero);
        }
        if !covered {
            continue;
        }
        let (other, bv) = owners(model, faces[planar], w)?;
        let (_, av) = owners(model, faces[1 - planar], w)?;
        let common = av.intersection(&bv).copied().collect::<Vec<_>>();
        let other_box = hull(p.control_points.iter().flatten().map(|p| p.as_slice()), w)?;
        let mut edges = Vec::new();
        let mut vertex = None;
        for row in [0, n[axis] - 1] {
            let mut found = None;
            'wires: for wire in std::iter::once(a.outer).chain(a.holes.iter().copied()) {
                for c in &model.loops[wire].coedges {
                    w.charge()?;
                    if !other.contains(&c.edge) || c.trim().is_some() {
                        continue;
                    }
                    let e = &model.edges[c.edge];
                    let curve = &e.curve;
                    if curve.control_points.first()?.as_slice()
                        != model.vertices[e.vertices[0]].point
                        || curve.control_points.last()?.as_slice()
                            != model.vertices[e.vertices[1]].point
                    {
                        continue;
                    }
                    // Complete rational boundary identity, never endpoint-only
                    // recognition. Failed exact predicate work is charged too.
                    w.charge()?;
                    if boundary(s, &c.pcurve, curve) == Some((axis, row)) {
                        found = Some(c.edge);
                        break 'wires;
                    }
                }
            }
            if let Some(edge) = found {
                edges.push(edge);
                continue;
            }
            let row_box = hull(
                (0..n[1 - axis]).map(|j| {
                    if axis == 0 {
                        s.control_points[row][j].as_slice()
                    } else {
                        s.control_points[j][row].as_slice()
                    }
                }),
                w,
            )?;
            if (0..3).any(|k| row_box[k][1] < other_box[k][0] || other_box[k][1] < row_box[k][0]) {
                continue;
            }
            let overlap: [[f64; 2]; 3] = std::array::from_fn(|k| {
                [
                    row_box[k][0].max(other_box[k][0]),
                    row_box[k][1].min(other_box[k][1]),
                ]
            });
            if overlap.iter().any(|d| d[0] != d[1]) {
                return None;
            }
            let point = overlap.map(|d| d[0]);
            let mut owned = None;
            for &v in &common {
                w.charge()?;
                if model.vertices[v].point == point {
                    owned = Some(v);
                    break;
                }
            }
            let v = owned?;
            if vertex.is_some_and(|previous| previous != v) {
                return None;
            }
            vertex = Some(v);
        }
        if edges.len() == 2 && edges[0] == edges[1] {
            return None;
        }
        // The existing certificate represents either complete shared edges
        // or one owned vertex, not an unrecorded union of separate contacts.
        if edges.is_empty() && vertex.is_none() || !edges.is_empty() && vertex.is_some() {
            return None;
        }
        let mut enclosure = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
        for &e in &edges {
            for pole in &model.edges[e].curve.control_points {
                w.charge()?;
                for k in 0..3 {
                    enclosure[k][0] = enclosure[k][0].min(pole[k]);
                    enclosure[k][1] = enclosure[k][1].max(pole[k]);
                }
            }
        }
        if let Some(v) = vertex {
            w.charge()?;
            enclosure = model.vertices[v].point.map(|x| [x, x]);
        }
        return Some(crate::boundary_hull_contact::Certificate {
            faces,
            contact_enclosure: enclosure,
            edges,
            vertex,
            joined_proof: None,
            polar_join: None,
            source_join: None,
        });
    }
    None
}
