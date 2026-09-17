//! Point-to-point ICP benchmark over the accelerated nearest-neighbor kernel.
//!
//! `cargo run --release -p osv-math --features cuda --example icp_registration`
//! (drop `--features cuda` for wgpu/Metal, or use
//! `ICP_ACCELERATION=cpu|auto|gpu|metal|cuda`).
use math_core::{Acceleration, IcpOptions, RigidTransform, V3, icp_register, rotation};
use std::time::Instant;

fn cloud(n: usize) -> Vec<V3> {
    let side = (n as f64).cbrt().ceil() as usize;
    (0..n)
        .map(|i| {
            let x = (i % side) as f64 * 0.01;
            let y = ((i / side) % side) as f64 * 0.01;
            let z = (i / (side * side)) as f64 * 0.01;
            [
                x + (y * 17.).sin() * 0.001,
                y + (z * 13.).cos() * 0.001,
                z + (x * 11.).sin() * 0.001,
            ]
        })
        .collect()
}

fn timed(source: &[V3], target: &[V3], acceleration: Acceleration) -> (f64, f64, usize) {
    let start = Instant::now();
    let report = icp_register(
        source,
        target,
        IcpOptions {
            max_iterations: 10,
            tolerance: 1e-14,
            max_correspondence_distance: Some(0.05),
            acceleration,
        },
    )
    .unwrap();
    (
        start.elapsed().as_secs_f64() * 1000.,
        report.mean_squared_error,
        report.iterations,
    )
}

fn main() {
    let acceleration = std::env::var("ICP_ACCELERATION")
        .ok()
        .and_then(|value| Acceleration::parse(&value))
        .unwrap_or(Acceleration::Auto);
    let known = RigidTransform {
        rotation: rotation([0.006, -0.004, 0.005]),
        translation: [0.002, -0.0015, 0.001],
    };
    println!("acceleration = {acceleration:?}");
    println!(
        "{:>9} {:>12} | {:>12} {:>12} | {:>8} {:>8}",
        "points", "work/iter", "cpu_ms", "auto_ms", "cpu_it", "auto_it"
    );
    for &n in &[1_000usize, 5_000, 20_000] {
        let source = cloud(n);
        let target: Vec<_> = source.iter().map(|&p| known.apply(p)).collect();
        let _ = timed(&source, &target, acceleration);
        let (cpu_ms, cpu_mse, cpu_it) = timed(&source, &target, Acceleration::Cpu);
        let (auto_ms, auto_mse, auto_it) = timed(&source, &target, acceleration);
        assert!(cpu_mse < 1e-8, "cpu mse {cpu_mse}");
        assert!(auto_mse < 1e-8, "accelerated mse {auto_mse}");
        println!(
            "{:>9} {:>12} | {:>12.3} {:>12.3} | {:>8} {:>8}",
            n,
            n * n,
            cpu_ms,
            auto_ms,
            cpu_it,
            auto_it
        );
    }
}
