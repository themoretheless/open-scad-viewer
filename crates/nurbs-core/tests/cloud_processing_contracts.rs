use nurbs_core::analysis::{
    cloud_normals::orient_normals_consistent,
    cloud_segmentation::{ApproxKind, SegmentationOptions, segment_cloud},
    ransac_primitives::{PrimitiveKind, PrimitiveParams, RansacOptions, detect_primitives},
};
use std::collections::BTreeSet;
fn plane() -> Vec<[f64; 3]> {
    (0..4)
        .flat_map(|x| (0..4).map(move |y| [x as f64, y as f64, 0.]))
        .collect()
}
fn options() -> RansacOptions {
    RansacOptions {
        threshold: 1e-6,
        min_inliers: 32,
        max_iterations: 160,
        max_primitives: 2,
        early_exit_fraction: 1.,
        seed: 512,
        detect_plane: true,
        detect_sphere: false,
        detect_cylinder: false,
        detect_cone: false,
    }
}

#[test]
fn ransac_plane_recovers_source_inliers_despite_distant_outliers() {
    let mut points: Vec<_> = (0..8)
        .flat_map(|x| (0..8).map(move |y| [x as f64, y as f64, 0.]))
        .collect();
    points.extend((0..18).map(|i| [2. * i as f64, 3. * i as f64, 10. + i as f64]));
    let before = points.clone();
    let mut o = options();
    o.max_primitives = 1;
    let found = detect_primitives(&points, None, &o).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, PrimitiveKind::Plane);
    assert_eq!(
        found[0]
            .inlier_indices
            .iter()
            .copied()
            .collect::<BTreeSet<_>>(),
        (0..64).collect()
    );
    let PrimitiveParams::Plane { normal, offset } = found[0].params else {
        panic!("wrong model")
    };
    assert!(normal[2].abs() > 1. - 1e-10 && offset.abs() < 1e-10);
    assert!(found[0].rms < 1e-10);
    assert_eq!(points, before);
}

#[test]
fn ransac_extracts_two_models_without_reusing_source_indices() {
    let mut points: Vec<_> = (0..8)
        .flat_map(|x| (0..8).map(move |y| [x as f64 - 4., y as f64 - 4., -5.]))
        .collect();
    for z in [-0.75_f64, -0.25, 0.25, 0.75] {
        for i in 0..12 {
            let a = std::f64::consts::TAU * i as f64 / 12.;
            let r = 2. * (1. - z * z).sqrt();
            points.push([10. + r * a.cos(), r * a.sin(), 2. * z]);
        }
    }
    let mut o = options();
    o.detect_sphere = true;
    let found = detect_primitives(&points, None, &o).unwrap();
    assert_eq!(found.len(), 2);
    assert!(found.iter().any(|f| f.kind == PrimitiveKind::Plane));
    assert!(found.iter().any(|f| f.kind == PrimitiveKind::Sphere));
    let mut owned = BTreeSet::new();
    for f in &found {
        for &index in &f.inlier_indices {
            assert!(owned.insert(index));
            let p = points[index as usize];
            let error = match f.params {
                PrimitiveParams::Plane { normal, offset } => {
                    (0..3).map(|k| normal[k] * p[k]).sum::<f64>() + offset
                }
                PrimitiveParams::Sphere { center, radius } => {
                    (p[0] - center[0])
                        .hypot(p[1] - center[1])
                        .hypot(p[2] - center[2])
                        - radius
                }
                _ => panic!("unexpected model"),
            };
            assert!(error.abs() < o.threshold);
        }
    }
    assert_eq!(owned.len(), points.len());
}

#[test]
fn orientation_aligns_opposite_plane_normals_without_mutating_inputs() {
    let points = plane();
    let normals: Vec<_> = (0..points.len())
        .map(|i| [0., 0., if i % 2 == 0 { 2. } else { -3. }])
        .collect();
    let before = normals.clone();
    let report = orient_normals_consistent(&points, &normals).unwrap();
    assert_eq!(report.components, 1);
    assert_eq!(report.flipped, 8);
    assert!(report.normals.iter().all(|n| (n[2] - 1.).abs() < 1e-12));
    assert_eq!(normals, before);
}

