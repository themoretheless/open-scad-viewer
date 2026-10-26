//! Reproducible hostile-input coverage for the indexed mesh transport.

use manifold_core::{IndexedMeshError, validate_indexed_mesh};

#[path = "support/property_rng.rs"]
mod property_rng;
use property_rng::Rng;

#[test]
fn malformed_buffers_are_rejected_without_panics() {
    let mut rng = Rng::for_property(0x5EED_2001);
    for case in 0..4000 {
        let vertex_count = 1 + rng.below(20);
        let triangle_count = 1 + rng.below(30);
        let mut positions = Vec::with_capacity(vertex_count * 3);
        for _ in 0..vertex_count * 3 {
            positions.push(rng.coord());
        }
        let mut indices = Vec::with_capacity(triangle_count * 3);
        for _ in 0..triangle_count * 3 {
            indices.push(rng.below(vertex_count));
        }
        let mutation = case % 6;
        match mutation {
            0 => {
                positions.pop();
            }
            1 => {
                indices.push(0);
            }
            2 => {
                let index = rng.below(positions.len());
                positions[index] = f64::NAN;
            }
            3 => {
                let index = rng.below(positions.len());
                positions[index] = f64::INFINITY;
            }
            4 => {
                let index = rng.below(indices.len());
                indices[index] = vertex_count + rng.below(1000);
            }
            _ => {}
        }
        let result = validate_indexed_mesh(&positions, &indices);
        match mutation {
            0 => assert!(
                matches!(result, Err(IndexedMeshError::PositionsLength { .. })),
                "case {case}"
            ),
            1 => assert!(
                matches!(result, Err(IndexedMeshError::IndicesLength { .. })),
                "case {case}"
            ),
            2 | 3 => assert!(
                matches!(result, Err(IndexedMeshError::NonFiniteCoordinate { .. })),
                "case {case}"
            ),
            4 => assert!(
                matches!(result, Err(IndexedMeshError::IndexOutOfRange { .. })),
                "case {case}"
            ),
            _ => assert_eq!(result, Ok(()), "case {case}"),
        }
    }
}
