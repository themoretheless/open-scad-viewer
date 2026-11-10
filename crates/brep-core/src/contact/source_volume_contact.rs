//! Boundaries of one exact four-wall polynomial source cell.
//! A strictly monotone fixed projection of the entire volume separates opposite
//! walls and confines adjacent-wall contacts to their shared edge.
//! This is used only after joint exact-domain admission.
use crate::{Model, Result};
use nurbs_core::surface::Surface;
#[path = "source_volume_contact/monotonicity.rs"]
mod monotonicity;
#[derive(Clone, Debug)]
pub struct Certificate {
    pub faces: [usize; 2],
    pub source_faces: [usize; 4],
    pub shared_edge: Option<usize>,
    pub projection: [[f64; 3]; 3],
    pub cells: usize,
    pub principal_minor_lower: [f64; 3],
}
pub struct Outcome {
    pub certificate: Option<Certificate>,
    pub cells: usize,
}
fn longitudinal_edges(model: &Model, face: usize) -> Option<[usize; 2]> {
    let face = &model.faces[face];
    let s = &face.surface;
    if !face.holes.is_empty()
        || s.degree_u != 1
        || s.periodic_u
        || s.periodic_v
        || s.degree_v == 0
        || s.degree_v > 8
        || s.control_points.len() != 2
        || s.control_points[0].len() != s.degree_v + 1
        || s.knots_u != [0., 0., 1., 1.]
        || s.knots_v[..=s.degree_v].iter().any(|x| *x != 0.)
        || s.knots_v[s.degree_v + 1..].iter().any(|x| *x != 1.)
        || s.weights.iter().any(|w| w.iter().any(|x| *x != w[0]))
    {
        return None;
    }
    let wire = &model.loops[face.outer];
    if wire.coedges.len() != 4 {
        return None;
    }
    let mut edges = [None, None];
    let mut sides = [false; 4];
    for c in &wire.coedges {
        let p = &c.pcurve;
        if p.degree != 1 || p.control_points.len() != 2 || p.knots != [0., 0., 1., 1.] {
            return None;
        }
        let [a, b] = [&p.control_points[0], &p.control_points[1]];
        let axis = (0..2).find(|&k| {
            a[k] == b[k]
                && (a[k] == 0. || a[k] == 1.)
                && ((a[1 - k] == 0. && b[1 - k] == 1.) || (a[1 - k] == 1. && b[1 - k] == 0.))
        })?;
        let side = usize::from(a[axis] == 1.);
        let slot = 2 * axis + side;
        if sides[slot] {
            return None;
        }
        sides[slot] = true;
        if axis == 0 {
            edges[side] = Some(c.edge);
        }
    }
    Some([edges[0]?, edges[1]?])
}
fn orient(s: &Surface, row: usize, reference: &[Vec<f64>]) -> Option<Surface> {
    if s.control_points[row] == reference {
        return Some(s.clone());
    }
    if !s.control_points[row].iter().rev().eq(reference) {
        return None;
    }
    let mut s = s.clone();
    for row in &mut s.control_points {
        row.reverse();
    }
    for row in &mut s.weights {
        row.reverse();
    }
    Some(s)
}
pub(crate) struct Prepared<'a> {
    model: &'a Model,
    edges: Vec<Option<[usize; 2]>>,
    owners: std::collections::HashMap<usize, Vec<(usize, usize)>>,
}
impl<'a> Prepared<'a> {
    pub(crate) fn new(model: &'a Model) -> Self {
        let edges = (0..model.faces.len())
            .map(|f| longitudinal_edges(model, f))
            .collect::<Vec<_>>();
        let mut owners = std::collections::HashMap::<usize, Vec<(usize, usize)>>::new();
        for (face, edges) in edges.iter().enumerate() {
            if let Some(edges) = edges {
                for (side, edge) in edges.iter().enumerate() {
                    owners.entry(*edge).or_default().push((face, side));
                }
            }
        }
        Self {
            model,
            edges,
            owners,
        }
    }
    fn neighbor(&self, edge: usize, face: usize) -> Option<(usize, usize)> {
        let owners = self.owners.get(&edge)?;
        if owners.len() != 2 || !owners.iter().any(|(f, _)| *f == face) {
            return None;
        }
        let &(other, side) = owners.iter().find(|(f, _)| *f != face)?;
        Some((other, self.edges[other]?[1 - side]))
    }
    pub(crate) fn certify(&self, faces: [usize; 2], max_cells: usize) -> Result<Outcome> {
        let model = self.model;
        let mut out = Outcome {
            certificate: None,
            cells: 0,
        };
        if max_cells == 0 {
            return Ok(out);
        }
        let Some(ae) = self.edges[faces[0]] else {
            return Ok(out);
        };
        let Some(requested_edges) = self.edges[faces[1]] else {
            return Ok(out);
        };
        let Some((n0, e0)) = self.neighbor(ae[0], faces[0]) else {
            return Ok(out);
        };
        let Some((n1, e1)) = self.neighbor(ae[1], faces[0]) else {
            return Ok(out);
        };
        if n0 == n1 || e0 == e1 {
            return Ok(out);
        }
        let Some((opposite, other_edge)) = self.neighbor(e0, n0) else {
            return Ok(out);
        };
        if [faces[0], n0, n1].contains(&opposite) || other_edge != e1 {
            return Ok(out);
        }
        if ![opposite, n0, n1].contains(&faces[1]) {
            return Ok(out);
        }
        let shared: Vec<_> = model.loops[model.faces[faces[0]].outer]
            .coedges
            .iter()
            .map(|c| c.edge)
            .filter(|e| {
                model.loops[model.faces[faces[1]].outer]
                    .coedges
                    .iter()
                    .any(|c| c.edge == *e)
            })
            .collect();
        let shared_edge = ae.iter().copied().find(|e| requested_edges.contains(e));
        if shared.len() != usize::from(shared_edge.is_some())
            || shared.first().copied() != shared_edge
        {
            return Ok(out);
        }
        let be = self.edges[opposite].unwrap();
        let c0 = n0;
        let c1 = n1;
        let side0 = self.edges[c0]
            .unwrap()
            .iter()
            .position(|e| *e == ae[0])
            .unwrap();
        let side1 = self.edges[c1]
            .unwrap()
            .iter()
            .position(|e| *e == ae[1])
            .unwrap();
        let Some(b0) = be.iter().position(|e| *e == e0) else {
            return Ok(out);
        };
        let Some(b1) = be.iter().position(|e| *e == e1) else {
            return Ok(out);
        };
        if c0 == c1 || b0 == b1 {
            return Ok(out);
        }
        let a = &model.faces[faces[0]].surface;
        let Some(first) = orient(&model.faces[c0].surface, side0, &a.control_points[0]) else {
            return Ok(out);
        };
        let Some(b) = orient(
            &model.faces[opposite].surface,
            b0,
            &first.control_points[1 - side0],
        ) else {
            return Ok(out);
        };
        let Some(last) = orient(&model.faces[c1].surface, side1, &a.control_points[1]) else {
            return Ok(out);
        };
        if last.control_points[1 - side1] != b.control_points[b1]
            || [b.degree_v, first.degree_v, last.degree_v]
                .iter()
                .any(|p| *p != a.degree_v)
        {
            return Ok(out);
        }
        let net = [
            [
                a.control_points[0]
                    .iter()
                    .map(|p| [p[0], p[1], p[2]])
                    .collect::<Vec<_>>(),
                a.control_points[1]
                    .iter()
                    .map(|p| [p[0], p[1], p[2]])
                    .collect(),
            ],
            [
                b.control_points[b0]
                    .iter()
                    .map(|p| [p[0], p[1], p[2]])
                    .collect(),
                b.control_points[b1]
                    .iter()
                    .map(|p| [p[0], p[1], p[2]])
                    .collect(),
            ],
        ];
        let proof = monotonicity::inspect(&net, max_cells)?;
        out.cells = proof.cells;
        if proof.certified {
            out.certificate = Some(Certificate {
                faces,
                source_faces: [faces[0], opposite, c0, c1],
                shared_edge,
                projection: proof.projection,
                cells: proof.cells,
                principal_minor_lower: proof.principal_minor_lower,
            });
        }
        Ok(out)
    }
}
#[cfg(test)]
fn certify(model: &Model, faces: [usize; 2], max_cells: usize) -> Result<Outcome> {
    Prepared::new(model).certify(faces, max_cells)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nurbs_core::curve::Curve;
    fn model() -> Model {
        let section = |z| {
            let points = [[0., 0., z], [1., 0., z], [1., 1., z], [0., 1., z]];
            vec![(0..4)
                .map(|i| Curve {
                    degree: 1,
                    knots: vec![0., 0., 1., 1.],
                    control_points: vec![points[i].to_vec(), points[(i + 1) % 4].to_vec()],
                    weights: vec![1., 2.],
                    periodic: false,
                })
                .collect()]
        };
        crate::rational_section_loft(&[section(0.), section(1.)]).unwrap()
    }
    #[test]
    fn original_source_ring_distinguishes_shared_and_opposite_boundaries() {
        let model = model();
        let adjacent = certify(&model, [0, 1], 1000).unwrap();
        assert!(adjacent.certificate.as_ref().unwrap().shared_edge.is_some());
        let opposite = certify(&model, [0, 2], 1000).unwrap();
        assert!(opposite.certificate.as_ref().unwrap().shared_edge.is_none());
        for budget in [0, opposite.cells - 1] {
            let denied = certify(&model, [0, 2], budget).unwrap();
            assert!(denied.certificate.is_none() && denied.cells <= budget);
        }
        assert!(certify(&model, [0, 4], 1000).unwrap().certificate.is_none());
    }
    #[test]
    fn one_ulp_source_corner_mutation_refuses_before_volume_work() {
        let mut model = model();
        model.faces[1].surface.control_points[0][0][2] = f64::from_bits(1);
        let denied = certify(&model, [0, 2], 1000).unwrap();
        assert!(denied.certificate.is_none());
        assert_eq!(denied.cells, 0);
    }
}
