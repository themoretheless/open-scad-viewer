//! Bounded body- and face-centered graphs for CAD lightening.
#[cfg(test)]
use super::{Result, Value, encode};

#[cfg(test)]
pub(super) fn graph(
    min: [f64; 3],
    max: [f64; 3],
    cells: [usize; 3],
    pattern: &str,
) -> Result<Value> {
    let g = polygon_core::lattice_tools::centered_graph(min, max, cells, pattern)?;
    encode(value_codec::json!({"nodes":g.nodes,"edges":g.edges}))
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn bounded_centered_topologies() {
        for (pattern, cells, nodes_expected, edges_expected) in [
            ("bcc", [1, 1, 1], 9, 8),
            ("bcc", [3, 3, 3], 91, 216),
            ("octet", [1, 1, 1], 14, 36),
            ("octet", [2, 2, 2], 63, 240),
        ] {
            let v = graph([0.; 3], [12.; 3], cells, pattern).unwrap();
            let nodes: Vec<[f64; 3]> = crate::field(&v, "nodes").unwrap();
            let edges: Vec<[usize; 2]> = crate::field(&v, "edges").unwrap();
            assert_eq!(nodes.len(), nodes_expected);
            assert_eq!(edges.len(), edges_expected);
            assert_eq!(edges.iter().collect::<BTreeSet<_>>().len(), edges.len());
            for [a, b] in edges {
                assert!(a < b && b < nodes.len());
                assert_ne!(nodes[a], nodes[b]);
            }
        }
        assert!(graph([0.; 3], [12.; 3], [4, 4, 4], "bcc").is_err());
        // 113 nodes fit, but 464 edges exceed the independent edge budget.
        assert!(graph([0.; 3], [12.; 3], [4, 2, 2], "octet").is_err());
    }
}
