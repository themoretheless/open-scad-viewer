use math_core::{
    Acceleration, ID, V3, rotation, transform_points, transformed_squared_distance_pair_sum,
    transformed_squared_distance_pair_sum_accelerated,
};
use std::time::{Duration, Instant};

fn points(n: usize, offset: f64) -> Vec<V3> {
    (0..n)
        .map(|i| {
            let f = i as f64 + offset;
            [
                f.mul_add(0.013, -50.),
                (f * 0.007).sin() * 20.,
                (f * 0.011).cos() * 10.,
            ]
        })
        .collect()
}

fn bench(mut f: impl FnMut() -> f64) -> (Duration, f64) {
    let mut best = Duration::MAX;
    let mut value = 0.;
    for _ in 0..8 {
        let start = Instant::now();
        value = f();
        best = best.min(start.elapsed());
    }
    (best, value)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let m = rotation([0.02, -0.03, 0.015]);
    let t = [1.5, -2., 0.75];
    println!("pairs,cpu_ms,auto_ms,gpu_ms,cuda_ms,materialized_cpu_ms,value");
    for n in [1_000usize, 10_000, 100_000, 1_000_000, 5_000_000] {
        let source = points(n, 0.);
        let target = points(n, 3.);
        let (cpu, value) =
            bench(|| transformed_squared_distance_pair_sum(&source, &target, m, t).unwrap());
        let (auto, _) = bench(|| {
            transformed_squared_distance_pair_sum_accelerated(
                &source,
                &target,
                m,
                t,
                Acceleration::Auto,
            )
            .unwrap()
        });
        let (gpu, _) = bench(|| {
            transformed_squared_distance_pair_sum_accelerated(
                &source,
                &target,
                m,
                t,
                Acceleration::Gpu,
            )
            .unwrap()
        });
        let (cuda, _) = bench(|| {
            transformed_squared_distance_pair_sum_accelerated(
                &source,
                &target,
                m,
                t,
                Acceleration::Cuda,
            )
            .unwrap()
        });
        let (materialized, _) = bench(|| {
            let transformed = transform_points(&source, m, t);
            transformed_squared_distance_pair_sum(&transformed, &target, ID, [0.; 3]).unwrap()
        });
        println!(
            "{},{:.3},{:.3},{:.3},{:.3},{:.3},{}",
            n,
            cpu.as_secs_f64() * 1e3,
            auto.as_secs_f64() * 1e3,
            gpu.as_secs_f64() * 1e3,
            cuda.as_secs_f64() * 1e3,
            materialized.as_secs_f64() * 1e3,
            value
        );
    }
    Ok(())
}
