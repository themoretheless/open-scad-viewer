use super::super::test_utils::{rotated_translated, translated};
use super::*;

type SphereSphereCircle<'a> = (
    &'a Curve,
    [f64; 3],
    f64,
    [f64; 3],
    &'a [SpherePatchCircle],
    &'a [SpherePatchCircle],
    f64,
);

fn only_circle(report: &Report<SphereSphereComponent>) -> SphereSphereCircle<'_> {
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert!(report.unresolved.is_empty(), "{report:?}");
    assert!(!report.permits_topology_change());
    let [
        SphereSphereComponent::Circle {
            curve,
            center,
            radius,
            normal,
            first_uv,
            second_uv,
            max_sample_residual,
        },
    ] = &report.components[..]
    else {
        panic!("expected one circle component: {report:?}")
    };
    (
        curve,
        *center,
        *radius,
        *normal,
        first_uv,
        second_uv,
        *max_sample_residual,
    )
}
fn sphere_residual(point: [f64; 3], center: [f64; 3], radius: f64) -> f64 {
    (sub(point, center)[0]
        .hypot(sub(point, center)[1])
        .hypot(sub(point, center)[2])
        - radius)
        .abs()
}
/// Worst both-sphere residual over 16 circle samples.
fn circle_samples(curve: &Curve, c1: [f64; 3], r1: f64, c2: [f64; 3], r2: f64) -> f64 {
    let mut worst = 0_f64;
    for i in 0..16 {
        let p = curve.evaluate(i as f64 / 4.).unwrap().point;
        let p = [p[0], p[1], p[2]];
        worst = worst
            .max(sphere_residual(p, c1, r1))
            .max(sphere_residual(p, c2, r2));
    }
    worst
}
/// Worst residual of lifted UV arcs evaluated through their patch surface.
fn uv_samples(
    model: &Model,
    lifts: &[SpherePatchCircle],
    own: ([f64; 3], f64),
    other: ([f64; 3], f64),
) -> f64 {
    let mut worst = 0_f64;
    for lift in lifts {
        let surface = &model.faces[lift.patch].surface;
        for arc in &lift.arcs {
            for k in 0..=8 {
                let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                assert!(uv[0] >= -1e-9 && uv[1] >= -1e-9);
                assert!(uv[0] * uv[0] + uv[1] * uv[1] <= 1. + 1e-9);
                let p = surface
                    .evaluate(uv[0].clamp(0., 1.), uv[1].clamp(0., 1.))
                    .unwrap()
                    .point;
                let p = [p[0], p[1], p[2]];
                worst = worst
                    .max(sphere_residual(p, own.0, own.1))
                    .max(sphere_residual(p, other.0, other.1));
            }
        }
    }
    worst
}

#[test]
fn equal_spheres_intersect_in_exact_circle() {
    let first = crate::analytic::sphere(2.).unwrap();
    let second = translated(&first, [2., 0., 0.]);
    let report = intersect_sphere_sphere(&first, &second, Options::default()).unwrap();
    let (curve, center, radius, normal, first_uv, second_uv, sampled) = only_circle(&report);
    // Oracle: circle radius sqrt(r^2 - d^2/4) = sqrt(3), center on the axis.
    let oracle = (4_f64 - 1.).sqrt();
    assert!((radius - oracle).abs() <= 1e-12, "{radius} vs {oracle}");
    assert!(
        sub(center, [1., 0., 0.]).iter().all(|x| x.abs() <= 1e-12),
        "{center:?}"
    );
    assert!(
        sub(normal, [1., 0., 0.]).iter().all(|x| x.abs() <= 1e-12),
        "{normal:?}"
    );
    // Exact rational circle: four 90-degree arcs, weights cos(pi/4).
    assert_eq!(curve.degree, 2);
    assert_eq!(
        curve.knots,
        vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.]
    );
    assert_eq!(curve.control_points.len(), 9);
    assert_eq!(
        curve.weights,
        vec![
            1., ARC_WEIGHT, 1., ARC_WEIGHT, 1., ARC_WEIGHT, 1., ARC_WEIGHT, 1.
        ]
    );
    // Sixteen sampled points satisfy both sphere equations.
    let worst = circle_samples(curve, [0., 0., 0.], 2., [2., 0., 0.], 2.);
    assert!(worst <= 1e-12, "{worst}");
    assert!(sampled <= 1e-12, "{sampled}");
    // The circle plane x=1 is seen from either center within a +-60-degree
    // cone around +-x: it crosses quadrants 0,3 (both hemispheres) of the
    // first sphere and quadrants 1,2 of the second.
    assert_eq!(
        first_uv.iter().map(|l| l.patch).collect::<Vec<_>>(),
        [0, 3, 4, 7]
    );
    assert_eq!(
        second_uv.iter().map(|l| l.patch).collect::<Vec<_>>(),
        [1, 2, 5, 6]
    );
    assert!(uv_samples(&first, first_uv, ([0., 0., 0.], 2.), ([2., 0., 0.], 2.)) <= 1e-12);
    assert!(uv_samples(&second, second_uv, ([2., 0., 0.], 2.), ([0., 0., 0.], 2.)) <= 1e-12);
    // Operand swap keeps the circle, flips the axis and the UV roles.
    let swapped = intersect_sphere_sphere(&second, &first, Options::default()).unwrap();
    let (_, s_center, s_radius, s_normal, s_first, s_second, _) = only_circle(&swapped);
    assert!((s_radius - radius).abs() <= 1e-15);
    assert!(sub(s_center, center).iter().all(|x| x.abs() <= 1e-12));
    assert!(
        s_normal
            .iter()
            .zip(normal)
            .all(|(x, y)| (x + y).abs() <= 1e-12)
    );
    assert_eq!(s_first.len(), second_uv.len());
    assert_eq!(s_second.len(), first_uv.len());
}

