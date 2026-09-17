//! Fused point-cloud bounds + moments benchmark.
//! `cargo run --release -p osv-math --features cuda --example bench_point_cloud_stats`.
use math_core::{
    Acceleration, V3, point_bounds_accelerated, point_cloud_stats_accelerated,
    point_moments_accelerated,
};
use std::time::Instant;

fn modes() -> Vec<Acceleration> {
    let mut modes = vec![Acceleration::Cpu, Acceleration::Auto, Acceleration::Gpu];
    if cfg!(feature = "cuda") {
        modes.push(Acceleration::Cuda);
    }
    if let Ok(filter) = std::env::var("POINT_STATS_BENCH_MODES") {
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
    let reference = point_cloud_stats_accelerated(points, Acceleration::Cpu).unwrap();
    let mut fused_times = vec![Vec::new(); modes.len()];
    let mut separate_times = vec![Vec::new(); modes.len()];
    for round in 0..rounds {
        for (index, &mode) in modes.iter().enumerate() {
            let start = Instant::now();
            let got = point_cloud_stats_accelerated(points, mode).unwrap();
            fused_times[index].push(start.elapsed().as_secs_f64() * 1000.);
            if round == 0 {
                let resolved = mode.resolve_for_point_cloud_stats(points.len());
                let tol = if resolved.is_gpu() { 1e-1 } else { 1e-9 };
                for axis in 0..3 {
                    assert!((got.bounds.min[axis] - reference.bounds.min[axis]).abs() < 1e-4);
                    assert!((got.bounds.max[axis] - reference.bounds.max[axis]).abs() < 1e-4);
                    assert!(
                        (got.moments.centroid[axis] - reference.moments.centroid[axis]).abs() < tol
                    );
                }
            }

            let start = Instant::now();
            let bounds = point_bounds_accelerated(points, mode).unwrap();
            let moments = point_moments_accelerated(points, mode).unwrap();
            separate_times[index].push(start.elapsed().as_secs_f64() * 1000.);
            if round == 0 {
                assert_eq!(bounds.samples, moments.samples);
            }
        }
    }
    let fused: Vec<String> = modes
        .iter()
        .zip(fused_times.iter_mut())
        .map(|(mode, times)| format!("{} {:.3}ms", mode.label(), median(times)))
        .collect();
    let separate: Vec<String> = modes
        .iter()
        .zip(separate_times.iter_mut())
        .map(|(mode, times)| format!("{} {:.3}ms", mode.label(), median(times)))
        .collect();
    println!(
        "point_cloud_stats ({} points): fused [{}], separate [{}], centroid={:?}",
        points.len(),
        fused.join(", "),
        separate.join(", "),
        reference.moments.centroid
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
