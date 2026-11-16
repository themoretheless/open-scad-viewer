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
/// How one zero natural row of the strip meets the second carrier.
enum RowContact {
    /// The row is a complete shared edge.
    Edge(usize),
    /// The row's hull is strictly outside the second original hull.
    Disjoint,
    /// The row's hull touches the second hull at one actual shared vertex.
    Vertex(usize),
}

/// Exact data shared by both rows of one covered axis.
struct AxisProof<'a> {
    model: &'a Model,
    face: &'a brep_topology::Face<Surface>,
    surface: &'a Surface,
    other_edges: BTreeSet<usize>,
    common: Vec<usize>,
    other_box: [[f64; 2]; 3],
    axis: usize,
}

impl AxisProof<'_> {
    fn row_box(&self, row: usize, free: usize, w: &mut Work) -> Option<[[f64; 2]; 3]> {
        let s = self.surface;
        let mut poles = Vec::with_capacity(free);
        for j in 0..free {
            let pole = if self.axis == 0 {
                s.control_points.get(row)?.get(j)?
            } else {
                s.control_points.get(j)?.get(row)?
            };
            poles.push(pole.as_slice());
        }
        hull(poles.into_iter(), w)
    }

    fn row_contact(&self, row: usize, free: usize, w: &mut Work) -> Option<RowContact> {
        let found = strip::boundary_edge(
            self.model,
            self.face,
            self.surface,
            &self.other_edges,
            (self.axis, row),
            w,
        )?;
        if let Some(edge) = found {
            return Some(RowContact::Edge(edge));
        }
        let row_box = self.row_box(row, free, w)?;
        let other_box = self.other_box;
        if (0..3).any(|k| row_box[k][1] < other_box[k][0] || other_box[k][1] < row_box[k][0]) {
            return Some(RowContact::Disjoint);
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
        for &v in &self.common {
            w.charge()?;
            if self.model.vertices[v].point == point {
                return Some(RowContact::Vertex(v));
            }
        }
        None
    }

    fn certificate(
        &self,
        faces: [usize; 2],
        n: [usize; 2],
        w: &mut Work,
    ) -> Option<crate::boundary_hull_contact::Certificate> {
        let free = n[1 - self.axis];
        let mut edges = Vec::new();
        let mut vertex = None;
        for row in [0, n[self.axis] - 1] {
            match self.row_contact(row, free, w)? {
                RowContact::Edge(edge) => edges.push(edge),
                RowContact::Disjoint => {}
                RowContact::Vertex(v) => {
                    if vertex.is_some_and(|previous| previous != v) {
                        return None;
                    }
                    vertex = Some(v);
                }
            }
        }
        if edges.len() == 2 && edges[0] == edges[1] {
            return None;
        }
        // The existing certificate represents either complete shared edges
        // or one owned vertex, not an unrecorded union of separate contacts.
        if edges.is_empty() && vertex.is_none() || !edges.is_empty() && vertex.is_some() {
            return None;
        }
        let mut enclosure = strip::edge_enclosure(self.model, &edges, w)?;
        if let Some(v) = vertex {
            w.charge()?;
            enclosure = self.model.vertices[v].point.map(|x| [x, x]);
        }
        Some(crate::boundary_hull_contact::Certificate {
            faces,
            contact_enclosure: enclosure,
            edges,
            vertex,
            joined_proof: None,
            polar_join: None,
            source_join: None,
        })
    }
}

pub(super) fn prove(
    model: &Model,
    faces: [usize; 2],
    planar: usize,
    w: &mut Work,
) -> Option<crate::boundary_hull_contact::Certificate> {
    let a = &model.faces[faces[1 - planar]];
    let p = &model.faces[faces[planar]].surface;
    let net = strip::signed_net(&a.surface, p, w)?;
    let n = net.n;
    for axis in 0..2 {
        if n[axis] < 3 || !net.covered(axis, w)? {
            continue;
        }
        let (other_edges, bv) = owners(model, faces[planar], w)?;
        let (_, av) = owners(model, faces[1 - planar], w)?;
        let proof = AxisProof {
            model,
            face: a,
            surface: &a.surface,
            other_edges,
            common: av.intersection(&bv).copied().collect(),
            other_box: hull(p.control_points.iter().flatten().map(|p| p.as_slice()), w)?,
            axis,
        };
        return proof.certificate(faces, n, w);
    }
    None
}
