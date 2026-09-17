//! Does `EvaluationOptions::acceleration = Auto`'s GPU/CUDA path ever beat
//! the exact KD-tree that `evaluate_clouds` uses with `Cpu`? KD-tree query is
//! O(log n) per point; brute force is O(target_count) per point regardless of
//! GPU parallelism, so the crossover (if any) should shrink and eventually
//! vanish as cloud size grows toward photogrammetry-core's real
//! 2,000,000-point ceiling.
//!
//! `cargo run --release -p photogrammetry-core --features cuda --example kdtree_vs_gpu`
//! (drop `--features cuda` to fall back to the wgpu shader; without the
//! `gpu` feature at all, `evaluate_clouds` always uses the KD-tree).
use photogrammetry_core::{
    Acceleration,
    evaluation::{EvaluationOptions, evaluate_clouds},
};
use std::time::Instant;

/// Deterministic synthetic point cloud: a wavy sheet, sampled on a roughly
/// square grid so `sqrt(n)` stays an integer-ish side length.
fn cloud(n: usize, phase: f64) -> Vec<[f64; 3]> {
    let side = (n as f64).sqrt().ceil() as usize;
    (0..n)
        .map(|i| {
            let x = (i % side) as f64 * 0.01;
            let y = (i / side) as f64 * 0.01;
            let z = (x * 8. + phase).sin() * 0.07 + (y * 11. + phase).cos() * 0.05;
            [x, y, z]
        })
        .collect()
}

fn main() {
    let acceleration = match std::env::var("KDTREE_VS_GPU_ACCELERATION") {
        Ok(value) => Acceleration::parse(&value).unwrap_or(Acceleration::Auto),
        Err(_) => Acceleration::Auto,
    };
    let cpu_options = EvaluationOptions {
        tolerance: 0.004,
        voxel_size: None,
        acceleration: Acceleration::Cpu,
    };
    let accelerated_options = EvaluationOptions {
        acceleration,
        ..cpu_options.clone()
    };
    println!("acceleration = {acceleration:?}");
    println!(
        "{:>10} {:>10} {:>10} | {:>12} {:>12} | {:>10}",
        "model_n", "ref_n", "work", "kdtree_ms", "auto_ms", "winner"
    );
    // Sizes bracketing the crossover found for the single-direction
    // benchmark (`bench_gpu.rs`), then climbing toward photogrammetry-core's
    // real workloads (evaluation caps each cloud at 2,000,000 points).
    // Squaring the recommendation's `work` threshold at equal-size clouds
    // already lands in the tens-of-billions at 200K points, so this stops
    // well short of the full 2,000,000-point ceiling: a naive quadratic scan
    // at that scale would take on the order of hours, not a benchmark run.
    for &n in &[
        2_000usize, 8_000, 40_000, 100_000, 200_000, 500_000, 1_000_000,
    ] {
        let model = cloud(n, 0.0);
        let reference = cloud(n, 0.37);
        // Warm up device buffers/allocations before timing either path.
        let _ = evaluate_clouds(&model, &reference, &cpu_options, |_, _, _| true).unwrap();
        let _ = evaluate_clouds(&model, &reference, &accelerated_options, |_, _, _| true).unwrap();

        let start = Instant::now();
        let exact = evaluate_clouds(&model, &reference, &cpu_options, |_, _, _| true).unwrap();
        let kdtree_ms = start.elapsed().as_secs_f64() * 1000.;

        let start = Instant::now();
        let accelerated =
            evaluate_clouds(&model, &reference, &accelerated_options, |_, _, _| true).unwrap();
        let gpu_ms = start.elapsed().as_secs_f64() * 1000.;

        // Sanity: brute force (possibly f32 on GPU/CUDA) should still agree
        // with the exact f64 KD-tree distances to within a fraction of a
        // percent, otherwise the "comparison" would be meaningless.
        let rel = |a: f64, b: f64| ((a - b).abs() / a.max(b).max(1e-12)) < 5e-3;
        assert!(rel(
            exact.reconstructed_to_reference.mean,
            accelerated.reconstructed_to_reference.mean
        ));
        assert!(rel(
            exact.reference_to_reconstructed.mean,
            accelerated.reference_to_reconstructed.mean
        ));

        println!(
            "{:>10} {:>10} {:>10} | {:>12.3} {:>12.3} | {:>10}",
            n,
            n,
            n * n,
            kdtree_ms,
            gpu_ms,
            if gpu_ms < kdtree_ms { "gpu" } else { "kdtree" }
        );
    }
}
