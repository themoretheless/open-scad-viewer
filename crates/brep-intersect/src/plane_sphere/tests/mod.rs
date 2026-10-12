use brep_core::intersections::sphere_sphere::ARC_WEIGHT;
use super::super::test_utils::rotated_translated;
use super::*;

#[allow(clippy::type_complexity)]
fn only_circle(
    report: &Report<PlaneSphereComponent>,
    full: bool,
) -> (
    &Curve,
    [f64; 3],
    f64,
    [f64; 3],
    &[PlanePatchCurve],
    &[SpherePatchCircle],
    f64,
) {
    assert!(report.unresolved.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert!(!report.permits_topology_change());
    let [
        PlaneSphereComponent::Circle {
            curve,
            center,
            radius,
            normal,
            full: is_full,
            plane_uv,
            sphere_uv,
            max_sample_residual,
        },
    ] = &report.components[..]
    else {
        panic!("expected one circle component: {report:?}")
    };
    assert_eq!(*is_full, full);
    (
        curve,
        *center,
        *radius,
        *normal,
        plane_uv,
        sphere_uv,
        *max_sample_residual,
    )
}
fn sphere_residual(point: [f64; 3], center: [f64; 3], radius: f64) -> f64 {
    let d = sub(point, center);
    (d[0].hypot(d[1]).hypot(d[2]) - radius).abs()
}
/// Worst residual of lifted UV arcs on both operands, evaluated through
/// their own surfaces, against the sphere and plane equations.
fn uv_samples(
    plane_model: &Model,
    sphere_model: &Model,
    plane_uv: &[PlanePatchCurve],
    sphere_uv: &[SpherePatchCircle],
    sphere_def: ([f64; 3], f64),
    plane_def: ([f64; 3], [f64; 3]),
) -> f64 {
    let mut worst = 0_f64;
    let check = |p: [f64; 3]| {
        sphere_residual(p, sphere_def.0, sphere_def.1)
            .max(dot(plane_def.0, sub(p, plane_def.1)).abs())
    };
    for lift in plane_uv {
        let surface = &plane_model.faces[lift.patch].surface;
        for arc in &lift.arcs {
            for k in 0..=8 {
                let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                assert!((-1e-9..=1. + 1e-9).contains(&uv[0]), "{uv:?}");
                assert!((-1e-9..=1. + 1e-9).contains(&uv[1]), "{uv:?}");
                let p = surface
                    .evaluate(uv[0].clamp(0., 1.), uv[1].clamp(0., 1.))
                    .unwrap()
                    .point;
                worst = worst.max(check([p[0], p[1], p[2]]));
            }
        }
    }
    for lift in sphere_uv {
        let surface = &sphere_model.faces[lift.patch].surface;
        for arc in &lift.arcs {
            for k in 0..=8 {
                let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                assert!(uv[0] >= -1e-9 && uv[1] >= -1e-9, "{uv:?}");
                assert!(uv[0] * uv[0] + uv[1] * uv[1] <= 1. + 1e-9, "{uv:?}");
                let p = surface
                    .evaluate(uv[0].clamp(0., 1.), uv[1].clamp(0., 1.))
                    .unwrap()
                    .point;
                worst = worst.max(check([p[0], p[1], p[2]]));
            }
        }
    }
    worst
}

#[test]
fn full_section_matches_the_sqrt_oracle() {
    // Plane z = 1, patch [-3,3]^2; sphere r = 2 at the origin: the exact
    // circle of radius sqrt(4 - 1) at z = 1, fully inside the patch.
    let plane = plane_patch([-3., -3., 1.], [6., 0., 0.], [0., 6., 0.]);
    let sphere = brep_core::analytic::sphere(2.).unwrap();
    let report = intersect_plane_sphere(&plane, &sphere, Options::default()).unwrap();
    let (curve, center, radius, normal, plane_uv, sphere_uv, sampled) =
        only_circle(&report, true);
    let oracle = (4_f64 - 1.).sqrt();
    assert!((radius - oracle).abs() <= 1e-12, "{radius} vs {oracle}");
    assert!(
        sub(center, [0., 0., 1.]).iter().all(|x| x.abs() <= 1e-12),
        "{center:?}"
    );
    assert!(
        sub(normal, [0., 0., 1.]).iter().all(|x| x.abs() <= 1e-12),
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
    // Sixteen sampled points satisfy both implicit equations.
    let mut worst = 0_f64;
    for i in 0..16 {
        let p = curve.evaluate(i as f64 / 4.).unwrap().point;
        let p = [p[0], p[1], p[2]];
        worst = worst
            .max(sphere_residual(p, [0., 0., 0.], 2.))
            .max((p[2] - 1.).abs());
    }
    assert!(worst <= 1e-12, "{worst}");
    assert!(sampled <= 1e-12, "{sampled}");
    // Plane UV: the exact UV circle of radius sqrt(3)/6 about [1/2, 1/2].
    assert_eq!(plane_uv.len(), 1);
    assert_eq!(plane_uv[0].patch, 0);
    assert_eq!(plane_uv[0].arcs.len(), 4);
    for arc in &plane_uv[0].arcs {
        assert_eq!(arc.degree, 2);
        assert_eq!(arc.weights, vec![1., ARC_WEIGHT, 1.]);
        let endpoint = &arc.control_points[0];
        let uvr = (endpoint[0] - 0.5).hypot(endpoint[1] - 0.5);
        assert!((uvr - oracle / 6.).abs() <= 1e-12, "{uvr}");
    }
    assert!(!sphere_uv.is_empty());
    let uv_worst = uv_samples(
        &plane,
        &sphere,
        plane_uv,
        sphere_uv,
        ([0., 0., 0.], 2.),
        ([0., 0., 1.], [0., 0., 1.]),
    );
    assert!(uv_worst <= 1e-9, "{uv_worst}");
    // Operand order is fixed: the sphere as plane operand refuses.
    let swapped = intersect_plane_sphere(&sphere, &plane, Options::default()).unwrap();
    assert_eq!(
        swapped.unresolved[0].reason,
        UnresolvedReason::UnsupportedSurface
    );
}

#[test]
fn boundary_clipping_has_exact_trims_and_clipped_lifts() {
    // Plane z = 1, patch covering x in [0, 3], y in [-3, 3]: the circle of
    // radius sqrt(3) is clipped to its x >= 0 half, swept angle interval
    // [3pi/2, 5pi/2] in the (x, y) basis, endpoints (0, +-sqrt(3), 1).
    let plane = plane_patch([0., -3., 1.], [3., 0., 0.], [0., 6., 0.]);
    let sphere = brep_core::analytic::sphere(2.).unwrap();
    let report = intersect_plane_sphere(&plane, &sphere, Options::default()).unwrap();
    let (curve, center, radius, _, plane_uv, sphere_uv, sampled) = only_circle(&report, false);
    let oracle = 3_f64.sqrt();
    assert!((radius - oracle).abs() <= 1e-12);
    assert!(
        sub(center, [0., 0., 1.]).iter().all(|x| x.abs() <= 1e-12),
        "{center:?}"
    );
    // Two 90-degree pieces over the half circle.
    assert_eq!(curve.knots, vec![0., 0., 0., 1., 1., 2., 2., 2.]);
    let start = curve.evaluate(0.).unwrap().point;
    let end = curve.evaluate(2.).unwrap().point;
    assert!(
        start[0].abs() <= 1e-12 && (start[1] + oracle).abs() <= 1e-12,
        "{start:?}"
    );
    assert!((start[2] - 1.).abs() <= 1e-12);
    assert!(
        end[0].abs() <= 1e-12 && (end[1] - oracle).abs() <= 1e-12,
        "{end:?}"
    );
    assert!((end[2] - 1.).abs() <= 1e-12);
    let mut worst = 0_f64;
    for i in 0..=16 {
        let p = curve.evaluate(i as f64 / 8.).unwrap().point;
        let p = [p[0], p[1], p[2]];
        assert!(p[0] >= -1e-12, "{p:?}");
        worst = worst
            .max(sphere_residual(p, [0., 0., 0.], 2.))
            .max((p[2] - 1.).abs());
    }
    assert!(worst <= 1e-12, "{worst}");
    assert!(sampled <= 1e-12, "{sampled}");
    // Plane UV: two clipped ellipse arcs meeting at the phi = 0 seam
    // (1/sqrt(3), 1/2); the free endpoints sit exactly on u = 0 at
    // v = 1/2 +- sqrt(3)/6.
    assert_eq!(plane_uv[0].arcs.len(), 2);
    let mut edge_v: Vec<f64> = Vec::new();
    let mut seam: Vec<[f64; 2]> = Vec::new();
    for arc in &plane_uv[0].arcs {
        for endpoint in [&arc.control_points[0], &arc.control_points[2]] {
            if endpoint[0].abs() <= 1e-12 {
                edge_v.push(endpoint[1]);
            } else {
                seam.push([endpoint[0], endpoint[1]]);
            }
        }
    }
    edge_v.sort_by(f64::total_cmp);
    let oracle_v = oracle / 6.;
    assert_eq!(edge_v.len(), 2, "{edge_v:?}");
    assert!((edge_v[0] - (0.5 - oracle_v)).abs() <= 1e-12, "{edge_v:?}");
    assert!((edge_v[1] - (0.5 + oracle_v)).abs() <= 1e-12, "{edge_v:?}");
    assert_eq!(seam.len(), 2, "{seam:?}");
    for p in &seam {
        assert!((p[0] - 1. / oracle).abs() <= 1e-12, "{p:?}");
        assert!((p[1] - 0.5).abs() <= 1e-12, "{p:?}");
    }
    // Sphere UV: clipped per-patch lifts evaluate onto both surfaces.
    assert!(!sphere_uv.is_empty());
    let uv_worst = uv_samples(
        &plane,
        &sphere,
        plane_uv,
        sphere_uv,
        ([0., 0., 0.], 2.),
        ([0., 0., 1.], [0., 0., 1.]),
    );
    assert!(uv_worst <= 1e-9, "{uv_worst}");
}

#[test]
fn miss_tangency_and_band_classification() {
    let sphere = brep_core::analytic::sphere(2.).unwrap();
    let at = |z: f64| plane_patch([-1., -1., z], [2., 0., 0.], [0., 2., 0.]);
    // Miss: d = 5 > r, empty and resolved.
    let report = intersect_plane_sphere(&at(5.), &sphere, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    // Exact tangency and within the outward band: never a point.
    for z in [2., 2. - 3e-14] {
        let report = intersect_plane_sphere(&at(z), &sphere, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{z} {report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
    }
    // Just clear of the band above: empty resolved; below: a small circle.
    let report = intersect_plane_sphere(&at(2. + 1e-12), &sphere, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    let report = intersect_plane_sphere(&at(2. - 1e-12), &sphere, Options::default()).unwrap();
    let (_, _, radius, _, _, _, _) = only_circle(&report, true);
    // rho^2 = r^2 - d^2 loses digits to cancellation; compare squared.
    let d = 2. - 1e-12;
    let oracle = 4_f64 - d * d;
    assert!(
        (radius * radius - oracle).abs() <= 2e-15,
        "{} vs {oracle}",
        radius * radius
    );
}

#[test]
fn domain_edge_tangency_and_outside_cases() {
    let sphere = brep_core::analytic::sphere(2.).unwrap();
    // Circle of radius sqrt(3) at z = 1 tangent to the patch edge x = sqrt(3).
    let tangent = plane_patch([3_f64.sqrt(), -3., 1.], [3., 0., 0.], [0., 6., 0.]);
    let report = intersect_plane_sphere(&tangent, &sphere, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::TangencyOrMultipleRoot
    );
    // Circle clearly outside the rectangle: empty and resolved.
    let outside = plane_patch([2.5, -3., 1.], [3., 0., 0.], [0., 6., 0.]);
    let report = intersect_plane_sphere(&outside, &sphere, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    // Plane clear of the sphere in 3D regardless of patch size.
    let far = plane_patch([-10., -10., 4.5], [20., 0., 0.], [0., 20., 0.]);
    let report = intersect_plane_sphere(&far, &sphere, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
}

#[test]
fn rigid_placement_keeps_the_exact_circle() {
    let angle = 0.5;
    let offset = [0.3, -0.2, 1.1];
    let plane = rotated_translated(
        &plane_patch([-3., -3., 1.], [6., 0., 0.], [0., 6., 0.]),
        angle,
        offset,
    );
    let sphere = rotated_translated(&brep_core::analytic::sphere(2.).unwrap(), angle, offset);
    let report = intersect_plane_sphere(&plane, &sphere, Options::default()).unwrap();
    let (curve, center, radius, normal, plane_uv, sphere_uv, sampled) =
        only_circle(&report, true);
    let (sin, cos) = angle.sin_cos();
    let placed = |p: [f64; 3]| {
        [
            p[0] + offset[0],
            cos * p[1] - sin * p[2] + offset[1],
            sin * p[1] + cos * p[2] + offset[2],
        ]
    };
    let placed_center = placed([0., 0., 1.]);
    let placed_normal = sub(placed([0., 0., 2.]), placed([0., 0., 1.]));
    assert!((radius - 3_f64.sqrt()).abs() <= 1e-12, "{radius}");
    assert!(
        sub(center, placed_center).iter().all(|x| x.abs() <= 1e-12),
        "{center:?}"
    );
    assert!(
        sub(normal, placed_normal).iter().all(|x| x.abs() <= 1e-12),
        "{normal:?}"
    );
    let mut worst = 0_f64;
    for i in 0..16 {
        let p = curve.evaluate(i as f64 / 4.).unwrap().point;
        let p = [p[0], p[1], p[2]];
        worst = worst
            .max(sphere_residual(p, offset, 2.))
            .max(dot(placed_normal, sub(p, placed_center)).abs());
    }
    assert!(worst <= 1e-12, "{worst}");
    assert!(sampled <= 1e-12, "{sampled}");
    let uv_worst = uv_samples(
        &plane,
        &sphere,
        plane_uv,
        sphere_uv,
        (offset, 2.),
        (placed_normal, placed([-3., -3., 1.])),
    );
    assert!(uv_worst <= 1e-9, "{uv_worst}");
}

#[test]
fn noncanonical_operands_are_explicit_unsupported_regions() {
    let plane = plane_patch([-3., -3., 1.], [6., 0., 0.], [0., 6., 0.]);
    let sphere = brep_core::analytic::sphere(2.).unwrap();
    for (a, b) in [
        (plane.clone(), brep_core::analytic::cylinder(1., 2.).unwrap()),
        (
            plane.clone(),
            brep_core::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap(),
        ),
        (brep_core::analytic::cylinder(1., 2.).unwrap(), sphere.clone()),
        // A closed solid is not a planar patch even though it has faces.
        (
            brep_core::cuboid([0., 0., 0.], [4., 4., 1.]).unwrap(),
            sphere.clone(),
        ),
    ] {
        let report = intersect_plane_sphere(&a, &b, Options::default()).unwrap();
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
        assert!(!report.permits_topology_change());
    }
    // A skewed (non-rectangular) affine patch is not canonical.
    let skewed = plane_patch([-3., -3., 1.], [6., 0., 0.], [3., 6., 0.]);
    let report = intersect_plane_sphere(&skewed, &sphere, Options::default()).unwrap();
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::UnsupportedSurface
    );
    // A structurally perturbed patch (non-unit weight) no longer passes
    // model validation at all: the entry point refuses it outright.
    let mut perturbed = plane.clone();
    perturbed.faces[0].surface.weights[0][0] = 2.;
    assert!(intersect_plane_sphere(&perturbed, &sphere, Options::default()).is_err());
    // A valid canonical pair still resolves.
    let report = intersect_plane_sphere(&plane, &sphere, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
}
