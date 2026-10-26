//! Geometric Boolean properties on reproducible random solid pairs.

use manifold_ops::{BooleanOp, boolean_manifold};

#[path = "../../manifold-core/tests/support/property_rng.rs"]
mod property_rng;
use property_rng::Rng;

fn cube(origin: [f64; 3], size: f64) -> (Vec<f64>, Vec<usize>) {
    let [x, y, z] = origin;
    let s = size;
    #[rustfmt::skip]
    let positions = vec![
        x, y, z, x + s, y, z, x + s, y + s, z, x, y + s, z,
        x, y, z + s, x + s, y, z + s, x + s, y + s, z + s, x, y + s, z + s,
    ];
    #[rustfmt::skip]
    let indices = vec![
        0, 2, 1, 0, 3, 2,
        4, 5, 6, 4, 6, 7,
        0, 1, 5, 0, 5, 4,
        2, 3, 7, 2, 7, 6,
        1, 2, 6, 1, 6, 5,
        3, 0, 4, 3, 4, 7,
    ];
    (positions, indices)
}

fn assert_close(actual: f64, expected: f64, case: usize, op: BooleanOp, metric: &str) {
    let tolerance = 1e-8 * actual.abs().max(expected.abs()).max(1.0);
    assert!(
        (actual - expected).abs() <= tolerance,
        "case {case}, {op:?} {metric}: {actual} != {expected} (tolerance {tolerance})"
    );
}

#[test]
fn union_and_intersection_are_symmetric() {
    let mut rng = Rng::for_property(0x5EED_1001);
    for case in 0..48 {
        // Sizes exceed every possible origin difference, so intersection is
        // non-empty and both directions exercise the same solid-only domain.
        let (ap, ai) = cube(
            [
                rng.below(4) as f64,
                rng.below(4) as f64,
                rng.below(4) as f64,
            ],
            4.0 + rng.below(5) as f64,
        );
        let (bp, bi) = cube(
            [
                rng.below(4) as f64,
                rng.below(4) as f64,
                rng.below(4) as f64,
            ],
            4.0 + rng.below(5) as f64,
        );
        for op in [BooleanOp::Union, BooleanOp::Intersection] {
            let forward = boolean_manifold(&ap, &ai, &bp, &bi, op)
                .unwrap_or_else(|error| panic!("case {case}, forward {op:?}: {error}"));
            let reverse = boolean_manifold(&bp, &bi, &ap, &ai, op)
                .unwrap_or_else(|error| panic!("case {case}, reverse {op:?}: {error}"));
            assert_close(forward.volume(), reverse.volume(), case, op, "volume");
            assert_close(
                forward.surface_area(),
                reverse.surface_area(),
                case,
                op,
                "surface area",
            );
        }
    }
}