#[test]
fn concentric_equal_spheres_report_coincident_unresolved() {
    let first = crate::analytic::sphere(2.).unwrap();
    let second = crate::analytic::sphere(2.).unwrap();
    let report = intersect_sphere_sphere(&first, &second, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(report.unresolved.len(), 1);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::CoincidentTrim
    );
    assert!(!report.permits_topology_change());
}

#[test]
fn concentric_unequal_and_contained_spheres_are_empty_resolved() {
    let small = crate::analytic::sphere(2.).unwrap();
    let large = crate::analytic::sphere(3.).unwrap();
    let report = intersect_sphere_sphere(&small, &large, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    // Contained without contact: d = 1 < |3 - 2| is false; use radii 1 and 3.
    let tiny = crate::analytic::sphere(1.).unwrap();
    let shifted = translated(&large, [1., 0., 0.]);
    let report = intersect_sphere_sphere(&tiny, &shifted, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
}

#[test]
fn separate_spheres_are_empty_resolved() {
    let first = crate::analytic::sphere(2.).unwrap();
    let second = translated(&first, [5., 0., 0.]);
    let report = intersect_sphere_sphere(&first, &second, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
}

#[test]
fn external_tangency_band_stays_unresolved() {
    let first = crate::analytic::sphere(2.).unwrap();
    // Exact tangency and inside the outward classification band: no point.
    for gap in [0., -3e-14] {
        let second = translated(&first, [4. + gap, 0., 0.]);
        let report = intersect_sphere_sphere(&first, &second, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{gap} {report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
    }
    // Just outside the band the pair resolves empty; just inside, a circle.
    let apart = translated(&first, [4. + 1e-12, 0., 0.]);
    let report = intersect_sphere_sphere(&first, &apart, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    let closer = translated(&first, [4. - 1e-12, 0., 0.]);
    let report = intersect_sphere_sphere(&first, &closer, Options::default()).unwrap();
    let (_, _, radius, _, _, _, _) = only_circle(&report);
    // h^2 = r^2 - (d/2)^2 ~ 2e-12 loses four decimal digits to binary64
    // cancellation in any evaluation order; compare squared radii.
    let d = 4_f64 - 1e-12;
    let oracle = 4_f64 - (d / 2.).powi(2);
    assert!(
        (radius * radius - oracle).abs() <= 2e-15,
        "{} vs {oracle}",
        radius * radius
    );
}

#[test]
fn internal_tangency_band_stays_unresolved() {
    let small = crate::analytic::sphere(1.).unwrap();
    let large = translated(&crate::analytic::sphere(3.).unwrap(), [2., 0., 0.]);
    let report = intersect_sphere_sphere(&small, &large, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::TangencyOrMultipleRoot
    );
    // Clearly inside the band boundary: d below |r1-r2| is containment
    // (empty, resolved), d above it is a transverse circle.
    let large = translated(&crate::analytic::sphere(3.).unwrap(), [2. - 1e-9, 0., 0.]);
    let report = intersect_sphere_sphere(&small, &large, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    let large = translated(&crate::analytic::sphere(3.).unwrap(), [2. + 1e-9, 0., 0.]);
    let report = intersect_sphere_sphere(&small, &large, Options::default()).unwrap();
    only_circle(&report);
}

#[test]
fn axis_aligned_lifted_uv_radius_matches_polar_oracle() {
    let first = crate::analytic::sphere(3.).unwrap();
    let second = translated(&first, [0., 0., 2.]);
    let report = intersect_sphere_sphere(&first, &second, Options::default()).unwrap();
    let (curve, center, radius, normal, first_uv, second_uv, _) = only_circle(&report);
    assert!((radius - 8_f64.sqrt()).abs() <= 1e-12, "{radius}");
    assert!(
        sub(center, [0., 0., 1.]).iter().all(|x| x.abs() <= 1e-12),
        "{center:?}"
    );
    assert!(
        sub(normal, [0., 0., 1.]).iter().all(|x| x.abs() <= 1e-12),
        "{normal:?}"
    );
    // The parallel at height t = 1 above the equator lifts on sphere 1's
    // north patches (faces 0..4) to UV circles of radius sqrt((r-t)/(r+t));
    // on sphere 2 it sits at t = -1, lifting to the south patches (4..8).
    assert_eq!(
        first_uv.iter().map(|l| l.patch).collect::<Vec<_>>(),
        [0, 1, 2, 3]
    );
    assert_eq!(
        second_uv.iter().map(|l| l.patch).collect::<Vec<_>>(),
        [4, 5, 6, 7]
    );
    let t = 1_f64;
    let r = 3_f64;
    let rho = ((r - t) / (r + t)).sqrt();
    // asin-based oracle: polar angle phi = asin(t/r), stereographic radius
    // rho = tan(pi/4 - phi/2).
    let phi = (t / r).asin();
    let rho_asin = (std::f64::consts::FRAC_PI_4 - phi / 2.).tan();
    assert!((rho - rho_asin).abs() <= 1e-15);
    for lift in first_uv.iter().chain(second_uv) {
        assert_eq!(lift.arcs.len(), 1, "{lift:?}");
        let arc = &lift.arcs[0];
        assert_eq!(arc.degree, 2);
        assert_eq!(arc.weights, vec![1., ARC_WEIGHT, 1.]);
        for endpoint in [&arc.control_points[0], &arc.control_points[2]] {
            let modulus = endpoint[0].hypot(endpoint[1]);
            assert!((modulus - rho).abs() <= 1e-12, "{modulus} vs {rho}");
        }
    }
    assert!(uv_samples(&first, first_uv, ([0., 0., 0.], 3.), ([0., 0., 2.], 3.)) <= 1e-12);
    assert!(uv_samples(&second, second_uv, ([0., 0., 2.], 3.), ([0., 0., 0.], 3.)) <= 1e-12);
    let worst = circle_samples(curve, [0., 0., 0.], 3., [0., 0., 2.], 3.);
    assert!(worst <= 1e-12, "{worst}");
}

#[test]
fn rotated_translated_spheres_keep_exact_circle() {
    let first = crate::analytic::sphere(2.).unwrap();
    let second = rotated_translated(
        &crate::analytic::sphere(1.5).unwrap(),
        0.5,
        [1.5, 0.3, -0.2],
    );
    let c2: [f64; 3] = [1.5, 0.3, -0.2];
    let report = intersect_sphere_sphere(&first, &second, Options::default()).unwrap();
    let (curve, center, radius, _, first_uv, second_uv, sampled) = only_circle(&report);
    // Independent binary64 oracle from the two sphere definitions.
    let d = c2[0].hypot(c2[1]).hypot(c2[2]);
    let along = (4. - 2.25 + d * d) / (2. * d);
    let oracle = (4_f64 - along * along).sqrt();
    assert!((radius - oracle).abs() <= 1e-12, "{radius} vs {oracle}");
    let expected_center = [along * c2[0] / d, along * c2[1] / d, along * c2[2] / d];
    assert!(
        sub(center, expected_center)
            .iter()
            .all(|x| x.abs() <= 1e-12),
        "{center:?} vs {expected_center:?}"
    );
    let worst = circle_samples(curve, [0., 0., 0.], 2., c2, 1.5);
    assert!(worst <= 1e-12, "{worst}");
    assert!(sampled <= 1e-12, "{sampled}");
    assert!(!first_uv.is_empty() && !second_uv.is_empty());
    assert!(uv_samples(&first, first_uv, ([0., 0., 0.], 2.), (c2, 1.5)) <= 1e-12);
    assert!(uv_samples(&second, second_uv, (c2, 1.5), ([0., 0., 0.], 2.)) <= 1e-12);
}

#[test]
fn noncanonical_surfaces_are_explicit_unsupported_regions() {
    let sphere = crate::analytic::sphere(1.).unwrap();
    for other in [
        crate::analytic::cylinder(1., 2.).unwrap(),
        crate::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap(),
    ] {
        for (a, b) in [(&sphere, &other), (&other, &sphere)] {
            let report = intersect_sphere_sphere(a, b, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{report:?}");
            assert_eq!(report.coverage, Coverage::Incomplete);
            assert_eq!(report.unresolved.len(), 1);
            assert_eq!(
                report.unresolved[0].reason,
                UnresolvedReason::UnsupportedSurface
            );
            assert_eq!(
                report.unresolved[0].parameter_box,
                vec![0., 1., 0., 1., 0., 1., 0., 1.]
            );
        }
    }
    // A sphere with a perturbed patch is no longer a valid model at all:
    // like every other query, invalid input is a hard error, not a region.
    let mut perturbed = crate::analytic::sphere(1.).unwrap();
    perturbed.faces[0].surface.weights[1][1] = 1.5;
    assert!(intersect_sphere_sphere(&sphere, &perturbed, Options::default()).is_err());
}
