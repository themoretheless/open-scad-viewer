//! Point-cloud centroid/covariance reduction benchmark.
//! `cargo run --release -p osv-math --features cuda --example bench_point_moments`.
use math_core::{Acceleration, V3, point_moments_accelerated};
use std::time::Instant;

fn modes() -> Vec<Acceleration> {
    let mut modes = vec![Acceleration::Cpu, Acceleration::Auto, Acceleration::Gpu];
    if cfg!(feature = "cuda") {
        modes.push(Acceleration::Cuda);
    }
    if let Ok(filter) = std::env::var("POINT_MOMENTS_BENCH_MODES") {
        let wanted: Vec<Acceleration> = filter.split(',').filter_map(Acceleration::parse).collect();
        modes.retain(|mode| wanted.contains(mode));
    }
    modes
}

fn median(times: &mut [f64]) -> f64 {
    times.sort_by(f64::total_cmp);
    times[times.len() / 2]
}

fn points(n: usize) -> Vec<V3> {
    (0..n)
        .map(|i| {
            let f = i as f64;
            [
                (f * 0.013).sin() * 200. - 7.,
                (f * 0.017).cos() * 120. + 3.,
                f * 0.00031 - 50.,
            ]
        })
        .collect()
}

fn compare(points: &[V3], rounds: usize) {
    let modes = modes();
    let reference = point_moments_accelerated(points, Acceleration::Cpu).unwrap();
    let mut times = vec![Vec::new(); modes.len()];
    for round in 0..rounds {
        for (index, &mode) in modes.iter().enumerate() {
            let start = Instant::now();
            let got = point_moments_accelerated(points, mode).unwrap();
            times[index].push(start.elapsed().as_secs_f64() * 1000.);
            if round == 0 {
                let tol = if mode.is_gpu() && mode != Acceleration::Auto {
                    1e-1
                } else {
                    1e-9
                };
                for axis in 0..3 {
                    assert!((got.centroid[axis] - reference.centroid[axis]).abs() < tol);
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
        "point_moments ({} points): {}, centroid={:?}",
        points.len(),
        summary.join(", "),
        reference.centroid
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
    for &n in &[10_000usize, 100_000, 1_000_000, 5_000_000] {
        compare(&points(n), 7);
    }
}
