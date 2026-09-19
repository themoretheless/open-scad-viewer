//! Symmetric Chamfer distance benchmark.
//! `cargo run --release -p osv-math --features cuda --example bench_chamfer`.
use math_core::{Acceleration, V3, chamfer_distance};
use rbench::{DropPolicy, Suite};

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

fn add_cases(suite: &mut Suite, a: &[V3], b: &[V3]) {
    let modes = modes();
    let reference = chamfer_distance(a, b, Acceleration::Cpu).unwrap();
    for mode in modes {
        let input_a = a.to_vec();
        let input_b = b.to_vec();
        suite
            .bench_with_input(
                Box::leak(
                    format!("chamfer/{}x{}/{}", a.len(), b.len(), mode.label()).into_boxed_str(),
                ),
                move || (input_a.clone(), input_b.clone()),
                move |(a, b)| {
                    let got = chamfer_distance(a, b, mode).unwrap();
                    let tol = if mode == Acceleration::Cpu {
                        1e-12
                    } else {
                        5e-3 * reference.symmetric_mean_squared_distance.max(1.0)
                    };
                    assert!(
                        (got.symmetric_mean_squared_distance
                            - reference.symmetric_mean_squared_distance)
                            .abs()
                            < tol
                    );
                    got.symmetric_rms_distance
                },
                DropPolicy::InsideTiming,
            )
            .parameter("a", a.len() as f64)
            .parameter("b", b.len() as f64);
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
    let mut suite = Suite::new("math-core/chamfer");
    for &(a, b) in &[
        (1_000usize, 1_000usize),
        (5_000, 2_000),
        (20_000, 5_000),
        (100_000, 10_000),
    ] {
        let a = points(a, 0.);
        let b = points(b, 1_000_000.);
        add_cases(&mut suite, &a, &b);
    }
    suite.main()
}
