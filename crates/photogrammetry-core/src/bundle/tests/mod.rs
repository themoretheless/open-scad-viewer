use super::*;
use crate::math::ID;

fn scene(noise: bool, outliers: bool) -> (Vec<Option<Camera>>, Vec<V3>, Vec<Observation>) {
    let mut cameras = Vec::new();
    for i in 0..4 {
        let mut camera = Camera::identity(700. + 30. * i as f64, 480., 320.);
        camera.rotation = rotation([0.01 * i as f64, -0.045 * i as f64, 0.005 * i as f64]);
        let center = [0.65 * i as f64, 0.12 * (i as f64).sin(), 0.07 * i as f64];
        camera.translation = scale(mv(camera.rotation, center), -1.);
        cameras.push(Some(camera));
    }
    let points: Vec<_> = (0..120)
        .map(|i| {
            [
                (i % 12) as f64 * 0.18 - 0.6,
                (i / 12) as f64 * 0.16 - 0.7,
                4. + ((i * 37) % 19) as f64 * 0.095,
            ]
        })
        .collect();
    let mut observations = Vec::new();
    for (point, &position) in points.iter().enumerate() {
        for (camera, pose) in cameras.iter().enumerate() {
            let mut xy = pose.as_ref().unwrap().project(position).unwrap();
            if noise {
                xy[0] += 0.12 * ((point * 7 + camera * 3) as f64).sin();
                xy[1] += 0.12 * ((point * 5 + camera * 11) as f64).cos();
            }
            if outliers && (point * 4 + camera) % 43 == 0 {
                xy[0] += 35.;
                xy[1] -= 24.;
            }
            observations.push(Observation { camera, point, xy });
        }
    }
    (cameras, points, observations)
}

fn distort(cameras: &mut [Option<Camera>], points: &mut [V3]) {
    for (i, camera) in cameras.iter_mut().enumerate().skip(1) {
        let original = camera.as_ref().unwrap();
        let mut updated = perturb(original, &[0.006, -0.009, 0.004, 0.025, -0.02, 0.04]);
        // Preserve seed baseline in the initialization so known-scale shape error
        // measures recovery, not the unavoidable gauge ambiguity of photographs.
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
        point[0] += 0.04 * (i as f64 * 1.7).sin();
        point[1] += 0.03 * (i as f64 * 2.3).cos();
        point[2] += 0.07 * (i as f64 * 0.8).sin();
    }
}

#[test]
fn schur_elimination_matches_full_normal_equations() {
    let (mut cameras, mut points, mut observations) = scene(true, false);
    points.truncate(8);
    observations.retain(|observation| observation.point < points.len());
    distort(&mut cameras, &mut points);
    let options = BundleOptions::default();
    let mut keep_running = |_| true;
    let mut checkpoints = Checkpoints::new(&mut keep_running);
    let layout = validate(
        &cameras,
        &points,
        &observations,
        0,
        1,
        &options,
        &mut checkpoints,
    )
    .unwrap();
    let linear = linearize(
        &cameras,
        &points,
        &observations,
        &layout,
        options.huber_delta,
        &mut checkpoints,
    )
    .unwrap();
    let damping = 0.003;
    let step = schur_step(
        &linear,
        damping,
        &mut SchurScratch::default(),
        &mut checkpoints,
    )
    .unwrap();
    let nc = layout.dimension;
    let n = nc + linear.points.len() * 3;
    let mut h = vec![0.; n * n];
    let mut rhs = vec![0.; n];
    for i in 0..nc {
        rhs[i] = linear.camera_rhs[i];
        for j in 0..nc {
            h[i * n + j] = linear.camera_hessian[i * nc + j];
        }
    }
    for (index, point) in linear.points.iter().enumerate() {
        let column = nc + index * 3;
        for i in 0..3 {
            rhs[column + i] = point.rhs[i];
            for j in 0..3 {
                h[(column + i) * n + column + j] = point.hessian[i][j];
            }
        }
        for cross in &linear.cross[point.cross.clone()] {
            for i in 0..6 {
                for j in 0..3 {
                    h[(cross.column + i) * n + column + j] = cross.values[i][j];
                    h[(column + j) * n + cross.column + i] = cross.values[i][j];
                }
            }
        }
    }
    for i in 0..n {
        h[i * n + i] += damping * h[i * n + i].max(1e-9);
    }
    solve_reduced(&mut h, &mut rhs, &mut Vec::new(), &mut checkpoints).unwrap();
    let full = rhs;
    for (actual, expected) in step
        .cameras
        .iter()
        .chain(step.points.iter().flatten())
        .zip(full)
    {
        assert!(
            (actual - expected).abs() < 1e-8,
            "Schur {actual}, full {expected}"
        );
    }
}

