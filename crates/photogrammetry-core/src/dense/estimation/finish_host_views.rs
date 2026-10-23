use super::*;

/// Builds depth maps from host-computed score rows (indexed by image, aligned
/// with `prepare_host_views` output). Scores are f32 from the GPU shader and
/// widened exactly to f64 for the shared CPU selection.
pub fn finish_host_views(
    images: &[Image],
    sparse: &Reconstruction,
    options: &DenseOptions,
    grayscale: &[Option<GrayImage>],
    prepared: &[Option<PreparedView>],
    scores: &[Option<Vec<f32>>],
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<(Vec<DepthMap>, DenseDiagnostics)> {
    if prepared.len() != images.len() || scores.len() != images.len() {
        return Err(crate::error(
            "Host sweep state does not match the session images",
        ));
    }
    let active: Vec<_> = sparse
        .cameras
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.as_ref().map(|c| (i, c)))
        .collect();
    let mut maps = Vec::new();
    let mut diagnostics = DenseDiagnostics::default();
    let patch_side = options.patch_radius * 2 + 1;
    for (view_number, &(index, reference)) in active.iter().enumerate() {
        cancelled(
            progress,
            "depth",
            view_number * options.max_side,
            active.len() * options.max_side,
        )?;
        let (Some(prep), Some(view_scores)) = (&prepared[index], &scores[index]) else {
            continue;
        };
        let expected = prep.width * prep.height * options.depth_hypotheses;
        if view_scores.len() != expected {
            return Err(crate::error(format!(
                "Host sweep scores for image {index} have {}, expected {expected} values",
                view_scores.len()
            )));
        }
        let sources = build_view_sources(
            sparse,
            grayscale,
            &ViewPreamble {
                reference,
                width: prep.width,
                height: prep.height,
                step: prep.step,
                near: prep.near,
                far: prep.far,
                neighbors: prep.neighbors.clone(),
            },
            options.patch_radius,
            prep.step,
        );
        // The host evaluated every hypothesis/source/tap without early exits;
        // counters report that full work, as in the native GPU mode.
        let pixels = (prep.width - 6) * (prep.height - 6);
        diagnostics.evaluated_hypotheses += pixels * options.depth_hypotheses;
        diagnostics.evaluated_source_patches += pixels * options.depth_hypotheses * sources.len();
        diagnostics.sampled_source_pixels +=
            (pixels * options.depth_hypotheses * sources.len() * patch_side * patch_side) as u64;
        let inverse_span = 1. / prep.near - 1. / prep.far;
        let (depth, confidence) = select_from_scores(
            view_scores,
            prep.width,
            prep.height,
            prep.ranges.as_ref(),
            prep.near,
            prep.far,
            inverse_span,
            options,
            &mut |row| {
                cancelled(
                    progress,
                    "depth",
                    view_number * options.max_side + row,
                    active.len() * options.max_side,
                )
            },
        )?;
        let _ = sources;
        maps.push(DepthMap {
            image: index,
            width: prep.width,
            height: prep.height,
            step: prep.step,
            depth,
            confidence,
            neighbors: prep.neighbors.clone(),
        });
    }
    Ok((maps, diagnostics))
}

/// Per-pixel sweep range as `(near, far)` map arrays; zeroed entries mean the
/// pixel keeps the global range.
pub(super) type SweepRanges = (Vec<f64>, Vec<f64>);

/// Window radius in map pixels for sparse-anchor aggregation; six covers the
/// typical sparse grid spacing without blending distant surfaces.
const PRIOR_WINDOW: usize = 6;

