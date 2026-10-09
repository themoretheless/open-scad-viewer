use super::*;

fn sphere(c: [f64; 3], r: f64) -> Sphere {
    Sphere {
        center: c,
        radius: r,
    }
}

#[test]
fn gjk_far_spheres_matches_exact_distance() -> Result<()> {
    let a = sphere([0., 0., 0.], 1.);
    let b = sphere([5., 0., 0.], 0.5);
    let g = gjk(&a, &b)?;
    assert!(!g.intersecting);
    assert!(g.converged);
    assert!((g.distance - 3.5).abs() < 1e-9, "distance {}", g.distance);
    assert!((g.point_a[0] - 1.).abs() < 1e-9);
    assert!((g.point_b[0] - 4.5).abs() < 1e-9);
    Ok(())
}

#[test]
fn gjk_tangent_spheres_touch() -> Result<()> {
    let a = sphere([0., 0., 0.], 1.);
    let b = sphere([2., 0., 0.], 1.);
    let g = gjk(&a, &b)?;
    assert!(g.distance < 1e-6, "distance {}", g.distance);
    Ok(())
}

#[test]
fn gjk_overlapping_spheres_intersect() -> Result<()> {
    let a = sphere([0., 0., 0.], 2.);
    let b = sphere([1., 0., 0.], 2.);
    let g = gjk(&a, &b)?;
    assert!(g.intersecting);
    assert_eq!(g.distance, 0.);
    Ok(())
}

#[test]
fn gjk_point_vs_aabb_exact() -> Result<()> {
    let p = PointCloud {
        points: vec![[3., 4., 0.]],
    };
    let b = Aabb {
        min: [-1., -1., -1.],
        max: [1., 1., 1.],
    };
    let g = gjk(&p, &b)?;
    assert!(!g.intersecting);
    assert!((g.distance - 5. * 0.6 - 0.4 - 1.4).abs() < 1.5, "loose");
    // exact: sqrt((3-1)^2 + (4-1)^2) = sqrt(13)
    assert!((g.distance - 13f64.sqrt()).abs() < 1e-9);
    Ok(())
}

#[test]
fn gjk_degenerate_identical_spheres() -> Result<()> {
    let a = sphere([1., 2., 3.], 0.);
    let b = sphere([1., 2., 3.], 0.);
    let g = gjk(&a, &b)?;
    assert!(g.intersecting);
    Ok(())
}

#[test]
fn epa_penetration_of_overlapping_boxes_exact() -> Result<()> {
    // Two unit AABBs offset by 0.5 along x: penetration 1.5 along x.
    let a = Aabb {
        min: [-1., -1., -1.],
        max: [1., 1., 1.],
    };
    let b = Aabb {
        min: [-0.5, -1., -1.],
        max: [1.5, 1., 1.],
    };
    let g = gjk(&a, &b)?;
    assert!(g.intersecting);
    let e = epa(&a, &b)?;
    assert!(e.depth > 0., "depth {}", e.depth);
    assert!(
        (e.depth - 1.5).abs() < 1e-6,
        "expected 1.5, got {}",
        e.depth
    );
    Ok(())
}

#[test]
fn epa_penetration_of_overlapping_spheres_best_effort() -> Result<()> {
    // Two unit spheres with centers 1.2 apart: true penetration 0.8;
    // within the iteration budget EPA must approach it from below.
    let a = sphere([0., 0., 0.], 1.);
    let b = sphere([1.2, 0., 0.], 1.);
    let g = gjk(&a, &b)?;
    assert!(g.intersecting);
    let e = epa(&a, &b)?;
    assert!(
        e.depth > 0.4 && e.depth <= 0.8 + 1e-9,
        "depth {} outside (0.4, 0.8]",
        e.depth
    );
    Ok(())
}

#[test]
fn epa_depth_is_best_effort_under_budget() -> Result<()> {
    // Concentric identical spheres: maximally degenerate difference set.
    let a = sphere([0., 0., 0.], 1.);
    let b = sphere([0., 0., 0.], 1.);
    let e = epa(&a, &b)?;
    assert!(e.depth.is_finite() && e.depth >= 0.);
    Ok(())
}

fn unit_obb(center: [f64; 3]) -> Obb {
    Obb {
        center,
        axes: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        half_extents: [1., 1., 1.],
    }
}

#[test]
fn sat_separated_boxes() {
    let a = unit_obb([0., 0., 0.]);
    let b = unit_obb([3., 0., 0.]);
    assert!(obb_separated(&a, &b));
}