#[test]
fn disconnected_and_single_view_tracks_are_rejected() {
    let (mut cameras, mut points, observations) = scene(false, false);
    let disconnected: Vec<_> = observations
        .iter()
        .copied()
        .filter(|o| (o.point < 60) == (o.camera < 2))
        .collect();
    assert!(
        optimize(
            &mut cameras,
            &mut points,
            &disconnected,
            0,
            1,
            &BundleOptions::default(),
            |_| true
        )
        .unwrap_err()
        .message
        .contains("disconnected")
    );
    let single_view: Vec<_> = observations
        .iter()
        .copied()
        .filter(|o| o.point != 0 || o.camera == 0)
        .collect();
    assert!(
        optimize(
            &mut cameras,
            &mut points,
            &single_view,
            0,
            1,
            &BundleOptions::default(),
            |_| true
        )
        .unwrap_err()
        .message
        .contains("two distinct")
    );
}

#[test]
fn pure_rotation_tracks_are_rejected_even_with_distinct_camera_ids() {
    let (mut cameras, mut points, mut observations) = scene(false, false);
    cameras[2] = cameras[0].clone();
    let camera = cameras[2].as_mut().unwrap();
    camera.rotation = rotation([0., 0.03, 0.]);
    // Other points still connect a valid-baseline camera; only point0 is depth-degenerate.
    observations.retain(|o| o.point != 0 || o.camera == 0 || o.camera == 2);
    for o in &mut observations {
        if o.camera == 2 {
            o.xy = camera.project(points[o.point]).unwrap();
        }
    }
    assert!(
        optimize(
            &mut cameras,
            &mut points,
            &observations,
            0,
            1,
            &BundleOptions::default(),
            |_| true
        )
        .unwrap_err()
        .message
        .contains("parallax")
    );
}

#[test]
fn rejected_steps_leave_an_exact_reconstruction_unchanged() {
    let (mut cameras, mut points, observations) = scene(false, false);
    let original_points = points.clone();
    let original_cameras = format!("{cameras:?}");
    let report = optimize(
        &mut cameras,
        &mut points,
        &observations,
        0,
        1,
        &BundleOptions::default(),
        |_| true,
    )
    .unwrap();
    assert_eq!(report.initial_cost, 0.);
    assert_eq!(report.accepted_steps, 0);
    assert_eq!(points, original_points);
    assert_eq!(format!("{cameras:?}"), original_cameras);
}

#[test]
fn analytic_jacobians_match_central_differences() {
    let camera = Camera {
        rotation: rotation([0.16, -0.08, 0.04]),
        translation: [-0.7, 0.2, 0.4],
        focal: 730.,
        cx: 480.,
        cy: 320.,
    };
    let point = [0.6, -0.4, 4.2];
    let (jc, jp) = jacobians(&camera, projection(&camera, point).unwrap().1);
    let epsilon = 1e-6;
    for column in 0..6 {
        let mut delta = [0.; 6];
        delta[column] = epsilon;
        let plus = perturb(&camera, &delta).project(point).unwrap();
        delta[column] = -epsilon;
        let minus = perturb(&camera, &delta).project(point).unwrap();
        for row in 0..2 {
            let numerical = (plus[row] - minus[row]) / (2. * epsilon);
            assert!(
                (numerical - jc[row][column]).abs() < 1e-5,
                "camera {row},{column}: {numerical} != {}",
                jc[row][column]
            );
        }
    }
    for column in 0..3 {
        let mut plus = point;
        let mut minus = point;
        plus[column] += epsilon;
        minus[column] -= epsilon;
        let plus = camera.project(plus).unwrap();
        let minus = camera.project(minus).unwrap();
        for row in 0..2 {
            assert!(((plus[row] - minus[row]) / (2. * epsilon) - jp[row][column]).abs() < 1e-5);
        }
    }
}

