use photogrammetry_core::Acceleration;
use photogrammetry_core::features::{
    Feature, FeatureOptions, matches_with_options, recommended_for_descriptor_matching,
};
use rbench::{DropPolicy, Suite};
use std::sync::Arc;

fn descriptor(seed: &mut u64) -> [f32; 128] {
    let mut raw = [0f32; 128];
    for value in &mut raw {
        *seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *value = ((*seed >> 33) as f32 / u32::MAX as f32).fract();
    }
    let norm = raw.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-12);
    raw.map(|v| v / norm)
}

fn features(count: usize, seed: u64) -> Vec<Feature> {
    let mut state = seed;
    (0..count)
        .map(|i| Feature {
            x: (i % 4096) as f64,
            y: (i / 4096) as f64,
            descriptor: descriptor(&mut state),
        })
        .collect()
}

fn options(acceleration: Acceleration) -> FeatureOptions {
    FeatureOptions {
        acceleration,
        ..FeatureOptions::ROOT
    }
}

fn add_case(suite: &mut Suite, a_count: usize, b_count: usize, acceleration: Acceleration) {
    let a = features(a_count, 0x5317_91ab);
    let b = features(b_count, 0x89ab_13df);
    let reference = matches_with_options(&a, &b, &options(Acceleration::Cpu));
    let options = options(acceleration);
    let a = Arc::new(a);
    let b = Arc::new(b);
    suite
        .bench_with_input(
            Box::leak(
                format!("matching/{a_count}x{b_count}/{}", acceleration.label()).into_boxed_str(),
            ),
            move || (Arc::clone(&a), Arc::clone(&b)),
            move |(a, b)| {
                let matches = matches_with_options(a, b, &options);
                assert_eq!(
                    matches.iter().map(|m| (m.a, m.b)).collect::<Vec<_>>(),
                    reference.iter().map(|m| (m.a, m.b)).collect::<Vec<_>>()
                );
                matches
                    .iter()
                    .take(1024)
                    .map(|m| (m.a as u64) ^ ((m.b as u64) << 32))
                    .sum::<u64>()
            },
            DropPolicy::InsideTiming,
        )
        .parameter("features_a", a_count as f64)
        .parameter("features_b", b_count as f64);
}

fn main() -> rbench::Result<()> {
    #[cfg(feature = "gpu")]
    println!(
        "wgpu backend: {}",
        photogrammetry_core::gpu::backend_label().unwrap_or("none (falls back to cpu)")
    );
    #[cfg(feature = "cuda")]
    println!(
        "cuda device: {}",
        photogrammetry_core::gpu::cuda_device_name()
            .unwrap_or_else(|| "none (falls back to wgpu/cpu)".into())
    );
    let mut suite = Suite::new("photogrammetry/matching");
    for (a_count, b_count) in [
        (64, 64),
        (128, 128),
        (256, 512),
        (512, 512),
        (1_024, 1_024),
        (2_048, 2_048),
    ] {
        for acceleration in [
            Acceleration::Cpu,
            Acceleration::Auto,
            Acceleration::Gpu,
            Acceleration::Cuda,
        ] {
            add_case(&mut suite, a_count, b_count, acceleration);
        }
        let _ = recommended_for_descriptor_matching(a_count, b_count);
    }
    suite.main()
}
