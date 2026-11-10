//! Exact supporting-line separation of polygonal trims on an IDENTICAL carrier.
//! Called only with fresh joint exact-boundary and valid-trim prerequisites.
use crate::Model;
use num_rational::BigRational as Q;
use num_traits::{Signed, Zero};
#[path = "same_support_contact/network.rs"]
mod network;
pub(super) use network::contact_network;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Contact {
    Edge(usize),
    Vertex(usize),
}
#[derive(Clone, Debug)]
pub struct Certificate {
    pub faces: [usize; 2],
    pub contact: Contact,
    pub carrier: nurbs_core::surface_linear_monotonicity::Report,
    pub cells: usize,
}
impl Certificate {
    pub fn edge(&self) -> Option<usize> {
        if let Contact::Edge(e) = self.contact {
            Some(e)
        } else {
            None
        }
    }
    pub fn vertex(&self) -> Option<usize> {
        if let Contact::Vertex(v) = self.contact {
            Some(v)
        } else {
            None
        }
    }
}
pub struct Attempt {
    pub certificate: Option<Certificate>,
    pub cells: usize,
}
pub(super) struct Polygon {
    pub(super) points: Vec<[f64; 2]>,
    pub(super) vertices: Vec<usize>,
    pub(super) edges: Vec<usize>,
}
pub(super) struct Work {
    pub(super) used: usize,
    pub(super) limit: usize,
}
pub(super) fn polygon(m: &Model, f: usize) -> Option<Polygon> {
    let face = &m.faces[f];
    if !face.holes.is_empty() {
        return None;
    }
    let ring = &m.loops[face.outer].coedges;
    if !(3..=64).contains(&ring.len()) {
        return None;
    }
    let mut p = Polygon {
        points: Vec::new(),
        vertices: Vec::new(),
        edges: Vec::new(),
    };
    for (i, c) in ring.iter().enumerate() {
        let q = &c.pcurve;
        let d = q.domain();
        if c.trim().is_some()
            || q.periodic
            || q.degree != 1
            || q.control_points.len() != 2
            || q.control_points[0].len() != 2
            || q.weights[0] != q.weights[1]
            || q.knots != [d[0], d[0], d[1], d[1]]
        {
            return None;
        }
        let next = &ring[(i + 1) % ring.len()];
        if q.control_points[1] != next.pcurve.control_points[0]
            || m.edges[c.edge].vertices[usize::from(!c.reversed)]
                != m.edges[next.edge].vertices[usize::from(next.reversed)]
        {
            return None;
        }
        p.points
            .push([q.control_points[0][0], q.control_points[0][1]]);
        p.vertices
            .push(m.edges[c.edge].vertices[usize::from(c.reversed)]);
        p.edges.push(c.edge);
    }
    Some(p)
}
fn side(p: &Polygon, a: [f64; 2], b: [f64; 2], w: &mut Work) -> Option<(i8, Vec<usize>)> {
    let q = |x: f64| Q::from_float(x);
    let dx = q(b[0])? - q(a[0])?;
    let dy = q(b[1])? - q(a[1])?;
    if dx.is_zero() && dy.is_zero() {
        return None;
    }
    let mut sign = 0;
    let mut zero = Vec::new();
    for (i, v) in p.points.iter().enumerate() {
        if w.used == w.limit {
            return None;
        }
        w.used += 1;
        let d = &dx * (q(v[1])? - q(a[1])?) - &dy * (q(v[0])? - q(a[0])?);
        if d.numer().bits() > 8192 || d.denom().bits() > 8192 {
            return None;
        }
        let s = if d.is_zero() {
            0
        } else if d.is_positive() {
            1
        } else {
            -1
        };
        if s == 0 {
            zero.push(i);
        } else if sign == 0 {
            sign = s;
        } else if sign != s {
            return None;
        }
    }
    (sign != 0 && !zero.is_empty()).then_some((sign, zero))
}
pub(super) fn contact(a: &Polygon, b: &Polygon, w: &mut Work) -> Option<Contact> {
    for p in [a, b] {
        for i in 0..p.points.len() {
            let (x, y) = (p.points[i], p.points[(i + 1) % p.points.len()]);
            let Some((sa, za)) = side(a, x, y, w) else {
                continue;
            };
            let Some((sb, zb)) = side(b, x, y, w) else {
                continue;
            };
            if sa == sb {
                continue;
            }
            let k = usize::from(x[0] == y[0]);
            let range = |p: &Polygon, z: &[usize]| {
                [
                    z.iter()
                        .map(|&i| p.points[i][k])
                        .fold(f64::INFINITY, f64::min),
                    z.iter()
                        .map(|&i| p.points[i][k])
                        .fold(f64::NEG_INFINITY, f64::max),
                ]
            };
            let ra = range(a, &za);
            let rb = range(b, &zb);
            let lo = ra[0].max(rb[0]);
            let hi = ra[1].min(rb[1]);
            if lo > hi {
                continue;
            }
            if lo == hi {
                for &u in &za {
                    for &v in &zb {
                        if a.points[u][k] == lo
                            && a.points[u] == b.points[v]
                            && a.vertices[u] == b.vertices[v]
                        {
                            return Some(Contact::Vertex(a.vertices[u]));
                        }
                    }
                }
            } else {
                for &u in &za {
                    for &v in &zb {
                        let ua = (u + 1) % a.points.len();
                        let vb = (v + 1) % b.points.len();
                        if !za.contains(&ua) || !zb.contains(&vb) || a.edges[u] != b.edges[v] {
                            continue;
                        }
                        let ends = [
                            a.points[u][k].min(a.points[ua][k]),
                            a.points[u][k].max(a.points[ua][k]),
                        ];
                        let same = (a.points[u] == b.points[v]
                            && a.points[ua] == b.points[vb]
                            && a.vertices[u] == b.vertices[v]
                            && a.vertices[ua] == b.vertices[vb])
                            || (a.points[u] == b.points[vb]
                                && a.points[ua] == b.points[v]
                                && a.vertices[u] == b.vertices[vb]
                                && a.vertices[ua] == b.vertices[v]);
                        if same && ends == [lo, hi] {
                            return Some(Contact::Edge(a.edges[u]));
                        }
                    }
                }
            }
        }
    }
    None
}
pub fn certify(m: &Model, faces: [usize; 2], max_cells: usize) -> nurbs_core::Result<Attempt> {
    let mut out = Attempt {
        certificate: None,
        cells: 0,
    };
    if max_cells == 0
        || max_cells > 100_000
        || m.faces[faces[0]].surface != m.faces[faces[1]].surface
    {
        return Ok(out);
    }
    let (Some(a), Some(b)) = (polygon(m, faces[0]), polygon(m, faces[1])) else {
        return Ok(out);
    };
    let mut work = Work {
        used: 0,
        limit: max_cells,
    };
    let kind = contact(&a, &b, &mut work);
    out.cells = work.used;
    if let Some(kind) = kind {
        if out.cells < max_cells {
            let carrier = nurbs_core::surface_linear_monotonicity::inspect_candidate(
                &m.faces[faces[0]].surface,
                max_cells - out.cells,
            )?;
            out.cells += carrier.cells;
            if carrier.certified {
                out.certificate = Some(Certificate {
                    faces,
                    contact: kind,
                    carrier,
                    cells: out.cells,
                });
            }
        }
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn p(points: &[[f64; 2]], vertices: &[usize], edges: &[usize]) -> Polygon {
        Polygon {
            points: points.to_vec(),
            vertices: vertices.to_vec(),
            edges: edges.to_vec(),
        }
    }
    fn attempt(a: &Polygon, b: &Polygon, limit: usize) -> Option<Contact> {
        contact(a, b, &mut Work { used: 0, limit })
    }
    #[test]
    fn shared_edge_and_vertex_are_distinguished_from_overlap_and_unowned_contact() {
        let a = p(&[[0., 0.], [1., 0.], [0., 1.]], &[0, 1, 2], &[0, 1, 2]);
        let b = p(&[[1., 0.], [1., 1.], [0., 1.]], &[1, 3, 2], &[3, 4, 1]);
        assert_eq!(attempt(&a, &b, 1000), Some(Contact::Edge(1)));
        assert_eq!(attempt(&a, &a, 1000), None);
        let v = p(&[[1., 0.], [2., 0.], [1., 1.]], &[1, 4, 3], &[5, 6, 3]);
        assert_eq!(attempt(&a, &v, 1000), Some(Contact::Vertex(1)));
        let unowned = p(&b.points, &[10, 11, 12], &[10, 11, 12]);
        assert_eq!(attempt(&a, &unowned, 1000), None);
        assert_eq!(attempt(&a, &b, 1), None);
    }
}