#[test]
fn joint_refinement_recovers_nonplanar_shape_and_holds_gauge() {
    let (mut cameras, truth, observations) = scene(true, false);
    let mut points = truth.clone();
    distort(&mut cameras, &mut points);
    let anchor = cameras[0].as_ref().unwrap().clone();
    let baseline = norm(cameras[1].as_ref().unwrap().center());
    let initial_error: f64 = points
        .iter()
        .zip(&truth)
        .map(|(&a, &b)| norm(sub(a, b)))
        .sum::<f64>()
        / truth.len() as f64;
    let report = optimize(
        &mut cameras,
        &mut points,
        &observations,
        0,
        1,
        &BundleOptions::default(),
        |_| true,
    )
    .unwrap();
    let final_error = points
        .iter()
        .zip(&truth)
        .map(|(&a, &b)| norm(sub(a, b)))
        .sum::<f64>()
        / truth.len() as f64;
    assert!(report.final_cost < report.initial_cost * 0.01, "{report:?}");
    assert!(
        final_error < initial_error * 0.2 && final_error < 0.01,
        "{initial_error} -> {final_error}"
    );
    assert_eq!(cameras[0].as_ref().unwrap().rotation, anchor.rotation);
    assert_eq!(cameras[0].as_ref().unwrap().translation, anchor.translation);
    assert!((norm(cameras[1].as_ref().unwrap().center()) - baseline).abs() < 1e-12);
    eprintln!(
        "bundle synthetic: cost {} -> {}, mean shape error {} -> {}, accepted {}",
        report.initial_cost,
        report.final_cost,
        initial_error,
        final_error,
        report.accepted_steps
    );
}

#[test]
fn robust_loss_reduces_outlier_damage() {
    let (mut cameras, truth, observations) = scene(true, true);
    let mut points = truth.clone();
    distort(&mut cameras, &mut points);
    let mut least_squares_cameras = cameras.clone();
    let mut least_squares_points = points.clone();
    let report = optimize(
        &mut cameras,
        &mut points,
        &observations,
        0,
        1,
        &BundleOptions {
            max_iterations: 20,
            ..Default::default()
        },
        |_| true,
    )
    .unwrap();
    optimize(
        &mut least_squares_cameras,
        &mut least_squares_points,
        &observations,
        0,
        1,
        &BundleOptions {
            max_iterations: 20,
            huber_delta: 1e6,
            ..Default::default()
        },
        |_| true,
    )
    .unwrap();
    let clean_error = |positions: &[V3]| {
        let mut errors: Vec<_> = positions
            .iter()
            .zip(&truth)
            .enumerate()
            .filter(|(i, _)| (0..4).all(|c| (i * 4 + c) % 43 != 0))
            .map(|(_, (&a, &b))| norm(sub(a, b)))
            .collect();
        errors.sort_by(f64::total_cmp);
        errors[errors.len() / 2]
    };
    let robust_error = clean_error(&points);
    let ordinary_error = clean_error(&least_squares_points);
    assert!(report.final_cost < report.initial_cost * 0.4, "{report:?}");
    assert!(
        robust_error < ordinary_error * 0.5,
        "Huber median shape error {robust_error}, ordinary least squares {ordinary_error}"
    );
    eprintln!(
        "bundle outliers: Huber median shape error {robust_error}, ordinary least squares {ordinary_error}"
    );
}

