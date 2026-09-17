//! GPU/CUDA vs CPU batch-transform benchmark (may be removed after qualification).
//! `cargo run --release -p osv-math --features cuda --example bench_gpu`.
use math_core::{Acceleration, V3, rotation, transform_points, transform_points_accelerated};
use std::time::Instant;

/// Placements to compare: CPU reference, wgpu shader and (feature `cuda`) the
/// CUDA driver port. `TRANSFORM_BENCH_MODES=cpu,cuda` narrows the set.
fn modes() -> Vec<Acceleration> {
    let mut modes = vec![Acceleration::Cpu, Acceleration::Gpu];
    if cfg!(feature = "cuda") {
        modes.push(Acceleration::Cuda);
    }
    if let Ok(filter) = std::env::var("TRANSFORM_BENCH_MODES") {
        let wanted: Vec<Acceleration> = filter.split(',').filter_map(Acceleration::parse).collect();
        modes.retain(|mode| wanted.contains(mode));
    }
    modes
}

fn median(times: &mut [f64]) -> f64 {
    times.sort_by(f64::total_cmp);
    times[times.len() / 2]
}

fn compare(label: &str, points: &[V3], rounds: usize) {
    let m = rotation([0.3, -0.2, 0.7]);
    let t = [1., -2., 0.5];
    let modes = modes();
    let reference = transform_points(points, m, t);
    let mut times = vec![Vec::new(); modes.len()];
    for round in 0..rounds {
        for (index, &acceleration) in modes.iter().enumerate() {
            let start = Instant::now();
            let got = transform_points_accelerated(points, m, t, acceleration);
            times[index].push(start.elapsed().as_secs_f64() * 1000.);
            if round == 0 {
                for (a, b) in reference.iter().zip(&got) {
                    for k in 0..3 {
                        let tol = if acceleration == Acceleration::Cpu {
                            1e-12
                        } else {
                            5e-3
                        };
                        assert!(
                            (a[k] - b[k]).abs() < tol,
                            "{acceleration:?} mismatch: {a:?} vs {b:?}"
                        );
                    }
                }
            }
        }
    }
    let summary: Vec<String> = modes
        .iter()
        .zip(times.iter_mut())
        .map(|(mode, times)| format!("{} median {:.3}ms", mode.label(), median(times)))
        .collect();
    println!("{label} ({} points): {}", points.len(), summary.join(", "));
}

fn main() {
    #[cfg(feature = "cuda")]
    println!(
        "cuda device: {}",
        math_core::cuda::device_name().unwrap_or_else(|| "none (falls back to wgpu/cpu)".into())
    );
    for &n in &[1_000usize, 100_000, 1_000_000, 5_000_000] {
        let points: Vec<V3> = (0..n)
            .map(|i| {
                let f = i as f64;
                [
                    f * 0.001 - 500.,
                    (f * 0.0007).sin() * 200.,
                    (f * 0.0003).cos() * 200.,
                ]
            })
            .collect();
        compare("transform_points", &points, 5);
    }
}