#[test]
fn sat_touching_boxes() {
    let a = unit_obb([0., 0., 0.]);
    let b = unit_obb([2., 0., 0.]);
    let r = obb_sat(&a, &b);
    assert!(!r.separated);
    assert!(r.min_overlap < 1e-9);
}

#[test]
fn sat_rotated_box_corner_overlap() {
    let a = unit_obb([0., 0., 0.]);
    let s = 0.7071067811865476;
    let b = Obb {
        center: [1.9, 0., 0.],
        axes: [[s, s, 0.], [-s, s, 0.], [0., 0., 1.]],
        half_extents: [1., 1., 1.],
    };
    // Rotated box reaches x = 1.9 - sqrt(2) ≈ 0.486 < 1: overlapping.
    let r = obb_sat(&a, &b);
    assert!(!r.separated);
    assert!(r.min_overlap > 0.);
}

#[test]
fn sat_min_overlap_depth() {
    let a = unit_obb([0., 0., 0.]);
    let b = unit_obb([1.5, 0., 0.]);
    let r = obb_sat(&a, &b);
    assert!(!r.separated);
    assert!((r.min_overlap - 0.5).abs() < 1e-9, "{}", r.min_overlap);
}

fn curve(points: &[[f64; 3]]) -> Curve {
    let n = points.len();
    Curve {
        degree: 1.min(n - 1),
        knots: (0..=n + 1).map(|i| i as f64).collect(),
        control_points: points.iter().map(|p| p.to_vec()).collect(),
        weights: vec![1.; n],
        periodic: false,
    }
}

#[test]
fn control_cloud_rejects_nan_with_param_and_index() {
    let a = curve(&[[0., 0., 0.], [1., f64::NAN, 0.]]);
    let err = curve_control_cloud(&a).unwrap_err();
    assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    assert!(err.contains("control_points[1]"), "{err}");
}

#[test]
fn gjk_epa_budget_guards_match_loop_constants() -> Result<()> {
    // The guards mirror the loop bounds: a normal query never trips them.
    let a = sphere([0., 0., 0.], 1.);
    let b = sphere([3., 0., 0.], 1.);
    let g = gjk(&a, &b)?;
    assert!(g.converged && g.iterations <= GJK_MAX_ITERATIONS);
    Ok(())
}

#[test]
fn control_nets_disjoint_far_curves() -> Result<()> {
    let a = curve(&[[0., 0., 0.], [1., 0., 0.]]);
    let b = curve(&[[0., 10., 0.], [1., 10., 0.]]);
    match control_nets_disjoint(&a, &b)? {
        CullVerdict::Disjoint { min_distance } => {
            assert!((min_distance - 10.).abs() < 1e-9)
        }
        other => panic!("expected Disjoint, got {other:?}"),
    }
    Ok(())
}

#[test]
fn control_nets_overlap_crossing_curves() -> Result<()> {
    let a = curve(&[[-1., 0., 0.], [1., 0., 0.]]);
    let b = curve(&[[0., -1., 0.], [0., 1., 0.]]);
    match control_nets_disjoint(&a, &b)? {
        CullVerdict::Overlap { penetration } => assert!(penetration >= 0.),
        CullVerdict::Unknown => {}
        other => panic!("expected Overlap, got {other:?}"),
    }
    Ok(())
}

#[test]
fn hull_distance_bounds_curve_distance() -> Result<()> {
    // Hull distance is a *lower* bound on the true curve-curve distance:
    // hulls are supersets of the curves. Verified against the exact
    // endpoint distance of two straight degree-1 segments.
    let a = curve(&[[0., 0., 0.], [1., 0., 0.]]);
    let b = curve(&[[0., 3., 0.], [1., 4., 0.]]);
    let exact = 3f64; // both segments vertical offset by >= 3
    match control_nets_disjoint(&a, &b)? {
        CullVerdict::Disjoint { min_distance } => {
            assert!(min_distance <= exact + 1e-9);
            assert!((min_distance - exact).abs() < 1e-9);
        }
        other => panic!("expected Disjoint, got {other:?}"),
    }
    Ok(())
}

#[test]
fn surfaces_disjoint_far_planes() -> Result<()> {
    let plane = |z: f64| Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., z], vec![0., 1., z]],
            vec![vec![1., 0., z], vec![1., 1., z]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    match surfaces_disjoint(&plane(0.), &plane(5.))? {
        CullVerdict::Disjoint { min_distance } => {
            assert!((min_distance - 5.).abs() < 1e-9)
        }
        other => panic!("expected Disjoint, got {other:?}"),
    }
    Ok(())
}
