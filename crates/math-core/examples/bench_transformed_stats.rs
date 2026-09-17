use math_core::{
    Acceleration, V3, point_cloud_stats, rotation, transform_points, transformed_point_cloud_stats,
    transformed_point_cloud_stats_accelerated,
};
use std::time::{Duration, Instant};

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

fn bench<T>(mut f: impl FnMut() -> T) -> (Duration, T) {
    let mut best = Duration::MAX;
    let mut value = None;
    for _ in 0..8 {
        let start = Instant::now();
        let got = f();
        best = best.min(start.elapsed());
        value = Some(got);
    }
    (best, value.unwrap())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let m = rotation([0.2, -0.1, 0.3]);
    let t = [1., -0.5, 0.25];
    println!("points,cpu_ms,auto_ms,gpu_ms,cuda_ms,materialized_cpu_ms,centroid_x");
    for n in [10_000usize, 100_000, 1_000_000, 5_000_000] {
        let points = points(n);
        let (cpu, stats) = bench(|| transformed_point_cloud_stats(&points, m, t).unwrap());
        let (auto, _) = bench(|| {
            transformed_point_cloud_stats_accelerated(&points, m, t, Acceleration::Auto).unwrap()
        });
        let (gpu, _) = bench(|| {
            transformed_point_cloud_stats_accelerated(&points, m, t, Acceleration::Gpu).unwrap()
        });
        let (cuda, _) = bench(|| {
            transformed_point_cloud_stats_accelerated(&points, m, t, Acceleration::Cuda).unwrap()
        });
        let (materialized, _) = bench(|| {
            let transformed = transform_points(&points, m, t);
            point_cloud_stats(&transformed).unwrap()
        });
        println!(
            "{},{:.3},{:.3},{:.3},{:.3},{:.3},{}",
            n,
            cpu.as_secs_f64() * 1e3,
            auto.as_secs_f64() * 1e3,
            gpu.as_secs_f64() * 1e3,
            cuda.as_secs_f64() * 1e3,
            materialized.as_secs_f64() * 1e3,
            stats.moments.centroid[0]
        );
    }
    Ok(())
}
