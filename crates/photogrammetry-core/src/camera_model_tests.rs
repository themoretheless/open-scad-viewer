//! Synthetic multi-view checks for intrinsics bundle adjustment and pose verification.
use crate::bundle::{
    IntrinsicBundleOptions, IntrinsicBundleSetup, IntrinsicBundleState, IntrinsicsMask,
    Observation, Pose, optimize_intrinsics, project_calibrated,
};
use crate::calibration::Calibration;
use crate::camera::{PoseStatus, PoseVerificationOptions, verify_poses};
use crate::math::{V3, add, cross, mm, mv, norm, rotation, scale, sub, unit};

struct Rng(u64);
impl Rng {
    fn uniform(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }
    fn gauss(&mut self) -> f64 {
        let (a, b) = (self.uniform(), self.uniform());
        (-2. * a.ln()).sqrt() * (std::f64::consts::TAU * b).cos()
    }
}

fn truth() -> Calibration {
    Calibration {
        width: 640,
        height: 480,
        fx: 720.,
        fy: 705.,
        cx: 326.,
        cy: 236.,
        k1: -0.15,
        k2: 0.04,
        k3: 0.,
        p1: 0.0008,
        p2: -0.0006,
    }
}

fn look_at(center: V3) -> Pose {
    let z = unit(scale(center, -1.));
    let x = unit(cross([0., 1., 0.], z));
    let y = cross(z, x);
    let rotation = [x, y, z];
    Pose {
        rotation,
        translation: scale(mv(rotation, center), -1.),
    }
}

fn with_center(mut pose: Pose, center: V3) -> Pose {
    pose.translation = scale(mv(pose.rotation, center), -1.);
    pose
}

struct Scene {
    poses: Vec<Pose>,
    points: Vec<V3>,
    observations: Vec<Observation>,
}

/// Cameras on an arc looking at a cube of points, projected through `truth()` plus noise.
fn scene(cameras: usize, points: usize, noise: f64, seed: u64) -> Scene {
    let calibration = truth();
    let mut rng = Rng(seed);
    let poses: Vec<Pose> = (0..cameras)
        .map(|i| {
            let a = -0.9 + 1.8 * i as f64 / (cameras - 1) as f64;
            let pose = look_at([3.6 * a.sin(), 0.9 * (i % 3) as f64 - 0.9, -3.6 * a.cos()]);
            // Varied roll decorrelates the principal point from the orientations.
            let roll = Pose {
                rotation: mm(rotation([0., 0., 0.5 * i as f64 - 1.]), pose.rotation),
                ..pose
            };
            with_center(roll, pose.center())
        })
        .collect();
    let points: Vec<V3> = (0..points)
        .map(|_| [0, 1, 2].map(|_| 1.2 * rng.uniform() - 0.6))
        .collect();
    let mut observations = Vec::new();
    for (point, &p) in points.iter().enumerate() {
        for (camera, pose) in poses.iter().enumerate() {
            let q = project_calibrated(&calibration, pose, p).unwrap();
            assert!(q[0] > 0. && q[0] < 640. && q[1] > 0. && q[1] < 480.);
            observations.push(Observation {
                camera,
                point,
                xy: [q[0] + noise * rng.gauss(), q[1] + noise * rng.gauss()],
            });
        }
    }
    Scene {
        poses,
        points,
        observations,
    }
}

#[test]
fn intrinsic_bundle_recovers_perturbed_intrinsics_and_poses() {
    let truth = truth();
    let scene = scene(10, 250, 0.1, 11);
    let mut rng = Rng(5);
    let mut poses = scene.poses.clone();
    for (i, pose) in poses.iter_mut().enumerate().skip(1) {
        let center = pose.center();
        pose.rotation = mm(
            rotation([0, 1, 2].map(|_| 0.01 * rng.gauss())),
            pose.rotation,
        );
        // Camera 1 sets the scale gauge, so only its orientation is perturbed.
        let center = if i == 1 {
            center
        } else {
            add(center, [0, 1, 2].map(|_| 0.05 * rng.gauss()))
        };
        *pose = with_center(*pose, center);
    }
    let points = scene
        .points
        .iter()
        .map(|p| [0, 1, 2].map(|k| p[k] + 0.02 * rng.gauss()))
        .collect();
    let start = Calibration {
        fx: truth.fx * 1.04,
        fy: truth.fy * 0.97,
        cx: truth.cx + 8.,
        cy: truth.cy - 6.,
        k1: 0.,
        k2: 0.,
        p1: 0.,
        p2: 0.,
        ..truth.clone()
    };
    let mut state = IntrinsicBundleState {
        intrinsics: vec![start.clone()],
        poses,
        points,
    };
    let mut fixed_poses = vec![false; 10];
    fixed_poses[0] = true;
    let setup = IntrinsicBundleSetup {
        masks: vec![IntrinsicsMask::STANDARD],
        camera_intrinsics: vec![0; 10],
        fixed_poses,
        fixed_points: false,
        scale_gauge: Some([0, 1]),
    };
    let options = IntrinsicBundleOptions::default();

    // With intrinsics frozen the wrong focal/distortion keeps the residual high.
    let mut frozen_state = state.clone();
    let frozen_setup = IntrinsicBundleSetup {
        masks: vec![IntrinsicsMask::FIXED],
        ..setup.clone()
    };
    let frozen = optimize_intrinsics(
        &mut frozen_state,
        &frozen_setup,
        &scene.observations,
        &options,
    )
    .unwrap();
    assert_eq!(frozen_state.intrinsics[0], start);

    let report = optimize_intrinsics(&mut state, &setup, &scene.observations, &options).unwrap();
    let c = &state.intrinsics[0];
    assert!(report.initial_rms > 20. * report.final_rms, "{report:?}");
    assert!(report.final_rms < 0.15, "{report:?}");
    assert!(
        frozen.final_rms > 1.5 * report.final_rms,
        "frozen {} free {}",
        frozen.final_rms,
        report.final_rms
    );
    assert!((c.fx - truth.fx).abs() / truth.fx < 0.005, "fx {}", c.fx);
    assert!((c.fy - truth.fy).abs() / truth.fy < 0.005, "fy {}", c.fy);
    assert!((c.cx - truth.cx).abs() < 2., "cx {}", c.cx);
    assert!((c.cy - truth.cy).abs() < 2., "cy {}", c.cy);
    assert!((c.k1 - truth.k1).abs() < 0.02, "k1 {}", c.k1);
    assert!((c.p1 - truth.p1).abs() < 0.001 && (c.p2 - truth.p2).abs() < 0.001);
    for (estimate, expected) in state.poses.iter().zip(&scene.poses) {
        let error = norm(sub(estimate.center(), expected.center()));
        assert!(error < 0.02, "pose centre error {error}");
    }
}

