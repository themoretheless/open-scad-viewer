//! Original document edge/vertex identities for sewn roof source boundaries.
//! Read-only binding only; remaining original faces are not appended here.
use super::*;
use crate::{Model, TopoId};
use nurbs_core::{curve::Curve, original_graph_transition::polynomial_points_equal};
use std::collections::BTreeMap;
#[derive(Clone, Debug)]
pub struct OriginalEdgeLink {
    pub roof_edge: usize,
    pub source_edge: usize,
    pub source_id: TopoId,
    pub source_range: [Q; 2],
    pub reversed_from_source: bool,
}
#[derive(Clone, Debug)]
pub struct OriginalEndpointLink {
    pub roof_vertex: usize,
    pub source_edge_id: TopoId,
    pub source_parameter: Q,
    pub source_vertex: Option<usize>,
    pub source_vertex_id: Option<TopoId>,
}
pub struct OriginalBoundaryMap {
    links: Vec<OriginalEdgeLink>,
    endpoints: Vec<OriginalEndpointLink>,
    face_indices: [usize; 2],
    faces: [crate::Face; 2],
    loops: [crate::Loop; 2],
    face_ids: [TopoId; 2],
    curves: BTreeMap<usize, (crate::Edge, TopoId)>,
    vertices: BTreeMap<usize, ([f64; 3], TopoId)>,
    roof_roots: Vec<(usize, Arc<RootCurve>, [Q; 2])>,
}
impl OriginalBoundaryMap {
    pub fn edges(&self) -> &[OriginalEdgeLink] {
        &self.links
    }
    pub fn endpoints(&self) -> &[OriginalEndpointLink] {
        &self.endpoints
    }
    pub fn is_current(&self, source: &Model, roof: &Roof) -> bool {
        for side in 0..2 {
            let i = self.face_indices[side];
            if source.faces.get(i) != Some(&self.faces[side])
                || source.1.faces.get(i) != Some(&self.face_ids[side])
                || source.loops.get(self.faces[side].outer) != Some(&self.loops[side])
            {
                return false;
            }
        }
        self.curves.iter().all(|(i, (c, id))| {
            source.edges.get(*i).is_some_and(|e| e == c)
                && source.1.edges.get(*i) == Some(id)
        }) && self.vertices.iter().all(|(i, (p, id))| {
            source.vertices.get(*i).is_some_and(|v| &v.point == p)
                && source.1.vertices.get(*i) == Some(id)
        }) && self.roof_roots.iter().all(|(i, root, range)| {
            roof.topology()
                .edges
                .get(*i)
                .is_some_and(|e| Arc::ptr_eq(&e.curve.root, root) && &e.curve.range == range)
        })
    }
}
fn polynomial(c: &Curve) -> Result<Vec<[Q; 3]>> {
    c.validate()?;
    let d = c.domain();
    require(
        !c.periodic
            && c.control_points.len() == c.degree + 1
            && c.control_points.iter().all(|p| p.len() == 3)
            && c.weights.iter().all(|w| *w == c.weights[0])
            && c.knots[..=c.degree].iter().all(|x| *x == d[0])
            && c.knots[c.degree + 1..].iter().all(|x| *x == d[1]),
        "Original boundary must retain a polynomial Bezier definition",
    )?;
    Ok(c.control_points
        .iter()
        .map(|p| std::array::from_fn(|k| Q::from_float(p[k]).unwrap()))
        .collect())
}
fn canonical_uv(c: &crate::Coedge) -> Result<[[Q; 2]; 2]> {
    let p = &c.pcurve;
    p.validate()?;
    require(
        c.trim().is_none()
            && !p.periodic
            && p.degree == 1
            && p.control_points.len() == 2
            && p.weights[0] == p.weights[1]
            && p.control_points.iter().all(|p| p.len() == 2),
        "Original source boundary needs an untrimmed polynomial UV line",
    )?;
    let mut uv = std::array::from_fn(|i| {
        std::array::from_fn(|k| Q::from_float(p.control_points[i][k]).unwrap())
    });
    if c.reversed {
        uv.reverse();
    }
    Ok(uv)
}
fn parameters(original: &[[Q; 2]; 2], actual: &[[Q; 2]; 2]) -> Option<[Q; 2]> {
    let axis = usize::from(original[0][0] == original[1][0]);
    let delta = &original[1][axis] - &original[0][axis];
    if delta.is_zero() {
        return None;
    }
    let r = actual
        .each_ref()
        .map(|p| (&p[axis] - &original[0][axis]) / &delta);
    if r[0] == r[1] || r.iter().any(|t| t < &Q::zero() || t > &Q::one()) {
        return None;
    }
    for i in 0..2 {
        for k in 0..2 {
            if &original[0][k] + (&original[1][k] - &original[0][k]) * &r[i] != actual[i][k] {
                return None;
            }
        }
    }
    Some(r)
}
impl Roof {
    /// Bind named original chart intervals, then prove complete world-curve
    /// equality after exact restriction/elevation. No spatial search/welding.
    pub fn bind_original_boundaries(
        &self,
        source: &Model,
        source_faces: [usize; 2],
        budget: &mut Budget,
    ) -> Result<OriginalBoundaryMap> {
        source.validate()?;
        require(
            source_faces[0] != source_faces[1]
                && source_faces.iter().all(|&i| i < source.faces.len())
                && source.1.faces.len() == source.faces.len()
                && source.1.edges.len() == source.edges.len()
                && source.1.vertices.len() == source.vertices.len(),
            "Original topology identities or source-face indices are missing",
        )?;
        let faces = source_faces.map(|i| source.faces[i].clone());
        require(
            faces.iter().all(|f| f.holes.is_empty()),
            "Original source-hole mapping is unsupported",
        )?;
        let loops = std::array::from_fn(|s| source.loops[faces[s].outer].clone());
        let mut map = OriginalBoundaryMap {
            links: Vec::new(),
            endpoints: Vec::new(),
            face_indices: source_faces,
            face_ids: source_faces.map(|i| source.1.faces[i]),
            faces,
            loops,
            curves: BTreeMap::new(),
            vertices: BTreeMap::new(),
            roof_roots: Vec::new(),
        };
        let mut by_roof = BTreeMap::new();
        for face in &self.topology().faces {
            let FaceGeometry::Retained(region) = &face.surface else {
                continue;
            };
            let side = region.side();
            require(
                region.source() == &map.faces[side].surface,
                "Original document source face changed",
            )?;
            for (index, (u, reverse)) in region.uses().iter().enumerate() {
                let RetainedUse::Natural(_) = u else {
                    continue;
                };
                let c = &self.topology().loops[face.outer].coedges[index];
                let mut actual = [
                    u.uv_points()[0].clone(),
                    u.uv_points().last().unwrap().clone(),
                ];
                if *reverse ^ c.reversed {
                    actual.reverse();
                }
                let mut matches = Vec::new();
                for original in &map.loops[side].coedges {
                    let uv = canonical_uv(original)?;
                    let Some(r) = parameters(&uv, &actual) else {
                        continue;
                    };
                    let reversed = r[0] > r[1];
                    let range = if reversed {
                        [r[1].clone(), r[0].clone()]
                    } else {
                        r
                    };
                    let curve = &source.edges[original.edge].curve;
                    let points = polynomial(curve)?;
                    let mut restricted = BoundaryDefinition {
                        points,
                        uv: [[Q::zero(), Q::zero()], [Q::one(), Q::zero()]],
                    }
                    .restricted(range.clone(), budget)?
                    .points;
                    if reversed {
                        restricted.reverse();
                    }
                    if polynomial_points_equal(
                        &restricted,
                        self.topology().edges[c.edge].curve.points(),
                        budget,
                    )? {
                        matches.push(OriginalEdgeLink {
                            roof_edge: c.edge,
                            source_edge: original.edge,
                            source_id: source.1.edges[original.edge],
                            source_range: range,
                            reversed_from_source: reversed,
                        });
                    }
                }
                require(
                    matches.len() == 1,
                    "Original named source boundary is missing or ambiguous",
                )?;
                let link = matches.pop().unwrap();
                if let Some(&old) = by_roof.get(&link.roof_edge) {
                    let prior: &OriginalEdgeLink = &map.links[old];
                    require(
                        prior.source_edge == link.source_edge
                            && prior.source_id == link.source_id
                            && prior.source_range == link.source_range
                            && prior.reversed_from_source == link.reversed_from_source,
                        "Shared source crease resolved to conflicting original identities",
                    )?;
                    continue;
                }
                let edge = &self.topology().edges[link.roof_edge];
                let source_edge = &source.edges[link.source_edge];
                for end in 0..2 {
                    let parameter =
                        link.source_range[usize::from(link.reversed_from_source) ^ end].clone();
                    let vertex = if parameter == Q::zero() {
                        Some(source_edge.vertices[0])
                    } else if parameter == Q::one() {
                        Some(source_edge.vertices[1])
                    } else {
                        None
                    };
                    if let Some(v) = vertex {
                        let point = source.vertices[v].point.map(|x| Q::from_float(x).unwrap());
                        require(
                            point == self.topology().vertices[edge.vertices[end]].point,
                            "Original vertex identity and exact roof endpoint disagree",
                        )?;
                        map.vertices
                            .insert(v, (source.vertices[v].point, source.1.vertices[v]));
                    }
                    map.endpoints.push(OriginalEndpointLink {
                        roof_vertex: edge.vertices[end],
                        source_edge_id: link.source_id,
                        source_parameter: parameter,
                        source_vertex: vertex,
                        source_vertex_id: vertex.map(|i| source.1.vertices[i]),
                    });
                }
                map.curves.insert(
                    link.source_edge,
                    (source_edge.clone(), link.source_id),
                );
                map.roof_roots.push((
                    link.roof_edge,
                    edge.curve.root.clone(),
                    edge.curve.range.clone(),
                ));
                by_roof.insert(link.roof_edge, map.links.len());
                map.links.push(link);
            }
        }
        require(
            self.geometry()
                .open_edges
                .iter()
                .all(|e| by_roof.contains_key(e)),
            "An outer roof edge has no original document binding",
        )?;
        Ok(map)
    }
}
