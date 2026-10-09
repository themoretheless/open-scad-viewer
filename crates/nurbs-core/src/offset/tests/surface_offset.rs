use super::*;
use crate::primitives;

fn plane(z: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., z], vec![0., 10., z]],
            vec![vec![10., 0., z], vec![10., 10., z]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}

/// Sampled deviation used by the tests, on the same grid as the report.
fn test_deviation(original: &Surface, offset: &Surface, distance: f64) -> f64 {
    let ([u0, u1], [v0, v1]) = domain(original);
    let nu = (2 * original.control_points.len()).clamp(4, 64);
    let nv = (2 * original.control_points[0].len()).clamp(4, 64);
    let mut worst = 0_f64;
    for i in 0..=nu {
        for j in 0..=nv {
            let u = u0 + (u1 - u0) * i as f64 / nu as f64;
            let v = v0 + (v1 - v0) * j as f64 / nv as f64;
            let e = original.evaluate(u, v).unwrap();
            let Some(n) = e.unit_normal() else { continue };
            let q = offset.evaluate(u, v).unwrap().point;
            let delta: [f64; 3] = std::array::from_fn(|k| q[k] - e.point[k]);
            let along = dot(delta, n);
            let lateral: [f64; 3] = std::array::from_fn(|k| delta[k] - along * n[k]);
            worst = worst.max((along - distance).abs().hypot(norm(lateral)));
        }
    }
    worst
}

#[test]
fn plane_offset_is_exact() {
    let p = plane(0.);
    let report = offset(&p, 2., 1e-9).unwrap();
    assert!(report.exact);
    assert!(report.max_deviation < 1e-10, "{}", report.max_deviation);
    assert!(report.degenerate_regions.is_empty());
    for i in 0..=8 {
        for j in 0..=8 {
            let q = report
                .surface
                .evaluate(i as f64 / 8., j as f64 / 8.)
                .unwrap()
                .point;
            assert!((q[2] - 2.).abs() < 1e-12);
        }
    }
    // Certified distance query between the two parallel planes.
    let d = crate::surface_distance::distance(&p, &report.surface, 1e-6, 1000).unwrap();
    assert!(d.distance_interval_mm[0] <= 2. && d.distance_interval_mm[1] >= 2.);
    assert!(d.converged);
}

#[test]
fn sphere_offset_scales_radius() {
    let center = [1., 2., 3.];
    let s = primitives::sphere(center, 5.).unwrap();
    // Expected radius follows the surface's own normal orientation.
    let probe = s.evaluate(0.4, 0.3).unwrap();
    let n = probe.unit_normal().unwrap();
    let radial: [f64; 3] = std::array::from_fn(|k| probe.point[k] - center[k]);
    let side = if dot(radial, n) >= 0. { 1. } else { -1. };
    let expected = 5. + side * 1.5;
    let report = offset(&s, 1.5, 1e-6).unwrap();
    assert!(report.exact);
    assert!(report.max_deviation < 1e-6, "{}", report.max_deviation);
    // Poles are recorded, never fatal.
    for i in 1..10 {
        for j in 0..=10 {
            let u = i as f64 / 10.;
            let v = j as f64 / 10.;
            let q = report.surface.evaluate(u, v).unwrap().point;
            let r = norm(std::array::from_fn(|k| q[k] - center[k]));
            assert!((r - expected).abs() < 1e-6, "r = {r}");
        }
    }
}

#[test]
fn sphere_poles_do_not_panic_and_inward_offset_is_reported() {
    let s = primitives::sphere([0.; 3], 2.).unwrap();
    // Small outward offset through the general degeneracy bookkeeping.
    let report = offset(&s, 0.25, 1e-9).unwrap();
    assert!(report.max_deviation.is_finite());
    // Inward offset larger than the radius must not panic; the focal
    // crossing is surfaced as degenerate information.
    let inward = offset(&s, -3., 1e-6).unwrap();
    assert!(inward.max_deviation.is_finite());
}

#[test]
fn cylinder_offset_scales_radius() {
    let s = primitives::cylinder([0.; 3], 3., 4.).unwrap();
    let report = offset(&s, 0.5, 1e-6).unwrap();
    assert!(report.exact);
    for i in 0..=10 {
        for j in 0..=4 {
            let q = report
                .surface
                .evaluate(i as f64 / 10., j as f64 / 4.)
                .unwrap()
                .point;
            assert!((q[0].hypot(q[1]) - 3.5).abs() < 1e-6);
        }
    }
}