#[test]
fn cancellation_precedes_preparation_and_preserves_inputs() {
    let (mut cameras, mut points, mut observations) = scene(false, false);
    observations[0].point = usize::MAX;
    let before_cameras = format!("{cameras:?}");
    let before_points = points.clone();
    let mut calls = 0;
    let error = optimize(
        &mut cameras,
        &mut points,
        &observations,
        0,
        1,
        &BundleOptions::default(),
        |event| {
            calls += 1;
            assert_eq!(event.iteration, 0);
            assert_eq!(event.cost, 0.);
            false
        },
    )
    .unwrap_err();
    assert_eq!(error.message, "Cancelled");
    assert_eq!(calls, 1);
    assert_eq!(points, before_points);
    assert_eq!(format!("{cameras:?}"), before_cameras);
}

#[test]
fn cancellation_inside_the_first_linearization_is_transactional() {
    let (mut cameras, mut points, observations) = scene(false, false);
    distort(&mut cameras, &mut points);
    let before_cameras = format!("{cameras:?}");
    let before_points = points.clone();
    let mut numerical_checkpoints = 0;
    let error = optimize(
        &mut cameras,
        &mut points,
        &observations,
        0,
        1,
        &BundleOptions::default(),
        |event| {
            assert_eq!(event.iteration, 0);
            if event.cost > 0. {
                numerical_checkpoints += 1;
            }
            numerical_checkpoints < 3
        },
    )
    .unwrap_err();
    assert_eq!(error.message, "Cancelled");
    assert_eq!(numerical_checkpoints, 3);
    assert_eq!(points, before_points);
    assert_eq!(format!("{cameras:?}"), before_cameras);
}

#[test]
fn schur_elimination_checks_cancellation_between_point_blocks() {
    let (cameras, points, observations) = scene(true, false);
    let mut keep_running = |_| true;
    let mut checkpoints = Checkpoints::new(&mut keep_running);
    let options = BundleOptions::default();
    let layout = validate(
        &cameras,
        &points,
        &observations,
        0,
        1,
        &options,
        &mut checkpoints,
    )
    .unwrap();
    let linear = linearize(
        &cameras,
        &points,
        &observations,
        &layout,
        options.huber_delta,
        &mut checkpoints,
    )
    .unwrap();
    let mut calls = 0;
    let mut cancel_during_elimination = |_| {
        calls += 1;
        calls < 2
    };
    let mut checkpoints = Checkpoints::new(&mut cancel_during_elimination);
    assert!(
        schur_step(
            &linear,
            options.initial_damping,
            &mut SchurScratch::default(),
            &mut checkpoints
        )
        .is_none()
    );
    assert!(checkpoints.cancelled);
    assert_eq!(calls, 2);
}

#[test]
fn cancellation_is_transactional_after_an_accepted_step() {
    let (mut cameras, mut points, observations) = scene(false, false);
    distort(&mut cameras, &mut points);
    let initial_cameras = format!("{cameras:?}");
    let initial_points = points.clone();
    let mut last_cost = f64::INFINITY;
    let mut saw_improvement = false;
    let error = optimize(
        &mut cameras,
        &mut points,
        &observations,
        0,
        1,
        &BundleOptions::default(),
        |p| {
            saw_improvement |= p.cost < last_cost && last_cost.is_finite();
            last_cost = p.cost;
            p.iteration < 1
        },
    )
    .unwrap_err();
    assert_eq!(error.message, "Cancelled");
    assert!(saw_improvement);
    assert_eq!(points, initial_points);
    assert_eq!(format!("{cameras:?}"), initial_cameras);
}

#[test]
fn invalid_and_degenerate_inputs_do_not_mutate() {
    let (mut cameras, mut points, mut observations) = scene(false, false);
    let initial_points = points.clone();
    cameras[1] = cameras[0].clone();
    assert!(
        optimize(
            &mut cameras,
            &mut points,
            &observations,
            0,
            1,
            &BundleOptions::default(),
            |_| true
        )
        .unwrap_err()
        .message
        .contains("baseline")
    );
    let (mut cameras, _, _) = scene(false, false);
    observations.push(observations[0]);
    assert!(
        optimize(
            &mut cameras,
            &mut points,
            &observations,
            0,
            1,
            &BundleOptions::default(),
            |_| true
        )
        .unwrap_err()
        .message
        .contains("Duplicate")
    );
    observations.pop();
    points[0][2] = -1.;
    assert!(
        optimize(
            &mut cameras,
            &mut points,
            &observations,
            0,
            1,
            &BundleOptions::default(),
            |_| true
        )
        .unwrap_err()
        .message
        .contains("depth")
    );
    points = initial_points.clone();
    let initial_cameras = format!("{cameras:?}");
    let options = BundleOptions {
        max_cameras: 3,
        ..Default::default()
    };
    assert!(
        optimize(
            &mut cameras,
            &mut points,
            &observations,
            0,
            1,
            &options,
            |_| true
        )
        .is_err()
    );
    assert_eq!(points, initial_points);
    assert_eq!(format!("{cameras:?}"), initial_cameras);
}

