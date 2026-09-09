use crate::{
    bundle,
    diagnostics::{ImageReport, ReconstructionReport},
    matching::{AssociationVotes, MatchGraph},
};
use crate::{
    camera::{self, triangulate, Camera},
    features::{self, Feature},
    math::*,
    Image, Point, Reconstruction, Result,
};
fn pixel(f: &Feature) -> [f64; 2] {
    [f.x, f.y]
}
fn color(image: &Image, f: &Feature) -> [u8; 3] {
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
            if error == "Cancelled" && !image.registered {
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
fn optimize_geometry(
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
                return Err("Cancelled".into());
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
    if !(32..=4000).contains(&options.feature_limit)
        || !(1..=128).contains(&options.max_seed_pairs)
        || !(1..=8).contains(&options.max_seed_attempts)
    {
        for image in &mut report.images {
            image.reason = "invalid_options";
        }
        return Err("Invalid reconstruction options".into());
    }
    if let Err(error) = options.geometry_options.validate() {
        for image in &mut report.images {
            image.reason = "invalid_options";
        }
        return Err(error);
    }
    if images.len() < 2 || images.len() > 200 {
        return Err("Provide 2 to 200 overlapping photos".into());
    }
    if let Some(bundle) = &options.bundle {
        if let Err(error) = bundle.validate() {
            for image in &mut report.images {
                image.reason = "invalid_options";
            }
            return Err(error);
        }
        if images.len() > bundle.max_cameras {
            for image in &mut report.images {
                image.reason = "invalid_options";
            }
            return Err(format!("Joint refinement is configured for at most {} photos; increase its camera limit up to 64 or explicitly disable it", bundle.max_cameras));
        }
    }
    for (i, image) in images.iter().enumerate() {
        if let Err(error) = image.validate() {
            report.images[i].reason = "invalid_image";
            return Err(error);
        }
    }
    let mut features = Vec::new();
    for (i, image) in images.iter().enumerate() {
        if !progress("features", i, images.len()) {
            return Err("Cancelled".into());
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
    let mut cache = MatchGraph::with_options(options.feature_options);
    let mut seeds = crate::seeding::propose_with_options(
        images,
        &features,
        &mut cache,
        options.max_seed_pairs,
        &options.geometry_options,
        &options.seed_options,
        report,
        progress,
    )?;
    let mut best: Option<(Reconstruction, ReconstructionReport)> = None;
    let mut trials = Vec::new();
    let mut last_error = "Could not initialize 3D: use sharper overlapping views with camera translation, texture, and correct focal lengths".to_string();
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
            &features,
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
                if error == "Cancelled" {
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
                    error: Some(error.clone()),
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
fn grow(
    images: &[Image],
    features: &[Vec<Feature>],
    cache: &mut MatchGraph,
    seed: crate::seeding::Seed,
    options: &ReconstructionOptions,
    report: &mut ReconstructionReport,
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<Reconstruction> {
    let crate::seeding::Seed {
        a,
        b,
        pose,
        inliers,
        matches,
        ..
    } = seed;
    report.initial_pair = Some([a, b]);
    report.images[a].registered = true;
    report.images[b].registered = true;
    let mut cameras = vec![None; images.len()];
    cameras[a] = Some(images[a].camera());
    cameras[b] = Some(pose);
    let mut points = Vec::<Point>::new();
    let mut tracks: Vec<Vec<Option<usize>>> =
        features.iter().map(|f| vec![None; f.len()]).collect();
    for i in inliers {
        let m = matches[i];
        let p = triangulate(&[
            (cameras[a].as_ref().unwrap(), pixel(&features[a][m.a])),
            (cameras[b].as_ref().unwrap(), pixel(&features[b][m.b])),
        ]);
        if let Some(position) = p {
            let id = points.len();
            points.push(Point {
                position,
                color: color(&images[a], &features[a][m.a]),
                observations: vec![(a, m.a), (b, m.b)],
            });
            tracks[a][m.a] = Some(id);
            tracks[b][m.b] = Some(id);
        }
    }
    let mut ba_positions = Vec::new();
    let mut ba_observations = Vec::new();
    loop {
        let registered = cameras.iter().filter(|c| c.is_some()).count();
        if !progress("cameras", registered, images.len()) {
            return Err("Cancelled".into());
        }
        let mut candidates = Vec::new();
        for i in 0..images.len() {
            if cameras[i].is_some() {
                continue;
            }
            let mut votes = AssociationVotes::default();
            for j in 0..images.len() {
                if cameras[j].is_none() {
                    continue;
                }
                if !progress("register_matches", i, images.len()) {
                    return Err("Cancelled".into());
                }
                for m in cache.between(features, j, i) {
                    if let Some(point) = tracks[j][m.a] {
                        votes.add(m.b, point);
                    }
                }
            }
            let (associations, rejected) = votes.resolve();
            report.images[i].conflicting_matches = rejected;
            report.images[i].candidate_correspondences = associations.len();
            candidates.push((associations.len(), i, associations));
        }
        candidates.sort_by_key(|a| std::cmp::Reverse(a.0));
        let mut added = false;
        for (_, i, associations) in candidates {
            if associations.len() < 8 {
                report.images[i].reason = "insufficient_correspondences";
                continue;
            }
            report.images[i].reason = "pose_rejected";
            let pairs: Vec<_> = associations
                .iter()
                .map(|(&f, &p)| (points[p].position, pixel(&features[i][f])))
                .collect();
            let mut guesses: Vec<_> = cameras
                .iter()
                .enumerate()
                .filter_map(|(j, c)| {
                    c.clone().map(|c| {
                        (
                            associations
                                .values()
                                .filter(|&&p| {
                                    points[p]
                                        .observations
                                        .iter()
                                        .any(|&(camera, _)| camera == j)
                                })
                                .count(),
                            c,
                        )
                    })
                })
                .collect();
            guesses.sort_by_key(|a| std::cmp::Reverse(a.0));
            let mut recovered = None;
            for (_, mut guess) in guesses.into_iter().take(3) {
                if !progress("pose", i, images.len()) {
                    return Err("Cancelled".into());
                }
                let intrinsic = images[i].camera();
                guess.focal = intrinsic.focal;
                guess.cx = intrinsic.cx;
                guess.cy = intrinsic.cy;
                report.images[i].pose_attempts += 1;
                if let Some(c) = camera::pnp_with_options(
                    &guess,
                    &pairs,
                    &options.geometry_options,
                    &mut camera::RobustReport::default(),
                ) {
                    recovered = Some(c);
                    break;
                }
            }
            let Some(c) = recovered else {
                continue;
            };
            let accepted = associations
                .iter()
                .filter_map(|(&f, &p)| {
                    c.project(points[p].position)
                        .filter(|uv| {
                            let q = pixel(&features[i][f]);
                            (uv[0] - q[0]).powi(2) + (uv[1] - q[1]).powi(2) < 16.
                        })
                        .map(|_| (f, p))
                })
                .collect::<Vec<_>>();
            if accepted.len() < 8 {
                continue;
            }
            for (f, p) in accepted {
                tracks[i][f] = Some(p);
                points[p].observations.push((i, f));
            }
            cameras[i] = Some(c);
            report.images[i].registered = true;
            report.images[i].reason = "registered";
            let ci_center = cameras[i].as_ref().unwrap().center();
            for j in 0..images.len() {
                if j == i || cameras[j].is_none() {
                    continue;
                }
                if !progress("register_matches", i, images.len()) {
                    return Err("Cancelled".into());
                }
                let cj_center = cameras[j].as_ref().unwrap().center();
                for m in cache.between(features, j, i) {
                    let (cj, ci) = (cameras[j].as_ref().unwrap(), cameras[i].as_ref().unwrap());
                    let (x, y) = (pixel(&features[j][m.a]), pixel(&features[i][m.b]));
                    if tracks[j][m.a].is_some() || tracks[i][m.b].is_some() {
                        continue;
                    }
                    let Some(p) = triangulate(&[(cj, x), (ci, y)]) else {
                        continue;
                    };
                    let parallax = dot(unit(sub(p, cj_center)), unit(sub(p, ci_center)))
                        .clamp(-1., 1.)
                        .acos();
                    if parallax < 0.008 {
                        continue;
                    }
                    if [(cj, x), (ci, y)].iter().any(|(c, q)| {
                        c.project(p)
                            .is_none_or(|uv| (uv[0] - q[0]).powi(2) + (uv[1] - q[1]).powi(2) > 9.)
                    }) {
                        continue;
                    }
                    let id = points.len();
                    points.push(Point {
                        position: p,
                        color: color(&images[i], &features[i][m.b]),
                        observations: vec![(j, m.a), (i, m.b)],
                    });
                    tracks[j][m.a] = Some(id);
                    tracks[i][m.b] = Some(id);
                }
            }
            added = true;
            break;
        }
        if !added {
            break;
        }
        if let Some(bundle) = &options.bundle {
            optimize_geometry(
                &mut cameras,
                &mut points,
                features,
                [a, b],
                bundle,
                report,
                progress,
                &mut ba_positions,
                &mut ba_observations,
            )?;
        }
    }
    if let Some(bundle) = &options.bundle {
        optimize_geometry(
            &mut cameras,
            &mut points,
            features,
            [a, b],
            bundle,
            report,
            progress,
            &mut ba_positions,
            &mut ba_observations,
        )?;
        // The track table is dead past this point, so filtered tracks can leave.
        if bundle.filter.is_some() {
            points.retain(|p| p.observations.len() >= 2);
        }
    }
    finalize(cameras, points, features, report)
}
fn finalize(
    mut cameras: Vec<Option<Camera>>,
    mut points: Vec<Point>,
    features: &[Vec<Feature>],
    report: &mut ReconstructionReport,
) -> Result<Reconstruction> {
    points.retain(|point| {
        point.observations.iter().all(|&(i, f)| {
            cameras[i]
                .as_ref()
                .and_then(|c| c.project(point.position))
                .is_some_and(|uv| {
                    let q = pixel(&features[i][f]);
                    (uv[0] - q[0]).powi(2) + (uv[1] - q[1]).powi(2) <= 25.
                })
        })
    });
    // Pruning points can invalidate a previously accepted camera. Iterate to a fixed
    // point so neither registration counts nor dense view selection uses unsupported poses.
    loop {
        let mut support = vec![0; cameras.len()];
        for point in &points {
            for &(i, _) in &point.observations {
                support[i] += 1;
            }
        }
        let mut changed = false;
        for (i, camera) in cameras.iter_mut().enumerate() {
            if camera.is_some() && support[i] < 8 {
                *camera = None;
                report.images[i].reason = "insufficient_consistent_observations";
                changed = true;
            }
        }
        if !changed {
            break;
        }
        for point in &mut points {
            point.observations.retain(|&(i, _)| cameras[i].is_some());
        }
        points.retain(|p| p.observations.len() >= 2);
    }
    let mut squared = 0.;
    let mut count = 0;
    for point in &points {
        for &(i, f) in &point.observations {
            let uv = cameras[i]
                .as_ref()
                .unwrap()
                .project(point.position)
                .unwrap();
            let q = pixel(&features[i][f]);
            squared += (uv[0] - q[0]).powi(2) + (uv[1] - q[1]).powi(2);
            count += 1;
        }
    }
    for (i, camera) in cameras.iter().enumerate() {
        let r = &mut report.images[i];
        r.registered = camera.is_some();
        r.accepted_observations = points
            .iter()
            .filter(|p| p.observations.iter().any(|&(j, _)| i == j))
            .count();
        if r.registered {
            r.reason = "registered";
        } else if r.reason == "features_ready" {
            r.reason = "not_connected";
        }
    }
    if points.len() < 12 || cameras.iter().flatten().count() < 2 {
        return Err("Too few geometrically consistent points".into());
    }
    Ok(Reconstruction {
        input_images: cameras.len(),
        cameras,
        points,
        reprojection_rmse: (squared / count as f64).sqrt(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Six known cameras and sixty points with mild noise and ~8% outlier
    /// observations; poses and points are returned distorted, truths aside.
    fn outlier_scene() -> (
        Vec<Option<Camera>>,
        Vec<Point>,
        Vec<Vec<Feature>>,
        Vec<V3>,
        Vec<Option<Camera>>,
    ) {
        let mut truth_cameras = Vec::new();
        for i in 0..6 {
            let mut camera = Camera::identity(700., 480., 320.);
            camera.rotation = rotation([0.01 * i as f64, -0.04 * i as f64, 0.005 * i as f64]);
            let center = [0.6 * i as f64, 0.1 * (i as f64).sin(), 0.06 * i as f64];
            camera.translation = scale(mv(camera.rotation, center), -1.);
            truth_cameras.push(Some(camera));
        }
        let truth: Vec<V3> = (0..60)
            .map(|i| {
                [
                    (i % 10) as f64 * 0.15 - 0.6,
                    (i / 10) as f64 * 0.14 - 0.4,
                    4. + ((i * 37) % 13) as f64 * 0.08,
                ]
            })
            .collect();
        let mut features = vec![Vec::<Feature>::new(); 6];
        let mut points = Vec::new();
        for (p, &position) in truth.iter().enumerate() {
            let mut observations = Vec::new();
            for (c, camera) in truth_cameras.iter().enumerate() {
                let mut xy = camera.as_ref().unwrap().project(position).unwrap();
                xy[0] += 0.1 * ((p * 7 + c * 3) as f64).sin();
                xy[1] += 0.1 * ((p * 5 + c * 11) as f64).cos();
                if (p * 6 + c) % 41 == 0 {
                    xy[0] += 28.;
                    xy[1] -= 19.;
                }
                observations.push((c, features[c].len()));
                features[c].push(Feature {
                    x: xy[0],
                    y: xy[1],
                    descriptor: [0.; 128],
                });
            }
            points.push(Point {
                position,
                color: [0; 3],
                observations,
            });
        }
        let mut cameras = truth_cameras.clone();
        for (i, camera) in cameras.iter_mut().enumerate().skip(1) {
            let original = camera.as_ref().unwrap();
            let mut delta = [0.005, -0.008, 0.003, 0.02, -0.015, 0.03];
            delta[0] *= i as f64;
            let rotated = rotation([delta[0], delta[1], delta[2]]);
            let mut updated = original.clone();
            updated.rotation = mm(rotated, original.rotation);
            updated.translation = add(mv(rotated, original.translation), [delta[3], delta[4], delta[5]]);
            if i == 1 {
                let center = scale(
                    updated.center(),
                    norm(original.center()) / norm(updated.center()),
                );
                updated.translation = scale(mv(updated.rotation, center), -1.);
            }
            *camera = Some(updated);
        }
        for (i, point) in points.iter_mut().enumerate() {
            point.position[0] += 0.03 * (i as f64 * 1.7).sin();
            point.position[1] += 0.025 * (i as f64 * 2.3).cos();
            point.position[2] += 0.06 * (i as f64 * 0.8).sin();
        }
        (cameras, points, features, truth, truth_cameras)
    }

    /// Two BA passes, as the pipeline runs one after each registration step;
    /// with the filter enabled the first pass prunes outliers for the second.
    fn optimize_twice(
        cameras: &mut Vec<Option<Camera>>,
        points: &mut Vec<Point>,
        features: &[Vec<Feature>],
        options: &bundle::BundleOptions,
    ) -> ReconstructionReport {
        let mut report = ReconstructionReport::default();
        let mut positions = Vec::new();
        let mut observations = Vec::new();
        for _ in 0..2 {
            optimize_geometry(
                cameras,
                points,
                features,
                [0, 1],
                options,
                &mut report,
                &mut |_, _, _| true,
                &mut positions,
                &mut observations,
            )
            .unwrap();
        }
        report
    }

    /// RMSE over the observations that were clean before refinement.
    fn clean_rmse(cameras: &[Option<Camera>], points: &[Point], features: &[Vec<Feature>]) -> f64 {
        let mut squared = 0.;
        let mut count = 0;
        for (p, point) in points.iter().enumerate() {
            for &(c, f) in &point.observations {
                if (p * 6 + c) % 41 == 0 {
                    continue;
                }
                let uv = cameras[c].as_ref().unwrap().project(point.position).unwrap();
                squared += (uv[0] - features[c][f].x).powi(2) + (uv[1] - features[c][f].y).powi(2);
                count += 1;
            }
        }
        (squared / count as f64).sqrt()
    }

    #[test]
    fn disabled_filter_keeps_every_observation() {
        let (mut cameras, mut points, features, _, _) = outlier_scene();
        let counts: Vec<usize> = points.iter().map(|p| p.observations.len()).collect();
        let report = optimize_twice(
            &mut cameras,
            &mut points,
            &features,
            &bundle::BundleOptions::default(),
        );
        assert_eq!(report.bundle_runs.len(), 2);
        assert!(report
            .bundle_runs
            .iter()
            .all(|run| run.filtered_observations == 0 && run.filtered_tracks == 0));
        let after: Vec<usize> = points.iter().map(|p| p.observations.len()).collect();
        assert_eq!(counts, after);
    }

    #[test]
    fn enabled_filter_removes_outliers_deterministically_and_improves_rmse() {
        let options = bundle::BundleOptions {
            filter: Some(bundle::FilterOptions {
                max_reprojection_error: 4.,
                min_parallax: 0.,
            }),
            ..Default::default()
        };
        let (mut off_cameras, mut off_points, features, _, _) = outlier_scene();
        let off_report = optimize_twice(
            &mut off_cameras,
            &mut off_points,
            &features,
            &bundle::BundleOptions::default(),
        );
        let off_rmse = clean_rmse(&off_cameras, &off_points, &features);
        let (mut cameras, mut points, features, _, _) = outlier_scene();
        let report = optimize_twice(&mut cameras, &mut points, &features, &options);
        let on_rmse = clean_rmse(&cameras, &points, &features);
        assert_eq!(report.bundle_runs.len(), 2);
        assert!(report.bundle_runs[0].filtered_observations > 0);
        assert!(
            on_rmse < off_rmse,
            "filtered clean RMSE {on_rmse}, unfiltered {off_rmse}"
        );
        let (mut again_cameras, mut again_points, features, _, _) = outlier_scene();
        let again = optimize_twice(&mut again_cameras, &mut again_points, &features, &options);
        assert_eq!(format!("{cameras:?}"), format!("{again_cameras:?}"));
        assert_eq!(format!("{points:?}"), format!("{again_points:?}"));
        assert_eq!(
            format!("{:?}", report.bundle_runs),
            format!("{:?}", again.bundle_runs)
        );
        assert!(off_report
            .bundle_runs
            .iter()
            .all(|run| run.filtered_observations == 0));
        eprintln!(
            "pipeline filter: removed {} observations / {} tracks, clean RMSE {off_rmse} -> {on_rmse}",
            report.bundle_runs[0].filtered_observations,
            report.bundle_runs[0].filtered_tracks,
        );
    }
    #[test]
    fn final_camera_counts_require_retained_geometric_support() {
        let cameras = (0..3)
            .map(|i| {
                let mut c = Camera::identity(100., 32., 32.);
                c.translation = [i as f64 * -0.1, 0., 0.];
                Some(c)
            })
            .collect::<Vec<_>>();
        let mut features = vec![Vec::new(); 3];
        let points = (0..20)
            .map(|p| {
                let position = [p as f64 * 0.02, 0., 4.];
                let mut observations = Vec::new();
                for i in 0..3 {
                    let uv = cameras[i].as_ref().unwrap().project(position).unwrap();
                    if i == 2 && p >= 4 {
                        continue;
                    }
                    observations.push((i, features[i].len()));
                    features[i].push(Feature {
                        x: uv[0],
                        y: uv[1],
                        descriptor: [0.; 128],
                    });
                }
                Point {
                    position,
                    color: [0; 3],
                    observations,
                }
            })
            .collect();
        let mut report = ReconstructionReport {
            images: (0..3)
                .map(|image| ImageReport {
                    image,
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        let result = finalize(cameras, points, &features, &mut report).unwrap();
        assert_eq!(result.cameras.iter().flatten().count(), 2);
        assert!(result.cameras[2].is_none());
        assert_eq!(result.points.len(), 20);
        assert!(result.points.iter().all(|p| p.observations.len() == 2));
        assert_eq!(
            report.images[2].reason,
            "insufficient_consistent_observations"
        );
        assert_eq!(report.images[2].accepted_observations, 0);
    }
    #[test]
    fn failed_diagnostics_distinguish_cancellation_input_and_texture() {
        let image = Image {
            width: 64,
            height: 64,
            rgb: vec![0; 64 * 64 * 3],
            focal: 100.,
        };
        let cancelled = reconstruct_detailed(
            &[image.clone(), image.clone()],
            &ReconstructionOptions::default(),
            |_, _, _| false,
        );
        assert_eq!(cancelled.report.images[0].reason, "cancelled");
        let mut invalid = image.clone();
        invalid.rgb.clear();
        let failed = reconstruct_detailed(
            &[invalid, image.clone()],
            &ReconstructionOptions::default(),
            |_, _, _| true,
        );
        assert_eq!(failed.report.images[0].reason, "invalid_image");
        assert_eq!(failed.report.images[1].reason, "not_processed");
        let blank = reconstruct_detailed(
            &[image.clone(), image],
            &ReconstructionOptions::default(),
            |_, _, _| true,
        );
        assert!(blank.reconstruction.is_err());
        assert!(blank
            .report
            .images
            .iter()
            .all(|i| i.reason == "insufficient_features"));
    }
}
