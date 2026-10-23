use math_core::{Acceleration, V3, point_fit_plane};
use std::time::{Duration, Instant};

fn points(n: usize) -> Vec<V3> {
    let side = (n as f64).sqrt().ceil() as usize;
    (0..n)
        .map(|i| {
            let x = (i % side) as f64 * 0.01 - 5.;
            let y = (i / side) as f64 * 0.01 - 5.;
            let z = 2. + 0.25 * x - 0.5 * y + (x * 11.).sin() * 0.001;
            [x, y, z]
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
    println!("points,cpu_ms,auto_ms,gpu_ms,cuda_ms,rms_distance");
    for n in [10_000usize, 100_000, 1_000_000, 5_000_000] {
        let points = points(n);
        let (cpu, rms) = bench(|| {
            point_fit_plane(&points, Acceleration::Cpu)
                .unwrap()
                .rms_distance
        });
        let (auto, _) = bench(|| {
            point_fit_plane(&points, Acceleration::Auto)
                .unwrap()
                .rms_distance
        });
        let (gpu, _) = bench(|| {
            point_fit_plane(&points, Acceleration::Gpu)
                .unwrap()
                .rms_distance
        });
        let (cuda, _) = bench(|| {
            point_fit_plane(&points, Acceleration::Cuda)
                .unwrap()
                .rms_distance
        });
        println!(
            "{},{:.3},{:.3},{:.3},{:.3},{}",
            n,
            cpu.as_secs_f64() * 1e3,
            auto.as_secs_f64() * 1e3,
            gpu.as_secs_f64() * 1e3,
            cuda.as_secs_f64() * 1e3,
            rms
        );
    }
    Ok(())
}