#[test]
fn region_growing_separates_orthogonal_planar_patches() {
    let mut points = plane();
    let mut normals = vec![[0., 0., 1.]; points.len()];
    points.extend((0..4).flat_map(|y| (0..4).map(move |z| [5., y as f64, z as f64])));
    normals.extend(vec![[1., 0., 0.]; 16]);
    let regions = segment_cloud(
        &points,
        &normals,
        SegmentationOptions {
            curvature_threshold: 1.,
            min_region_size: 4,
            neighbor_count: 8,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(regions.len(), 2);
    let mut seen = BTreeSet::new();
    for r in regions {
        assert_eq!(r.approx_kind, ApproxKind::Planar);
        assert_eq!(r.point_indices.len(), 16);
        let first = r.point_indices[0] < 16;
        for i in r.point_indices {
            assert_eq!(i < 16, first);
            assert!(seen.insert(i));
        }
    }
    assert_eq!(seen.len(), 32);
}

#[test]
fn finite_large_normals_still_produce_unit_directions() {
    let points = plane();
    for (normal, expected) in [
        ([0., 0., f64::MAX], [0., 0., 1.]),
        ([f64::MAX; 3], [1. / 3_f64.sqrt(); 3]),
    ] {
        let normals = vec![normal; points.len()];
        let report = orient_normals_consistent(&points, &normals).unwrap();
        for n in report.normals {
            assert!(n.iter().all(|x| x.is_finite()));
            assert!((n[0].hypot(n[1]).hypot(n[2]) - 1.).abs() < 1e-12);
            assert!((0..3).map(|i| n[i] * expected[i]).sum::<f64>().abs() > 1. - 1e-12);
        }
    }
}

#[test]
fn coincident_points_with_a_large_common_translation_do_not_overflow_grid_keys() {
    let points = vec![[1e10, 1e10, 1e10]; 16];
    let normals = vec![[0., 0., 1.]; 16];
    let report = orient_normals_consistent(&points, &normals).unwrap();
    assert_eq!(report.components, 1);
    assert!(report.normals.iter().all(|n| (n[2] - 1.).abs() < 1e-12));
}

#[test]
fn segmentation_uses_unit_directions_for_large_finite_normal_scales() {
    let points = plane();
    let normals = vec![[0., 0., f64::MAX]; 16];
    let regions = segment_cloud(
        &points,
        &normals,
        SegmentationOptions {
            min_region_size: 4,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].point_indices.len(), 16);
}

#[test]
fn cloud_apis_reject_invalid_inputs_and_settings() {
    let points = plane();
    let normals = vec![[0., 0., 1.]; 16];
    assert!(orient_normals_consistent(&points, &normals[..15]).is_err());
    assert!(orient_normals_consistent(&points, &vec![[0.; 3]; 16]).is_err());
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut p = points.clone();
        p[2][1] = bad;
        assert!(orient_normals_consistent(&p, &normals).is_err());
        assert!(segment_cloud(&p, &normals, Default::default()).is_err());
        let mut o = options();
        o.threshold = bad;
        assert!(detect_primitives(&points, None, &o).is_err());
    }
    let o = RansacOptions {
        detect_plane: false,
        detect_sphere: false,
        detect_cylinder: false,
        detect_cone: false,
        ..options()
    };
    assert!(detect_primitives(&points, None, &o).is_err());
    assert!(
        segment_cloud(
            &points,
            &normals,
            SegmentationOptions {
                neighbor_count: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn ransac_cylinder_is_invariant_to_large_finite_normal_scaling() {
    let mut points = Vec::new();
    let mut normals = Vec::new();
    for z in [-1., 0., 1.] {
        for i in 0..24 {
            let a = std::f64::consts::TAU * i as f64 / 24.;
            points.push([2. * a.cos(), 2. * a.sin(), z]);
            normals.push([a.cos(), a.sin(), 0.]);
        }
    }
    let o = RansacOptions {
        min_inliers: 48,
        max_primitives: 1,
        detect_plane: false,
        detect_cylinder: true,
        ..options()
    };
    let reference = detect_primitives(&points, Some(&normals), &o).unwrap();
    let large: Vec<_> = normals.iter().map(|n| n.map(|x| x * 1e200)).collect();
    let scaled = detect_primitives(&points, Some(&large), &o).unwrap();
    assert_eq!(reference.len(), 1);
    assert_eq!(scaled.len(), 1);
    assert_eq!(scaled[0].inlier_indices, reference[0].inlier_indices);
    let PrimitiveParams::Cylinder {
        direction, radius, ..
    } = scaled[0].params
    else {
        panic!("wrong model")
    };
    assert!((radius - 2.).abs() < 1e-8);
    assert!(direction[2].abs() > 1. - 1e-8);
}

#[test]
fn ransac_cone_recovers_the_oriented_single_nappe_model() {
    let alpha = 0.35_f64;
    let mut points = Vec::new();
    for t in [1., 2., 4., 7.] {
        for j in 0..24 {
            let a = std::f64::consts::TAU * j as f64 / 24.;
            let r = t * alpha.tan();
            points.push([1. + r * a.cos(), 2. + r * a.sin(), -1. + t]);
        }
    }
    let o = RansacOptions {
        threshold: 1e-4,
        min_inliers: 60,
        max_primitives: 1,
        detect_plane: false,
        detect_cone: true,
        ..options()
    };
    let found = detect_primitives(&points, None, &o).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].inlier_indices.len(), points.len());
    let PrimitiveParams::Cone {
        vertex,
        axis,
        half_angle,
    } = found[0].params
    else {
        panic!("wrong model")
    };
    assert!(
        (vertex[0] - 1.).abs() < 1e-6
            && (vertex[1] - 2.).abs() < 1e-6
            && (vertex[2] + 1.).abs() < 1e-6
    );
    assert!(axis[2] > 1. - 1e-8);
    assert!((half_angle - alpha).abs() < 1e-8);
}

#[test]
fn cloud_direction_boundaries_and_point_budgets_remain_enforced() {
    let points = plane();
    for invalid in [
        [f64::NAN, 0., 1.],
        [0., f64::INFINITY, 1.],
        [0., 0., 1e-13],
        [0., 0., 1e-12],
    ] {
        let normals = vec![invalid; points.len()];
        assert!(orient_normals_consistent(&points, &normals).is_err());
        assert!(segment_cloud(&points, &normals, Default::default()).is_err());
    }
    let valid = vec![[0., 0., 2e-12]; points.len()];
    assert!(orient_normals_consistent(&points, &valid).is_ok());
    let excess = vec![[0.; 3]; 65_537];
    let normals = vec![[0., 0., 1.]; 65_537];
    assert!(orient_normals_consistent(&excess, &normals).is_err());
    assert!(segment_cloud(&excess, &normals, Default::default()).is_err());
    assert!(detect_primitives(&excess, None, &options()).is_err());
}

#[test]
fn region_mean_includes_members_that_do_not_continue_region_growth() {
    let mut points = Vec::new();
    let mut normals = Vec::new();
    for x in 0..5 {
        for y in 0..5 {
            let (x, y) = (x as f64, y as f64);
            points.push([x, y, 0.05 * (x * x + y * y)]);
            normals.push([-0.1 * x, -0.1 * y, 1.]);
        }
    }
    let regions = segment_cloud(
        &points,
        &normals,
        SegmentationOptions {
            curvature_threshold: 1e-12,
            min_region_size: 1,
            neighbor_count: 8,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!regions.is_empty());
    let mut covered = BTreeSet::new();
    for region in regions {
        let mut sum = [0.; 3];
        for &i in &region.point_indices {
            assert!(covered.insert(i));
            let n = normals[i as usize];
            let length = n[0].hypot(n[1]).hypot(n[2]);
            for k in 0..3 {
                sum[k] += n[k] / length;
            }
        }
        let length = sum[0].hypot(sum[1]).hypot(sum[2]);
        let agreement = (0..3)
            .map(|k| region.mean_normal[k] * sum[k] / length)
            .sum::<f64>();
        assert!(
            agreement > 1. - 1e-12,
            "region mean excludes some source members: {agreement}"
        );
    }
    assert_eq!(covered.len(), points.len());
}
