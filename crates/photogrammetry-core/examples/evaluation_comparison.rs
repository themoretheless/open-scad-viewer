//! Exact spatial-index metrics versus exhaustive distances on identical samples.
use photogrammetry_core::evaluation::{EvaluationOptions, evaluate_clouds};
use std::time::Instant;

fn exhaustive(a: &[[f64; 3]], b: &[[f64; 3]], threshold: f64) -> (f64, f64) {
    let mut sum = 0.;
    let mut within = 0;
    for p in a {
        let minimum = b
            .iter()
            .map(|q| (p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2))
            .fold(f64::INFINITY, f64::min)
            .sqrt();
        sum += minimum;
        within += usize::from(minimum <= threshold);
    }
    (sum / a.len() as f64, within as f64 / a.len() as f64)
}

fn main() {
    let reference = (0..8000)
        .map(|i| {
            let x = (i % 100) as f64 * 0.01;
            let y = (i / 100) as f64 * 0.01;
            [x, y, (x * 8.).sin() * 0.07 + (y * 11.).cos() * 0.05]
        })
        .collect::<Vec<_>>();
    let model = reference
        .iter()
        .enumerate()
        .filter(|(i, _)| i % 17 != 0)
        .map(|(i, p)| [p[0], p[1], p[2] + (i as f64 * 0.91).sin() * 0.005])
        .collect::<Vec<_>>();
    let options = EvaluationOptions {
        tolerance: 0.004,
        voxel_size: None,
    };
    for run in 0..4 {
        // Alternate execution order; both paths include both distance directions.
        let fast = || {
            let start = Instant::now();
            let result = evaluate_clouds(&model, &reference, &options, |_, _, _| true).unwrap();
            (result, start.elapsed().as_secs_f64() * 1000.)
        };
        let slow = || {
            let start = Instant::now();
            let a = exhaustive(&model, &reference, options.tolerance);
            let b = exhaustive(&reference, &model, options.tolerance);
            (a, b, start.elapsed().as_secs_f64() * 1000.)
        };
        let ((r, fast_ms), (a, b, slow_ms)) = if run % 2 == 0 {
            (fast(), slow())
        } else {
            let s = slow();
            (fast(), s)
        };
        assert!((r.reconstructed_to_reference.mean - a.0).abs() < 1e-12);
        assert!((r.reference_to_reconstructed.mean - b.0).abs() < 1e-12);
        assert_eq!(r.precision, a.1);
        assert_eq!(r.recall, b.1);
        println!(
            "{{\"run\":{},\"warmup\":{},\"model_samples\":{},\"reference_samples\":{},\"indexed_ms\":{},\"exhaustive_ms\":{},\"accuracy_mean\":{},\"completeness_mean\":{},\"precision\":{},\"recall\":{},\"f1\":{},\"matches_exhaustive\":true}}",
            run,
            run == 0,
            model.len(),
            reference.len(),
            fast_ms,
            slow_ms,
            a.0,
            b.0,
            r.precision,
            r.recall,
            r.f1
        );
    }
}