/// Deterministic per-pixel depth intervals from sparse anchors observed in the
/// reference view (COLMAP-style geometric prior). Anchors are splatted to the
/// map grid, then each pixel takes the padded min/max depth of anchors within
/// a fixed square window. Returns None when the view has no usable anchors.
#[expect(
    clippy::too_many_arguments,
    reason = "projection scalar inputs are kept explicit for the dense kernel; group into a context object in the next API pass"
)]
pub(super) fn sparse_intervals(
    sparse: &Reconstruction,
    image: usize,
    reference: &Camera,
    width: usize,
    height: usize,
    step: f64,
    near: f64,
    far: f64,
) -> Option<SweepRanges> {
    let mut cell_lo = vec![f64::INFINITY; width * height];
    let mut cell_hi = vec![0f64; width * height];
    let mut any = false;
    for point in &sparse.points {
        if !point.observations.iter().any(|&(view, _)| view == image) {
            continue;
        }
        let z = add(
            mv(reference.rotation, point.position),
            reference.translation,
        )[2];
        if !z.is_finite() || z <= 0. {
            continue;
        }
        let Some(uv) = reference.project(point.position) else {
            continue;
        };
        if uv[0] < 0. || uv[1] < 0. {
            continue;
        }
        let (gx, gy) = ((uv[0] / step) as usize, (uv[1] / step) as usize);
        if gx >= width || gy >= height {
            continue;
        }
        let i = gy * width + gx;
        cell_lo[i] = cell_lo[i].min(z);
        cell_hi[i] = cell_hi[i].max(z);
        any = true;
    }
    if !any {
        return None;
    }
    let mut lo = vec![0.; width * height];
    let mut hi = vec![0.; width * height];
    for y in 0..height {
        for x in 0..width {
            let mut l = f64::INFINITY;
            let mut h = 0f64;
            for wy in y.saturating_sub(PRIOR_WINDOW)..=(y + PRIOR_WINDOW).min(height - 1) {
                for wx in x.saturating_sub(PRIOR_WINDOW)..=(x + PRIOR_WINDOW).min(width - 1) {
                    let i = wy * width + wx;
                    if cell_hi[i] > 0. {
                        l = l.min(cell_lo[i]);
                        h = h.max(cell_hi[i]);
                    }
                }
            }
            if h > 0. {
                let a = (l * 0.95).max(near);
                let b = (h * 1.05).min(far);
                lo[y * width + x] = a;
                hi[y * width + x] = b;
            }
        }
    }
    Some((lo, hi))
}

/// Per-pixel refinement intervals around a half-resolution depth map: four
/// coarse hypothesis steps of inverse depth on either side of the lifted
/// depth, clamped to the global range.
pub(super) fn coarse_intervals(
    coarse_depth: &[f64],
    coarse_width: usize,
    width: usize,
    height: usize,
    near: f64,
    far: f64,
    hypotheses: usize,
) -> SweepRanges {
    let spacing = (1. / near - 1. / far) / (hypotheses - 1) as f64;
    let mut lo = vec![0.; width * height];
    let mut hi = vec![0.; width * height];
    for y in 0..height {
        for x in 0..width {
            let z = coarse_depth[(y / 2) * coarse_width + x / 2];
            if z <= 0. {
                continue;
            }
            let w = 1. / z;
            let w_lo = (w - 4. * spacing).max(1. / far);
            let w_hi = (w + 4. * spacing).min(1. / near);
            lo[y * width + x] = 1. / w_hi;
            hi[y * width + x] = 1. / w_lo;
        }
    }
    (lo, hi)
}

/// Per-pixel selection over a host/GPU-computed score map (f32 widened to f64,
/// exact widening), shared by the native GPU sweep and the browser host path.
#[allow(clippy::too_many_arguments)]
fn select_from_scores(
    scores_map: &[f32],
    width: usize,
    height: usize,
    ranges: Option<&SweepRanges>,
    near: f64,
    far: f64,
    inverse_span: f64,
    options: &DenseOptions,
    row_progress: &mut impl FnMut(usize) -> Result<()>,
) -> Result<(Vec<f64>, Vec<f32>)> {
    let mut depth = vec![0.; width * height];
    let mut confidence = vec![0.; width * height];
    for y in 3..height - 3 {
        row_progress(y)?;
        for x in 3..width - 3 {
            let pixel = y * width + x;
            let start = pixel * options.depth_hypotheses;
            let row = &scores_map[start..start + options.depth_hypotheses];
            let mut scores = [0.; 128];
            for (d, &v) in row.iter().enumerate() {
                scores[d] = v as f64;
            }
            if let Some((d, c)) = pick_depth(
                &scores[..options.depth_hypotheses],
                ranges,
                pixel,
                near,
                far,
                inverse_span,
                options,
            ) {
                depth[pixel] = d;
                confidence[pixel] = c;
            }
        }
    }
    Ok((depth, confidence))
}

/// Inclusive hypothesis-bin interval of one pixel. Maps the depth interval onto
/// the global hypothesis grid; intervals wider than 15% relative depth can
/// straddle a depth edge, so they are not trusted and the pixel keeps
/// full-range behavior. Shared by the CPU selection and the GPU shader input.
fn bin_range(
    ranges: Option<&SweepRanges>,
    pixel: usize,
    near: f64,
    inverse_span: f64,
    options: &DenseOptions,
) -> (usize, usize) {
    let last = options.depth_hypotheses - 1;
    match ranges {
        Some((lo, hi)) if lo[pixel] > 0. => {
            let (range_lo, range_hi) = (lo[pixel], hi[pixel]);
            if range_hi > range_lo * 1.15 {
                (0, last)
            } else {
                let bin_of = |z: f64| (1. / near - 1. / z) / inverse_span * last as f64;
                let lo_bin = bin_of(range_lo).floor().max(0.) as usize;
                let hi_bin = (bin_of(range_hi).ceil() as usize).min(last);
                (lo_bin, hi_bin.max(lo_bin))
            }
        }
        _ => (0, last),
    }
}