#[test]
fn intrinsic_bundle_supports_per_camera_blocks_and_rejects_free_gauge() {
    let scene = scene(4, 60, 0., 3);
    let mut perturbed = truth();
    perturbed.fx *= 1.02;
    let state = IntrinsicBundleState {
        intrinsics: vec![truth(), perturbed],
        poses: scene.poses.clone(),
        points: scene.points.clone(),
    };
    let setup = IntrinsicBundleSetup {
        masks: vec![IntrinsicsMask::FIXED, IntrinsicsMask::STANDARD],
        camera_intrinsics: vec![0, 0, 1, 1],
        fixed_poses: vec![false; 4],
        fixed_points: false,
        scale_gauge: None,
    };
    let options = IntrinsicBundleOptions::default();
    assert!(
        optimize_intrinsics(&mut state.clone(), &setup, &scene.observations, &options).is_err()
    );
    let setup = IntrinsicBundleSetup {
        fixed_poses: vec![true, true, false, false],
        ..setup
    };
    let mut solved = state.clone();
    let report = optimize_intrinsics(&mut solved, &setup, &scene.observations, &options).unwrap();
    assert!(report.final_rms < 1e-4, "{report:?}");
    assert_eq!(solved.intrinsics[0], truth());
    assert!(
        (solved.intrinsics[1].fx - truth().fx).abs() < 0.05,
        "{}",
        solved.intrinsics[1].fx
    );
}

#[test]
fn pose_verification_flags_corrupted_pose() {
    let calibration = truth();
    let scene = scene(6, 120, 0.2, 21);
    let mut poses: Vec<Option<Pose>> = scene.poses.iter().copied().map(Some).collect();
    let options = PoseVerificationOptions::default();
    let clean = verify_poses(
        std::slice::from_ref(&calibration),
        &[0; 6],
        &poses,
        &scene.observations,
        &options,
    )
    .unwrap();
    assert!(
        clean
            .cameras
            .iter()
            .all(|c| c.status == PoseStatus::Consistent),
        "{clean:?}"
    );
    assert!(
        clean
            .cameras
            .iter()
            .all(|c| c.median_residual < 0.5 && c.checks >= 100)
    );

    // Rotate camera 3 by two degrees about its optical axis and shift it sideways.
    let pose = poses[3].unwrap();
    let center = add(pose.center(), [0.15, 0., 0.]);
    let rotated = Pose {
        rotation: mm(rotation([0., 0., 2f64.to_radians()]), pose.rotation),
        ..pose
    };
    poses[3] = Some(with_center(rotated, center));
    poses[5] = None;
    let report = verify_poses(
        std::slice::from_ref(&calibration),
        &[0; 6],
        &poses,
        &scene.observations,
        &options,
    )
    .unwrap();
    assert_eq!(report.inconsistent(), vec![3], "{report:?}");
    assert!(report.cameras[3].median_residual > 5.);
    assert_eq!(report.cameras[5].status, PoseStatus::Missing);
    for camera in [0, 1, 2, 4] {
        assert_eq!(
            report.cameras[camera].status,
            PoseStatus::Consistent,
            "{:?}",
            report.cameras[camera]
        );
    }

    // A pose turned to face away from the scene fails cheirality.
    let mut flipped: Vec<Option<Pose>> = scene.poses.iter().copied().map(Some).collect();
    let pose = scene.poses[2];
    let turned = Pose {
        rotation: mm(rotation([0., std::f64::consts::PI, 0.]), pose.rotation),
        ..pose
    };
    flipped[2] = Some(with_center(turned, pose.center()));
    let report = verify_poses(
        &[calibration],
        &[0; 6],
        &flipped,
        &scene.observations,
        &options,
    )
    .unwrap();
    assert_eq!(report.inconsistent(), vec![2], "{report:?}");
    assert!(report.cameras[2].cheirality_failures > 0);
}

#[test]
fn pose_verification_reports_insufficient_evidence() {
    let scene = scene(2, 40, 0., 4);
    let poses: Vec<Option<Pose>> = scene.poses.iter().copied().map(Some).collect();
    let report = verify_poses(
        &[truth()],
        &[0; 2],
        &poses,
        &scene.observations,
        &PoseVerificationOptions::default(),
    )
    .unwrap();
    assert!(
        report
            .cameras
            .iter()
            .all(|c| c.status == PoseStatus::Insufficient)
    );
}
