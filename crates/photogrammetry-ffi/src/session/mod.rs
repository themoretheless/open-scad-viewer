//! Worker-owned storage and serialization. Reconstruction algorithms do not depend on Value.
use super::*;
use photogrammetry_core::{
    calibration::{Calibration, RectificationOptions, RectificationReport, RectifiedImage},
    dense::{
        DenseDiagnostics, DenseEstimator, DenseOptions, HostSweepView, PreparedView, Surface,
        SWEEP_WGSL,
    },
    diagnostics::ReconstructionReport,
    Image, Reconstruction, ReconstructionOptions,
};
use std::{cell::RefCell, collections::BTreeMap};

#[derive(Default)]
struct Session {
    images: Vec<Image>,
    calibrations: Vec<Value>,
    groups: BTreeMap<String, Value>,
    /// Grid-search focal per (calibration group id, width, height); pixel-independent.
    rectify_high: BTreeMap<(String, usize, usize), f64>,
    sparse: Option<Reconstruction>,
    dense: Option<Surface>,
    /// Opt-in GPU stages (native builds with the `gpu` feature); wasm32 and
    /// unsupported platforms fall back to the CPU reference.
    acceleration: photogrammetry_core::Acceleration,
    diagnostics: Option<Value>,
    /// Options and kernel bookkeeping between `dense_prepare` and
    /// `dense_finish` on the host-GPU (browser WebGPU) path.
    pending_sweep: Option<(DenseOptions, Vec<Option<PreparedView>>)>,
}

thread_local! {
    static PHOTO: RefCell<Session> = RefCell::new(Session::default());
}

