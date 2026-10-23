use super::*;

/// Six known cameras and sixty points with mild noise and ~8% outlier
/// observations; poses and points are returned distorted, truths aside.
#[expect(clippy::type_complexity, reason = "compact deterministic bundle-adjustment fixture")]
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
        updated.translation = add(
            mv(rotated, original.translation),
            [delta[3], delta[4], delta[5]],
        );
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
    cameras: &mut [Option<Camera>],
    points: &mut [Point],
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
            let uv = cameras[c]
                .as_ref()
                .unwrap()
                .project(point.position)
                .unwrap();
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
    assert!(
        report
            .bundle_runs
            .iter()
            .all(|run| run.filtered_observations == 0 && run.filtered_tracks == 0)
    );
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
    assert!(
        off_report
            .bundle_runs
            .iter()
            .all(|run| run.filtered_observations == 0)
    );
    eprintln!(
        "pipeline filter: removed {} observations / {} tracks, clean RMSE {off_rmse} -> {on_rmse}",
        report.bundle_runs[0].filtered_observations, report.bundle_runs[0].filtered_tracks,
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
    assert!(
        blank
            .report
            .images
            .iter()
            .all(|i| i.reason == "insufficient_features")
    );
}