#[test]
fn unobserved_data_and_nonidentity_anchor_are_preserved() {
    let (mut cameras, mut points, observations) = scene(false, false);
    let shift = [3., -2., 0.7];
    let world_rotation = rotation([0.07, 0.08, -0.13]);
    for camera in cameras.iter_mut().flatten() {
        camera.rotation = mm(camera.rotation, tr(world_rotation));
        camera.translation = sub(camera.translation, mv(camera.rotation, shift));
    }
    for point in &mut points {
        *point = add(mv(world_rotation, *point), shift);
    }
    let anchor = cameras[0].as_ref().unwrap().clone();
    let baseline = norm(sub(cameras[1].as_ref().unwrap().center(), anchor.center()));
    for point in &mut points {
        point[2] += 0.01;
    }
    points.push([100., 200., -300.]);
    cameras.push(Some(Camera {
        rotation: ID,
        translation: [8., 4., -9.],
        focal: 900.,
        cx: 0.,
        cy: 0.,
    }));
    let unused_camera = format!("{:?}", cameras.last());
    let report = optimize(
        &mut cameras,
        &mut points,
        &observations,
        0,
        1,
        &BundleOptions::default(),
        |_| true,
    )
    .unwrap();
    assert!(report.final_cost < report.initial_cost);
    assert_eq!(points.last().unwrap(), &[100., 200., -300.]);
    assert_eq!(format!("{:?}", cameras.last()), unused_camera);
    assert_eq!(cameras[0].as_ref().unwrap().rotation, anchor.rotation);
    assert_eq!(cameras[0].as_ref().unwrap().translation, anchor.translation);
    assert!(
        (norm(sub(cameras[1].as_ref().unwrap().center(), anchor.center())) - baseline).abs()
            < 1e-12
    );
}

#[test]
fn filter_options_are_validated_only_when_enabled() {
    assert!(BundleOptions::default().validate().is_ok());
    let options = |max_reprojection_error: f64, min_parallax: f64| BundleOptions {
        filter: Some(FilterOptions {
            max_reprojection_error,
            min_parallax,
        }),
        ..Default::default()
    };
    assert!(options(4., 0.).validate().is_ok());
    assert!(options(2., 0.01).validate().is_ok());
    for bad in [0., -1., f64::NAN, f64::INFINITY, 100.5] {
        assert!(options(bad, 0.).validate().is_err(), "{bad}");
    }
    for bad in [-0.1, 1.1, f64::NAN] {
        assert!(options(4., bad).validate().is_err(), "{bad}");
    }
}

#[test]
fn outlier_mask_drops_only_large_errors_deterministically() {
    let (cameras, points, mut observations) = scene(true, true);
    let filter = FilterOptions {
        max_reprojection_error: 4.,
        min_parallax: 0.,
    };
    let expected: Vec<bool> = observations
        .iter()
        .map(|o| (o.point * 4 + o.camera) % 43 != 0)
        .collect();
    let first = outlier_mask(&cameras, &points, &observations, &filter);
    assert_eq!(
        first,
        outlier_mask(&cameras, &points, &observations, &filter)
    );
    assert_eq!(first, expected);
    let report = filter_observations(&cameras, &points, &mut observations, &filter);
    assert_eq!(
        report.observations_removed,
        expected.iter().filter(|&&k| !k).count()
    );
    assert_eq!(report.tracks_removed, 0);
    assert_eq!(observations.len(), expected.iter().filter(|&&k| k).count());
    // Every surviving observation stays within the threshold.
    for observation in &observations {
        let uv = cameras[observation.camera]
            .as_ref()
            .unwrap()
            .project(points[observation.point])
            .unwrap();
        let error = (uv[0] - observation.xy[0]).hypot(uv[1] - observation.xy[1]);
        assert!(error <= 4., "{error}");
    }
}

