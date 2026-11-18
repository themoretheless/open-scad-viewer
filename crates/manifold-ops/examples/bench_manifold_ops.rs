//! manifold-ops and diagnostics microbenchmarks via rbench (release build).
//!
//! ```sh
//! cargo run --release -p manifold-ops --example bench_manifold_ops -- --profile quick
//! ```
//!
//! Companion of `manifold-core`'s `bench_manifold`: booleans, diagnostics
//! (self-intersection, mass properties), hull and decimation on fixed
//! icosphere fixtures. Every case validates its result so a faster wrong
//! answer fails. `tools/manifold-bench/rbench-regression.mjs` runs both
//! suites and compares them against the per-platform baseline.
#[path = "support/large_mesh.rs"]
mod large_mesh;

use large_mesh::{icosphere, place};
use manifold_core::{SelfIntersectionOptions, mass_properties, self_intersections};
use manifold_ops::decimate::{DecimateOptions, decimate};
use manifold_ops::hull::{HullOptions, hull_mesh};
use manifold_ops::{BooleanOp, boolean_manifold};
use rbench::{DropPolicy, Suite};

fn main() -> rbench::Result<()> {
    let (p, i) = icosphere(4); // 5120 triangles, 2562 vertices
    let (a, ai) = icosphere(2); // 320 triangles
    let (mut b, bi) = icosphere(2);
    place(&mut b, 1.0, [0.5, 0.3, 0.2]);

    let mut suite = Suite::new("manifold-ops/kernels");
    suite
        .bench_with_input(
            "boolean-union/icosphere-320x2",
            || (a.clone(), ai.clone(), b.clone(), bi.clone()),
            |(a, ai, b, bi)| {
                let out = boolean_manifold(a, ai, b, bi, BooleanOp::Union).expect("union");
                assert!(out.output_report.is_manifold());
                out.indices.len()
            },
            DropPolicy::InsideTiming,
        )
        .parameter("triangles", 640.);
    suite
        .bench_with_input(
            "boolean-difference/icosphere-320x2",
            || (a.clone(), ai.clone(), b.clone(), bi.clone()),
            |(a, ai, b, bi)| {
                let out =
                    boolean_manifold(a, ai, b, bi, BooleanOp::Difference).expect("difference");
                assert!(out.output_report.is_manifold());
                out.indices.len()
            },
            DropPolicy::InsideTiming,
        )
        .parameter("triangles", 640.);
    suite
        .bench_with_input(
            "self-intersections/icosphere-5120",
            || (p.clone(), i.clone()),
            |(p, i)| {
                let report =
                    self_intersections(p, i, SelfIntersectionOptions::default()).expect("scan");
                assert!(report.pairs.is_empty());
                report.candidate_pairs
            },
            DropPolicy::InsideTiming,
        )
        .parameter("triangles", 5120.);
    suite
        .bench_with_input(
            "mass-properties/icosphere-5120",
            || (p.clone(), i.clone()),
            |(p, i)| {
                let m = mass_properties(p, i, 1.0).expect("closed");
                assert!(m.volume > 4.0 && m.volume < 4.2);
                m.volume.to_bits()
            },
            DropPolicy::InsideTiming,
        )
        .parameter("triangles", 5120.);
    suite
        .bench_with_input(
            "hull-mesh/icosphere-5120",
            || (p.clone(), i.clone()),
            |(p, i)| {
                let hull = hull_mesh(p, i, HullOptions::default()).expect("hull");
                assert_eq!(hull.indices.len(), 5120 * 3);
                hull.indices.len()
            },
            DropPolicy::InsideTiming,
        )
        .parameter("triangles", 5120.);
    suite
        .bench_with_input(
            "decimate/icosphere-5120-to-1280",
            || (p.clone(), i.clone()),
            |(p, i)| {
                let options = DecimateOptions {
                    target_triangles: 1280,
                    measure_deviation: false,
                    ..Default::default()
                };
                let out = decimate(p, i, options).expect("decimate");
                assert!(out.report.target_reached);
                out.indices.len()
            },
            DropPolicy::InsideTiming,
        )
        .parameter("triangles", 5120.);

    suite.main()
}
