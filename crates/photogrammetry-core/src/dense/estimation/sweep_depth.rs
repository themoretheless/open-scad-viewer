use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn sweep_depth(
    gray: &GrayImage,
    reference: &Camera,
    sources: &mut [Source],
    width: usize,
    height: usize,
    step: f64,
    near: f64,
    far: f64,
    ranges: Option<&SweepRanges>,
    options: &DenseOptions,
    diagnostics: &mut DenseDiagnostics,
    row_progress: &mut impl FnMut(usize) -> Result<()>,
) -> Result<(Vec<f64>, Vec<f32>)> {
    let hypotheses = hypothesis_grid(near, far, options.depth_hypotheses);
    let inverse_span = 1. / near - 1. / far;
    let patch_side = options.patch_radius * 2 + 1;
    let mut depth = vec![0.; width * height];
    let mut confidence = vec![0.; width * height];
    #[cfg(feature = "gpu")]
    if options.acceleration.is_gpu() {
        if let Some(maps) = gpu_sweep_single(
            gray,
            reference,
            sources,
            width,
            height,
            step,
            &hypotheses[..options.depth_hypotheses],
            ranges,
            near,
            far,
            inverse_span,
            options,
        ) {
            count_gpu_work(diagnostics, width, height, sources.len(), options);
            row_progress(height - 4)?;
            return Ok(maps);
        }
    }
    for y in 3..height - 3 {
        row_progress(y)?;
        for x in 3..width - 3 {
            let (px, py) = (x as f64 * step, y as f64 * step);
            let mut values = [0.; 25];
            for (i, value) in values[..patch_side * patch_side].iter_mut().enumerate() {
                *value = gray
                    .sample(
                        px + ((i % patch_side) as f64 - options.patch_radius as f64) * step,
                        py + ((i / patch_side) as f64 - options.patch_radius as f64) * step,
                    )
                    .unwrap();
            }
            let Some(patch) = Patch::from_values(values, patch_side * patch_side) else {
                continue;
            };
            let ray = reference.ray([px, py]);
            for source in sources.iter_mut() {
                source.prepare_rays(ray, patch.len);
            }
            let mut scores = [-1.; 128];
            let needed = sources.len().min(2).max(options.min_support_views);
            for (d, score) in scores[..options.depth_hypotheses].iter_mut().enumerate() {
                diagnostics.evaluated_hypotheses += 1;
                let mut sum = 0.;
                let mut count = 0;
                for (s, source) in sources.iter().enumerate() {
                    // The remaining sources cannot lift count to needed;
                    // the hypothesis score stays -1, so stop scoring early.
                    if count + (sources.len() - s) < needed {
                        break;
                    }
                    diagnostics.evaluated_source_patches += 1;
                    if let Some(ncc) = source.score(
                        &patch,
                        hypotheses[d],
                        &mut diagnostics.sampled_source_pixels,
                    ) && ncc > 0.4
                    {
                        sum += ncc;
                        count += 1;
                    }
                }
                if count >= needed {
                    *score = sum / count as f64;
                }
            }
            if let Some((d, c)) = pick_depth(
                &scores[..options.depth_hypotheses],
                ranges,
                y * width + x,
                near,
                far,
                inverse_span,
                options,
            ) {
                depth[y * width + x] = d;
                confidence[y * width + x] = c;
            }
        }
    }
    Ok((depth, confidence))
}
