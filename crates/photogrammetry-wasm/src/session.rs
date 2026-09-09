//! Worker-owned storage and serialization. Reconstruction algorithms do not depend on Value.
use super::*;
use photogrammetry_kernel::{
    calibration::{rectify, Calibration, RectificationOptions, RectificationReport},
    dense::{DenseDiagnostics, DenseEstimator, DenseOptions, Surface},
    diagnostics::ReconstructionReport,
    Image, Reconstruction, ReconstructionOptions,
};
use std::{cell::RefCell, collections::BTreeMap};

#[derive(Default)]
struct Session {
    images: Vec<Image>,
    calibrations: Vec<Value>,
    groups: BTreeMap<String, Value>,
    sparse: Option<Reconstruction>,
    dense: Option<Surface>,
    diagnostics: Option<Value>,
}

thread_local! {
    static PHOTO: RefCell<Session> = RefCell::new(Session::default());
}

pub fn add(width: usize, height: usize, focal: f64, rgb: &[u8]) -> Result<usize> {
    add_image(width, height, focal, rgb, None)
}

struct MeasuredInput {
    id: String,
    group: Value,
    calibration: Calibration,
    source_size: [usize; 2],
}
impl MeasuredInput {
    fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > 16 * 1024 {
            return Err(input("Calibration metadata exceeds 16 KiB"));
        }
        let value = value_codec::decode_binary(bytes).map_err(|e| e.to_string())?;
        let group = value["group"].clone();
        let id: String = field(&group, "id")?;
        let label: String = field(&group, "label")?;
        let source: String = field(&group, "source")?;
        if id.is_empty()
            || id.len() > 128
            || label.is_empty()
            || label.len() > 512
            || source.is_empty()
            || source.len() > 8192
        {
            return Err(input(
                "Calibration group requires a bounded id, label and measurement source",
            ));
        }
        let distortion = &group["distortion"];
        if distortion["model"].as_str() != Some("brown-conrady") {
            return Err(input(
                "Only measured Brown-Conrady calibration is supported",
            ));
        }
        let calibration = Calibration {
            width: field(&group, "imageWidth")?,
            height: field(&group, "imageHeight")?,
            fx: field(&group, "fx")?,
            fy: field(&group, "fy")?,
            cx: field(&group, "cx")?,
            cy: field(&group, "cy")?,
            k1: field(distortion, "k1")?,
            k2: field(distortion, "k2")?,
            k3: field(distortion, "k3")?,
            p1: field(distortion, "p1")?,
            p2: field(distortion, "p2")?,
        };
        calibration.validate().map_err(input)?;
        let source_size = [
            field(&value, "sourceWidth")?,
            field(&value, "sourceHeight")?,
        ];
        if source_size != [calibration.width, calibration.height] {
            return Err(input(
                "Calibration dimensions do not match the oriented original photo",
            ));
        }
        Ok(Self {
            id,
            group,
            calibration,
            source_size,
        })
    }
}

pub fn add_calibrated(
    width: usize,
    height: usize,
    focal: f64,
    rgb: &[u8],
    metadata: &[u8],
) -> Result<usize> {
    add_image(
        width,
        height,
        focal,
        rgb,
        Some(MeasuredInput::parse(metadata)?),
    )
}

fn calibration_provenance(
    index: usize,
    image: &Image,
    measured: Option<(&MeasuredInput, &RectificationReport)>,
) -> Value {
    let output = json!({"width": image.width, "height": image.height, "focal": image.focal,
        "cx": image.width as f64 / 2., "cy": image.height as f64 / 2.});
    match measured {
        Some((input, report)) => json!({
            "image": index, "mode": "measured-brown", "group": input.group,
            "sourceSize": input.source_size, "inputSize": report.input_size,
            "output": output, "zoom": report.zoom,
            "outputFovDegrees": report.output_fov_degrees,
            "resampled": report.resampled, "borderPolicy": "fully-valid",
        }),
        None => {
            json!({"image": index, "mode": "focal-hint", "inputSize": [image.width, image.height],
            "output": output, "resampled": false})
        }
    }
}

fn add_image(
    width: usize,
    height: usize,
    focal: f64,
    rgb: &[u8],
    measured: Option<MeasuredInput>,
) -> Result<usize> {
    if width.checked_mul(height).and_then(|n| n.checked_mul(3)) != Some(rgb.len())
        || rgb.len() > 3 * 2048 * 2048
    {
        return Err(input("Invalid RGB image dimensions"));
    }
    PHOTO.with(|session| {
        let mut s = session.borrow_mut();
        if s.images.len() >= 24
            || s.images.iter().map(|i| i.rgb.len()).sum::<usize>() + rgb.len() > 96 * 1024 * 1024
        {
            return Err(input("Photo session exceeds 24 images or 96 MiB"));
        }
        if let Some(input) = &measured {
            if s.groups
                .get(&input.id)
                .is_some_and(|group| group != &input.group)
            {
                return Err("The same calibration group id has conflicting measurements".into());
            }
        }
        let raw = Image {
            width,
            height,
            focal,
            rgb: rgb.to_vec(),
        };
        raw.validate().map_err(input)?;
        let (image, provenance) = if let Some(input) = &measured {
            // Rectify exactly once. Both sparse and dense read these same stored pixels.
            // Worker termination owns browser cancellation during synchronous WASM.
            let rectified = rectify(
                &raw,
                &input.calibration,
                input.source_size,
                &RectificationOptions::default(),
                |_, _| true,
            )?;
            let provenance = calibration_provenance(
                s.images.len(),
                &rectified.image,
                Some((input, &rectified.report)),
            );
            (rectified.image, provenance)
        } else {
            let provenance = calibration_provenance(s.images.len(), &raw, None);
            (raw, provenance)
        };
        // Publish pixels and metadata together only after complete rectification.
        if let Some(input) = measured {
            s.groups.insert(input.id, input.group);
        }
        s.images.push(image);
        s.calibrations.push(provenance);
        s.sparse = None;
        s.dense = None;
        s.diagnostics = None;
        Ok(s.images.len())
    })
}

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

