use nurbs_core::{ray_surface::ray_intersections, surface::Surface};
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 7., 7.],
        knots_v: vec![-3., -3., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 2.], vec![0., 4., 2.]],
            vec![vec![4., 0., 2.], vec![4., 4., 2.]],
        ],
        weights: vec![vec![1., 2.], vec![3., 6.]],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn forward_root_matches_analytic_rational_uv_and_scaled_ray() {
    let r = ray_intersections(&plane(), [0.7, 1.3, 0.], [0., 0., 2.], 1e-6, 1000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.roots.len(), 1);
    let root = &r.roots[0];
    for (b, x) in [
        (root.parameter, 1.),
        (root.uv[0], 2. + 5. * 0.7 / (12. - 1.4)),
        (root.uv[1], -3. + 4. * 1.3 / (8. - 1.3)),
    ] {
        assert!(b[0] <= x && x <= b[1], "{b:?} misses {x}");
    }
}
#[test]
fn backward_root_is_excluded_and_negative_direction_is_supported() {
    let behind = ray_intersections(&plane(), [0.7, 1.3, 3.], [0., 0., 1.], 1e-6, 1000).unwrap();
    assert!(behind.complete);
    assert!(behind.roots.is_empty());
    let forward = ray_intersections(&plane(), [0.7, 1.3, 3.], [0., 0., -2.], 1e-6, 1000).unwrap();
    assert!(forward.complete);
    assert_eq!(forward.roots.len(), 1);
    assert!(forward.roots[0].parameter[0] <= 0.5 && 0.5 <= forward.roots[0].parameter[1]);
}
#[test]
fn zero_boundary_and_coincidence_stay_unresolved() {
    let s = plane();
    for (origin, direction) in [
        ([2., 2., 2.], [0., 0., 1.]),
        ([0., 2., 0.], [0., 0., 1.]),
        ([2., 2., 2.], [1., 0., 0.]),
    ] {
        let r = ray_intersections(&s, origin, direction, 1e-5, 100).unwrap();
        assert!(!r.complete, "{r:?}");
        assert!(!r.unresolved.is_empty());
    }
}
#[test]
fn invalid_ray_and_budget_fail() {
    let s = plane();
    for direction in [[0.; 3], [f64::NAN, 0., 1.]] {
        assert!(ray_intersections(&s, [0.; 3], direction, 1e-5, 100).is_err());
    }
    for accuracy in [0., -1., f64::NAN] {
        assert!(ray_intersections(&s, [0.; 3], [0., 0., 1.], accuracy, 100).is_err());
    }
    for budget in [0, 100001] {
        assert!(ray_intersections(&s, [0.; 3], [0., 0., 1.], 1e-5, budget).is_err());
    }
}

#[test]
fn subdivision_boundary_root_stays_covered_with_complete_and_limited_search() {
    let u = 3.25;
    let v = -3. + 4. / 3.;
    for (accuracy, budget, complete) in [(1e-6, 1000, true), (1e-30, 1, false)] {
        let r = ray_intersections(&plane(), [2., 2., 0.], [0., 0., 2.], accuracy, budget).unwrap();
        let covered = r
            .roots
            .iter()
            .map(|x| x.uv)
            .chain(r.unresolved.iter().copied())
            .any(|uv| uv[0][0] <= u && u <= uv[0][1] && uv[1][0] <= v && v <= uv[1][1]);
        assert!(covered, "{r:?}");
        assert_eq!(r.complete, complete, "{r:?}");
        assert!(r.cells <= budget);
        if complete {
            assert_eq!(r.roots.len(), 1);
            assert!(r.roots[0].parameter[0] <= 1. && 1. <= r.roots[0].parameter[1]);
        } else {
            assert!(!r.unresolved.is_empty());
        }
    }
}
