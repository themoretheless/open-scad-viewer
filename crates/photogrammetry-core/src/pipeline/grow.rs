use super::*;

pub(super) fn grow(
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
            return Err(crate::error("Cancelled"));
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
                    return Err(crate::error("Cancelled"));
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
            // One pass over the candidate points: for each point, bump every
            // camera that observes it (at most once per point, matching the
            // previous `any(...)` semantics). Enumerate order and the stable
            // reverse-count sort keep the selection, ties included, identical.
            let mut overlap = vec![0usize; cameras.len()];
            for &p in associations.values() {
                let mut seen: Vec<usize> = Vec::new();
                for &(camera, _) in &points[p].observations {
                    if !seen.contains(&camera) {
                        seen.push(camera);
                        overlap[camera] += 1;
                    }
                }
            }
            let mut guesses: Vec<_> = cameras
                .iter()
                .enumerate()
                .filter_map(|(j, c)| c.clone().map(|c| (overlap[j], c)))
                .collect();
            guesses.sort_by_key(|a| std::cmp::Reverse(a.0));
            let mut recovered = None;
            for (_, mut guess) in guesses.into_iter().take(3) {
                if !progress("pose", i, images.len()) {
                    return Err(crate::error("Cancelled"));
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
                    return Err(crate::error("Cancelled"));
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
pub(super) fn finalize(
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
    // One pass over the surviving points: for each point, bump every camera
    // that observes it (at most once per point, matching `any(...)`).
    let mut accepted = vec![0usize; cameras.len()];
    for point in &points {
        let mut seen: Vec<usize> = Vec::new();
        for &(i, _) in &point.observations {
            if !seen.contains(&i) {
                seen.push(i);
                accepted[i] += 1;
            }
        }
    }
    for (i, camera) in cameras.iter().enumerate() {
        let r = &mut report.images[i];
        r.registered = camera.is_some();
        r.accepted_observations = accepted[i];
        if r.registered {
            r.reason = "registered";
        } else if r.reason == "features_ready" {
            r.reason = "not_connected";
        }
    }
    if points.len() < 12 || cameras.iter().flatten().count() < 2 {
        return Err(crate::error("Too few geometrically consistent points"));
    }
    Ok(Reconstruction {
        input_images: cameras.len(),
        cameras,
        points,
        reprojection_rmse: (squared / count as f64).sqrt(),
    })
}
