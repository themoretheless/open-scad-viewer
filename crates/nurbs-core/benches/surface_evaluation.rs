//! cargo bench --manifest-path crates/Cargo.toml -p nurbs-core --bench surface_evaluation
use nurbs_core::surface::{Evaluation, Surface, SurfaceEvaluator, SurfaceSampler};
use std::{hint::black_box, time::Instant};

const ROUNDS: usize = 7;

fn fixture() -> Surface {
    Surface {
        degree_u: 3,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 0., 0.17, 0.63, 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0.31, 0.78, 1., 1., 1.],
        control_points: (0..6)
            .map(|i| {
                (0..5)
                    .map(|j| {
                        let x = i as f64;
                        let y = j as f64;
                        vec![
                            1000. + x + 0.07 * y * y,
                            -500. + y + 0.03 * x * y,
                            20. + (0.7 * x).sin() + (0.4 * y).cos(),
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: (0..6)
            .map(|i| {
                (0..5)
                    .map(|j| 0.5 + ((i * 7 + j * 3) % 11) as f64 * 0.25)
                    .collect()
            })
            .collect(),
        periodic_u: false,
        periodic_v: false,
    }
}

fn parameters(scattered: bool) -> Vec<[f64; 2]> {
    let mut uv = Vec::with_capacity(4096);
    for j in 0..64 {
        for i in 0..64 {
            uv.push([(i as f64 + 0.37) / 64., (j as f64 + 0.61) / 64.]);
        }
    }
    if scattered {
        uv = (0..4096usize)
            .map(|i| {
                let reversed = (i as u16).reverse_bits() >> 4;
                uv[reversed as usize]
            })
            .collect();
    }
    uv
}

fn consume(evaluation: &Evaluation) -> f64 {
    let evaluation = black_box(evaluation);
    let mut sum = evaluation.point.iter().sum::<f64>();
    if let Some((du, dv)) = evaluation.first_derivatives() {
        sum += du.iter().chain(&dv).sum::<f64>();
    }
    if let Some((duu, duv, dvv)) = evaluation.second_derivatives() {
        sum += duu.iter().chain(&duv).chain(&dvv).sum::<f64>();
    }
    if let Some(normal) = evaluation.unit_normal() {
        sum += normal.iter().sum::<f64>();
    }
    if let Some((gaussian, mean)) = evaluation.curvatures() {
        sum += gaussian + mean;
    }
    sum
}

fn surface_batch(surface: &Surface, uv: &[[f64; 2]]) -> f64 {
    let mut checksum = 0.;
    for &[u, v] in black_box(uv) {
        let evaluation = black_box(surface)
            .evaluate(black_box(u), black_box(v))
            .unwrap();
        checksum += consume(black_box(&evaluation));
    }
    black_box(checksum)
}

fn sampler_batch(sampler: &SurfaceSampler, uv: &[[f64; 2]]) -> f64 {
    let mut checksum = 0.;
    for &[u, v] in black_box(uv) {
        let evaluation = black_box(sampler)
            .evaluate(black_box(u), black_box(v))
            .unwrap();
        checksum += consume(black_box(&evaluation));
    }
    black_box(checksum)
}

fn evaluator_batch(evaluator: &mut SurfaceEvaluator<'_>, uv: &[[f64; 2]]) -> f64 {
    let mut checksum = 0.;
    for &[u, v] in black_box(uv) {
        let evaluation = black_box(&mut *evaluator)
            .evaluate(black_box(u), black_box(v))
            .unwrap();
        checksum += consume(black_box(&evaluation));
    }
    black_box(checksum)
}

fn measure(mut batch: impl FnMut() -> f64) -> (f64, f64) {
    let checksum = batch();
    black_box(checksum);
    let mut timings = Vec::with_capacity(ROUNDS);
    for _ in 0..ROUNDS {
        let start = Instant::now();
        black_box(batch());
        timings.push(start.elapsed().as_secs_f64() * 1e9 / 4096.);
    }
    timings.sort_by(f64::total_cmp);
    (timings[ROUNDS / 2], checksum)
}

fn assert_same_checksum(reference: f64, actual: f64) {
    let tolerance = 1e-10 * reference.abs().max(1.);
    assert!(
        (reference - actual).abs() <= tolerance,
        "checksum mismatch: reference={reference}, actual={actual}"
    );
}

fn main() {
    let surface = fixture();
    surface.validate().unwrap();
    let sampler = SurfaceSampler::new(&surface).unwrap();

    for (label, scattered) in [("monotone", false), ("scattered", true)] {
        let uv = parameters(scattered);
        let mut evaluator = SurfaceEvaluator::new(&surface).unwrap();
        let reference = surface_batch(&surface, &uv);
        assert_same_checksum(reference, sampler_batch(&sampler, &uv));
        assert_same_checksum(reference, evaluator_batch(&mut evaluator, &uv));

        let (direct_ns, _) = measure(|| surface_batch(&surface, &uv));
        let (sampler_ns, _) = measure(|| sampler_batch(&sampler, &uv));
        let (evaluator_ns, _) = measure(|| evaluator_batch(&mut evaluator, &uv));
        println!(
            "order={label} evaluations=4096 direct_with_validate_ns={direct_ns:.1} sampler_steady_ns={sampler_ns:.1} evaluator_steady_ns={evaluator_ns:.1} evaluator_vs_sampler={:.2}x",
            sampler_ns / evaluator_ns
        );
    }
}