#[test]
fn filter_drops_tracks_reduced_below_two_views_or_parallax() {
    let (cameras, points, _) = scene(false, false);
    // Point 0 keeps only camera 0 within the threshold: a corrupted camera 1
    // observation is dropped, leaving a single view, so the track goes too.
    let mut observations = Vec::new();
    for (point, position) in points.iter().enumerate().take(8) {
        for (camera, pose) in cameras.iter().enumerate() {
            let mut xy = pose.as_ref().unwrap().project(*position).unwrap();
            if point == 0 && camera > 0 {
                xy[0] += 30.;
            }
            observations.push(Observation { camera, point, xy });
        }
    }
    let filter = FilterOptions {
        max_reprojection_error: 4.,
        min_parallax: 0.,
    };
    let mut retained = observations.clone();
    let report = filter_observations(&cameras, &points, &mut retained, &filter);
    assert_eq!(report.observations_removed, 4);
    assert_eq!(report.tracks_removed, 1);
    assert!(retained.iter().all(|o| o.point != 0));
    // A parallax floor above the widest baseline angle drops every track.
    let mut retained = observations;
    let report = filter_observations(
        &cameras,
        &points,
        &mut retained,
        &FilterOptions {
            max_reprojection_error: 400.,
            min_parallax: 0.9,
        },
    );
    assert_eq!(report.tracks_removed, 8);
    assert!(retained.is_empty());
}

#[test]
fn filtering_between_runs_improves_outlier_recovery() {
    let (mut cameras, truth, observations) = scene(true, true);
    let mut points = truth.clone();
    distort(&mut cameras, &mut points);
    let mut single_pass_cameras = cameras.clone();
    let mut single_pass_points = points.clone();
    optimize(
        &mut single_pass_cameras,
        &mut single_pass_points,
        &observations,
        0,
        1,
        &BundleOptions {
            max_iterations: 20,
            ..Default::default()
        },
        |_| true,
    )
    .unwrap();
    let filter = FilterOptions {
        max_reprojection_error: 4.,
        min_parallax: 0.,
    };
    optimize(
        &mut cameras,
        &mut points,
        &observations,
        0,
        1,
        &BundleOptions {
            max_iterations: 20,
            ..Default::default()
        },
        |_| true,
    )
    .unwrap();
    let mut retained = observations.clone();
    let removed = filter_observations(&cameras, &points, &mut retained, &filter);
    assert!(removed.observations_removed > 0);
    let second = optimize(
        &mut cameras,
        &mut points,
        &retained,
        0,
        1,
        &BundleOptions {
            max_iterations: 20,
            ..Default::default()
        },
        |_| true,
    )
    .unwrap();
    let clean_error = |positions: &[V3]| {
        let mut errors: Vec<_> = positions
            .iter()
            .zip(&truth)
            .enumerate()
            .filter(|(i, _)| (0..4).all(|c| (i * 4 + c) % 43 != 0))
            .map(|(_, (&a, &b))| norm(sub(a, b)))
            .collect();
        errors.sort_by(f64::total_cmp);
        errors[errors.len() / 2]
    };
    let filtered_error = clean_error(&points);
    let unfiltered_error = clean_error(&single_pass_points);
    assert!(
        filtered_error < unfiltered_error,
        "filtered median {filtered_error}, unfiltered {unfiltered_error}"
    );
    eprintln!(
        "bundle filter: removed {} observations / {} tracks, clean median shape error {unfiltered_error} -> {filtered_error}, second run cost {}",
        removed.observations_removed, removed.tracks_removed, second.final_cost,
    );
}