#[test]
fn torus_offset_inside_minor_radius_works() {
    let t = primitives::torus([0.; 3], 5., 2., 2.).unwrap();
    let report = offset(&t, 0.5, 1e-3).unwrap();
    assert!(report.max_deviation.is_finite() && report.max_deviation >= 0.);
    // The declared bound covers the sampled deviation.
    let actual = test_deviation(&t, &report.surface, 0.5);
    assert!(
        report.max_deviation >= actual - 1e-12,
        "declared {}, actual {actual}",
        report.max_deviation
    );
    // Offset surface stays near the true torus tube of radius 2.5.
    let d = crate::surface_distance::distance(&t, &report.surface, 0.05, 100000).unwrap();
    assert!(d.distance_interval_mm[0] <= 0.5 + report.max_deviation + 0.1);
}

#[test]
fn saddle_reports_honest_bound() {
    let saddle = primitives::quadratic_patch([0., 1., 0., 1.], [1., 0., -1., 0., 0., 0.]).unwrap();
    let report = offset(&saddle, 0.2, 1e-9).unwrap();
    assert!(!report.exact);
    assert!(report.max_deviation.is_finite());
    // Declared bound covers the actual sampled deviation.
    let actual = test_deviation(&saddle, &report.surface, 0.2);
    assert!(
        report.max_deviation >= actual - 1e-12,
        "declared {}, actual {actual}",
        report.max_deviation
    );
}

#[test]
fn negative_distance_flips_side() {
    let p = plane(0.);
    let report = offset(&p, -2., 1e-9).unwrap();
    assert!(report.exact);
    for i in 0..=4 {
        let q = report.surface.evaluate(0.5, i as f64 / 4.).unwrap().point;
        assert!((q[2] + 2.).abs() < 1e-12);
    }
    let positive = offset(&p, 2., 1e-9).unwrap();
    let q = positive.surface.evaluate(0.25, 0.75).unwrap().point;
    assert!((q[2] - 2.).abs() < 1e-12);
}

#[test]
fn zero_distance_is_the_identity() {
    let t = primitives::torus([0.; 3], 5., 2., 2.).unwrap();
    let report = offset(&t, 0., 1e-9).unwrap();
    assert!(report.exact && report.max_deviation == 0.);
    assert_eq!(report.surface, t);
}

#[test]
fn invalid_inputs_return_errors() {
    let p = plane(0.);
    assert!(offset(&p, f64::NAN, 1e-6).is_err());
    assert!(offset(&p, 1., 0.).is_err());
    assert!(offset(&p, 1., -1e-6).is_err());
    assert!(offset(&p, 1., f64::INFINITY).is_err());
}

/// Sign of `distance` that offsets toward the surface's own center of
/// curvature (the folding side) for a sphere: opposite to the normal's
/// radial side.
fn sphere_fold_sign(s: &Surface, center: [f64; 3]) -> f64 {
    let probe = s.evaluate(0.4, 0.3).unwrap();
    let n = probe.unit_normal().unwrap();
    let radial: [f64; 3] = std::array::from_fn(|k| probe.point[k] - center[k]);
    if dot(radial, n) >= 0. { -1. } else { 1. }
}

#[test]
fn sphere_validity_bounds_radius() {
    let center = [1., 2., 3.];
    let s = primitives::sphere(center, 3.).unwrap();
    let fold = sphere_fold_sign(&s, center);
    let report = offset_validity(&s, fold * 1.).unwrap();
    // The certified interval brackets the true radius.
    assert!(report.min_radius[0] <= 3. && 3. <= report.min_radius[1],
        "{:?}", report.min_radius);
    assert!(report.min_radius[1] - report.min_radius[0] < 0.05 * 3.,
        "{:?}", report.min_radius);
    // max_offset is the conservative (rounded-down) end.
    assert!(report.max_offset <= 3. && report.max_offset > 2.9,
        "{}", report.max_offset);
    assert!(report.max_offset <= report.min_radius[0]);
    // Just inside is fine, just beyond is not.
    assert!(offset_validity(&s, fold * (3. - 0.1)).unwrap().requested_ok);
    assert!(!offset_validity(&s, fold * (3. + 0.1)).unwrap().requested_ok);
    // The convex side never folds.
    assert!(offset_validity(&s, -fold * 100.).unwrap().requested_ok);
    assert_eq!(offset_validity(&s, -fold * 1.).unwrap().max_offset, f64::INFINITY);
}

