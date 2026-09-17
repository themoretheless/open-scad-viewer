use math_core::{Acceleration, V3, nearest_four_accelerated};
use std::time::Instant;

fn points(count: usize, phase: f64) -> Vec<V3> {
    (0..count)
        .map(|i| {
            let f = i as f64;
            [
                f * 0.00037 + phase,
                (f * 0.011 + phase).sin() * 12.,
                (f * 0.019 - phase).cos() * 8.,
            ]
        })
        .collect()
}

fn median_ms(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn run_one(query_count: usize, target_count: usize, acceleration: Acceleration) -> (f64, u64) {
    let queries = points(query_count, 0.25);
    let targets = points(target_count, -0.75);
    let mut times = Vec::new();
    let mut checksum = 0u64;
    for _ in 0..5 {
        let start = Instant::now();
        let values = nearest_four_accelerated(&queries, &targets, acceleration);
        times.push(start.elapsed().as_secs_f64() * 1000.);
        checksum = values
            .iter()
            .take(1024)
            .map(|neighbors| neighbors.iter().map(|n| n.0 as u64).sum::<u64>())
            .sum();
    }
    (median_ms(times), checksum)
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
    println!("queries,targets,work,cpu_ms,auto_ms,gpu_ms,cuda_ms,auto_speedup");
    for (query_count, target_count) in [(256, 512), (1_024, 1_024), (4_096, 4_096), (16_384, 8_192)]
    {
        let (cpu, cpu_sum) = run_one(query_count, target_count, Acceleration::Cpu);
        let (auto, auto_sum) = run_one(query_count, target_count, Acceleration::Auto);
        let (gpu, gpu_sum) = run_one(query_count, target_count, Acceleration::Gpu);
        let (cuda, cuda_sum) = run_one(query_count, target_count, Acceleration::Cuda);
        assert_eq!(auto_sum, cpu_sum);
        assert_eq!(gpu_sum, cpu_sum);
        assert_eq!(cuda_sum, cpu_sum);
        println!(
            "{query_count},{target_count},{},{cpu:.3},{auto:.3},{gpu:.3},{cuda:.3},{:.2}x",
            query_count * target_count,
            cpu / auto.max(1e-9)
        );
    }
}
