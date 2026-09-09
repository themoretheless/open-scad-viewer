use super::*;
use crate::{camera::Camera, Point};
#[path = "../../examples/support/dense_fixture.rs"]
mod fixture;
use fixture::*;

#[test]
fn inclined_surface_improves_depth_at_identical_hypothesis_budget() {
    let scene = Scene {
        angle: 60.,
        thin: false,
    };
    let (images, sparse, _) = fixture(scene);
    let options = DenseOptions {
        max_side: 64,
        patch_radius: 2,
        ..Default::default()
    };
    let baseline = densify_with_options(&images, &sparse, &options, |_, _, _| true).unwrap();
    let candidate = densify_with_options(
        &images,
        &sparse,
        &DenseOptions {
            estimator: DenseEstimator::SlantedPlane,
            ..options
        },
        |_, _, _| true,
    )
    .unwrap();
    let mean = |surface: &Surface| {
        surface
            .positions
            .iter()
            .map(|&p| scene.distance(p))
            .sum::<f64>()
            / surface.positions.len() as f64
    };
    assert!(
        mean(&candidate.surface) < mean(&baseline.surface) * 0.85,
        "baseline={} candidate={}",
        mean(&baseline.surface),
        mean(&candidate.surface)
    );
    // This support guard prevents passing by returning only a tiny accurate
    // subset. Spatial completeness is separately measured by the example.
    assert!(
        candidate.diagnostics.consistent_samples * 10
            >= baseline.diagnostics.consistent_samples * 9
    );
    assert_eq!(
        baseline.diagnostics.evaluated_hypotheses,
        candidate.diagnostics.evaluated_hypotheses
    );
}

#[test]
fn refinement_can_be_cancelled_after_global_initialization() {
    let (images, sparse, _) = fixture(Scene {
        angle: 30.,
        thin: false,
    });
    let options = DenseOptions {
        estimator: DenseEstimator::SlantedPlane,
        max_side: 80,
        patch_radius: 2,
        ..Default::default()
    };
    let mut reached_refinement = false;
    let error = densify_with_options(&images, &sparse, &options, |stage, n, _| {
        if stage == "depth" && n > SIDE {
            reached_refinement = true;
            false
        } else {
            true
        }
    })
    .unwrap_err();
    assert!(reached_refinement);
    assert_eq!(error, "Cancelled");
}

#[test]
fn all_valid_hypothesis_budgets_remain_bounded_and_deterministic() {
    let (mut images, mut sparse, _) = fixture(Scene {
        angle: 30.,
        thin: false,
    });
    images.truncate(2);
    sparse.cameras.truncate(2);
    sparse.input_images = 2;
    for point in &mut sparse.points {
        point.observations.retain(|&(view, _)| view < 2);
    }
    // Smallest and largest advertised budgets exercise stack-array boundaries.
    for depth_hypotheses in [16, 128] {
        let options = DenseOptions {
            estimator: DenseEstimator::SlantedPlane,
            max_side: 64,
            depth_hypotheses,
            ..Default::default()
        };
        let (_, report) =
            estimation::estimate(&images, &sparse, &options, &mut |_, _, _| true).unwrap();
        assert_eq!(
            report.evaluated_hypotheses,
            2 * (64 - 6) * (64 - 6) * depth_hypotheses
        );
        assert!(report.sampled_source_pixels <= report.evaluated_source_patches as u64 * 9);
    }
}

#[test]
fn planning_budget_accounts_for_temporary_plane_states() {
    let image = Image {
        width: 512,
        height: 512,
        rgb: vec![0; 512 * 512 * 3],
        focal: 900.,
    };
    let images = vec![image; 6];
    let sparse = Reconstruction {
        cameras: images.iter().map(|i| Some(i.camera())).collect(),
        points: vec![],
        input_images: 6,
        reprojection_rmse: 0.,
    };
    let legacy = DenseOptions {
        max_side: 384,
        ..Default::default()
    };
    let slanted = DenseOptions {
        estimator: DenseEstimator::SlantedPlane,
        ..legacy.clone()
    };
    assert!(estimated_working_bytes(&images, &sparse, &legacy).unwrap() <= MAX_WORKING_BYTES);
    assert!(estimated_working_bytes(&images, &sparse, &slanted).unwrap() > MAX_WORKING_BYTES);
    let error = densify_with_options(&images, &sparse, &slanted, |_, _, _| {
        panic!("allocation must be rejected before computation")
    })
    .unwrap_err();
    assert!(error.contains("512 MiB"));
}

#[test]
fn invalid_patch_radius_is_rejected() {
    let (images, sparse, _) = fixture(Scene {
        angle: 0.,
        thin: false,
    });
    let options = DenseOptions {
        estimator: DenseEstimator::SlantedPlane,
        patch_radius: 3,
        ..Default::default()
    };
    assert!(
        densify_with_options(&images, &sparse, &options, |_, _, _| panic!(
            "invalid options must fail before computation"
        ))
        .is_err()
    );
}

#[test]
fn dual_scale_requires_large_patch_and_cancels_secondary_pass() {
    assert!(DenseOptions {dual_scale:true,..Default::default()}.validate().is_err());
    let (images,sparse,_)=fixture(Scene {angle:0.,thin:false});
    let mut saw_secondary=false;
    let result=densify_with_options(&images,&sparse,&DenseOptions {
        dual_scale:true,patch_radius:2,max_side:64,depth_hypotheses:16,..Default::default()
    },|stage,_,_| {if stage=="depth-secondary" {saw_secondary=true;false}else{true}});
    assert!(saw_secondary);
    assert!(result.is_err());
}
