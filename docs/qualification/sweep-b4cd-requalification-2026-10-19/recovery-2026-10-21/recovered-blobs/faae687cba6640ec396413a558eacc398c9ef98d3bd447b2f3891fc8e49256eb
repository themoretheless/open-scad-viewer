use math_core::{
    Acceleration, V3, squared_distance_pair_sum_accelerated, squared_distance_pairs_accelerated,
};
use std::time::Instant;

fn points(count: usize, phase: f64) -> Vec<V3> {
    (0..count)
        .map(|i| {
            let f = i as f64;
            [
                f * 0.00031 + phase,
                (f * 0.013 + phase).sin() * 10.,
                (f * 0.017 - phase).cos() * 7.,
            ]
        })
        .collect()
}

fn median_ms(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn run_one(count: usize, acceleration: Acceleration) -> (f64, f64) {
    let a = points(count, 0.25);
    let b = points(count, -0.5);
    let mut times = Vec::new();
    let mut checksum = 0.;
    for _ in 0..5 {
        let start = Instant::now();
        let values = squared_distance_pairs_accelerated(&a, &b, acceleration).unwrap();
        times.push(start.elapsed().as_secs_f64() * 1000.);
        checksum = values.iter().take(1024).sum();
    }
    (median_ms(times), checksum)
}

fn run_sum(count: usize, acceleration: Acceleration) -> (f64, f64) {
    let a = points(count, 0.25);
    let b = points(count, -0.5);
    let mut times = Vec::new();
    let mut sum = 0.;
    for _ in 0..5 {
        let start = Instant::now();
        sum = squared_distance_pair_sum_accelerated(&a, &b, acceleration).unwrap();
        times.push(start.elapsed().as_secs_f64() * 1000.);
    }
    (median_ms(times), sum)
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
    println!("pair_count,cpu_ms,auto_ms,gpu_ms,cuda_ms,auto_speedup");
    for count in [10_000, 100_000, 250_000, 1_000_000, 3_000_000] {
        let (cpu, cpu_sum) = run_one(count, Acceleration::Cpu);
        let (auto, auto_sum) = run_one(count, Acceleration::Auto);
        let (gpu, gpu_sum) = run_one(count, Acceleration::Gpu);
        let (cuda, cuda_sum) = run_one(count, Acceleration::Cuda);
        assert!((auto_sum - cpu_sum).abs() < 1e-2 * cpu_sum.abs().max(1.0));
        assert!((gpu_sum - cpu_sum).abs() < 1e-2 * cpu_sum.abs().max(1.0));
        assert!((cuda_sum - cpu_sum).abs() < 1e-2 * cpu_sum.abs().max(1.0));
        println!(
            "{count},{cpu:.3},{auto:.3},{gpu:.3},{cuda:.3},{:.2}x",
            cpu / auto.max(1e-9)
        );
    }
    println!("pair_count,cpu_sum_ms,auto_sum_ms,gpu_sum_ms,cuda_sum_ms,auto_sum_speedup");
    for count in [10_000, 100_000, 250_000, 1_000_000, 3_000_000] {
        let (cpu, cpu_sum) = run_sum(count, Acceleration::Cpu);
        let (auto, auto_sum) = run_sum(count, Acceleration::Auto);
        let (gpu, gpu_sum) = run_sum(count, Acceleration::Gpu);
        let (cuda, cuda_sum) = run_sum(count, Acceleration::Cuda);
        assert!((auto_sum - cpu_sum).abs() < 1e-2 * cpu_sum.abs().max(1.0));
        assert!((gpu_sum - cpu_sum).abs() < 1e-2 * cpu_sum.abs().max(1.0));
        assert!((cuda_sum - cpu_sum).abs() < 1e-2 * cpu_sum.abs().max(1.0));
        println!(
            "{count},{cpu:.3},{auto:.3},{gpu:.3},{cuda:.3},{:.2}x",
            cpu / auto.max(1e-9)
        );
    }
}
