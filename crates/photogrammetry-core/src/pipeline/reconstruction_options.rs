use super::*;

pub(super) fn pixel(f: &Feature) -> [f64; 2] {
    [f.x, f.y]
}
pub(super) fn color(image: &Image, f: &Feature) -> [u8; 3] {
    let i = ((f.y as usize).min(image.height - 1) * image.width
        + (f.x as usize).min(image.width - 1))
        * 3;
    image.rgb[i..i + 3].try_into().unwrap()
}
#[derive(Clone, Debug)]
pub struct ReconstructionOptions {
    pub feature_limit: usize,
    pub feature_options: features::FeatureOptions,
    pub geometry_options: camera::GeometryOptions,
    pub max_seed_pairs: usize,
    pub max_seed_attempts: usize,
    /// Opt-in seed selection extensions; the default reproduces the original ranking.
    pub seed_options: crate::seeding::SeedOptions,
    /// None explicitly disables joint refinement. Native callers may configure up to 64 cameras.
    pub bundle: Option<bundle::BundleOptions>,
}
impl Default for ReconstructionOptions {
    fn default() -> Self {
        Self {
            feature_limit: 900,
            feature_options: features::FeatureOptions::default(),
            geometry_options: camera::GeometryOptions::default(),
            max_seed_pairs: 12,
            max_seed_attempts: 4,
            seed_options: crate::seeding::SeedOptions::default(),
            bundle: Some(bundle::BundleOptions {
                max_cameras: 64,
                ..Default::default()
            }),
        }
    }
}
pub struct ReconstructionOutcome {
    pub reconstruction: Result<Reconstruction>,
    pub report: ReconstructionReport,
}
/// Keeps diagnostics even when reconstruction cannot initialize.
pub fn reconstruct_detailed(
    images: &[Image],
    options: &ReconstructionOptions,
    mut progress: impl FnMut(&str, usize, usize) -> bool,
) -> ReconstructionOutcome {
    let mut report = ReconstructionReport {
        images: (0..images.len())
            .map(|image| ImageReport {
                image,
                reason: "not_processed",
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    };
    let reconstruction = run(images, options, &mut report, &mut progress);
    if let Err(error) = &reconstruction {
        for image in &mut report.images {
            if error.message == "Cancelled" && !image.registered {
                image.reason = "cancelled";
            } else if image.reason == "features_ready" {
                image.reason = "initialization_failed";
            }
        }
    }
    ReconstructionOutcome {
        reconstruction,
        report,
    }
}

/// Sparse reconstruction with every seed-candidate pair pre-matched externally
/// (browser WebGPU round trip, see `host_matching`). The match cache is filled
/// before seeding, so no descriptor matching runs on the host; features come
/// from the plan so extraction never runs twice. Eligibility (pair-grid
/// coverage, no `second_chance`) is decided in `host_matching::prepare_host_matching`.
pub fn reconstruct_detailed_with_matches(
    images: &[Image],
    options: &ReconstructionOptions,
    plan: &crate::host_matching::HostMatchingPlan,
    response: &[u8],
    mut progress: impl FnMut(&str, usize, usize) -> bool,
) -> ReconstructionOutcome {
    let mut report = ReconstructionReport {
        images: (0..images.len())
            .map(|image| ImageReport {
                image,
                reason: "not_processed",
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    };
    let reconstruction = try {
        validate_inputs(images, options, &mut report)?;
        let pair_matches =
            crate::host_matching::matches_from_gpu_response(plan, response, &options.feature_options)?;
        let mut cache = MatchGraph::with_options(options.feature_options);
        for ((lo, hi), matches) in pair_matches {
            cache.precompute(lo, hi, matches);
        }
        solve(
            images,
            &plan.features,
            options,
            cache,
            &mut report,
            &mut progress,
        )?
    };
    if let Err(error) = &reconstruction {
        for image in &mut report.images {
            if error.message == "Cancelled" && !image.registered {
                image.reason = "cancelled";
            } else if image.reason == "features_ready" {
                image.reason = "initialization_failed";
            }
        }
    }
    ReconstructionOutcome {
        reconstruction,
        report,
    }
}
#[expect(
    clippy::too_many_arguments,
    reason = "pipeline state is passed explicitly across the solver boundary; a typed optimization context is the next modularization step"
)]
pub(super) fn optimize_geometry(
    cameras: &mut [Option<Camera>],
    points: &mut [Point],
    features: &[Vec<Feature>],
    seed_pair: [usize; 2],
    options: &bundle::BundleOptions,
    report: &mut ReconstructionReport,
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
    positions: &mut Vec<V3>,
    observations: &mut Vec<bundle::Observation>,
) -> Result<()> {
    positions.clear();
    positions.extend(points.iter().map(|p| p.position));
    observations.clear();
    for (point, p) in points.iter().enumerate() {
        // Tracks pruned by a previous filter pass keep their `points` slot (the
        // caller's track table references it) but must not reach the solver again.
        if options.filter.is_some() && p.observations.len() < 2 {
            continue;
        }
        for &(camera, f) in &p.observations {
            observations.push(bundle::Observation {
                camera,
                point,
                xy: pixel(&features[camera][f]),
            });
        }
    }
    let mut cancelled = false;
    match bundle::optimize(
        cameras,
        positions,
        observations,
        seed_pair[0],
        seed_pair[1],
        options,
        |event| {
            let keep = progress("bundle", event.iteration, options.max_iterations);
            cancelled |= !keep;
            keep
        },
    ) {
        Ok(mut summary) => {
            for (point, position) in points.iter_mut().zip(positions.iter().copied()) {
                point.position = position;
            }
            if let Some(filter) = &options.filter {
                let keep = bundle::outlier_mask(cameras, positions, observations, filter);
                // Flat observations are grouped by live point in ascending order,
                // so each point's slice of the mask aligns with its observation list.
                let mut offset = 0;
                for p in points.iter_mut() {
                    let n = p.observations.len();
                    if n < 2 {
                        continue;
                    }
                    let kept = keep[offset..offset + n].iter().filter(|&&k| k).count();
                    if kept < n {
                        summary.filtered_observations += n - kept;
                        summary.filtered_tracks += usize::from(kept == 0);
                        let mut index = offset;
                        let mut retained = std::mem::take(&mut p.observations);
                        retained.retain(|_| {
                            let kept = keep[index];
                            index += 1;
                            kept
                        });
                        p.observations = retained;
                    }
                    offset += n;
                }
            }
            report.bundle_runs.push(summary);
        }
        Err(error) => {
            if cancelled {
                return Err(crate::error("Cancelled"));
            }
            let warning = format!("Bundle adjustment skipped: {error}");
            if !report.warnings.contains(&warning) {
                report.warnings.push(warning);
            }
        }
    }
    Ok(())
}
/// Reconstruct connected views. Unregistered photos remain explicit `None` entries.
/// Progress is called between bounded stages; return false to cancel.
pub fn reconstruct(
    images: &[Image],
    progress: impl FnMut(&str, usize, usize) -> bool,
) -> Result<Reconstruction> {
    reconstruct_detailed(images, &ReconstructionOptions::default(), progress).reconstruction
}
fn run(
    images: &[Image],
    options: &ReconstructionOptions,
    report: &mut ReconstructionReport,
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<Reconstruction> {
    validate_inputs(images, options, report)?;
    let mut features = Vec::new();
    for (i, image) in images.iter().enumerate() {
        if !progress("features", i, images.len()) {
            do yeet crate::error("Cancelled");
        }
        features.push(features::extract_with_options(
            image,
            options.feature_limit,
            &options.feature_options,
        )?);
        report.images[i].features = features[i].len();
        report.images[i].reason = if features[i].len() < 12 {
            "insufficient_features"
        } else {
            "features_ready"
        };
    }
    let cache = MatchGraph::with_options(options.feature_options);
    solve(images, &features, options, cache, report, progress)
}

/// Validates options and images before any stage runs (shared by the lazy
/// matcher and the host-GPU pre-matched entries).
fn validate_inputs(
    images: &[Image],
    options: &ReconstructionOptions,
    report: &mut ReconstructionReport,
) -> Result<()> {
    if !(32..=4000).contains(&options.feature_limit)
        || !(1..=128).contains(&options.max_seed_pairs)
        || !(1..=8).contains(&options.max_seed_attempts)
    {
        for image in &mut report.images {
            image.reason = "invalid_options";
        }
        do yeet crate::error("Invalid reconstruction options");
    }
    if let Err(error) = options.geometry_options.validate() {
        for image in &mut report.images {
            image.reason = "invalid_options";
        }
        do yeet error;
    }
    if images.len() < 2 || images.len() > 200 {
        do yeet crate::error("Provide 2 to 200 overlapping photos");
    }
    if let Some(bundle) = &options.bundle {
        if let Err(error) = bundle.validate() {
            for image in &mut report.images {
                image.reason = "invalid_options";
            }
            do yeet error;
        }
        if images.len() > bundle.max_cameras {
            for image in &mut report.images {
                image.reason = "invalid_options";
            }
            do yeet crate::error(format!(
                "Joint refinement is configured for at most {} photos; increase its camera limit up to 64 or explicitly disable it",
                bundle.max_cameras
            ));
        }
    }
    for (i, image) in images.iter().enumerate() {
        if let Err(error) = image.validate() {
            report.images[i].reason = "invalid_image";
            do yeet error;
        }
    }
    Ok(())
}

/// Seeding and incremental registration over a (possibly pre-filled) match
/// cache; descriptor extraction and cache construction happen in the caller.
fn solve(
    images: &[Image],
    features: &[Vec<Feature>],
    options: &ReconstructionOptions,
    mut cache: MatchGraph,
    report: &mut ReconstructionReport,
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<Reconstruction> {
    let mut seeds = crate::seeding::propose_with_options(
        images,
        features,
        &mut cache,
        options.max_seed_pairs,
        &options.geometry_options,
        &options.seed_options,
        report,
        progress,
    )?;
    let mut best: Option<(Reconstruction, ReconstructionReport)> = None;
    let mut trials = Vec::new();
    let mut last_error = crate::error(
        "Could not initialize 3D: use sharper overlapping views with camera translation, texture, and correct focal lengths",
    );
    for _ in 0..options.max_seed_attempts {
        if seeds.is_empty() {
            break;
        }
        // When a component stalls, favor a seed containing a still-unregistered view.
        // Features and pair matches are shared across attempts; geometry is transactional.
        let next = seeds
            .iter()
            .position(|seed| {
                best.as_ref()
                    .is_none_or(|(r, _)| r.cameras[seed.a].is_none() || r.cameras[seed.b].is_none())
            })
            .unwrap_or(0);
        let seed = seeds.remove(next);
        let pair = [seed.a, seed.b];
        let mut trial_report = report.clone();
        let result = grow(
            images,
            features,
            &mut cache,
            seed,
            options,
            &mut trial_report,
            progress,
        );
        match result {
            Ok(reconstruction) => {
                let registered = reconstruction
                    .cameras
                    .iter()
                    .filter(|c| c.is_some())
                    .count();
                trials.push(crate::diagnostics::SeedTrialReport {
                    pair,
                    registered_images: registered,
                    points: reconstruction.points.len(),
                    reprojection_rmse: Some(reconstruction.reprojection_rmse),
                    error: None,
                });
                let better = best.as_ref().is_none_or(|(old, _)| {
                    registered > old.cameras.iter().filter(|c| c.is_some()).count()
                        || (registered == old.cameras.iter().filter(|c| c.is_some()).count()
                            && reconstruction.points.len() > old.points.len())
                });
                if better {
                    best = Some((reconstruction, trial_report));
                }
                if registered == images.len() {
                    break;
                }
            }
            Err(error) => {
                if error.message == "Cancelled" {
                    report.seed_trials = trials;
                    report.matching_requests = cache.requests;
                    report.computed_pairs = cache.computed_pairs;
                    return Err(error);
                }
                trials.push(crate::diagnostics::SeedTrialReport {
                    pair,
                    registered_images: 0,
                    points: 0,
                    reprojection_rmse: None,
                    error: Some(error.message.clone()),
                });
                last_error = error;
            }
        }
    }
    let result = if let Some((reconstruction, selected)) = best {
        *report = selected;
        Ok(reconstruction)
    } else {
        Err(last_error)
    };
    report.seed_trials = trials;
    report.matching_requests = cache.requests;
    report.computed_pairs = cache.computed_pairs;
    result
}
