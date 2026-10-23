use super::*;

pub(in super::super) fn estimate_prepared(
    images: &[Image],
    sparse: &Reconstruction,
    options: &DenseOptions,
    grayscale: &[Option<GrayImage>],
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<(Vec<DepthMap>, DenseDiagnostics)> {
    let active: Vec<_> = sparse
        .cameras
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.as_ref().map(|c| (i, c)))
        .collect();
    let passes = if options.estimator == DenseEstimator::SlantedPlane {
        3
    } else {
        1
    };
    let mut maps = Vec::new();
    let mut diagnostics = DenseDiagnostics::default();
    // GPU batch: every frontoparallel view of this pass shares one gray upload
    // and one submit; coarse-to-fine passes depend on each other and keep the
    // per-view path inside `sweep_depth`.
    #[cfg(feature = "gpu")]
    let batch_eligible = options.acceleration.is_gpu()
        && options.estimator == DenseEstimator::FrontoparallelSweep
        && !options.coarse_to_fine;
    // Dispatch order for the batched pass: native CUDA when it was requested
    // and a device exists, then the wgpu sweep, then the CPU sweep.
    #[cfg(feature = "cuda")]
    let cuda_sweep = (batch_eligible && options.acceleration == crate::Acceleration::Cuda)
        .then(crate::gpu::sweep::cuda::shared)
        .flatten();
    #[cfg(feature = "gpu")]
    let wgpu_sweep = batch_eligible.then(crate::gpu::sweep::shared).flatten();
    #[cfg(all(feature = "gpu", not(feature = "cuda")))]
    let gpu_batch = wgpu_sweep.is_some();
    #[cfg(feature = "cuda")]
    let gpu_batch = wgpu_sweep.is_some() || cuda_sweep.is_some();
    #[cfg(feature = "gpu")]
    let mut gpu_jobs: Vec<crate::gpu::sweep::SweepJob> = Vec::new();
    #[cfg(feature = "gpu")]
    let mut gpu_pending: Vec<(usize, ViewPreamble, usize)> = Vec::new();
    for (view_number, &(index, reference)) in active.iter().enumerate() {
        cancelled(
            progress,
            "depth",
            view_number * options.max_side * passes,
            active.len() * options.max_side * passes,
        )?;
        let Some(preamble) = view_preamble(images, sparse, options, index, reference) else {
            continue;
        };
        let gray = grayscale[index].as_ref().unwrap();
        #[cfg(feature = "gpu")]
        if gpu_batch {
            let sources = build_view_sources(
                sparse,
                grayscale,
                &preamble,
                options.patch_radius,
                preamble.step,
            );
            let ranges = if options.sparse_depth_prior {
                sparse_intervals(
                    sparse,
                    index,
                    reference,
                    preamble.width,
                    preamble.height,
                    preamble.step,
                    preamble.near,
                    preamble.far,
                )
            } else {
                None
            };
            let hypotheses = hypothesis_grid(preamble.near, preamble.far, options.depth_hypotheses);
            gpu_jobs.push(gpu_job(
                index,
                reference,
                &sources,
                preamble.width,
                preamble.height,
                preamble.step,
                &hypotheses[..options.depth_hypotheses],
                ranges.as_ref(),
                preamble.near,
                1. / preamble.near - 1. / preamble.far,
                options,
            ));
            gpu_pending.push((index, preamble, sources.len()));
            continue;
        }
        let (width, height, step, near, far) = (
            preamble.width,
            preamble.height,
            preamble.step,
            preamble.near,
            preamble.far,
        );
        let neighbors = &preamble.neighbors;
        let build_sources = |source_step: f64| {
            build_view_sources(
                sparse,
                grayscale,
                &preamble,
                options.patch_radius,
                source_step,
            )
        };
        let mut sources = build_sources(step);
        if options.estimator == DenseEstimator::SlantedPlane {
            let (depth, confidence) = super::super::plane::estimate(
                gray,
                reference,
                &mut sources,
                width,
                height,
                step,
                near,
                far,
                None,
                options,
                &mut diagnostics,
                &mut |_, row, _| {
                    progress(
                        "depth",
                        view_number * options.max_side * 3 + row,
                        active.len() * options.max_side * 3,
                    )
                },
            )?;
            maps.push(DepthMap {
                image: index,
                width,
                height,
                step,
                depth,
                confidence,
                neighbors: neighbors.clone(),
            });
            continue;
        }
        // Opt-in per-pixel sweep ranges: a coarse half-resolution pass fills
        // the map, then sparse anchors override the pixels they cover.
        let mut ranges: Option<(Vec<f64>, Vec<f64>)> = None;
        if options.coarse_to_fine && width >= 16 && height >= 16 {
            let coarse_width = width / 2;
            let coarse_height = height / 2;
            let coarse_step = step * 2.;
            let coarse_prior = if options.sparse_depth_prior {
                sparse_intervals(
                    sparse,
                    index,
                    reference,
                    coarse_width,
                    coarse_height,
                    coarse_step,
                    near,
                    far,
                )
            } else {
                None
            };
            let mut coarse_sources = build_sources(coarse_step);
            let (coarse_depth, _) = sweep_depth(
                gray,
                reference,
                &mut coarse_sources,
                coarse_width,
                coarse_height,
                coarse_step,
                near,
                far,
                coarse_prior.as_ref(),
                options,
                &mut diagnostics,
                &mut |row| {
                    cancelled(
                        progress,
                        "depth",
                        view_number * options.max_side + row,
                        active.len() * options.max_side,
                    )
                },
            )?;
            let (lo, hi) = coarse_intervals(
                &coarse_depth,
                coarse_width,
                width,
                height,
                near,
                far,
                options.depth_hypotheses,
            );
            ranges = Some((lo, hi));
        }
        if options.sparse_depth_prior {
            let sparse_ranges =
                sparse_intervals(sparse, index, reference, width, height, step, near, far);
            match (&mut ranges, sparse_ranges) {
                (Some((lo, hi)), Some((sparse_lo, sparse_hi))) => {
                    for i in 0..lo.len() {
                        if sparse_lo[i] > 0. {
                            lo[i] = sparse_lo[i];
                            hi[i] = sparse_hi[i];
                        }
                    }
                }
                (None, sparse) => ranges = sparse,
                (Some(_), None) => {}
            }
        }
        let (depth, confidence) = sweep_depth(
            gray,
            reference,
            &mut sources,
            width,
            height,
            step,
            near,
            far,
            ranges.as_ref(),
            options,
            &mut diagnostics,
            &mut |row| {
                cancelled(
                    progress,
                    "depth",
                    view_number * options.max_side + row,
                    active.len() * options.max_side,
                )
            },
        )?;
        maps.push(DepthMap {
            image: index,
            width,
            height,
            step,
            depth,
            confidence,
            neighbors: neighbors.clone(),
        });
    }
    #[cfg(feature = "gpu")]
    if gpu_batch {
        let rasters: Vec<Option<(&[f32], usize, usize)>> = grayscale
            .iter()
            .map(|g| g.as_ref().map(|g| (g.values.as_slice(), g.width, g.height)))
            .collect();
        cancelled(progress, "depth", 0, active.len() * options.max_side)?;
        #[cfg(feature = "cuda")]
        let selections = cuda_sweep.and_then(|sweep| {
            let atlas = sweep.upload_grays(&rasters)?;
            sweep.run_batch(&atlas, &gpu_jobs)
        });
        #[cfg(not(feature = "cuda"))]
        let selections: Option<Vec<Vec<Option<crate::gpu::sweep::Selection>>>> = None;
        let selections = selections.or_else(|| {
            wgpu_sweep.and_then(|sweep| {
                let atlas = sweep.upload_grays(&rasters);
                let mut batch = sweep.batch(&atlas);
                for job in &gpu_jobs {
                    batch.push(job);
                }
                batch.try_finish().ok()
            })
        });
        match selections {
            Some(selections) => {
                if selections.len() != gpu_pending.len() {
                    return Err(crate::error("GPU sweep returned an unexpected view count"));
                }
                for ((index, preamble, source_count), selection) in
                    gpu_pending.into_iter().zip(selections)
                {
                    count_gpu_work(
                        &mut diagnostics,
                        preamble.width,
                        preamble.height,
                        source_count,
                        options,
                    );
                    let (depth, confidence) =
                        maps_from_selection(&selection, preamble.near, preamble.far, options);
                    maps.push(DepthMap {
                        image: index,
                        width: preamble.width,
                        height: preamble.height,
                        step: preamble.step,
                        depth,
                        confidence,
                        neighbors: preamble.neighbors,
                    });
                }
            }
            // Available GPU backends failed; finish pending views on the CPU.
            None => {
                let cpu_options = DenseOptions {
                    acceleration: crate::Acceleration::Cpu,
                    ..options.clone()
                };
                for (view_number, (index, preamble, _)) in gpu_pending.into_iter().enumerate() {
                    let gray = grayscale[index].as_ref().unwrap();
                    let mut sources = build_view_sources(
                        sparse,
                        grayscale,
                        &preamble,
                        options.patch_radius,
                        preamble.step,
                    );
                    let ranges = if options.sparse_depth_prior {
                        sparse_intervals(
                            sparse,
                            index,
                            preamble.reference,
                            preamble.width,
                            preamble.height,
                            preamble.step,
                            preamble.near,
                            preamble.far,
                        )
                    } else {
                        None
                    };
                    let (depth, confidence) = sweep_depth(
                        gray,
                        preamble.reference,
                        &mut sources,
                        preamble.width,
                        preamble.height,
                        preamble.step,
                        preamble.near,
                        preamble.far,
                        ranges.as_ref(),
                        &cpu_options,
                        &mut diagnostics,
                        &mut |row| {
                            cancelled(
                                progress,
                                "depth",
                                view_number * options.max_side + row,
                                active.len() * options.max_side,
                            )
                        },
                    )?;
                    maps.push(DepthMap {
                        image: index,
                        width: preamble.width,
                        height: preamble.height,
                        step: preamble.step,
                        depth,
                        confidence,
                        neighbors: preamble.neighbors,
                    });
                }
            }
        }
        cancelled(
            progress,
            "depth",
            active.len() * options.max_side,
            active.len() * options.max_side,
        )?;
    }
    Ok((maps, diagnostics))
}

