//! Native load assembly cost, including its rank and resultant checks.
use mechanics_core::truss_loads::{Wrench, distribute};
use std::{hint::black_box, time::Instant};
use value_codec::json;

fn main() {
    let load = Wrench {
        origin_mm: [0.; 3],
        force_n: [100., -200., 300.],
        moment_n_mm: [0., 2000., 3000.],
    };
    let mut rows = Vec::new();
    for nodes in [2, 4, 25, 125] {
        let points: Vec<[f64; 3]> = (0..nodes)
            .map(|i| [(i % 5) as f64, ((i / 5) % 5) as f64, (i / 25) as f64])
            .collect();
        let forces = distribute(&points, &load).unwrap();
        let mut actual_force = [0.; 3];
        let mut actual_moment = [0.; 3];
        for (p, f) in points.iter().zip(&forces) {
            for i in 0..3 {
                actual_force[i] += f[i];
            }
            actual_moment[0] += p[1] * f[2] - p[2] * f[1];
            actual_moment[1] += p[2] * f[0] - p[0] * f[2];
            actual_moment[2] += p[0] * f[1] - p[1] * f[0];
        }
        for i in 0..3 {
            assert!((actual_force[i] - load.force_n[i]).abs() < 1e-8);
            assert!((actual_moment[i] - load.moment_n_mm[i]).abs() < 1e-8);
        }
        for _ in 0..20 {
            black_box(distribute(black_box(&points), black_box(&load)).unwrap());
        }
        let mut samples = Vec::new();
        for _ in 0..31 {
            let start = Instant::now();
            for _ in 0..100 {
                black_box(distribute(black_box(&points), black_box(&load)).unwrap());
            }
            samples.push(start.elapsed().as_secs_f64() * 1000. / 100.);
        }
        let mut ordered = samples.clone();
        ordered.sort_by(f64::total_cmp);
        rows.push(
            json!({"nodes":nodes,"p50Ms":ordered[15],"p95Ms":ordered[29],"samplesMs":samples}),
        );
    }
    println!(
        "{}",
        json!({"os":std::env::consts::OS,"arch":std::env::consts::ARCH,
        "scope":"native load assembly, rank admission and resultant validation; no structural solve or WASM",
        "warmups":20,"samples":31,"batch":100,"results":rows})
    );
}
