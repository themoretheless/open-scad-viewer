use super::*;
#[expect(clippy::type_complexity, reason = "compact deterministic camera fixture")]
fn scene() -> (Camera, Camera, Vec<(V3, [f64; 2], [f64; 2])>) {
    let a = Camera::identity(600., 320., 240.);
    let mut b = a.clone();
    b.rotation = rotation([0.03, 0.08, -0.01]);
    b.translation = [-0.8, 0.03, 0.04];
    let mut rng = Rng::new();
    let p = (0..80)
        .map(|_| {
            let p = [
                rng.next(1000) as f64 / 400. - 1.2,
                rng.next(1000) as f64 / 500. - 1.,
                3. + rng.next(1000) as f64 / 300.,
            ];
            (p, a.project(p).unwrap(), b.project(p).unwrap())
        })
        .collect();
    (a, b, p)
}
#[test]
fn shared_projection_rejects_nonfinite_coordinates_and_intrinsics() {
    let camera = Camera::identity(700., 320., 240.);
    let point = [0.5, -0.25, 4.];
    assert_eq!(
        camera.project(point),
        camera.project_camera_point(camera.camera_point(point))
    );
    for invalid in [[f64::NAN, 0., 1.], [0., f64::INFINITY, 1.], [0., 0., -1.]] {
        assert!(camera.project(invalid).is_none());
        assert!(camera.project_camera_point(invalid).is_none());
    }
    let mut invalid = camera;
    invalid.focal = f64::INFINITY;
    assert!(invalid.project(point).is_none());
}

