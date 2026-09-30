//! Admission of the represented straight-edge graph, without source topology claims.
use crate::{
    Result, check,
    chord_arrangement::Arrangement,
    curve_offset::Segment,
    curve_offset_diagnostics::{apart, orientation, shared_vertex_only},
    numeric,
};
use std::collections::BTreeSet;
pub fn admit(graph: &Arrangement, max_pairs: usize) -> Result<usize> {
    check(
        !graph.vertices.is_empty()
            && graph.vertices.len() <= 65536
            && !graph.edges.is_empty()
            && graph.edges.len() <= 131072
            && (1..=1000000).contains(&max_pairs),
        "Use a bounded graph and pair budget.",
    )?;
    let mut coordinates = BTreeSet::new();
    for vertex in &graph.vertices {
        check(
            vertex.point.iter().all(|x| x.is_finite() && x.abs() <= 1e9),
            "Graph coordinates exceed bounds.",
        )?;
        let key = vertex.point.map(|x| if x == 0. { 0 } else { x.to_bits() });
        numeric(
            coordinates.insert(key),
            "Coincident graph vertices need reconciliation.",
        )?;
    }
    let mut lines = Vec::new();
    for edge in &graph.edges {
        let [a, b] = edge.vertices;
        check(
            a < graph.vertices.len() && b < graph.vertices.len() && a != b,
            "Invalid graph edge vertices.",
        )?;
        lines.push(Segment {
            domain: [0., 1.],
            points: [graph.vertices[a].point, graph.vertices[b].point],
            error_upper_mm: 0.,
        });
    }
    let mut checks = 0;
    for i in 0..lines.len() {
        for j in i + 1..lines.len() {
            check(checks < max_pairs, "Graph embedding pair budget exceeded.")?;
            checks += 1;
            let (a, b) = (&lines[i], &lines[j]);
            if apart(a, b) {
                continue;
            }
            let shared = graph.edges[i]
                .vertices
                .iter()
                .filter(|v| graph.edges[j].vertices.contains(v))
                .count();
            if shared == 1 && shared_vertex_only(a, b)? {
                continue;
            }
            numeric(shared == 0, "Graph edges overlap at shared vertices.")?;
            let signs = [
                orientation(a.points[0], a.points[1], b.points[0])?,
                orientation(a.points[0], a.points[1], b.points[1])?,
                orientation(b.points[0], b.points[1], a.points[0])?,
                orientation(b.points[0], b.points[1], a.points[1])?,
            ];
            let separated = matches!((signs[0],signs[1]),(Some(x),Some(y)) if x*y>0)
                || matches!((signs[2],signs[3]),(Some(x),Some(y)) if x*y>0);
            numeric(
                separated,
                "Represented graph has an unjoined crossing, contact or unresolved pair.",
            )?;
        }
    }
    Ok(checks)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::chord_arrangement::{Edge, Vertex};
    fn edge(a: usize, b: usize) -> Edge {
        Edge {
            vertices: [a, b],
            source_edge: 0,
            source_domain: [0., 1.],
            source_parameters: [[0., 0.], [1., 1.]],
            error_upper_mm: 0.,
        }
    }
    fn graph(points: &[[f64; 2]], edges: &[[usize; 2]]) -> Arrangement {
        Arrangement {
            vertices: points
                .iter()
                .map(|p| Vertex {
                    point: *p,
                    error_upper_mm: 0.,
                })
                .collect(),
            edges: edges.iter().map(|e| edge(e[0], e[1])).collect(),
        }
    }
    #[test]
    fn unjoined_crossings_contacts_and_duplicate_vertices_are_refused() {
        assert!(
            admit(
                &graph(&[[0., 0.], [2., 2.], [0., 2.], [2., 0.]], &[[0, 1], [2, 3]]),
                100
            )
            .is_err()
        );
        assert!(
            admit(
                &graph(&[[0., 0.], [2., 0.], [1., 0.], [1., 1.]], &[[0, 1], [2, 3]]),
                100
            )
            .is_err()
        );
        assert!(
            admit(
                &graph(&[[0., 0.], [1., 1.], [0., 0.]], &[[0, 1], [1, 2]]),
                100
            )
            .is_err()
        );
    }
    #[test]
    fn shared_endpoints_and_separated_edges_are_admitted() {
        let g = graph(
            &[[0., 0.], [1., 0.], [2., 0.], [0., 2.], [2., 2.]],
            &[[0, 1], [1, 2], [3, 4]],
        );
        assert_eq!(admit(&g, 100).unwrap(), 3);
        assert!(admit(&g, 1).is_err());
    }
}
