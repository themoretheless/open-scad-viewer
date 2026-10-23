//! Guards the qualified `DenseOptions::accurate()` bundle against the defaults
//! on the analytic scenes, and checks the batched GPU sweep reproduces the CPU
//! surface when an adapter is present.
#[allow(unused_imports)]
use math_core::*;
#[allow(unused_imports)]
use photogrammetry_core::{
    Acceleration, Image, Point, Reconstruction,
    camera::Camera,
    dense::{DenseOptions, densify_with_options},
};
#[path = "../examples/support/dense_fixture.rs"]
mod fixture;
use fixture::*;

const TOLERANCE: f64 = 0.04;
const SCENES: [Scene; 5] = [
    Scene {
        angle: 0.,
        thin: false,
    },
    Scene {
        angle: 15.,
        thin: false,
    },
    Scene {
        angle: 30.,
        thin: false,
    },
    Scene {
        angle: 60.,
        thin: false,
    },
    Scene {
        angle: 30.,
        thin: true,
    },
];

struct Metrics {
    mean_error: f64,
    f1: f64,
}

fn evaluate(options: &DenseOptions) -> Metrics {
    let mut error_sum = 0.;
    let mut f1_sum = 0.;
    for scene in SCENES {
        let (images, sparse, ground) = fixture(scene);
        let options = DenseOptions {
            max_side: SIDE,
            ..options.clone()
        };
        let result = densify_with_options(&images, &sparse, &options, |_, _, _| true).unwrap();
        let points = &result.surface.positions;
        assert!(
            !points.is_empty(),
            "scene {}° produced no surface",
            scene.angle
        );
        let errors: Vec<_> = points.iter().map(|&p| scene.distance(p)).collect();
        let precision =
            errors.iter().filter(|&&d| d <= TOLERANCE).count() as f64 / errors.len() as f64;
        let covered = ground
            .iter()
            .filter(|&&(p, _)| points.iter().any(|&q| norm(sub(p, q)) <= TOLERANCE))
            .count();
        let recall = covered as f64 / ground.len() as f64;
        f1_sum += 2. * precision * recall / (precision + recall).max(1e-12);
        error_sum += errors.iter().sum::<f64>() / errors.len() as f64;
    }
    let n = SCENES.len() as f64;
    Metrics {
        mean_error: error_sum / n,
        f1: f1_sum / n,
    }
}

#[test]
fn accurate_preset_beats_defaults_on_analytic_scenes() {
    let baseline = evaluate(&DenseOptions::default());
    let accurate = evaluate(&DenseOptions::accurate());
    // Qualified values: 0.01246 -> 0.00907 mean surface error (-27%) and
    // 0.892 -> 0.921 mean F1 (+3.3%). The bounds leave slack but reject any
    // regression back towards the defaults.
    assert!(
        accurate.mean_error < baseline.mean_error * 0.85,
        "mean surface error {:.6} (accurate) vs {:.6} (default)",
        accurate.mean_error,
        baseline.mean_error
    );
    assert!(
        accurate.f1 > baseline.f1 + 0.02,
        "mean F1 {:.6} (accurate) vs {:.6} (default)",
        accurate.f1,
        baseline.f1
    );
}

#[cfg(feature = "gpu")]
#[test]
fn gpu_accurate_preset_matches_cpu_surface() {
    if photogrammetry_core::gpu::sweep::shared().is_none() {
        eprintln!("skipping: no GPU adapter");
        return;
    }
    for scene in SCENES {
        let (images, sparse, _) = fixture(scene);
        let cpu = DenseOptions {
            max_side: SIDE,
            ..DenseOptions::accurate()
        };
        let gpu = DenseOptions {
            acceleration: Acceleration::Gpu,
            ..cpu.clone()
        };
        let a = densify_with_options(&images, &sparse, &cpu, |_, _, _| true).unwrap();
        let b = densify_with_options(&images, &sparse, &gpu, |_, _, _| true).unwrap();
        // GPU scores are f32, so a handful of pixels near the correlation
        // threshold may flip; the reconstructed surface must stay the same.
        let (na, nb) = (a.surface.positions.len(), b.surface.positions.len());
        let slack = (na.max(nb) as f64 * 0.02).max(8.);
        assert!(
            (na as f64 - nb as f64).abs() <= slack,
            "scene {}°: {na} CPU vs {nb} GPU vertices",
            scene.angle
        );
        let far = b
            .surface
            .positions
            .iter()
            .filter(|&&p| {
                a.surface
                    .positions
                    .iter()
                    .all(|&q| norm(sub(p, q)) > TOLERANCE)
            })
            .count();
        assert!(
            far as f64 <= slack,
            "scene {}°: {far} GPU vertices far from any CPU vertex",
            scene.angle
        );
    }
}