const JS_MAX_SAFE_COUNTER: u64 = 9_007_199_254_740_991;

fn browser_dense_options(side: usize, preset: u32) -> Result<DenseOptions> {
    if !(64..=256).contains(&side) {
        return Err(input("Browser dense resolution must be between 64 and 256"));
    }
    let mut options = DenseOptions {
        max_side: side,
        ..Default::default()
    };
    match preset {
        0 => {}
        1 | 2 => {
            if preset == 2 && side > 128 {
                return Err(input("Experimental volume resolution must not exceed 128"));
            }
            options.shared_volume = preset == 2;
            options.dual_scale = preset == 2;
            options.estimator = DenseEstimator::SlantedPlane;
            options.patch_radius = 2;
            options.depth_hypotheses = 64;
        }
        _ => return Err(input("Unknown dense reconstruction preset")),
    }
    Ok(options)
}

fn dense_report_value(report: &DenseDiagnostics, options: &DenseOptions) -> Result<Value> {
    // MGV1 preserves u64, but the browser decoder intentionally rejects integers
    // beyond Number.MAX_SAFE_INTEGER. Check before constructing a JS-bound value.
    if [
        report.evaluated_hypotheses as u64,
        report.evaluated_source_patches as u64,
        report.sampled_source_pixels,
    ]
    .iter()
    .any(|&count| count > JS_MAX_SAFE_COUNTER)
    {
        return Err(input(
            "Dense work counters exceed the browser's safe integer range",
        ));
    }
    let preset = if options.shared_volume && options.dual_scale {
        "dual-scale-volume"
    } else { match options.estimator {
        DenseEstimator::FrontoparallelSweep => "baseline",
        DenseEstimator::SlantedPlane => "slanted-plane",
    }};
    let views = report
        .view_reports
        .iter()
        .map(|view| {
            json!({
                "image": view.image,
                "sourceImages": view.source_images,
                "photometricSamples": view.photometric_samples,
                "consistentSamples": view.consistent_samples,
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "preset": preset,
        "patchRadius": options.patch_radius,
        "depthHypotheses": options.depth_hypotheses,
        "evaluatedHypotheses": report.evaluated_hypotheses,
        "evaluatedSourcePatches": report.evaluated_source_patches,
        "sampledSourcePixels": report.sampled_source_pixels,
        "estimatedMaps": report.estimated_maps,
        "selectedSourcePairs": report.selected_source_pairs,
        "photometricSamples": report.photometric_samples,
        "consistentSamples": report.consistent_samples,
        "rejectedInconsistentSamples": report.rejected_inconsistent_samples,
        "fusedSamples": report.fused_samples,
        "vertices": report.vertices,
        "triangles": report.triangles,
        "viewReports": views,
    }))
}

impl Session {
    fn sparse(&mut self) -> Result<Vec<u8>> {
        self.sparse = None;
        self.dense = None;
        let outcome = photogrammetry_kernel::reconstruct_detailed(
            &self.images,
            &ReconstructionOptions::default(),
            |_, _, _| true,
        );
        let mut diagnostics = report_value(&outcome.report);
        diagnostics["calibrations"] = Value::Array(self.calibrations.clone());
        self.diagnostics = Some(diagnostics);
        let reconstruction = outcome.reconstruction.map_err(input)?;
        self.sparse = Some(reconstruction);
        response::sparse(
            self.sparse.as_ref().unwrap(),
            self.diagnostics.as_ref().unwrap(),
        )
    }

    fn dense(&mut self, side: usize, preset: u32) -> Result<Vec<u8>> {
        let options = browser_dense_options(side, preset)?;
        let sparse = self
            .sparse
            .as_ref()
            .ok_or_else(|| input("Reconstruct cameras first"))?;
        let run = photogrammetry_kernel::dense::densify_with_options(
            &self.images,
            sparse,
            &options,
            |_, _, _| true,
        )
        .map_err(input)?;
        let diagnostics = dense_report_value(&run.diagnostics, &options)?;
        self.dense = Some(run.surface);
        response::surface(self.dense.as_ref().unwrap(), Some(&diagnostics))
    }
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
                    &photogrammetry_kernel::dense::compact(mesh, 24).map_err(input)?,
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
    use photogrammetry_kernel::diagnostics::SeedTrialReport;

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
        add_calibrated(64, 64, 50., &rgb, &measured("a", 70.)).unwrap();
        add_calibrated(64, 64, 50., &rgb, &measured("b", 90.)).unwrap();
        add(64, 64, 55., &rgb).unwrap();
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
        add_calibrated(64, 64, 50., &rgb, &measured("lens", 70.)).unwrap();
        let before = dispatch(json!({"action": "report"})).unwrap();
        assert!(add_calibrated(64, 64, 50., &rgb, &measured("lens", 80.))
            .unwrap_err()
            .contains("conflicting"));
        let mut wrong_size = value_codec::decode_binary(&measured("other", 70.)).unwrap();
        wrong_size["sourceWidth"] = json!(65);
        assert!(add_calibrated(
            64,
            64,
            50.,
            &rgb,
            &value_codec::encode_binary(&wrong_size).unwrap()
        )
        .is_err());
        assert!(add_calibrated(64, 64, 50., &rgb, b"{broken JSON}").is_err());
        assert_eq!(dispatch(json!({"action": "report"})).unwrap(), before);
        assert_eq!(PHOTO.with(|session| session.borrow().images.len()), 1);
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