/// Plain-data sweep payload of one view for host execution (browser WebGPU).
/// Grayscale rasters are not duplicated here; the host reads them per image
/// through `HostSweepView::ref_image` / `HostSweepSource::image`.
pub struct HostSweepView {
    pub patch_radius: usize,
    pub ref_image: usize,
    pub map_width: usize,
    pub map_height: usize,
    pub step: f64,
    pub needed: usize,
    pub ref_focal: f64,
    pub ref_cx: f64,
    pub ref_cy: f64,
    pub hypotheses: Vec<f64>,
    pub sources: Vec<HostSweepSource>,
}
pub struct HostSweepSource {
    pub image: usize,
    pub rotation: [[f64; 3]; 3],
    pub translation: [f64; 3],
    pub focal: f64,
    pub cx: f64,
    pub cy: f64,
}

/// Kernel-side bookkeeping retained between `prepare_host_views` and
/// `finish_host_views`; opaque to callers, which only store and return it.
pub struct PreparedView {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) step: f64,
    pub(super) near: f64,
    pub(super) far: f64,
    pub(super) ranges: Option<SweepRanges>,
    pub(super) neighbors: Vec<usize>,
}

/// Host-side payloads indexed by image slot. Keeping this relationship named
/// avoids exposing a nested tuple through the dense pipeline API.
pub type HostSweepPreparation = (Vec<Option<HostSweepView>>, Vec<Option<PreparedView>>);

