//! GPU/CUDA vs CPU nearest-neighbor benchmark (may be removed after
//! qualification). `cargo run --release -p osv-math --features cuda --example bench_gpu`.
//!
//! Unlike a per-point affine transform (too cheap to amortize kernel-launch
//! overhead — see `crates/math-core/README.md`), brute-force nearest-neighbor
//! search does `target_count` work per query, so it has enough arithmetic
//! intensity for GPU/CUDA placements to actually win at moderate-to-large
//! sizes. This benchmark measures where that crossover is on this machine.
use math_core::{Acceleration, V3, nearest_neighbor, nearest_neighbor_accelerated};
use std::time::Instant;

/// Placements to compare: CPU reference, wgpu shader and (feature `cuda`) the
/// CUDA driver port. `NEAREST_NEIGHBOR_BENCH_MODES=cpu,cuda` narrows the set.
fn modes() -> Vec<Acceleration> {
    let mut modes = vec![Acceleration::Cpu, Acceleration::Gpu];
    if cfg!(feature = "cuda") {
        modes.push(Acceleration::Cuda);
    }
    if let Ok(filter) = std::env::var("NEAREST_NEIGHBOR_BENCH_MODES") {
        let wanted: Vec<Acceleration> = filter.split(',').filter_map(Acceleration::parse).collect();
        modes.retain(|mode| wanted.contains(mode));
    }
    modes
}

fn median(times: &mut [f64]) -> f64 {
    times.sort_by(f64::total_cmp);
    times[times.len() / 2]
}

fn compare(queries: &[V3], targets: &[V3], rounds: usize) {
    let modes = modes();
    let reference = nearest_neighbor(queries, targets);
    let mut times = vec![Vec::new(); modes.len()];
    for round in 0..rounds {
        for (index, &acceleration) in modes.iter().enumerate() {
            let start = Instant::now();
            let got = nearest_neighbor_accelerated(queries, targets, acceleration);
            times[index].push(start.elapsed().as_secs_f64() * 1000.);
            if round == 0 {
                for ((ri, rd), (gi, gd)) in reference.iter().zip(&got) {
                    // f32 vs f64 rounding can flip the argmin between two
                    // near-tied targets; only the winning *distance* is a
                    // meaningful correctness check across precisions.
                    let tol = if acceleration == Acceleration::Cpu {
                        1e-12
                    } else {
                        5e-3 * rd.max(1.0)
                    };
                    assert!(
                        (rd - gd).abs() < tol,
                        "{acceleration:?} dist mismatch: {rd:?} (idx {ri}) vs {gd:?} (idx {gi})"
                    );
                }
            }
        }
    }
    let summary: Vec<String> = modes
        .iter()
        .zip(times.iter_mut())
        .map(|(mode, times)| format!("{} median {:.3}ms", mode.label(), median(times)))
        .collect();
    println!(
        "nearest_neighbor ({} queries x {} targets): {}",
        queries.len(),
        targets.len(),
        summary.join(", ")
    );
}

fn make_points(n: usize, seed: f64) -> Vec<V3> {
    (0..n)
        .map(|i| {
            let f = i as f64 + seed;
            [
                f * 0.011 - 500.,
                (f * 0.0027).sin() * 200.,
                (f * 0.0043).cos() * 200.,
            ]
        })
        .collect()
}

fn main() {
    #[cfg(feature = "cuda")]
    println!(
        "cuda device: {}",
        math_core::cuda::device_name().unwrap_or_else(|| "none (falls back to wgpu/cpu)".into())
    );
    // (query_count, target_count) pairs spanning from clearly CPU-favored
    // (tiny target sets) to the regime where the O(queries * targets) brute
    // force gives the GPU/CUDA placements enough work to win.
    for &(q, t) in &[
        (1_000usize, 100usize),
        (3_000, 100),
        (1_000, 1_000),
        (3_000, 300),
        (5_000, 300),
        (3_000, 1_000),
        (10_000, 1_000),
        (100_000, 1_000),
        (100_000, 5_000),
        (1_000_000, 2_000),
    ] {
        let queries = make_points(q, 0.0);
        let targets = make_points(t, 1_000_000.0);
        compare(&queries, &targets, 5);
    }
}
