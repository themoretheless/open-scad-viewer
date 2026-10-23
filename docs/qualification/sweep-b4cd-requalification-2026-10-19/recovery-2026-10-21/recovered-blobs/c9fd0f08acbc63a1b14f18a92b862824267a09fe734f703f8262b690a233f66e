use math_core::{Acceleration, V3, nearest_two_accelerated};
use rbench::{DropPolicy, Suite};
use std::sync::Arc;

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

fn add_case(
    suite: &mut Suite,
    query_count: usize,
    target_count: usize,
    acceleration: Acceleration,
) {
    let queries = points(query_count, 0.25);
    let targets = points(target_count, -0.75);
    let reference = nearest_two_accelerated(&queries, &targets, Acceleration::Cpu);
    let queries = Arc::new(queries);
    let targets = Arc::new(targets);
    suite
        .bench_with_input(
            Box::leak(
                format!(
                    "nearest_two/{query_count}x{target_count}/{}",
                    acceleration.label()
                )
                .into_boxed_str(),
            ),
            move || (Arc::clone(&queries), Arc::clone(&targets)),
            move |(queries, targets)| {
                let values = nearest_two_accelerated(queries, targets, acceleration);
                assert_eq!(
                    values
                        .iter()
                        .map(|pair| [pair[0].0, pair[1].0])
                        .collect::<Vec<_>>(),
                    reference
                        .iter()
                        .map(|pair| [pair[0].0, pair[1].0])
                        .collect::<Vec<_>>()
                );
                values
                    .iter()
                    .take(1024)
                    .map(|pair| pair[0].0 as u64 + pair[1].0 as u64)
                    .sum::<u64>()
            },
            DropPolicy::InsideTiming,
        )
        .parameter("queries", query_count as f64)
        .parameter("targets", target_count as f64);
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
    let mut suite = Suite::new("math-core/nearest-two");
    for (query_count, target_count) in [(256, 512), (1_024, 1_024), (4_096, 4_096), (16_384, 8_192)]
    {
        for acceleration in [
            Acceleration::Cpu,
            Acceleration::Auto,
            Acceleration::Gpu,
            Acceleration::Cuda,
        ] {
            add_case(&mut suite, query_count, target_count, acceleration);
        }
    }
    suite.main()
}