impl PreparedView {
    /// Score-buffer length per hypothesis count: map pixels (all, including
    /// boundary rows that the shader fills with -1).
    pub fn map_area(&self) -> usize {
        self.width * self.height
    }
}

/// Per-view payloads for the host NCC sweep, indexed by image. Frontoparallel
/// only; `dense::prepare_host_sweep` rejects ineligible option sets first.
pub fn prepare_host_views(
    images: &[Image],
    sparse: &Reconstruction,
    options: &DenseOptions,
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<HostSweepPreparation> {
    let active: Vec<_> = sparse
        .cameras
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.as_ref().map(|c| (i, c)))
        .collect();
    let mut views: Vec<Option<HostSweepView>> = (0..images.len()).map(|_| None).collect();
    let mut prepared: Vec<Option<PreparedView>> = (0..images.len()).map(|_| None).collect();
    for (view_number, &(index, reference)) in active.iter().enumerate() {
        cancelled(
            progress,
            "depth",
            view_number * options.max_side,
            active.len() * options.max_side,
        )?;
        let Some(preamble) = view_preamble(images, sparse, options, index, reference) else {
            continue;
        };
        let hypotheses = hypothesis_grid(preamble.near, preamble.far, options.depth_hypotheses);
        let needed = preamble
            .neighbors
            .len()
            .min(2)
            .max(options.min_support_views);
        let ranges = if options.sparse_depth_prior {
            sparse_intervals(
                sparse,
                index,
                reference,
                preamble.width,
                preamble.height,
                preamble.step,
                preamble.near,
                preamble.far,
            )
        } else {
            None
        };
        let sources = preamble
            .neighbors
            .iter()
            .map(|&j| {
                let camera = sparse.cameras[j].as_ref().unwrap();
                let (rotation, translation) = source_transform(reference, camera);
                HostSweepSource {
                    image: j,
                    rotation,
                    translation,
                    focal: camera.focal,
                    cx: camera.cx,
                    cy: camera.cy,
                }
            })
            .collect();
        views[index] = Some(HostSweepView {
            patch_radius: options.patch_radius,
            ref_image: index,
            map_width: preamble.width,
            map_height: preamble.height,
            step: preamble.step,
            needed,
            ref_focal: reference.focal,
            ref_cx: reference.cx,
            ref_cy: reference.cy,
            hypotheses: hypotheses[..options.depth_hypotheses].to_vec(),
            sources,
        });
        prepared[index] = Some(PreparedView {
            width: preamble.width,
            height: preamble.height,
            step: preamble.step,
            near: preamble.near,
            far: preamble.far,
            ranges,
            neighbors: preamble.neighbors,
        });
    }
    Ok((views, prepared))
}
