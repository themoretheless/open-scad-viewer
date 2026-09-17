//! Symmetric Chamfer distance benchmark.
//! `cargo run --release -p osv-math --features cuda --example bench_chamfer`.
use math_core::{Acceleration, V3, chamfer_distance};
use std::time::Instant;

fn modes() -> Vec<Acceleration> {
    let mut modes = vec![Acceleration::Cpu, Acceleration::Auto, Acceleration::Gpu];
    if cfg!(feature = "cuda") {
        modes.push(Acceleration::Cuda);
    }
    if let Ok(filter) = std::env::var("CHAMFER_BENCH_MODES") {
        let wanted: Vec<Acceleration> = filter.split(',').filter_map(Acceleration::parse).collect();
        modes.retain(|mode| wanted.contains(mode));
    }
    modes
}

fn median(times: &mut [f64]) -> f64 {
    times.sort_by(f64::total_cmp);
    times[times.len() / 2]
}

fn points(n: usize, seed: f64) -> Vec<V3> {
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

fn compare(a: &[V3], b: &[V3], rounds: usize) {
    let modes = modes();
    let reference = chamfer_distance(a, b, Acceleration::Cpu).unwrap();
    let mut times = vec![Vec::new(); modes.len()];
    for round in 0..rounds {
        for (index, &mode) in modes.iter().enumerate() {
            let start = Instant::now();
            let got = chamfer_distance(a, b, mode).unwrap();
            times[index].push(start.elapsed().as_secs_f64() * 1000.);
            if round == 0 {
                let tol = if mode == Acceleration::Cpu {
                    1e-12
                } else {
                    5e-3 * reference.symmetric_mean_squared_distance.max(1.0)
                };
                assert!(
                    (got.symmetric_mean_squared_distance
                        - reference.symmetric_mean_squared_distance)
                        .abs()
                        < tol,
                    "{mode:?} mismatch: {:?} vs {:?}",
                    got,
                    reference
                );
            }
        }
    }
    let summary: Vec<String> = modes
        .iter()
        .zip(times.iter_mut())
        .map(|(mode, times)| format!("{} median {:.3}ms", mode.label(), median(times)))
        .collect();
    println!(
        "chamfer ({} x {}): {}, rms {:.6}",
        a.len(),
        b.len(),
        summary.join(", "),
        reference.symmetric_rms_distance
    );
}

fn main() {
    #[cfg(feature = "gpu")]
    println!(
        "wgpu backend: {}",
        math_core::gpu::backend_label().unwrap_or("none (falls back to cpu)")
    );
    #[cfg(feature = "cuda")]
    println!(
        "cuda device: {}",
        math_core::cuda::device_name().unwrap_or_else(|| "none (falls back to wgpu/cpu)".into())
    );
    for &(a, b) in &[
        (1_000usize, 1_000usize),
        (5_000, 2_000),
        (20_000, 5_000),
        (100_000, 10_000),
    ] {
        let a = points(a, 0.);
        let b = points(b, 1_000_000.);
        compare(&a, &b, 5);
    }
}
