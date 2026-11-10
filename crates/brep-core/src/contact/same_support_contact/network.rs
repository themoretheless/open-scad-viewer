//! Exact simple-polygon boundary networks; prerequisite: exact saved lifts.
use super::{Polygon, Work};
use num_rational::BigRational as Q;
use num_traits::{FromPrimitive, Signed, Zero};
use std::collections::BTreeSet;
type P = [Q; 2];
fn orient(a: &P, b: &P, c: &P, w: &mut Work) -> Option<i8> {
    if w.used >= w.limit {
        return None;
    }
    w.used += 1;
    let v = (&b[0] - &a[0]) * (&c[1] - &a[1]) - (&b[1] - &a[1]) * (&c[0] - &a[0]);
    if v.numer().bits() > 8192 || v.denom().bits() > 8192 {
        return None;
    }
    Some(if v.is_zero() {
        0
    } else if v.is_positive() {
        1
    } else {
        -1
    })
}
fn between(p: &P, a: &P, b: &P) -> bool {
    (0..2).all(|k| p[k] >= a[k].clone().min(b[k].clone()) && p[k] <= a[k].clone().max(b[k].clone()))
}
#[derive(Clone, Copy)]
enum Hit {
    Disjoint,
    Endpoint(usize, usize),
    Full(bool),
    Other,
}
fn hit(a: [&P; 2], b: [&P; 2], w: &mut Work) -> Option<Hit> {
    let o = [
        orient(a[0], a[1], b[0], w)?,
        orient(a[0], a[1], b[1], w)?,
        orient(b[0], b[1], a[0], w)?,
        orient(b[0], b[1], a[1], w)?,
    ];
    if o.iter().all(|s| *s == 0) {
        let k = usize::from(a[0][0] == a[1][0]);
        let lo = a[0][k]
            .clone()
            .min(a[1][k].clone())
            .max(b[0][k].clone().min(b[1][k].clone()));
        let hi = a[0][k]
            .clone()
            .max(a[1][k].clone())
            .min(b[0][k].clone().max(b[1][k].clone()));
        if lo > hi {
            return Some(Hit::Disjoint);
        }
        if lo < hi {
            return Some(if a[0] == b[0] && a[1] == b[1] {
                Hit::Full(true)
            } else if a[0] == b[1] && a[1] == b[0] {
                Hit::Full(false)
            } else {
                Hit::Other
            });
        }
    } else if o[0] * o[1] < 0 && o[2] * o[3] < 0 {
        return Some(Hit::Other);
    }
    let mut result = None;
    for i in 0..2 {
        if o[2 + i] == 0 && between(a[i], b[0], b[1]) {
            let Some(j) = (0..2).find(|&j| a[i] == b[j]) else {
                return Some(Hit::Other);
            };
            result = Some((i, j));
        }
        if o[i] == 0 && between(b[i], a[0], a[1]) {
            let Some(j) = (0..2).find(|&j| b[i] == a[j]) else {
                return Some(Hit::Other);
            };
            if result.is_some_and(|p| p != (j, i)) {
                return Some(Hit::Other);
            }
            result = Some((j, i));
        }
    }
    Some(result.map_or(Hit::Disjoint, |(i, j)| Hit::Endpoint(i, j)))
}
fn simple(p: &Polygon, w: &mut Work) -> Option<(Vec<P>, i8)> {
    let n = p.points.len();
    if !(3..=64).contains(&n) || p.vertices.len() != n || p.edges.len() != n {
        return None;
    }
    if p.vertices.iter().collect::<BTreeSet<_>>().len() != n
        || p.edges.iter().collect::<BTreeSet<_>>().len() != n
    {
        return None;
    }
    let points = p
        .points
        .iter()
        .map(|p| Some([Q::from_f64(p[0])?, Q::from_f64(p[1])?]))
        .collect::<Option<Vec<_>>>()?;
    if points.iter().collect::<BTreeSet<_>>().len() != n {
        return None;
    }
    let mut area = Q::zero();
    for i in 0..n {
        if w.used >= w.limit {
            return None;
        }
        w.used += 1;
        let j = (i + 1) % n;
        area += &points[i][0] * &points[j][1] - &points[i][1] * &points[j][0];
        if area.numer().bits() > 8192 || area.denom().bits() > 8192 {
            return None;
        }
        for j in i + 1..n {
            let h = hit(
                [&points[i], &points[(i + 1) % n]],
                [&points[j], &points[(j + 1) % n]],
                w,
            )?;
            match h {
                Hit::Endpoint(1, 0) if j == i + 1 => {}
                Hit::Endpoint(0, 1) if i == 0 && j == n - 1 => {}
                Hit::Disjoint if j != i + 1 && !(i == 0 && j == n - 1) => {}
                _ => return None,
            }
        }
    }
    if area.is_zero() {
        return None;
    }
    Some((points, if area.is_positive() { 1 } else { -1 }))
}
fn outside(point: &P, polygon: &[P], w: &mut Work) -> Option<bool> {
    let mut winding = 0;
    for i in 0..polygon.len() {
        let a = &polygon[i];
        let b = &polygon[(i + 1) % polygon.len()];
        let s = orient(a, b, point, w)?;
        if s == 0 && between(point, a, b) {
            return Some(false);
        }
        if a[1] <= point[1] && point[1] < b[1] && s > 0 {
            winding += 1;
        }
        if b[1] <= point[1] && point[1] < a[1] && s < 0 {
            winding -= 1;
        }
    }
    Some(winding == 0)
}
/// Boundary crossings, partial/unowned overlaps and T junctions refuse. Every
/// shared full edge must have opposite polygon-interior sides. A midpoint of
/// each nonshared edge is classified exactly: without boundary crossings its
/// entire open segment has the same strict side. This also rejects inscribed
/// polygons whose vertices all lie on the other boundary. Jordan separation
/// then excludes interior overlap. Every reported contact has actual shared
/// edge/vertex ownership; coincident coordinates cannot manufacture a role.
pub(in super::super) fn contact_network(
    a: &Polygon,
    b: &Polygon,
    w: &mut Work,
) -> Option<(Vec<usize>, Vec<usize>)> {
    let (pa, sa) = simple(a, w)?;
    let (pb, sb) = simple(b, w)?;
    let mut edges = BTreeSet::new();
    let mut vertices = BTreeSet::new();
    let mut shared_a = BTreeSet::new();
    let mut shared_b = BTreeSet::new();
    for i in 0..pa.len() {
        for j in 0..pb.len() {
            match hit(
                [&pa[i], &pa[(i + 1) % pa.len()]],
                [&pb[j], &pb[(j + 1) % pb.len()]],
                w,
            )? {
                Hit::Disjoint => {}
                Hit::Endpoint(x, y) => {
                    let ai = (i + x) % pa.len();
                    let bj = (j + y) % pb.len();
                    if a.vertices[ai] != b.vertices[bj] {
                        return None;
                    }
                    vertices.insert(a.vertices[ai]);
                }
                Hit::Full(same) => {
                    if a.edges[i] != b.edges[j] || (same && sa == sb) || (!same && sa != sb) {
                        return None;
                    }
                    let bj = if same { j } else { (j + 1) % pb.len() };
                    let bk = if same { (j + 1) % pb.len() } else { j };
                    if a.vertices[i] != b.vertices[bj]
                        || a.vertices[(i + 1) % pa.len()] != b.vertices[bk]
                    {
                        return None;
                    }
                    edges.insert(a.edges[i]);
                    vertices.insert(a.vertices[i]);
                    vertices.insert(a.vertices[(i + 1) % pa.len()]);
                    shared_a.insert(i);
                    shared_b.insert(j);
                }
                Hit::Other => return None,
            }
        }
    }
    if edges.is_empty() && vertices.is_empty() {
        return None;
    }
    for (p, other, shared) in [(&pa, &pb, &shared_a), (&pb, &pa, &shared_b)] {
        for i in 0..p.len() {
            if shared.contains(&i) {
                continue;
            }
            let j = (i + 1) % p.len();
            let mid = std::array::from_fn(|k| (&p[i][k] + &p[j][k]) / Q::from_integer(2.into()));
            if !outside(&mid, other, w)? {
                return None;
            }
        }
    }
    Some((edges.into_iter().collect(), vertices.into_iter().collect()))
}