#[test]
fn torus_validity_bounds_minor_radius_and_inner_equator() {
    // Torus (major 5, minor 2), side handling: only the principal
    // curvature that opposes the offset direction limits each side.
    // On one side the tube curvature 1/2 at the outer equator limits the
    // bound to the minor radius 2; on the other side the tube curvature
    // points the other way, so the meridional curvature 1/(5-2) = 1/3 at
    // the inner equator dominates and the bound is ~3.
    let t = primitives::torus([0.; 3], 5., 2., 2.).unwrap();
    let minor = offset_validity(&t, 1.).unwrap();
    assert!(minor.min_radius[0] <= 2. && 2. <= minor.min_radius[1],
        "{:?}", minor.min_radius);
    assert!(minor.min_radius[1] - minor.min_radius[0] < 0.2,
        "{:?}", minor.min_radius);
    assert!(offset_validity(&t, 1.5).unwrap().requested_ok);
    assert!(!offset_validity(&t, 2.5).unwrap().requested_ok);
    let inner = offset_validity(&t, -1.).unwrap();
    assert!(inner.min_radius[0] <= 3. && 3. <= inner.min_radius[1],
        "{:?}", inner.min_radius);
    assert!(inner.min_radius[1] - inner.min_radius[0] < 1.5,
        "{:?}", inner.min_radius);
    assert!(offset_validity(&t, -2.5).unwrap().requested_ok);
    assert!(!offset_validity(&t, -3.5).unwrap().requested_ok);
}

#[test]
fn saddle_validity_finds_limiting_corner() {
    // Hyperbolic patch z = x^2 - y^2 on [0,1]^2: principal curvatures
    // have opposite signs everywhere, so BOTH offset sides fold at the
    // same radius. Although the saddle sits at the center, the limiting
    // (largest |k|) location is the corner uv = (0,0), where the
    // parameterization is axis-aligned and k = +/-2 exactly (radius 0.5);
    // at the center the tilted first fundamental form reduces |k| to 2/3.
    let saddle = primitives::quadratic_patch([0., 1., 0., 1.], [1., 0., -1., 0., 0., 0.]).unwrap();
    // Sanity: the corner curvature really is +/-2 in this kernel.
    let (k, h) = saddle.evaluate(0., 0.).unwrap().curvatures().unwrap();
    let (k1, k2) = principal(k, h).unwrap();
    let corner_radius = 1. / k1.abs().max(k2.abs());
    assert!((corner_radius - 0.5).abs() < 1e-9, "{corner_radius}");
    for side in [1., -1.] {
        let report = offset_validity(&saddle, side).unwrap();
        assert!(report.max_offset.is_finite() && report.max_offset > 0.);
        assert!(
            report.min_radius[0] <= 0.5 && 0.5 <= report.min_radius[1],
            "side {side}: {:?}", report.min_radius
        );
        assert!(report.min_radius[1] - report.min_radius[0] < 0.05,
            "side {side}: {:?}", report.min_radius);
        assert!(report.limiting_uv[0] < 0.2 && report.limiting_uv[1] < 0.2,
            "side {side}: {:?}", report.limiting_uv);
    }
}

#[test]
fn plane_validity_is_unbounded() {
    let p = plane(0.);
    for d in [1e-3, 1., 1e6, -1e6] {
        let report = offset_validity(&p, d).unwrap();
        assert_eq!(report.max_offset, f64::INFINITY);
        assert!(report.requested_ok, "d = {d}");
    }
}

#[test]
fn offset_beyond_fold_free_bound_is_flagged_not_fatal() {
    let s = primitives::sphere([0.; 3], 2.).unwrap();
    let fold = sphere_fold_sign(&s, [0.; 3]);
    // Slightly beyond the radius: still returns, flags the excess, and
    // records the limiting region as degenerate.
    let report = offset(&s, fold * 2.4, 1e-6).unwrap();
    assert!(report.exceeds_fold_free_bound);
    assert!(!report.degenerate_regions.is_empty());
    assert!(report.max_deviation.is_finite());
    // Well inside the bound: no flag.
    let ok = offset(&s, fold * 0.5, 1e-6).unwrap();
    assert!(!ok.exceeds_fold_free_bound);
    // Plane offsets never exceed the (infinite) bound.
    let p = offset(&plane(0.), 100., 1e-9).unwrap();
    assert!(!p.exceeds_fold_free_bound);
    assert!(p.degenerate_regions.is_empty());
}

#[test]
fn non_finite_distance_and_tolerance_are_typed_rejections() {
    let p = plane(0.);
    let err = offset(&p, f64::NAN, 1e-9).err().expect("invalid geometry input must fail");
    assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    assert!(err.contains("distance"), "{err}");
    assert!(err.contains("must be finite"), "{err}");
    let err = offset(&p, 1., f64::NAN).err().expect("invalid geometry input must fail");
    assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    assert!(err.contains("tolerance"), "{err}");
    let err = offset_validity(&p, f64::INFINITY).err().expect("invalid geometry input must fail");
    assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    assert!(err.contains("distance"), "{err}");
}
