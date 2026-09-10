//! Host-GPU (WebGPU) sweep entry points and the dense/sparse session methods.
use super::*;
pub(super) const JS_MAX_SAFE_COUNTER: u64 = 9_007_199_254_740_991;

pub(super) fn browser_dense_options(side: usize, preset: u32) -> Result<DenseOptions> {
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

pub(super) fn dense_report_value(report: &DenseDiagnostics, options: &DenseOptions) -> Result<Value> {
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
    pub(super) fn sparse(&mut self) -> Result<Vec<u8>> {
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

    pub(super) fn dense(&mut self, side: usize, preset: u32) -> Result<Vec<u8>> {
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

