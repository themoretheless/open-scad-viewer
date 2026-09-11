//! Deterministic analytic scenes: identical images, calibrated cameras and sparse
//! range anchors for both dense estimators. Emits machine-readable JSON to stdout.
#[allow(unused_imports)]
use math_core::*;
use photogrammetry_core::{
    Image, Point, Reconstruction,
    camera::Camera,
    dense::{DenseEstimator, DenseOptions, densify_with_options},
};
use std::time::Instant;
#[path = "support/dense_fixture.rs"]
mod fixture;
use fixture::*;

fn save_xyz(path: &std::path::Path, points: impl IntoIterator<Item = V3>) {
    use std::io::Write;
    let mut file = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    for p in points {
        writeln!(file, "{:.9} {:.9} {:.9}", p[0], p[1], p[2]).unwrap();
    }
}
fn main() {
    let radius: usize = std::env::var("DENSE_PATCH_RADIUS")
        .ok()
        .map(|s| s.parse().unwrap())
        .unwrap_or(2);
    let repeats: usize = std::env::var("DENSE_REPEAT")
        .ok()
        .map(|s| s.parse().unwrap())
        .unwrap_or(3);
    assert!(repeats > 0);
    let export = std::env::var_os("DENSE_FIXTURES").map(std::path::PathBuf::from);
    if let Some(path) = &export {
        std::fs::create_dir_all(path).unwrap();
    }
    println!(
        "{{\"schema\":1,\"scene_units\":\"arbitrary; rear plane center z=4\",\"tolerance\":0.04,\"depth_hypotheses\":64,\"patch_radius\":{radius},\"patch_samples\":{},\"image_side\":{SIDE},\"repeats\":{repeats},\"order\":\"alternating per repetition, one warmup each\",\"measurements\":[",
        (radius * 2 + 1).pow(2)
    );
    let mut first = true;
    for scene in [
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
    ] {
        let (images, sparse, ground) = fixture(scene);
        let modes = [
            DenseEstimator::FrontoparallelSweep,
            DenseEstimator::SlantedPlane,
        ];
        let mut results = [None, None];
        let mut times = [Vec::new(), Vec::new()];
        for &estimator in &modes {
            let options = DenseOptions {
                estimator,
                max_side: 80,
                patch_radius: radius,
                ..Default::default()
            };
            let _ = densify_with_options(&images, &sparse, &options, |_, _, _| true).unwrap();
        }
        for repeat in 0..repeats {
            for turn in 0..2 {
                let m = (repeat + turn) % 2;
                let options = DenseOptions {
                    estimator: modes[m],
                    max_side: 80,
                    patch_radius: radius,
                    ..Default::default()
                };
                let start = Instant::now();
                let r = densify_with_options(&images, &sparse, &options, |_, _, _| true).unwrap();
                times[m].push(start.elapsed().as_secs_f64() * 1000.);
                results[m] = Some(r);
            }
        }
        if let Some(path) = &export {
            let name = format!(
                "plane-{}{}",
                scene.angle,
                if scene.thin { "-occlusion" } else { "" }
            );
            save_xyz(
                &path.join(format!("{name}-reference.xyz")),
                ground.iter().map(|&(p, _)| p),
            );
            for (m, r) in results.iter().enumerate() {
                save_xyz(
                    &path.join(format!(
                        "{name}-{}-r{radius}.xyz",
                        if m == 0 { "baseline" } else { "slanted" }
                    )),
                    r.as_ref().unwrap().surface.positions.iter().copied(),
                );
            }
        }
        for (m, result) in results.into_iter().enumerate() {
            times[m].sort_by(f64::total_cmp);
            let result = result.unwrap();
            let points = &result.surface.positions;
            let mut errors: Vec<_> = points.iter().map(|&p| scene.distance(p)).collect();
            errors.sort_by(f64::total_cmp);
            let precision =
                errors.iter().filter(|&&d| d <= 0.04).count() as f64 / errors.len() as f64;
            let completeness = |object: Option<usize>| {
                let mut total = 0;
                let mut covered = 0;
                for &(p, o) in &ground {
                    if object.is_some_and(|v| if v == usize::MAX { o == 0 } else { v != o }) {
                        continue;
                    }
                    total += 1;
                    if points.iter().any(|&q| norm(sub(p, q)) <= 0.04) {
                        covered += 1;
                    }
                }
                if total == 0 {
                    0.
                } else {
                    covered as f64 / total as f64
                }
            };
            let recall = completeness(None);
            if !first {
                println!(",");
            }
            first = false;
            print!(
                "{{\"angle_deg\":{},\"occlusion_thin\":{},\"estimator\":\"{:?}\",\"runtime_median_ms\":{:.3},\"hypotheses\":{},\"source_patches\":{},\"sampled_source_pixels\":{},\"vertices\":{},\"photometric_samples\":{},\"consistent_samples\":{},\"mean_surface_error\":{:.6},\"median_surface_error\":{:.6},\"p90_surface_error\":{:.6},\"precision\":{:.6},\"completeness\":{:.6},\"f1\":{:.6},\"foreground_completeness\":{:.6},\"ribbon_completeness\":{:.6},\"occluder_completeness\":{:.6}}}",
                scene.angle,
                scene.thin,
                modes[m],
                times[m][repeats / 2],
                result.diagnostics.evaluated_hypotheses,
                result.diagnostics.evaluated_source_patches,
                result.diagnostics.sampled_source_pixels,
                points.len(),
                result.diagnostics.photometric_samples,
                result.diagnostics.consistent_samples,
                errors.iter().sum::<f64>() / errors.len() as f64,
                errors[errors.len() / 2],
                errors[errors.len() * 9 / 10],
                precision,
                recall,
                2. * precision * recall / (precision + recall).max(1e-12),
                completeness(Some(usize::MAX)),
                completeness(Some(1)),
                completeness(Some(2))
            );
        }
    }
    println!("\n]}}");
}
