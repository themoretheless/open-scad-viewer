//! Point-cloud AABB reduction benchmark.
//! `cargo run --release -p osv-math --features cuda --example bench_point_bounds`.
use math_core::{Acceleration, V3, point_bounds_accelerated};
use rbench::{DropPolicy, Suite};
use std::sync::Arc;

fn modes() -> Vec<Acceleration> {
    let mut modes = vec![Acceleration::Cpu, Acceleration::Auto, Acceleration::Gpu];
    if cfg!(feature = "cuda") {
        modes.push(Acceleration::Cuda);
    }
    if let Ok(filter) = std::env::var("POINT_BOUNDS_BENCH_MODES") {
        let wanted: Vec<Acceleration> = filter.split(',').filter_map(Acceleration::parse).collect();
        modes.retain(|mode| wanted.contains(mode));
    }
    modes
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

fn add_cases(suite: &mut Suite, points: &[V3], n: usize) {
    let modes = modes();
    let reference = point_bounds_accelerated(points, Acceleration::Cpu).unwrap();
    for mode in modes {
        let input = Arc::new(points.to_vec());
        suite
            .bench_with_input(
                Box::leak(format!("point_bounds/{n}/{}", mode.label()).into_boxed_str()),
                move || Arc::clone(&input),
                move |points| {
                    let got = point_bounds_accelerated(points, mode).unwrap();
                    let tol = if mode.is_gpu() && mode != Acceleration::Auto {
                        1e-4
                    } else {
                        1e-12
                    };
                    for axis in 0..3 {
                        assert!((got.min[axis] - reference.min[axis]).abs() < tol);
                        assert!((got.max[axis] - reference.max[axis]).abs() < tol);
                    }
                    got.extent
                },
                DropPolicy::InsideTiming,
            )
            .parameter("points", n as f64);
    }
}

fn main() -> rbench::Result<()> {
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
    let mut suite = Suite::new("math-core/point-bounds");
    for &n in &[10_000usize, 100_000, 1_000_000, 5_000_000] {
        let data = points(n);
        add_cases(&mut suite, &data, n);
    }
    suite.main()
}
