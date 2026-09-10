//! Worker-owned storage and serialization. Reconstruction algorithms do not depend on Value.
use super::*;
use photogrammetry_core::{
    calibration::{Calibration, RectificationOptions, RectificationReport, RectifiedImage},
    dense::{DenseDiagnostics, DenseEstimator, DenseOptions, HostSweepView, PreparedView, Surface, SWEEP_WGSL},
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

pub fn add(width: usize, height: usize, focal: f64, rgb: Box<[u8]>) -> Result<usize> {
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
    rgb: Box<[u8]>,
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

// Session-local rectification mirroring calibration::rectify, so the grid
// search focal — a function of (calibration, width, height) only, never of the
// pixels — can be reused across the photos of one calibration group. Every
// float expression is verbatim from the kernel; the cached-focal test below
// compares both paths bit for bit and fails if they ever drift apart.
const GRID: usize = 17;
const RENDER_ATTEMPTS: usize = 8;

fn source_pixel(
    calibration: &Calibration,
    width: usize,
    height: usize,
    focal: f64,
    x: f64,
    y: f64,
) -> Option<[f64; 2]> {
    let p = calibration.project_ray([
        (x - width as f64 / 2.) / focal,
        (y - height as f64 / 2.) / focal,
    ])?;
    (p[0] >= 0. && p[1] >= 0. && p[0] <= (width - 1) as f64 && p[1] <= (height - 1) as f64)
        .then_some(p)
}

fn bilinear(image: &Image, p: [f64; 2], out: &mut [u8]) {
    let (x, y) = (p[0].floor() as usize, p[1].floor() as usize);
    let (nx, ny) = ((x + 1).min(image.width - 1), (y + 1).min(image.height - 1));
    let (a, b) = (p[0] - x as f64, p[1] - y as f64);
    let (i00, i10) = ((y * image.width + x) * 3, (y * image.width + nx) * 3);
    let (i01, i11) = ((ny * image.width + x) * 3, (ny * image.width + nx) * 3);
    let at = |i: usize, channel: usize| image.rgb[i + channel] as f64;
    let c00 = [at(i00, 0), at(i00, 1), at(i00, 2)];
    let c10 = [at(i10, 0), at(i10, 1), at(i10, 2)];
    let c01 = [at(i01, 0), at(i01, 1), at(i01, 2)];
    let c11 = [at(i11, 0), at(i11, 1), at(i11, 2)];
    for (channel, value) in out.iter_mut().enumerate() {
        *value = ((1. - b) * ((1. - a) * c00[channel] + a * c10[channel])
            + b * ((1. - a) * c01[channel] + a * c11[channel]))
        .round()
        .clamp(0., 255.) as u8;
    }
}

/// Rectify like the kernel, skipping the grid search when the group cache
/// already holds its deterministic result. Returns the searched focal so the
/// caller can cache it; None on the undistorted copy path and on cache hits.
fn rectify_searched(
    image: &Image,
    calibration: &Calibration,
    source_size: [usize; 2],
    cached_high: Option<f64>,
) -> Result<(RectifiedImage, Option<f64>)> {
    let options = RectificationOptions::default();
    image.validate()?;
    if !options.max_zoom.is_finite()
        || !(1. ..=8.).contains(&options.max_zoom)
        || image.rgb.len() > options.max_output_bytes
        || options.max_pixel_evaluations == 0
    {
        return Err("Rectification exceeds configured image/work limits".into());
    }
    let cal = calibration.resized(image.width, image.height, source_size)?;
    let base_focal = (cal.fx * cal.fy).sqrt();
    if !(20. ..=20000.).contains(&base_focal) {
        return Err("Resized measured focal is outside the kernel's supported range".into());
    }
    let mut work = 0usize;
    let mut checkpoint = |amount: usize| -> Result<()> {
        work = work
            .checked_add(amount)
            .ok_or("Rectification work overflow")?;
        if work > options.max_pixel_evaluations {
            return Err("Rectification mapping work limit exceeded".into());
        }
        // The kernel's progress callback is always `true` here; the WASM host
        // cancels by terminating its disposable Worker instead.
        Ok(())
    };
    checkpoint(0)?;
    let report = |focal: f64, resampled: bool| RectificationReport {
        input_size: [image.width, image.height],
        output_size: [image.width, image.height],
        focal,
        zoom: focal / base_focal,
        output_fov_degrees: [image.width, image.height]
            .map(|side| (side as f64 / (2. * focal)).atan().to_degrees() * 2.),
        resampled,
    };
    if cal.fx == cal.fy
        && cal.cx == image.width as f64 / 2.
        && cal.cy == image.height as f64 / 2.
        && [cal.k1, cal.k2, cal.k3, cal.p1, cal.p2]
            .iter()
            .all(|&v| v == 0.)
    {
        checkpoint(1)?;
        let copy = Image {
            focal: cal.fx,
            ..image.clone()
        };
        checkpoint(0)?;
        return Ok((
            RectifiedImage {
                image: copy,
                report: report(cal.fx, false),
            },
            None,
        ));
    }
    let max_focal = (base_focal * options.max_zoom).min(20000.);
    let mut searched_high = None;
    let mut high = match cached_high {
        Some(high) => high,
        None => {
            let mut grid_valid = |focal: f64| -> Result<bool> {
                checkpoint(GRID * GRID)?;
                Ok((0..GRID).all(|gy| {
                    (0..GRID).all(|gx| {
                        source_pixel(
                            &cal,
                            image.width,
                            image.height,
                            focal,
                            (image.width - 1) as f64 * gx as f64 / (GRID - 1) as f64,
                            (image.height - 1) as f64 * gy as f64 / (GRID - 1) as f64,
                        )
                        .is_some()
                    })
                }))
            };
            let mut low = base_focal;
            let mut high = base_focal;
            while !grid_valid(high)? {
                if high >= max_focal {
                    return Err(
                        "Calibration has no fully valid view within the zoom limit".into()
                    );
                }
                low = high;
                high = (high * 1.2).min(max_focal);
            }
            if high > base_focal {
                for _ in 0..12 {
                    let middle = (low + high) * 0.5;
                    if grid_valid(middle)? {
                        high = middle;
                    } else {
                        low = middle;
                    }
                }
                high = (high * 1.00001).min(max_focal);
            }
            searched_high = Some(high);
            high
        }
    };
    // The coarse grid only chooses a candidate. Every actual output pixel is
    // checked before sampling; hidden folds/invalid intervals cannot be filled.
    let mut rgb = vec![0; image.rgb.len()];
    let half_width = image.width as f64 / 2.;
    let half_height = image.height as f64 / 2.;
    let max_u = (image.width - 1) as f64;
    let max_v = (image.height - 1) as f64;
    for _ in 0..RENDER_ATTEMPTS {
        let mut valid = true;
        'rows: for y in 0..image.height {
            checkpoint(image.width)?;
            // Row-invariant normalized coordinate; same expression source_pixel
            // would compute, evaluated once per row instead of per pixel.
            let v = (y as f64 - half_height) / high;
            for x in 0..image.width {
                let Some(p) = cal
                    .project_ray([(x as f64 - half_width) / high, v])
                    .filter(|p| p[0] >= 0. && p[1] >= 0. && p[0] <= max_u && p[1] <= max_v)
                else {
                    valid = false;
                    break 'rows;
                };
                let i = (y * image.width + x) * 3;
                bilinear(image, p, &mut rgb[i..i + 3]);
            }
        }
        if valid {
            checkpoint(0)?;
            return Ok((
                RectifiedImage {
                    image: Image {
                        width: image.width,
                        height: image.height,
                        focal: high,
                        rgb,
                    },
                    report: report(high, true),
                },
                searched_high,
            ));
        }
        if high >= max_focal {
            break;
        }
        high = (high * 1.04).min(max_focal);
    }
    Err("Calibration contains invalid or folded pixels; no border-filled image was produced".into())
}

fn add_image(
    width: usize,
    height: usize,
    focal: f64,
    rgb: Box<[u8]>,
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
            // The WASM host's buffer is adopted instead of copied.
            rgb: rgb.into_vec(),
        };
        raw.validate().map_err(input)?;
        let (image, provenance) = if let Some(input) = &measured {
            // Rectify exactly once. Both sparse and dense read these same stored pixels.
            // Worker termination owns browser cancellation during synchronous WASM.
            // The same group id carries identical measurements (checked above), so
            // its grid-search focal is deterministic and reusable across photos.
            let key = (input.id.clone(), width, height);
            let cached_high = s.rectify_high.get(&key).copied();
            let (rectified, searched_high) =
                rectify_searched(&raw, &input.calibration, input.source_size, cached_high)?;
            if let Some(high) = searched_high {
                s.rectify_high.insert(key, high);
            }
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
        let mut options = ReconstructionOptions::default();
        options.feature_options.acceleration = self.acceleration;
        let outcome = photogrammetry_core::reconstruct_detailed(
            &self.images,
            &options,
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
        let mut options = browser_dense_options(side, preset)?;
        options.acceleration = self.acceleration;
        let sparse = self
            .sparse
            .as_ref()
            .ok_or_else(|| input("Reconstruct cameras first"))?;
        let run = photogrammetry_core::dense::densify_with_options(
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

    /// Stage 1 of the browser WebGPU sweep: validates eligibility, keeps the
    /// kernel bookkeeping in the session and returns a packed binary payload
    /// (ptr/len into a freshly allocated buffer the caller must free).
    fn dense_prepare(&mut self, side: usize, preset: u32) -> Result<Value> {
        self.pending_sweep = None;
        let options = browser_dense_options(side, preset)?;
        if side > 128 {
            // The scores readback grows quadratically with the map side; keep
            // the host path at the default resolution for now.
            return Ok(Value::Null);
        }
        let sparse = self
            .sparse
            .as_ref()
            .ok_or_else(|| input("Reconstruct cameras first"))?;
        let Some((views, prepared)) = photogrammetry_core::dense::prepare_host_sweep(
            &self.images,
            sparse,
            &options,
            &mut |_, _, _| true,
        )
        .map_err(input)?
        else {
            return Ok(Value::Null);
        };
        let blob = pack_sweep_payload(&self.images, &views);
        let len = blob.len();
        let ptr = Box::into_raw(blob.into_boxed_slice()) as *mut u8 as usize;
        self.pending_sweep = Some((options, prepared));
        Ok(json!({
            "ptr": ptr as f64,
            "len": len as f64,
            "wgsl": SWEEP_WGSL,
        }))
    }

    /// Stage 2: consumes the host score buffer (ownership moves like in
    /// photo_add) and finishes the dense pipeline with the shared CPU logic.
    fn dense_finish(&mut self, flat: Vec<f32>) -> Result<Vec<u8>> {
        let (options, prepared) = self
            .pending_sweep
            .take()
            .ok_or_else(|| input("Prepare the dense sweep first"))?;
        let sparse = self
            .sparse
            .as_ref()
            .ok_or_else(|| input("Reconstruct cameras first"))?;
        // Split the flat score stream per prepared view, in image order.
        let mut offset = 0usize;
        let mut scores: Vec<Option<Vec<f32>>> = Vec::with_capacity(prepared.len());
        for view in &prepared {
            if let Some(prep) = view {
                let count = prep.map_area() * options.depth_hypotheses;
                if offset + count > flat.len() {
                    return Err(input("Host sweep scores are truncated"));
                }
                scores.push(Some(flat[offset..offset + count].to_vec()));
                offset += count;
            } else {
                scores.push(None);
            }
        }
        if offset != flat.len() {
            return Err(input("Host sweep scores length does not match the prepared views"));
        }
        let run = photogrammetry_core::dense::densify_with_host_scores(
            &self.images,
            sparse,
            &options,
            &prepared,
            &scores,
            &mut |_, _, _| true,
        )
        .map_err(input)?;
        let diagnostics = dense_report_value(&run.diagnostics, &options)?;
        self.dense = Some(run.surface);
        response::surface(self.dense.as_ref().unwrap(), Some(&diagnostics))
    }
}


/// Binary payload for the host sweep: all grayscale rasters once, then per-view
/// shader parameters. All integers little-endian u32, all floats f32/f64 as
/// marked. The reference gray of each view is repacked first by the host.
fn pack_sweep_payload(images: &[Image], views: &[Option<HostSweepView>]) -> Vec<u8> {
    let mut out = Vec::new();
    let u32s = |out: &mut Vec<u8>, v: u32| out.extend_from_slice(&v.to_le_bytes());
    let f32s = |out: &mut Vec<u8>, v: f32| out.extend_from_slice(&v.to_le_bytes());
    u32s(&mut out, 0x3150_5753); // 'SWP1'
    u32s(&mut out, images.len() as u32);
    let first = views.iter().flatten().next();
    let (n_hyp, radius) = first
        .map(|v| (v.hypotheses.len() as u32, v.patch_radius as u32))
        .unwrap_or((0, 0));
    u32s(&mut out, n_hyp);
    u32s(&mut out, radius);
    // Gray headers first, then one contiguous gray block in image order, so
    // the host uploads it with a single zero-copy view.
    for image in images {
        u32s(&mut out, image.width as u32);
        u32s(&mut out, image.height as u32);
    }
    for image in images {
        for v in image.gray() {
            f32s(&mut out, v);
        }
    }
    u32s(&mut out, views.len() as u32);
    for view in views {
        let Some(view) = view else {
            u32s(&mut out, 0);
            continue;
        };
        u32s(&mut out, 1);
        u32s(&mut out, view.map_width as u32);
        u32s(&mut out, view.map_height as u32);
        u32s(&mut out, view.sources.len() as u32);
        u32s(&mut out, view.needed as u32);
        u32s(&mut out, view.ref_image as u32);
        for v in [view.step, view.ref_focal, view.ref_cx, view.ref_cy] {
            f32s(&mut out, v as f32);
        }
        for &z in &view.hypotheses {
            f32s(&mut out, z as f32);
        }
        for source in &view.sources {
            for row in source.rotation {
                for v in row {
                    f32s(&mut out, v as f32);
                }
            }
            for v in source.translation {
                f32s(&mut out, v as f32);
            }
            for v in [source.focal, source.cx, source.cy, 0.] {
                f32s(&mut out, v as f32);
            }
            u32s(&mut out, source.image as u32);
            u32s(&mut out, 0);
            u32s(&mut out, 0);
            u32s(&mut out, 0);
        }
    }
    out
}

/// Host-GPU (browser WebGPU) dense entry points; not part of the Value dispatch
/// because the score stream arrives as a raw buffer.
/// Selects the compute backend for subsequent sparse/dense runs: 0 = CPU
/// (default), 1 = GPU (opt-in, qualified separately; falls back to CPU when no
/// adapter or no `gpu` feature). Errors reset nothing.
pub fn set_acceleration_host(value: u32) -> Result<Vec<u8>> {
    let acceleration = match value {
        0 => photogrammetry_core::Acceleration::Cpu,
        1 => photogrammetry_core::Acceleration::Gpu,
        _ => return Err(input("Unknown acceleration mode")),
    };
    PHOTO.with(|session| session.borrow_mut().acceleration = acceleration);
    response::value(&Value::Bool(true))
}

pub fn dense_prepare_host(side: usize, preset: u32) -> Result<Value> {
    PHOTO.with(|session| session.borrow_mut().dense_prepare(side, preset))
}
pub fn dense_finish_host(scores: Vec<f32>) -> Result<Vec<u8>> {
    PHOTO.with(|session| session.borrow_mut().dense_finish(scores))
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
        assert!(add_calibrated(64, 64, 50., add_rgb(), &measured("lens", 80.))
            .unwrap_err()
            .contains("conflicting"));
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