/// Frontoparallel NCC sweep over inverse-depth hypotheses. Optional per-pixel
/// ranges restrict peak selection to the geometrically plausible bins and
/// exclude those bins from the uniqueness alternatives; pixels without a range
/// keep the historical full-range behavior bit-exactly.
///
/// Hypothesis selection shared by the CPU and GPU sweeps; takes the per-pixel
/// score row and applies the range clamp, uniqueness margin and the parabolic
/// subpixel offset. Returns (depth, confidence).
#[allow(clippy::too_many_arguments)]
pub(super) fn pick_depth(
    scores: &[f64],
    ranges: Option<&SweepRanges>,
    pixel: usize,
    near: f64,
    far: f64,
    inverse_span: f64,
    options: &DenseOptions,
) -> Option<(f64, f32)> {
    let last = options.depth_hypotheses - 1;
    let (bin_lo, bin_hi) = bin_range(ranges, pixel, near, inverse_span, options);
    // With a per-pixel range the prior vouches for everything inside
    // it; ambiguity only matters at competing depths outside. If the
    // in-range peak is implausible, the range was wrong for this pixel:
    // fall back to the historical full-range selection.
    let ranged = bin_lo > 0 || bin_hi < last;
    let best_ranged = (bin_lo..=bin_hi)
        .max_by(|&a, &b| scores[a].total_cmp(&scores[b]))
        .unwrap();
    let (best, alternative) = if ranged && scores[best_ranged] >= options.min_correlation {
        let alternative = (0..options.depth_hypotheses)
            .filter(|&d| d + 3 < bin_lo || d > bin_hi + 3)
            .map(|d| scores[d])
            .fold(-1., f64::max);
        (best_ranged, alternative)
    } else {
        let best = (0..options.depth_hypotheses)
            .max_by(|&a, &b| scores[a].total_cmp(&scores[b]))
            .unwrap();
        let alternative = (0..options.depth_hypotheses)
            .filter(|&d| d.abs_diff(best) > 3)
            .map(|d| scores[d])
            .fold(-1., f64::max);
        (best, alternative)
    };
    if scores[best] < options.min_correlation {
        return None;
    }
    if scores[best] - alternative < options.uniqueness_margin {
        return None;
    }
    let mut offset = 0.;
    if best > 0 && best < last {
        let (a, b, c) = (scores[best - 1], scores[best], scores[best + 1]);
        let denom = a - 2. * b + c;
        if a > 0. && c > 0. && denom.abs() > 1e-8 {
            offset = (0.5 * (a - c) / denom).clamp(-0.5, 0.5);
        }
    }
    let f = (best as f64 + offset) / last as f64;
    Some((1. / ((1. - f) / near + f / far), scores[best] as f32))
}

/// Smallest f32 not below `x`: for an f32 score `s`, `s < x` and `s >= x` in
/// f64 are exactly `s < ceil` and `s >= ceil` in f32.
#[cfg(feature = "gpu")]
fn ceil_f32(x: f64) -> f32 {
    let t = x as f32;
    if (t as f64) < x { t.next_up() } else { t }
}

/// Packs one view for the GPU sweep-and-select shader. Per-pixel hypothesis
/// bins come from the same `bin_range` the CPU selection uses.
#[cfg(feature = "gpu")]
#[allow(clippy::too_many_arguments)]
fn gpu_job(
    ref_index: usize,
    reference: &Camera,
    sources: &[Source],
    width: usize,
    height: usize,
    step: f64,
    hypotheses: &[f64],
    ranges: Option<&SweepRanges>,
    near: f64,
    inverse_span: f64,
    options: &DenseOptions,
) -> crate::gpu::sweep::SweepJob {
    let mut bins = Vec::with_capacity(width * height * 2);
    for pixel in 0..width * height {
        let (lo, hi) = bin_range(ranges, pixel, near, inverse_span, options);
        bins.push(lo as u32);
        bins.push(hi as u32);
    }
    crate::gpu::sweep::SweepJob {
        ref_image: ref_index,
        ref_focal: reference.focal as f32,
        ref_cx: reference.cx as f32,
        ref_cy: reference.cy as f32,
        sources: sources
            .iter()
            .map(|source| crate::gpu::sweep::SourcePayload {
                image: source.index,
                rotation: source.rotation.map(|row| row.map(|v| v as f32)),
                translation: source.translation.map(|v| v as f32),
                focal: source.camera.focal as f32,
                cx: source.camera.cx as f32,
                cy: source.camera.cy as f32,
            })
            .collect(),
        hypotheses: hypotheses.iter().map(|&z| z as f32).collect(),
        width,
        height,
        patch_radius: options.patch_radius,
        step: step as f32,
        needed: sources.len().min(2).max(options.min_support_views),
        bins,
        min_correlation: ceil_f32(options.min_correlation),
        uniqueness_margin: options.uniqueness_margin as f32,
    }
}

