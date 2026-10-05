//! Common endpoint enclosures on exact shell identities, without proximity welding.
use crate::source_shell_incidence::Shell;
use nurbs_core::{Error, Result};
use std::collections::BTreeMap;
#[derive(Debug, PartialEq)]
pub struct Vertex {
    pub id: usize,
    pub bounds: [[f64; 2]; 3],
    /// Canonical edge and increasing-parameter endpoint. Source roots stay on edges.
    pub ends: Vec<[usize; 2]>,
    pub poles: Vec<usize>,
}
#[derive(Debug, PartialEq)]
pub struct Network {
    pub vertices: Vec<Vertex>,
    pub spans: usize,
    pub endpoints: usize,
}
fn error(message: &str) -> Error {
    Error::new("BREP_SOURCE_BOUNDARY_NETWORK", message)
}
fn meet(
    vertices: &mut BTreeMap<usize, Vertex>,
    id: usize,
    bounds: [[f64; 2]; 3],
) -> Result<&mut Vertex> {
    if !bounds
        .iter()
        .all(|b| b[0].is_finite() && b[1].is_finite() && b[0] <= b[1])
    {
        return Err(error("Invalid source endpoint enclosure"));
    }
    let vertex = vertices.entry(id).or_insert_with(|| Vertex {
        id,
        bounds,
        ends: vec![],
        poles: vec![],
    });
    for axis in 0..3 {
        vertex.bounds[axis] = [
            vertex.bounds[axis][0].max(bounds[axis][0]),
            vertex.bounds[axis][1].min(bounds[axis][1]),
        ];
        if vertex.bounds[axis][0] > vertex.bounds[axis][1] {
            return Err(error("Enclosures of one exact vertex are disjoint"));
        }
    }
    Ok(vertex)
}
/// Incidence proves identities; interval intersections only tighten their
/// display enclosures. No centre point, rounded root or distance join is authority.
/// This does not certify tessellation, new topology, embedding or volume.
pub fn inspect(shell: &Shell, max_spans: usize, max_endpoints: usize) -> Result<Network> {
    if !(1..=100000).contains(&max_spans) || !(1..=100000).contains(&max_endpoints) {
        return Err(error("Bound source boundary work"));
    }
    let endpoints = shell
        .edges()
        .len()
        .checked_mul(2)
        .and_then(|n| n.checked_add(shell.poles().len()))
        .ok_or_else(|| error("Endpoint count overflow"))?;
    if endpoints > max_endpoints {
        return Err(error("Source endpoint work limit"));
    }
    let mut spans = 0usize;
    for edge in shell.edges() {
        let c = edge.world();
        let count = (c.degree..c.control_points.len())
            .filter(|&i| c.knots[i] < c.knots[i + 1])
            .count();
        spans = spans
            .checked_add(
                count
                    .checked_mul(2)
                    .ok_or_else(|| error("Span count overflow"))?,
            )
            .ok_or_else(|| error("Span count overflow"))?;
        if spans > max_spans {
            return Err(error("Source boundary span work limit"));
        }
    }
    let mut vertices = BTreeMap::new();
    for (index, edge) in shell.edges().iter().enumerate() {
        let use_ = shell.uses()[index][0];
        let mut ids = shell.vertices()[use_.face][use_.wire][use_.edge];
        if edge.reversed()[0] {
            ids.reverse();
        }
        let restriction = crate::source_edge_restriction::Restriction::from_edge(edge);
        for (end, bounds) in restriction
            .endpoint_boxes(max_spans)?
            .into_iter()
            .enumerate()
        {
            meet(&mut vertices, ids[end], bounds)?
                .ends
                .push([index, end]);
        }
    }
    for (index, (use_, pole)) in shell.poles().iter().enumerate() {
        let ids = shell.vertices()[use_.face][use_.wire][use_.edge];
        if ids[0] != ids[1] {
            return Err(error(
                "Collapsed boundary has different endpoint identities",
            ));
        }
        meet(&mut vertices, ids[0], pole.point().map(|p| [p, p]))?
            .poles
            .push(index);
    }
    Ok(Network {
        vertices: vertices.into_values().collect(),
        spans,
        endpoints,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coincident_enclosures_do_not_weld_distinct_identities() {
        let mut vertices = BTreeMap::new();
        let bounds = [[0., 1.]; 3];
        meet(&mut vertices, 7, bounds).unwrap();
        meet(&mut vertices, 8, bounds).unwrap();
        assert_eq!(vertices.len(), 2);
        meet(&mut vertices, 7, [[0.25, 0.75]; 3]).unwrap();
        assert_eq!(vertices[&7].bounds, [[0.25, 0.75]; 3]);
        assert_eq!(vertices[&8].bounds, bounds);
    }
    #[test]
    fn incompatible_or_nonfinite_enclosures_are_refused() {
        let mut vertices = BTreeMap::new();
        meet(&mut vertices, 0, [[0., 1.]; 3]).unwrap();
        assert!(meet(&mut vertices, 0, [[2., 3.]; 3]).is_err());
        assert!(meet(&mut BTreeMap::new(), 1, [[f64::NAN, 1.]; 3]).is_err());
        assert!(meet(&mut BTreeMap::new(), 1, [[0., f64::INFINITY]; 3]).is_err());
    }
}
