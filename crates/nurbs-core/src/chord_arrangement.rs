//! Split represented chord chains at proved proper crossings.
//! This is a bounded construction graph, not a region/topology certificate.
use crate::{
    Result, check, chord_intersection, curve_offset::Segment, curve_offset_diagnostics, numeric,
};
#[derive(Debug)]
pub struct Vertex {
    pub point: [f64; 2],
    pub error_upper_mm: f64,
}
#[derive(Debug)]
pub struct Edge {
    pub vertices: [usize; 2],
    pub source_edge: usize,
    pub source_domain: [f64; 2],
    /// Enclosures of the two exact normalized source-edge cut parameters in [0,1].
    pub source_parameters: [[f64; 2]; 2],
    pub error_upper_mm: f64,
}
#[derive(Debug)]
pub struct Arrangement {
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,
}
struct Cut {
    parameter: [f64; 2],
    vertex: usize,
}
pub fn split(
    chain: &[Segment],
    closed: bool,
    tolerance_mm: f64,
    max_pairs: usize,
) -> Result<Arrangement> {
    check(
        tolerance_mm.is_finite() && tolerance_mm > 0.,
        "Use a positive finite intersection tolerance.",
    )?;
    let diagnostics = curve_offset_diagnostics::inspect_chain(chain, closed, max_pairs)?;
    numeric(
        diagnostics.complete
            && diagnostics.contacts.is_empty()
            && diagnostics.uncertain.is_empty()
            && diagnostics.degenerate.is_empty(),
        "Chain splitting needs complete diagnostics without contacts, overlap or degeneracy.",
    )?;
    check(
        chain.len() + usize::from(!closed) + diagnostics.crossings.len() <= 65536,
        "Intersection graph exceeds vertex budget.",
    )?;
    let mut vertices = Vec::new();
    vertices.push(Vertex {
        point: chain[0].points[0],
        error_upper_mm: 0.,
    });
    for (index, edge) in chain.iter().enumerate() {
        if !closed || index + 1 < chain.len() {
            vertices.push(Vertex {
                point: edge.points[1],
                error_upper_mm: 0.,
            });
        }
    }
    let mut cuts: Vec<Vec<Cut>> = (0..chain.len())
        .map(|i| {
            vec![
                Cut {
                    parameter: [0., 0.],
                    vertex: i,
                },
                Cut {
                    parameter: [1., 1.],
                    vertex: if closed && (i + 1 == chain.len()) {
                        0
                    } else {
                        i + 1
                    },
                },
            ]
        })
        .collect();
    for [a, b] in diagnostics.crossings {
        let crossing = chord_intersection::proper(chain[a].points, chain[b].points, tolerance_mm)?;
        let vertex = vertices.len();
        vertices.push(Vertex {
            point: crossing.point,
            error_upper_mm: crossing.error_upper_mm,
        });
        cuts[a].push(Cut {
            parameter: crossing.parameter_a,
            vertex,
        });
        cuts[b].push(Cut {
            parameter: crossing.parameter_b,
            vertex,
        });
    }
    let mut edges = Vec::new();
    for (source_edge, mut row) in cuts.into_iter().enumerate() {
        row.sort_by(|a, b| a.parameter[0].total_cmp(&b.parameter[0]));
        for pair in row.windows(2) {
            numeric(
                pair[0].parameter[1] < pair[1].parameter[0],
                "Intersection order is unresolved; coincident or close cuts need graph reconciliation.",
            )?;
            check(
                edges.len() < 131072,
                "Intersection graph exceeds edge budget.",
            )?;
            let error_upper_mm = vertices[pair[0].vertex]
                .error_upper_mm
                .max(vertices[pair[1].vertex].error_upper_mm);
            edges.push(Edge {
                vertices: [pair[0].vertex, pair[1].vertex],
                source_edge,
                source_domain: chain[source_edge].domain,
                source_parameters: [pair[0].parameter, pair[1].parameter],
                error_upper_mm,
            });
        }
    }
    Ok(Arrangement { vertices, edges })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn chain(points: &[[f64; 2]]) -> Vec<Segment> {
        points
            .windows(2)
            .enumerate()
            .map(|(i, p)| Segment {
                domain: [i as f64, (i + 1) as f64],
                points: [p[0], p[1]],
                error_upper_mm: 0.,
            })
            .collect()
    }
    #[test]
    fn bowtie_splits_into_six_connected_edges_with_shared_crossing() {
        let input = chain(&[[0., 0.], [2., 2.], [0., 2.], [2., 0.], [0., 0.]]);
        let graph = split(&input, true, 1e-6, 100).unwrap();
        assert_eq!(graph.vertices.len(), 5);
        assert_eq!(graph.edges.len(), 6);
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|e| e.vertices.contains(&4))
                .count(),
            4
        );
        assert!(
            graph.vertices[4]
                .point
                .iter()
                .all(|v| (*v - 1.).abs() <= graph.vertices[4].error_upper_mm)
        );
        assert!(graph.edges.iter().all(|e| e.error_upper_mm <= 1e-6));
        assert_eq!(graph.edges.iter().filter(|e| e.source_edge == 0).count(), 2);
        assert_eq!(graph.edges.iter().filter(|e| e.source_edge == 2).count(), 2);
    }
    #[test]
    fn multiple_crossings_are_ordered_by_proved_parameter_intervals() {
        let input = chain(&[
            [0., 0.],
            [10., 0.],
            [8., 2.],
            [8., -2.],
            [2., -2.],
            [2., 2.],
        ]);
        let graph = split(&input, false, 1e-6, 100).unwrap();
        let pieces: Vec<_> = graph.edges.iter().filter(|e| e.source_edge == 0).collect();
        assert_eq!(pieces.len(), 3);
        assert_eq!(pieces[0].vertices[1], pieces[1].vertices[0]);
        assert_eq!(pieces[1].vertices[1], pieces[2].vertices[0]);
        assert!(pieces[0].source_parameters[1][1] < pieces[1].source_parameters[1][0]);
        assert_eq!(pieces[0].source_domain, [0., 1.]);
    }
    #[test]
    fn simple_chains_preserve_exact_vertices_and_budget_refusal_is_atomic() {
        let input = chain(&[[0., 0.], [2., 0.], [2., 2.], [0., 2.], [0., 0.]]);
        let graph = split(&input, true, 1e-6, 100).unwrap();
        assert_eq!(graph.vertices.len(), 4);
        assert_eq!(graph.edges.len(), 4);
        assert!(graph.vertices.iter().all(|v| v.error_upper_mm == 0.));
        assert!(split(&input, true, 1e-6, 1).is_err());
        let overlap = chain(&[[0., 0.], [2., 0.], [1., 0.]]);
        assert!(split(&overlap, false, 1e-6, 100).is_err());
    }
}