#[test]
fn triangulation_recovers_depth() {
    let (a, b, points) = scene();
    for (p, x, y) in points {
        let q = triangulate(&[(&a, x), (&b, y)]).unwrap();
        assert!(norm(sub(p, q)) < 1e-8);
    }
}
#[test]
fn robust_pose_with_outliers() {
    let (a, b, points) = scene();
    let mut pairs: Vec<_> = points.iter().map(|&(_, x, y)| (x, y)).collect();
    for (i, p) in pairs.iter_mut().take(15).enumerate() {
        p.1 = [(i * 31 % 600) as f64, (i * 53 % 400) as f64];
    }
    let (c, good) = relative(&a, &b, &pairs).expect("relative pose");
    assert!(good.len() > 55, "{}", good.len());
    assert!(norm(sub(unit(c.translation), unit(b.translation))) < 0.04);
    for i in 0..3 {
        assert!(norm(sub(c.rotation[i], b.rotation[i])) < 0.03);
    }
}
#[test]
fn resection_recovers_camera() {
    let (_, b, points) = scene();
    let pairs: Vec<_> = points.iter().map(|&(p, _, y)| (p, y)).collect();
    let c = pnp(&Camera::identity(600., 320., 240.), &pairs).unwrap();
    assert!(norm(sub(c.translation, b.translation)) < 1e-3);
}
#[test]
fn mixed_focal_sampson_has_pixel_units_and_is_symmetric() {
    let a = Camera::identity(280., 320., 240.);
    let b = Camera::identity(1500., 320., 240.);
    let r = rotation([0.03, 0.08, -0.01]);
    let t = [-0.8, 0.03, 0.04];
    let e = mm([[0., -t[2], t[1]], [t[2], 0., -t[0]], [-t[1], t[0], 0.]], r);
    let (x, y) = ([411.4, 301.2], [124.7, 516.3]);
    let constraint = |x, y| dot(b.ray(y), mv(e, a.ray(x)));
    let epsilon = 1e-4;
    let mut gradient_norm = 0.;
    for side in 0..2 {
        for k in 0..2 {
            let (mut xx, mut yy) = (x, y);
            if side == 0 {
                xx[k] += epsilon;
            } else {
                yy[k] += epsilon;
            }
            gradient_norm += ((constraint(xx, yy) - constraint(x, y)) / epsilon).powi(2);
        }
    }
    let measured = pixel_sampson(
        e,
        tr(e),
        a.ray(x),
        b.ray(y),
        a.focal.powi(2),
        b.focal.powi(2),
    );
    let numerical = constraint(x, y).powi(2) / gradient_norm;
    assert!((measured / numerical - 1.).abs() < 1e-6);
    let reversed = pixel_sampson(
        tr(e),
        e,
        b.ray(y),
        a.ray(x),
        b.focal.powi(2),
        a.focal.powi(2),
    );
    assert!((measured - reversed).abs() < 1e-10);
}
#[test]
fn robust_estimators_reject_nonfinite_correspondences_without_panics() {
    let camera = Camera::identity(600., 320., 240.);
    let pixels = vec![([f64::NAN, 0.], [0., 0.]); 20];
    let mut report = RobustReport::default();
    assert!(
        relative_with_options(
            &camera,
            &camera,
            &pixels,
            &GeometryOptions::ROBUST,
            &mut report
        )
        .is_none()
    );
    assert_eq!(report.iterations, 0);
    let points = vec![([0., 0., 4.], [f64::INFINITY, 0.]); 20];
    assert!(
        pnp_with_options(&camera, &points, &GeometryOptions::ROBUST, &mut report).is_none()
    );
    assert_eq!(report.iterations, 0);
}
#[test]
fn low_support_does_not_become_a_successful_seed() {
    let (a, b, points) = scene();
    let mut pixels = points.iter().map(|&(_, x, y)| (x, y)).collect::<Vec<_>>();
    for (i, pair) in pixels.iter_mut().enumerate().take(48) {
        pair.1 = [(i * 79 % 640) as f64, (i * 107 % 480) as f64];
    }
    let mut report = RobustReport::default();
    let pose = relative_with_options(&a, &b, &pixels, &GeometryOptions::default(), &mut report);
    assert!(pose.is_none());
    assert!(report.insufficient_support || report.inliers < 12);
}
fn mixed_pose_holdout_error(options: GeometryOptions, outliers: usize) -> f64 {
    let (_, mut b, points) = scene();
    let a = Camera::identity(280., 320., 240.);
    b.focal = 1500.;
    let pixels = points[..60]
        .iter()
        .enumerate()
        .map(|(i, (p, _, _))| {
            let x = a.project(*p).unwrap();
            let mut y = b.project(*p).unwrap();
            if i < outliers {
                y = [(i * 79 % 640) as f64, (i * 107 % 480) as f64];
            }
            (x, y)
        })
        .collect::<Vec<_>>();
    let (pose, good) =
        relative_with_options(&a, &b, &pixels, &options, &mut RobustReport::default())
            .expect("a supported pose");
    assert!(good.len() >= 45);
    let error = points[60..]
        .iter()
        .map(|(p, _, _)| {
            let projected = pose.project(scale(*p, 1. / norm(b.translation))).unwrap();
            let expected = b.project(*p).unwrap();
            (projected[0] - expected[0]).powi(2) + (projected[1] - expected[1]).powi(2)
        })
        .sum::<f64>();
    (error / 20.).sqrt()
}
#[test]
fn clean_mixed_focal_pose_predicts_unused_world_points() {
    assert!(mixed_pose_holdout_error(GeometryOptions::default(), 0) < 0.1);
}
#[test]
fn epipolar_outlier_does_not_regress_against_frozen_estimator() {
    // One accidental epipolar-consistent outlier passes both estimators. This
    // is a comparative regression check, not an assertion of metric accuracy.
    let baseline = mixed_pose_holdout_error(GeometryOptions::BASELINE, 12);
    let current = mixed_pose_holdout_error(GeometryOptions::default(), 12);
    assert!(
        current <= baseline + 1e-6,
        "baseline {baseline}, current {current}"
    );
}
#[test]
fn relative_pose_rejects_nonidentity_first_camera_before_sampling() {
    let (a, b, points) = scene();
    for translated in [false, true] {
        let mut first = a.clone();
        if translated {
            first.translation = [0.2, 0., 0.];
        } else {
            first.rotation = rotation([0., 0.1, 0.]);
        }
        let pixels = points
            .iter()
            .map(|(p, _, y)| (first.project(*p).unwrap(), *y))
            .collect::<Vec<_>>();
        let mut report = RobustReport::default();
        assert!(
            relative_with_options(
                &first,
                &b,
                &pixels,
                &GeometryOptions::default(),
                &mut report
            )
            .is_none()
        );
        assert!(report.invalid_first_camera);
        assert_eq!(report.iterations, 0);
        assert_eq!(report.hypotheses, 0);
    }
}
#[test]
fn relative_pose_identity_tolerance_is_finite_and_accepts_roundoff() {
    let (mut a, b, points) = scene();
    let pixels = points.iter().map(|&(_, x, y)| (x, y)).collect::<Vec<_>>();
    a.rotation[0][0] += 5e-13;
    a.translation[0] = 5e-13;
    let mut report = RobustReport::default();
    assert!(
        relative_with_options(&a, &b, &pixels, &GeometryOptions::default(), &mut report)
            .is_some()
    );
    assert!(!report.invalid_first_camera);
    for (rotation_value, translation_value) in [
        (f64::NAN, 0.),
        (1., f64::INFINITY),
        (1. + 2e-12, 0.),
        (1., 2e-12),
    ] {
        a.rotation = ID;
        a.rotation[0][0] = rotation_value;
        a.translation = [translation_value, 0., 0.];
        assert!(
            relative_with_options(&a, &b, &pixels, &GeometryOptions::default(), &mut report)
                .is_none()
        );
        assert!(report.invalid_first_camera);
        assert_eq!(report.iterations, 0);
    }
}
/// Noisy non-planar synthetic scene with a known relative pose. Uniform
/// pixel noise plus a fixed share of scrambled matches, all from the
/// deterministic kernel Rng.
#[expect(clippy::type_complexity, reason = "compact deterministic camera fixture")]
fn noisy_scene(
    fa: f64,
    fb: f64,
    noise: f64,
    outliers: usize,
) -> (
    Camera,
    Camera,
    Vec<([f64; 2], [f64; 2])>,
    Vec<(V3, [f64; 2])>,
) {
    let a = Camera::identity(fa, 320., 240.);
    let mut b = Camera::identity(fb, 320., 240.);
    b.rotation = rotation([0.05, 0.12, -0.02]);
    b.translation = [-0.8, 0.05, 0.1];
    let mut rng = Rng::new();
    let mut train = Vec::new();
    for i in 0..240 {
        let p = [
            rng.next(1000) as f64 / 250. - 2.,
            rng.next(1000) as f64 / 300. - 1.6,
            2.5 + rng.next(1000) as f64 / 250.,
        ];
        let mut x = a.project(p).unwrap();
        let mut y = b.project(p).unwrap();
        x[0] += (rng.next(2000) as f64 / 1000. - 1.) * noise;
        x[1] += (rng.next(2000) as f64 / 1000. - 1.) * noise;
        y[0] += (rng.next(2000) as f64 / 1000. - 1.) * noise;
        y[1] += (rng.next(2000) as f64 / 1000. - 1.) * noise;
        if i < outliers {
            y = [(i * 79 % 640) as f64, (i * 137 % 480) as f64];
        }
        train.push((x, y));
    }
    let holdout = (0..60)
        .map(|_| {
            let p = [
                rng.next(1000) as f64 / 250. - 2.,
                rng.next(1000) as f64 / 300. - 1.6,
                2.5 + rng.next(1000) as f64 / 250.,
            ];
            (p, b.project(p).unwrap())
        })
        .collect();
    (a, b, train, holdout)
}
/// Reprojection RMSE on independent points at the known baseline scale.
fn holdout_rmse(pose: &Camera, b: &Camera, holdout: &[(V3, [f64; 2])]) -> f64 {
    let baseline_scale = 1. / norm(b.translation);
    (holdout
        .iter()
        .map(|(p, y)| {
            let uv = pose.project(scale(*p, baseline_scale)).unwrap();
            (uv[0] - y[0]).powi(2) + (uv[1] - y[1]).powi(2)
        })
        .sum::<f64>()
        / holdout.len() as f64)
        .sqrt()
}
fn rotation_error_deg(pose: &Camera, b: &Camera) -> f64 {
    let mut trace = 0.;
    for i in 0..3 {
        for j in 0..3 {
            trace += pose.rotation[i][j] * b.rotation[i][j];
        }
    }
    ((trace - 1.) / 2.).clamp(-1., 1.).acos().to_degrees()
}
#[test]
fn joint_refinement_is_deterministic_and_improves_noisy_pose() {
    let (a, b, train, holdout) = noisy_scene(800., 800., 0.6, 24);
    let frozen = relative_with_options(
        &a,
        &b,
        &train,
        &GeometryOptions::default(),
        &mut RobustReport::default(),
    )
    .expect("default pose");
    let joint = relative_with_options(
        &a,
        &b,
        &train,
        &GeometryOptions::JOINT,
        &mut RobustReport::default(),
    )
    .expect("joint pose");
    let repeated = relative_with_options(
        &a,
        &b,
        &train,
        &GeometryOptions::JOINT,
        &mut RobustReport::default(),
    )
    .expect("repeated joint pose");
    for (x, y) in joint
        .0
        .rotation
        .iter()
        .flatten()
        .chain(joint.0.translation.iter())
        .zip(
            repeated
                .0
                .rotation
                .iter()
                .flatten()
                .chain(repeated.0.translation.iter()),
        )
    {
        assert_eq!(
            x.to_bits(),
            y.to_bits(),
            "joint refinement must be deterministic"
        );
    }
    let default_rmse = holdout_rmse(&frozen.0, &b, &holdout);
    let joint_rmse = holdout_rmse(&joint.0, &b, &holdout);
    assert!(
        joint_rmse <= default_rmse,
        "joint {joint_rmse} should not regress against default {default_rmse}"
    );
    assert!(joint_rmse < 1.0, "joint holdout RMSE {joint_rmse}");
    assert!(
        rotation_error_deg(&joint.0, &b) <= rotation_error_deg(&frozen.0, &b) + 1e-9,
        "joint rotation error must not exceed the default"
    );
}
#[test]
fn analytic_epipolar_jacobian_tracks_finite_differences() {
    let (a, b, train, _) = noisy_scene(600., 600., 0.4, 12);
    let finite = relative_with_options(
        &a,
        &b,
        &train,
        &GeometryOptions::ROBUST,
        &mut RobustReport::default(),
    )
    .expect("finite-difference pose");
    let analytic = GeometryOptions {
        analytic_epipolar_jacobian: true,
        ..GeometryOptions::ROBUST
    };
    let exact = relative_with_options(&a, &b, &train, &analytic, &mut RobustReport::default())
        .expect("analytic pose");
    for i in 0..3 {
        assert!(
            norm(sub(exact.0.rotation[i], finite.0.rotation[i])) < 1e-4,
            "rotation row {i} diverged"
        );
    }
    assert!(norm(sub(exact.0.translation, finite.0.translation)) < 1e-4);
}