mod hosts;
mod ingest;
#[cfg(test)]
use hosts::{browser_dense_options, dense_report_value, JS_MAX_SAFE_COUNTER};
pub use hosts::{dense_finish_host, dense_prepare_host, set_acceleration_host};
pub use ingest::{add, add_calibrated};
fn report_value(report: &ReconstructionReport) -> Value {
    let images = report
        .images
        .iter()
        .map(|image| {
            json!({
                "image": image.image,
                "features": image.features,
                "candidateCorrespondences": image.candidate_correspondences,
                "acceptedObservations": image.accepted_observations,
                "conflictingMatches": image.conflicting_matches,
                "poseAttempts": image.pose_attempts,
                "registered": image.registered,
                "reason": image.reason,
            })
        })
        .collect::<Vec<_>>();
    let bundle = report
        .bundle_runs
        .iter()
        .map(|run| {
            json!({
                "initialCost": run.initial_cost,
                "finalCost": run.final_cost,
                "iterations": run.iterations,
                "acceptedSteps": run.accepted_steps,
                "observations": run.observations,
            })
        })
        .collect::<Vec<_>>();
    let trials = report
        .seed_trials
        .iter()
        .map(|trial| {
            json!({
                "pair": trial.pair,
                "registeredImages": trial.registered_images,
                "points": trial.points,
                "reprojectionRmse": trial.reprojection_rmse,
                "error": trial.error,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "images": images,
        "initialPair": report.initial_pair,
        "seedPairsTested": report.seed_pairs_tested,
        "seedTrials": trials,
        "matchingRequests": report.matching_requests,
        "computedPairs": report.computed_pairs,
        "bundleRuns": bundle,
        "warnings": report.warnings,
    })
}

pub fn dispatch_bytes(value: Value) -> Result<Vec<u8>> {
    PHOTO.with(|session| {
        let mut s = session.borrow_mut();
        match value["action"].as_str().unwrap_or("") {
            "clear" => {
                *s = Session::default();
                response::value(&Value::Bool(true))
            }
            "sparse" => s.sparse(),
            "dense" => {
                let preset = if value
                    .as_object()
                    .is_some_and(|map| map.contains_key("preset"))
                {
                    field::<u32>(&value, "preset")?
                } else {
                    0
                };
                s.dense(field::<usize>(&value, "resolution")?, preset)
            }
            "report" => {
                if let Some(diagnostics) = &s.diagnostics {
                    response::value(diagnostics)
                } else if s.images.is_empty() {
                    response::value(&Value::Null)
                } else {
                    let mut diagnostics = report_value(&ReconstructionReport::default());
                    diagnostics["calibrations"] = Value::Array(s.calibrations.clone());
                    response::value(&diagnostics)
                }
            }
            "compact" => {
                let mesh = s
                    .dense
                    .as_ref()
                    .ok_or_else(|| input("Build a surface first"))?;
                response::surface(
                    &photogrammetry_core::dense::compact(mesh, 24).map_err(input)?,
                    None,
                )
            }
            _ => Err(input("Unknown photogrammetry action")),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use photogrammetry_core::diagnostics::SeedTrialReport;

    fn dispatch(value: Value) -> Result<Value> {
        let bytes = dispatch_bytes(value)?;
        let decoded = value_codec::decode_binary(&bytes).map_err(|e| e.to_string())?;
        assert_eq!(decoded["ok"], Value::Bool(true));
        Ok(decoded["value"].clone())
    }

    #[test]
    fn diagnostics_preserve_successful_and_failed_seed_trials() {
        let report = ReconstructionReport {
            seed_trials: vec![
                SeedTrialReport {
                    pair: [0, 1],
                    registered_images: 6,
                    points: 400,
                    reprojection_rmse: Some(0.4),
                    error: None,
                },
                SeedTrialReport {
                    pair: [2, 3],
                    registered_images: 0,
                    points: 0,
                    reprojection_rmse: None,
                    error: Some("Initialization failed".into()),
                },
            ],
            ..Default::default()
        };
        let value = report_value(&report);
        let decoded =
            value_codec::decode_binary(&value_codec::encode_binary(&value).unwrap()).unwrap();
        assert_eq!(decoded["seedTrials"][0]["pair"], json!([0, 1]));
        assert_eq!(decoded["seedTrials"][0]["registeredImages"], json!(6));
        assert_eq!(decoded["seedTrials"][0]["reprojectionRmse"], json!(0.4));
        assert_eq!(decoded["seedTrials"][0]["error"], Value::Null);
        assert_eq!(decoded["seedTrials"][1]["reprojectionRmse"], Value::Null);
        assert_eq!(
            decoded["seedTrials"][1]["error"],
            json!("Initialization failed")
        );
    }

    #[test]
    fn an_empty_session_reports_null() {
        dispatch(json!({"action": "clear"})).unwrap();
        assert_eq!(dispatch(json!({"action": "report"})).unwrap(), Value::Null);
    }
    fn measured(id: &str, focal: f64) -> Vec<u8> {
        value_codec::encode_binary(&json!({
            "group": {"id": id, "label": "Synthetic measured group", "source": "Synthetic calibration fixture",
                "imageWidth": 64, "imageHeight": 64, "fx": focal, "fy": focal, "cx": 32., "cy": 32.,
                "distortion": {"model": "brown-conrady", "k1": 0., "k2": 0., "k3": 0., "p1": 0., "p2": 0.}},
            "sourceWidth": 64, "sourceHeight": 64,
        })).unwrap()
    }

    #[test]
    fn mixed_groups_and_legacy_images_keep_pixel_and_provenance_alignment() {
        dispatch(json!({"action": "clear"})).unwrap();
        let rgb: Vec<u8> = (0..64 * 64 * 3).map(|i| (i % 251) as u8).collect();
        let add_rgb = || rgb.clone().into_boxed_slice();
        add_calibrated(64, 64, 50., add_rgb(), &measured("a", 70.)).unwrap();
        add_calibrated(64, 64, 50., add_rgb(), &measured("b", 90.)).unwrap();
        add(64, 64, 55., add_rgb()).unwrap();
        PHOTO.with(|session| {
            let s = session.borrow();
            assert_eq!(
                s.images.iter().map(|i| i.focal).collect::<Vec<_>>(),
                vec![70., 90., 55.]
            );
            assert!(s.images.iter().all(|i| i.rgb == rgb));
        });
        let report = dispatch(json!({"action": "report"})).unwrap();
        assert_eq!(report["calibrations"][0]["group"]["id"], json!("a"));
        assert_eq!(report["calibrations"][1]["output"]["focal"], json!(90.));
        assert_eq!(report["calibrations"][2]["mode"], json!("focal-hint"));
        assert_eq!(report["calibrations"][0]["resampled"], json!(false));
        // A failed reconstruction still carries the applied measurements.
        let _ = dispatch(json!({"action": "sparse"}));
        assert_eq!(
            dispatch(json!({"action": "report"})).unwrap()["calibrations"],
            report["calibrations"]
        );
        dispatch(json!({"action": "clear"})).unwrap();
    }

    #[test]
    fn failed_calibrated_add_does_not_mutate_an_existing_session() {
        dispatch(json!({"action": "clear"})).unwrap();
        let rgb = vec![128; 64 * 64 * 3];
        let add_rgb = || rgb.clone().into_boxed_slice();
        add_calibrated(64, 64, 50., add_rgb(), &measured("lens", 70.)).unwrap();
        let before = dispatch(json!({"action": "report"})).unwrap();
        assert!(
            add_calibrated(64, 64, 50., add_rgb(), &measured("lens", 80.))
                .unwrap_err()
                .contains("conflicting")
        );
        let mut wrong_size = value_codec::decode_binary(&measured("other", 70.)).unwrap();
        wrong_size["sourceWidth"] = json!(65);
        assert!(add_calibrated(
            64,
            64,
            50.,
            add_rgb(),
            &value_codec::encode_binary(&wrong_size).unwrap()
        )
        .is_err());
        assert!(add_calibrated(64, 64, 50., add_rgb(), b"{broken JSON}").is_err());
        assert_eq!(dispatch(json!({"action": "report"})).unwrap(), before);
        assert_eq!(PHOTO.with(|session| session.borrow().images.len()), 1);
        dispatch(json!({"action": "clear"})).unwrap();
    }

    fn distorted(id: &str, focal: f64, k1: f64) -> Vec<u8> {
        value_codec::encode_binary(&json!({
            "group": {"id": id, "label": "Synthetic measured group", "source": "Synthetic calibration fixture",
                "imageWidth": 64, "imageHeight": 64, "fx": focal, "fy": focal, "cx": 32., "cy": 32.,
                "distortion": {"model": "brown-conrady", "k1": k1, "k2": 0., "k3": 0., "p1": 0., "p2": 0.}},
            "sourceWidth": 64, "sourceHeight": 64,
        })).unwrap()
    }

    #[test]
    fn a_cached_group_focal_rectifies_the_same_pixels_as_the_kernel() {
        dispatch(json!({"action": "clear"})).unwrap();
        let rgb_a: Vec<u8> = (0..64 * 64 * 3).map(|i| (i % 251) as u8).collect();
        let rgb_b: Vec<u8> = (0..64 * 64 * 3).map(|i| (i * 7 % 256) as u8).collect();
        // Pincushion distortion makes the base focal grid-invalid, so the search
        // grows and bisects; the second photo of the group reuses its result.
        let metadata = distorted("lens", 70., 0.5);
        add_calibrated(64, 64, 50., rgb_a.clone().into_boxed_slice(), &metadata).unwrap();
        add_calibrated(64, 64, 50., rgb_b.clone().into_boxed_slice(), &metadata).unwrap();
        let calibration = Calibration {
            width: 64,
            height: 64,
            fx: 70.,
            fy: 70.,
            cx: 32.,
            cy: 32.,
            k1: 0.5,
            k2: 0.,
            k3: 0.,
            p1: 0.,
            p2: 0.,
        };
        PHOTO.with(|session| {
            let s = session.borrow();
            assert_eq!(s.rectify_high.len(), 1);
            for (rgb, stored) in [rgb_a, rgb_b].into_iter().zip(s.images.iter()) {
                let source = Image {
                    width: 64,
                    height: 64,
                    focal: 50.,
                    rgb,
                };
                let expected = photogrammetry_core::calibration::rectify(
                    &source,
                    &calibration,
                    [64, 64],
                    &RectificationOptions::default(),
                    |_, _| true,
                )
                .unwrap();
                assert_eq!(stored.focal, expected.image.focal);
                assert_eq!(stored.rgb, expected.image.rgb);
            }
        });
        dispatch(json!({"action": "clear"})).unwrap();
    }
    #[test]
    fn browser_dense_presets_keep_the_default_and_bound_the_experiment() {
        let baseline = browser_dense_options(128, 0).unwrap();
        assert_eq!(baseline.estimator, DenseEstimator::FrontoparallelSweep);
        assert_eq!(baseline.patch_radius, 1);
        assert_eq!(baseline.depth_hypotheses, 64);
        let slanted = browser_dense_options(192, 1).unwrap();
        assert_eq!(slanted.estimator, DenseEstimator::SlantedPlane);
        assert_eq!(slanted.max_side, 192);
        assert_eq!(slanted.patch_radius, 2);
        assert_eq!(slanted.depth_hypotheses, 64);
        assert_eq!(slanted.min_correlation, baseline.min_correlation);
        assert_eq!(slanted.max_source_views, baseline.max_source_views);
        let volume = browser_dense_options(128, 2).unwrap();
        assert!(volume.shared_volume && volume.dual_scale);
        assert_eq!(volume.patch_radius, 2);
        assert!(browser_dense_options(129, 2).is_err());
        assert!(browser_dense_options(128, 3)
            .unwrap_err()
            .contains("Unknown dense"));
        assert!(browser_dense_options(300, 1)
            .unwrap_err()
            .contains("resolution"));
    }

    #[test]
    fn dense_dispatch_rejects_invalid_presets_before_starting_reconstruction() {
        dispatch(json!({"action": "clear"})).unwrap();
        let legacy = dispatch(json!({"action": "dense", "resolution": 128})).unwrap_err();
        assert_eq!(legacy, "Reconstruct cameras first");
        assert_eq!(
            dispatch(json!({"action": "dense", "resolution": 128, "preset": 1})).unwrap_err(),
            legacy
        );
        assert!(
            dispatch(json!({"action": "dense", "resolution": 128, "preset": 3}))
                .unwrap_err()
                .contains("Unknown dense")
        );
        assert!(dispatch(json!({"action": "dense", "resolution": 128, "preset": null})).is_err());
        assert!(
            dispatch(json!({"action": "dense", "resolution": 128, "preset": "slanted"})).is_err()
        );
    }

    #[test]
    fn dense_work_counts_preserve_u64_and_reject_unsafe_javascript_integers() {
        let options = browser_dense_options(128, 1).unwrap();
        let mut report = DenseDiagnostics {
            evaluated_hypotheses: 1000,
            evaluated_source_patches: 2300,
            sampled_source_pixels: u32::MAX as u64 + 12,
            ..Default::default()
        };
        let value = dense_report_value(&report, &options).unwrap();
        let decoded =
            value_codec::decode_binary(&value_codec::encode_binary(&value).unwrap()).unwrap();
        assert_eq!(decoded["sampledSourcePixels"], json!(4_294_967_307u64));
        assert_eq!(decoded["evaluatedHypotheses"], json!(1000));
        assert_eq!(decoded["evaluatedSourcePatches"], json!(2300));
        assert_eq!(decoded["preset"], json!("slanted-plane"));
        assert_eq!(decoded["patchRadius"], json!(2));
        assert_eq!(decoded["depthHypotheses"], json!(64));
        report.sampled_source_pixels = JS_MAX_SAFE_COUNTER;
        assert!(dense_report_value(&report, &options).is_ok());
        report.sampled_source_pixels += 1;
        assert!(dense_report_value(&report, &options)
            .unwrap_err()
            .contains("safe integer"));
        report.sampled_source_pixels = u64::MAX;
        assert!(dense_report_value(&report, &options).is_err());
    }
}