/// Depth/confidence maps from GPU selections; depth is rebuilt in f64 from the
/// winning bin and its sub-bin offset, exactly as `pick_depth` does.
#[cfg(feature = "gpu")]
fn maps_from_selection(
    selection: &[Option<crate::gpu::sweep::Selection>],
    near: f64,
    far: f64,
    options: &DenseOptions,
) -> (Vec<f64>, Vec<f32>) {
    let last = (options.depth_hypotheses - 1) as f64;
    let mut depth = vec![0.; selection.len()];
    let mut confidence = vec![0.; selection.len()];
    for (i, picked) in selection.iter().enumerate() {
        if let Some(picked) = picked {
            let f = (picked.bin as f64 + picked.offset as f64) / last;
            depth[i] = 1. / ((1. - f) / near + f / far);
            confidence[i] = picked.score;
        }
    }
    (depth, confidence)
}

/// GPU work counters: the shader evaluates every hypothesis/source/tap
/// without early exits, so the diagnostics report that full work.
#[cfg(feature = "gpu")]
fn count_gpu_work(
    diagnostics: &mut DenseDiagnostics,
    width: usize,
    height: usize,
    sources: usize,
    options: &DenseOptions,
) {
    let pixels = (width - 6) * (height - 6);
    let taps = (options.patch_radius * 2 + 1).pow(2);
    diagnostics.evaluated_hypotheses += pixels * options.depth_hypotheses;
    diagnostics.evaluated_source_patches += pixels * options.depth_hypotheses * sources;
    diagnostics.sampled_source_pixels +=
        (pixels * options.depth_hypotheses * sources * taps) as u64;
}

/// Single-view GPU sweep (coarse-to-fine passes, which depend on each other,
/// cannot join the per-pass batch). Uploads only the rasters this view needs.
#[cfg(feature = "gpu")]
#[allow(clippy::too_many_arguments)]
fn gpu_sweep_single(
    gray: &GrayImage,
    reference: &Camera,
    sources: &[Source],
    width: usize,
    height: usize,
    step: f64,
    hypotheses: &[f64],
    ranges: Option<&SweepRanges>,
    near: f64,
    far: f64,
    inverse_span: f64,
    options: &DenseOptions,
) -> Option<(Vec<f64>, Vec<f32>)> {
    // Backend choice first: without any device the payload is never built.
    #[cfg(feature = "cuda")]
    let cuda_sweep = (options.acceleration == crate::Acceleration::Cuda)
        .then(crate::gpu::sweep::cuda::shared)
        .flatten();
    let wgpu_sweep = crate::gpu::sweep::shared();
    #[cfg(feature = "cuda")]
    if cuda_sweep.is_none() && wgpu_sweep.is_none() {
        return None;
    }
    #[cfg(not(feature = "cuda"))]
    wgpu_sweep?;
    // Local atlas: the reference at slot 0, sources renumbered after it.
    let mut rasters: Vec<Option<(&[f32], usize, usize)>> =
        vec![Some((gray.values.as_slice(), gray.width, gray.height))];
    let mut renumbered = Vec::with_capacity(sources.len());
    for source in sources {
        rasters.push(Some((
            source.image.values.as_slice(),
            source.image.width,
            source.image.height,
        )));
        renumbered.push(Source {
            index: rasters.len() - 1,
            image: source.image,
            camera: source.camera,
            rotation: source.rotation,
            translation: source.translation,
            offsets: source.offsets,
            rays: source.rays,
        });
    }
    let job = gpu_job(
        0,
        reference,
        &renumbered,
        width,
        height,
        step,
        hypotheses,
        ranges,
        near,
        inverse_span,
        options,
    );
    // Native CUDA first for Acceleration::Cuda, then the wgpu sweep; both
    // return None to leave the CPU sweep in charge.
    #[cfg(feature = "cuda")]
    if let Some(sweep) = cuda_sweep
        && let Some(atlas) = sweep.upload_grays(&rasters)
        && let Some(selection) = sweep.run(&atlas, &job)
    {
        return Some(maps_from_selection(&selection, near, far, options));
    }
    let sweep = wgpu_sweep?;
    let atlas = sweep.upload_grays(&rasters);
    let mut batch = sweep.batch(&atlas);
    batch.push(&job);
    let selection = batch.try_finish().ok()?.pop()?;
    Some(maps_from_selection(&selection, near, far, options))
}
