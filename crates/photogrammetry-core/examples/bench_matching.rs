use photogrammetry_core::Acceleration;
use photogrammetry_core::features::{
    Feature, FeatureOptions, matches_with_options, recommended_for_descriptor_matching,
};
use std::time::Instant;

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

fn median_ms(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn run_one(a_count: usize, b_count: usize, acceleration: Acceleration) -> (f64, usize, u64) {
    let a = features(a_count, 0x5317_91ab);
    let b = features(b_count, 0x89ab_13df);
    let options = options(acceleration);
    let mut times = Vec::new();
    let mut len = 0;
    let mut checksum = 0u64;
    for _ in 0..5 {
        let start = Instant::now();
        let matches = matches_with_options(&a, &b, &options);
        times.push(start.elapsed().as_secs_f64() * 1000.);
        len = matches.len();
        checksum = matches
            .iter()
            .take(1024)
            .map(|m| (m.a as u64) ^ ((m.b as u64) << 32))
            .sum();
    }
    (median_ms(times), len, checksum)
}

fn main() {
    #[cfg(feature = "gpu")]
    println!(
        "wgpu backend: {}",
        photogrammetry_core::gpu::backend_label().unwrap_or("none (falls back to cpu)")
    );
    #[cfg(feature = "gpu")]
    match photogrammetry_core::gpu::subgroup_report() {
        Some(report) => println!(
            "wgpu subgroups: {}, size_range={}..{}",
            report.supported, report.min_size, report.max_size
        ),
        None => println!("wgpu subgroups: false, size_range=0..0"),
    }
    #[cfg(feature = "cuda")]
    println!(
        "cuda device: {}",
        photogrammetry_core::gpu::cuda_device_name()
            .unwrap_or_else(|| "none (falls back to wgpu/cpu)".into())
    );
    println!("features_a,features_b,work,recommended,cpu_ms,auto_ms,gpu_ms,cuda_ms,auto_speedup");
    for (a_count, b_count) in [
        (64, 64),
        (128, 128),
        (256, 512),
        (512, 512),
        (1_024, 1_024),
        (2_048, 2_048),
    ] {
        let (cpu, cpu_len, cpu_sum) = run_one(a_count, b_count, Acceleration::Cpu);
        let (auto, auto_len, auto_sum) = run_one(a_count, b_count, Acceleration::Auto);
        let (gpu, gpu_len, gpu_sum) = run_one(a_count, b_count, Acceleration::Gpu);
        let (cuda, cuda_len, cuda_sum) = run_one(a_count, b_count, Acceleration::Cuda);
        assert_eq!(auto_len, cpu_len);
        assert_eq!(gpu_len, cpu_len);
        assert_eq!(cuda_len, cpu_len);
        assert_eq!(auto_sum, cpu_sum);
        assert_eq!(gpu_sum, cpu_sum);
        assert_eq!(cuda_sum, cpu_sum);
        println!(
            "{a_count},{b_count},{},{},{cpu:.3},{auto:.3},{gpu:.3},{cuda:.3},{:.2}x",
            a_count * b_count,
            recommended_for_descriptor_matching(a_count, b_count).label(),
            cpu / auto.max(1e-9)
        );
    }
}
