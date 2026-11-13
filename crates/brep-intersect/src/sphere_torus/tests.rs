use super::super::test_utils::{rotated_translated, translated};
use super::*;

fn only_circles(
    report: &Report<SphereTorusComponent>,
    count: usize,
) -> &Report<SphereTorusComponent> {
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert!(report.unresolved.is_empty(), "{report:?}");
    assert!(!report.permits_topology_change());
    assert_eq!(report.components.len(), count, "{report:?}");
    report
}
#[allow(clippy::type_complexity)]
fn circle_of(
    component: &SphereTorusComponent,
) -> (
    &Curve,
    [f64; 3],
    f64,
    [f64; 3],
    &[SpherePatchCircle],
    &[TorusPatchCurve],
    f64,
) {
    let SphereTorusComponent::Circle {
        curve,
        center,
        radius,
        normal,
        sphere_uv,
        torus_uv,
        max_sample_residual,
    } = component
    else {
        panic!("expected an exact circle, got {component:?}");
    };
    (
        curve,
        *center,
        *radius,
        *normal,
        sphere_uv,
        torus_uv,
        *max_sample_residual,
    )
}
fn sphere_residual(point: [f64; 3], center: [f64; 3], radius: f64) -> f64 {
    let d = sub(point, center);
    (d[0].hypot(d[1]).hypot(d[2]) - radius).abs()
}
/// Torus implicit residual (unscaled quartic) in the canonical z-up frame.
fn implicit(point: [f64; 3], major: f64, minor: f64) -> f64 {
    let total = dot(point, point);
    let rho2 = point[0] * point[0] + point[1] * point[1];
    let base = total + major * major - minor * minor;
    (base * base - 4. * major * major * rho2).abs()
}
/// Worst residual of lifted UV arcs evaluated through their own surface,
/// against the sphere equation and a caller-supplied torus predicate.
fn uv_samples(
    sphere_model: &Model,
    torus_model: &Model,
    sphere_center: [f64; 3],
    sphere_radius: f64,
    sphere_uv: &[SpherePatchCircle],
    torus_uv: &[TorusPatchCurve],
    torus_check: impl Fn([f64; 3]) -> f64,
) -> f64 {
    let mut worst = 0_f64;
    for lift in sphere_uv {
        let surface = &sphere_model.faces[lift.patch].surface;
        for arc in &lift.arcs {
            for k in 0..=8 {
                let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                let p = surface
                    .evaluate(uv[0].clamp(0., 1.), uv[1].clamp(0., 1.))
                    .unwrap()
                    .point;
                let p = [p[0], p[1], p[2]];
                worst = worst
                    .max(sphere_residual(p, sphere_center, sphere_radius))
                    .max(torus_check(p));
            }
        }
    }
    for lift in torus_uv {
        let surface = &torus_model.faces[lift.patch].surface;
        for arc in &lift.arcs {
            for k in 0..=8 {
                let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                assert!((0. ..=1.).contains(&uv[0]) && (0. ..=1.).contains(&uv[1]));
                let p = surface.evaluate(uv[0], uv[1]).unwrap().point;
                let p = [p[0], p[1], p[2]];
                worst = worst
                    .max(sphere_residual(p, sphere_center, sphere_radius))
                    .max(torus_check(p));
            }
        }
    }
    worst
}
/// Exact rational 90-degree-arc parameter (the plane_torus map).
fn arc_parameter(local: f64) -> f64 {
    let t = (local / 2.).tan();
    let s = std::f64::consts::SQRT_2 * t / (1. - t);
    s / (1. + s)
}
/// Meridian circle/circle oracle: roots of (rho-R)^2 + z^2 = r_t^2
/// against rho^2 + (z-h)^2 = r_s^2 as (rho, z) pairs.
fn meridian_roots(major: f64, minor: f64, h: f64, r_s: f64) -> [(f64, f64); 2] {
    let d = major.hypot(h);
    let a = (d * d + minor * minor - r_s * r_s) / (2. * d);
    let l = (minor * minor - a * a).sqrt();
    let root = |sign: f64| {
        (
            major - (a * major + sign * l * h) / d,
            (a * h - sign * l * major) / d,
        )
    };
    [root(1.), root(-1.)]
}

#[test]
fn equator_symmetric_pair_matches_the_circle_circle_oracle() {
    // Torus R=3, r=1; sphere r=sqrt(5.2) centered at the torus center:
    // d=3, a=0.8, l=0.6 — the exact circle pair of radius 2.2 at
    // z = +-0.6.
    let sphere = brep_core::analytic::sphere(5.2_f64.sqrt()).unwrap();
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let report = intersect_sphere_torus(&sphere, &torus, Options::default()).unwrap();
    let report = only_circles(&report, 2);
    for (component, z) in report.components.iter().zip([-0.6, 0.6]) {
        let (curve, center, radius, normal, sphere_uv, torus_uv, sampled) = circle_of(component);
        assert!((radius - 2.2).abs() <= 1e-12, "{radius}");
        assert!(
            sub(center, [0., 0., z]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        assert!(normal[2].abs() >= 1. - 1e-12, "{normal:?}");
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
        // Sixteen samples satisfy both implicit equations.
        let mut worst = 0_f64;
        for i in 0..16 {
            let p = curve.evaluate(i as f64 / 4.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            worst = worst
                .max(sphere_residual(p, [0., 0., 0.], 5.2_f64.sqrt()))
                .max(implicit(p, 3., 1.));
        }
        assert!(worst <= 1e-10, "{worst}");
        assert!(sampled <= 1e-10, "{sampled}");
        // Torus lift: one iso-v line per revolution quadrant patch of the
        // profile row, at the exact rational-arc v parameter.
        assert_eq!(torus_uv.len(), 4, "{torus_uv:?}");
        let phi = z.atan2(2.2 - 3.).rem_euclid(TAU);
        let quadrant = (phi / std::f64::consts::FRAC_PI_2).floor() as usize;
        let v0 = arc_parameter(phi - quadrant as f64 * std::f64::consts::FRAC_PI_2);
        for lift in torus_uv {
            assert_eq!(lift.arcs.len(), 1);
            let arc = &lift.arcs[0];
            assert_eq!(arc.degree, 1);
            assert!((arc.control_points[0][1] - v0).abs() <= 1e-12, "{arc:?}");
            assert!((arc.control_points[1][1] - v0).abs() <= 1e-12, "{arc:?}");
            assert_eq!(arc.control_points[0][0], 0.);
            assert_eq!(arc.control_points[1][0], 1.);
        }
        assert!(!sphere_uv.is_empty());
        let uv_worst = uv_samples(
            &sphere,
            &torus,
            [0., 0., 0.],
            5.2_f64.sqrt(),
            sphere_uv,
            torus_uv,
            |p| implicit(p, 3., 1.),
        );
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }
}

#[test]
fn off_center_sphere_yields_two_distinct_circles() {
    // Torus R=3, r=1; sphere r=sqrt(26) centered at z=-4: d=5, a=0,
    // l=1 — the exact circles of radii 3.8 at z=-0.6 and 2.2 at z=0.6.
    let sphere = translated(
        &brep_core::analytic::sphere(26_f64.sqrt()).unwrap(),
        [0., 0., -4.],
    );
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let report = intersect_sphere_torus(&sphere, &torus, Options::default()).unwrap();
    let report = only_circles(&report, 2);
    for (component, (rho, z)) in report.components.iter().zip([(3.8, -0.6), (2.2, 0.6)]) {
        let (curve, center, radius, _, sphere_uv, torus_uv, sampled) = circle_of(component);
        assert!((radius - rho).abs() <= 1e-12, "{radius}");
        assert!(
            sub(center, [0., 0., z]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        let mut worst = 0_f64;
        for i in 0..16 {
            let p = curve.evaluate(i as f64 / 4.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            worst = worst
                .max(sphere_residual(p, [0., 0., -4.], 26_f64.sqrt()))
                .max(implicit(p, 3., 1.));
        }
        assert!(worst <= 1e-10, "{worst}");
        assert!(sampled <= 1e-10, "{sampled}");
        assert_eq!(torus_uv.len(), 4);
        let uv_worst = uv_samples(
            &sphere,
            &torus,
            [0., 0., -4.],
            26_f64.sqrt(),
            sphere_uv,
            torus_uv,
            |p| implicit(p, 3., 1.),
        );
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }
}

#[test]
fn generic_height_pair_matches_the_meridian_oracle() {
    // Torus R=3, r=1; sphere r=2.2 centered at z=0.5: both roots land
    // strictly inside the meridian circle and revolve into two circles.
    let sphere = translated(&brep_core::analytic::sphere(2.2).unwrap(), [0., 0., 0.5]);
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let report = intersect_sphere_torus(&sphere, &torus, Options::default()).unwrap();
    let report = only_circles(&report, 2);
    let mut roots = meridian_roots(3., 1., 0.5, 2.2);
    roots.sort_by(|a, b| a.1.total_cmp(&b.1));
    for (component, (rho, z)) in report.components.iter().zip(roots) {
        let (_, center, radius, _, _, torus_uv, sampled) = circle_of(component);
        assert!((radius - rho).abs() <= 1e-12, "{radius} vs {rho}");
        assert!(
            sub(center, [0., 0., z]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?} vs {z}"
        );
        assert!(sampled <= 1e-10, "{sampled}");
        assert_eq!(torus_uv.len(), 4);
    }
}

#[test]
fn inner_equator_tangency_stays_a_tangency_region() {
    // Sphere centered at the torus center with r == R - r_t = 2: the
    // meridian circles touch externally (d = 3 = r_t + r_s), the contact
    // revolves into the inner equator circle — never a guessed circle.
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    for r in [2., 2. + 2e-15, 2. - 2e-15] {
        let sphere = brep_core::analytic::sphere(r).unwrap();
        let report = intersect_sphere_torus(&sphere, &torus, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
        assert!(!report.permits_topology_change());
    }
    // Just clear of the band on the outside: provable miss, resolved.
    let clear = brep_core::analytic::sphere(2. - 1e-9).unwrap();
    let report = intersect_sphere_torus(&clear, &torus, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    // Just across: two small transverse circles around the inner equator.
    let across = brep_core::analytic::sphere(2. + 1e-9).unwrap();
    let report = intersect_sphere_torus(&across, &torus, Options::default()).unwrap();
    only_circles(&report, 2);
}

#[test]
fn outer_equator_internal_tangency_stays_a_tangency_region() {
    // Sphere centered at the torus center with r == R + r_t = 4: the
    // meridian circles touch internally (d = 3 = |r_t - r_s|) at the
    // outer equator — never a guessed circle.
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    for r in [4., 4. + 2e-15, 4. - 2e-15] {
        let sphere = brep_core::analytic::sphere(r).unwrap();
        let report = intersect_sphere_torus(&sphere, &torus, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
    }
    // Just larger: the sphere swallows the tube, provably contained.
    let swallow = brep_core::analytic::sphere(4. + 1e-9).unwrap();
    let report = intersect_sphere_torus(&swallow, &torus, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    // Just smaller: two transverse circles around the outer equator.
    let across = brep_core::analytic::sphere(4. - 1e-9).unwrap();
    let report = intersect_sphere_torus(&across, &torus, Options::default()).unwrap();
    only_circles(&report, 2);
    // Off-center external tangency: r_s = hypot(R, h) - r_t at h = 0.5.
    let tangent = 3_f64.hypot(0.5) - 1.;
    let sphere = translated(
        &brep_core::analytic::sphere(tangent).unwrap(),
        [0., 0., 0.5],
    );
    let report = intersect_sphere_torus(&sphere, &torus, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::TangencyOrMultipleRoot
    );
}

#[test]
fn zero_circle_configurations_resolve_empty() {
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    // Small sphere floating in the hole, clear of the tube.
    let hole = translated(&brep_core::analytic::sphere(1.).unwrap(), [0., 0., 0.5]);
    // Sphere far along the axis, no reach back.
    let far = translated(&brep_core::analytic::sphere(1.).unwrap(), [0., 0., 10.]);
    // Huge sphere swallowing the whole torus (contained meridian circle).
    let swallow = brep_core::analytic::sphere(10.).unwrap();
    for sphere in [&hole, &far, &swallow] {
        let report = intersect_sphere_torus(sphere, &torus, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
    }
}

#[test]
fn near_axial_offset_within_the_band_stays_unresolved() {
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let sphere = translated(
        &brep_core::analytic::sphere(5.2_f64.sqrt()).unwrap(),
        [1e-10, 0., 0.],
    );
    let report = intersect_sphere_torus(&sphere, &torus, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::NearCoincidence
    );
}

#[test]
fn clearly_off_axis_contacts_are_traced_and_certified() {
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    for (radius, at) in [
        (5.2_f64.sqrt(), [0.5, 0., 0.]),
        (1., [4.5, 0., 0.]),
        (1., [3.5, 0.4, 0.3]),
    ] {
        let sphere = translated(&brep_core::analytic::sphere(radius).unwrap(), at);
        let report = intersect_sphere_torus(&sphere, &torus, Options::default()).unwrap();
        assert!(
            report.unresolved.is_empty(),
            "{radius} at {at:?}: {:?}",
            report.unresolved
        );
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        assert!(!report.permits_topology_change());
        assert!(!report.components.is_empty());
        for component in &report.components {
            let SphereTorusComponent::Traced {
                pieces,
                distance_bound,
                spans,
                sphere_uv,
                torus_uv,
                lift_deviation,
                complete,
            } = component
            else {
                panic!("expected a traced component off the axis");
            };
            assert!(*distance_bound <= TRACE_TOLERANCE && *spans > 0);
            assert!(*lift_deviation <= LIFT_TOLERANCE);
            assert!(*complete, "{radius} at {at:?}: component set not certified complete");
            assert!(!sphere_uv.is_empty() && !torus_uv.is_empty());
            // Independent check of the lifts: mapped pcurve samples sit on
            // both canonical surfaces and inside the patch domain.
            let lifted = |surface: &nurbs_core::surface::Surface, arcs: &[Curve]| {
                for arc in arcs {
                    for k in 0..=32 {
                        let uv = arc.evaluate(k as f64 / 32.).unwrap().point;
                        assert!(uv.iter().all(|x| (-1e-9..=1. + 1e-9).contains(x)), "{uv:?}");
                        let p = surface
                            .evaluate(uv[0].clamp(0., 1.), uv[1].clamp(0., 1.))
                            .unwrap()
                            .point;
                        let d = sub([p[0], p[1], p[2]], at);
                        let on_sphere = (d[0].hypot(d[1]).hypot(d[2]) - radius).abs();
                        let on_torus = ((p[0].hypot(p[1]) - 3.).hypot(p[2]) - 1.).abs();
                        // The certified bound covers every sample (plus the
                        // authored surfaces' own rounding).
                        let bound = lift_deviation + 1e-12;
                        assert!(
                            on_sphere.min(on_torus) <= 1e-12 && on_sphere.max(on_torus) <= bound,
                            "{on_sphere} {on_torus} > {lift_deviation}"
                        );
                    }
                }
            };
            for lift in sphere_uv {
                lifted(&sphere.faces[lift.patch].surface, &lift.arcs);
            }
            for lift in torus_uv {
                lifted(&torus.faces[lift.patch].surface, &lift.arcs);
            }
            // Independent dense samples against the canonical surfaces.
            for piece in pieces {
                for k in 0..=40 {
                    let p = piece.evaluate(k as f64 / 40.).unwrap().point;
                    let d = sub([p[0], p[1], p[2]], at);
                    let on_sphere = (d[0].hypot(d[1]).hypot(d[2]) - radius).abs();
                    let on_torus = ((p[0].hypot(p[1]) - 3.).hypot(p[2]) - 1.).abs();
                    assert!(on_sphere <= TRACE_TOLERANCE && on_torus <= TRACE_TOLERANCE);
                }
            }
            for w in pieces.windows(2) {
                assert_eq!(w[0].control_points.last(), w[1].control_points.first());
            }
        }
    }
}

#[test]
fn off_axis_misses_resolve_empty_and_tangencies_stay_unresolved() {
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let empty = |radius: f64, at: [f64; 3]| {
        let sphere = translated(&brep_core::analytic::sphere(radius).unwrap(), at);
        intersect_sphere_torus(&sphere, &torus, Options::default()).unwrap()
    };
    for (radius, at) in [
        // Inside the tube, off the axis: dmin = 0.1 < r_t, gap 0.6.
        (0.3, [3.1, 0., 0.]),
        // Beside the torus, clear of the tube.
        (1., [6., 1., 0.5]),
        // Above the hole, off the axis.
        (0.5, [1., 0.5, 2.]),
        // Swallowing the whole torus from off the axis: dmax + r_t < 9.
        (9., [0.7, -0.4, 1.]),
    ] {
        let report = empty(radius, at);
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{radius} at {at:?}: {report:?}"
        );
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
    }
    // A miss by 1e-6, beyond the rounding band, resolves empty.
    assert!(empty(0.9 - 1e-6, [3.1, 0., 0.]).unresolved.is_empty());
    // An exact internal tangency stays unresolved: the band never turns a
    // contact into a miss, and the tracer refuses to guess a touching point.
    {
        let (radius, at) = (0.9, [3.1, 0., 0.]);
        let report = empty(radius, at);
        assert_eq!(report.coverage, Coverage::Incomplete, "{radius} at {at:?}");
        assert!(report.components.is_empty());
        assert!(matches!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot | UnresolvedReason::UnsupportedSurface
        ));
    }
}

#[test]
fn noncanonical_operands_are_explicit_unsupported_regions() {
    let sphere = brep_core::analytic::sphere(5.2_f64.sqrt()).unwrap();
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    // Cylinders, tori, cuboids and tori-as-first-operand are neither
    // canonical operand; swapped operand order is refused by the fixed
    // order.
    for (a, b) in [
        (
            brep_core::analytic::cylinder(1., 3.).unwrap(),
            brep_core::analytic::torus(3., 1.).unwrap(),
        ),
        (
            brep_core::analytic::sphere(2.).unwrap(),
            brep_core::analytic::sphere(2.).unwrap(),
        ),
        (
            brep_core::analytic::torus(3., 1.).unwrap(),
            brep_core::analytic::torus(2., 1.).unwrap(),
        ),
        (
            brep_core::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap(),
            brep_core::analytic::torus(3., 1.).unwrap(),
        ),
        (
            brep_core::analytic::torus(3., 1.).unwrap(),
            brep_core::analytic::sphere(2.).unwrap(),
        ),
    ] {
        let report = intersect_sphere_torus(&a, &b, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(report.unresolved.len(), 1);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::UnsupportedSurface
        );
    }
    // A canonical pair still resolves.
    let report = intersect_sphere_torus(&sphere, &torus, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    assert_eq!(report.components.len(), 2);
    // A structurally perturbed torus (one moved control point) fails the
    // exact certification and is an unsupported region, never a fallback.
    let mut perturbed = torus.clone();
    perturbed.faces[0].surface.control_points[1][1][0] += 1e-6;
    perturbed.rebuild_topology_ids();
    let report = intersect_sphere_torus(&sphere, &perturbed, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::UnsupportedSurface
    );
}

#[test]
fn rotated_translated_placement_keeps_the_exact_circles() {
    // Rigid placement of the whole axial configuration about X by 0.5.
    let angle = 0.5;
    let offset = [0.3, -0.2, 1.1];
    let sphere = rotated_translated(
        &translated(
            &brep_core::analytic::sphere(26_f64.sqrt()).unwrap(),
            [0., 0., -4.],
        ),
        angle,
        offset,
    );
    let torus = rotated_translated(&brep_core::analytic::torus(3., 1.).unwrap(), angle, offset);
    let report = intersect_sphere_torus(&sphere, &torus, Options::default()).unwrap();
    let report = only_circles(&report, 2);
    // Independent binary64 oracle in the placed frame.
    let (sin, cos) = angle.sin_cos();
    let placed = |p: [f64; 3]| {
        [
            p[0] + offset[0],
            cos * p[1] - sin * p[2] + offset[1],
            sin * p[1] + cos * p[2] + offset[2],
        ]
    };
    let sphere_center = placed([0., 0., -4.]);
    let axis = sub(placed([0., 0., 1.]), placed([0., 0., 0.]));
    for (component, (rho, z)) in report.components.iter().zip([(3.8, -0.6), (2.2, 0.6)]) {
        let (curve, center, radius, normal, sphere_uv, torus_uv, sampled) = circle_of(component);
        let expected = placed([0., 0., z]);
        assert!((radius - rho).abs() <= 1e-12, "{radius}");
        assert!(
            sub(center, expected).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        assert!(
            sub(normal, axis).iter().all(|x| x.abs() <= 1e-12),
            "{normal:?}"
        );
        let mut worst = 0_f64;
        for i in 0..16 {
            let p = curve.evaluate(i as f64 / 4.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            worst = worst.max(sphere_residual(p, sphere_center, 26_f64.sqrt()));
            // Placed torus implicit via the axial/radial decomposition.
            let rel = sub(p, placed([0., 0., 0.]));
            let a = dot(rel, axis);
            let perp = sub(rel, axis.map(|x| x * a));
            let rho_p = perp[0].hypot(perp[1]).hypot(perp[2]);
            let base = rho_p * rho_p + a * a + 9. - 1.;
            worst = worst.max((base * base - 36. * rho_p * rho_p).abs());
        }
        assert!(worst <= 1e-9, "{worst}");
        assert!(sampled <= 1e-10);
        assert_eq!(torus_uv.len(), 4);
        assert!(!sphere_uv.is_empty());
        let axis = normal;
        let uv_worst = uv_samples(
            &sphere,
            &torus,
            sphere_center,
            26_f64.sqrt(),
            sphere_uv,
            torus_uv,
            |p| {
                let rel = sub(p, placed([0., 0., 0.]));
                let a = dot(rel, axis);
                let perp = sub(rel, axis.map(|x| x * a));
                let rho_p = perp[0].hypot(perp[1]).hypot(perp[2]);
                let base = rho_p * rho_p + a * a + 9. - 1.;
                (base * base - 36. * rho_p * rho_p).abs()
            },
        );
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }
}

#[test]
fn off_axis_sphere_torus_booleans_close_and_balance_volumes() {
    use std::f64::consts::PI;
    let volume = |m: &Model| {
        brep_core::analysis::mass_properties(m, 1e-5, 2_000_000)
            .unwrap()
            .signed_volume_mm3
    };
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let vt = 2. * PI * PI * 3. * 1.;
    for (radius, at) in [(1., [3.5, 0.4, 0.3]), (1., [4.5, 0., 0.])] {
        let sphere = translated(&brep_core::analytic::sphere(radius).unwrap(), at);
        let vs = 4. / 3. * PI * radius * radius * radius;
        let mut v = std::collections::BTreeMap::new();
        for op in ["union", "intersection", "difference"] {
            let result = sphere_torus_boolean(&sphere, &torus, op)
                .unwrap_or_else(|e| panic!("{op} {radius} at {at:?}: {e:?}"));
            result.validate().unwrap();
            assert!(result.tolerance_mm <= 1e-5, "{op}: {}", result.tolerance_mm);
            v.insert(op, volume(&result));
        }
        let (u, i, d) = (v["union"], v["intersection"], v["difference"]);
        let tol = 1e-3 * (vs + vt);
        assert!(i > 0. && i < vs.min(vt), "{i}");
        assert!((u + i - vs - vt).abs() <= tol, "union {u} + inter {i} vs {}", vs + vt);
        assert!((d - (vs - i)).abs() <= tol, "difference {d} vs {}", vs - i);
        // The other operand order mirrors the difference.
        let torus_minus = sphere_torus_boolean(&torus, &sphere, "difference").unwrap();
        assert!((volume(&torus_minus) - (vt - i)).abs() <= tol);
    }
}

#[test]
fn off_axis_sphere_cylinder_and_cone_booleans_balance_volumes() {
    use std::f64::consts::PI;
    let volume = |m: &Model| {
        brep_core::analysis::mass_properties(m, 1e-5, 2_000_000)
            .unwrap()
            .signed_volume_mm3
    };
    let cases: [(Model, f64, f64, [f64; 3], bool); 2] = [
        (brep_core::analytic::cylinder(2., 8.).unwrap(), PI * 4. * 8., 1., [2.3, 0.4, 3.], true),
        (
            brep_core::analytic::frustum(2., 4., 6.).unwrap(),
            PI * 6. * (4. + 8. + 16.) / 3.,
            1.,
            [3., 0., 3.],
            false,
        ),
    ];
    for (other, vo, radius, at, cylinder) in cases {
        let sphere = translated(&brep_core::analytic::sphere(radius).unwrap(), at);
        let vs = 4. / 3. * PI * radius.powi(3);
        let run = |a: &Model, b: &Model, op: &str| {
            if cylinder {
                sphere_cylinder_boolean(a, b, op)
            } else {
                sphere_cone_boolean(a, b, op)
            }
            .unwrap_or_else(|e| panic!("{op} at {at:?}: {e:?}"))
        };
        let u = run(&sphere, &other, "union");
        let i = run(&sphere, &other, "intersection");
        let d = run(&other, &sphere, "difference");
        for m in [&u, &i, &d] {
            m.validate().unwrap();
        }
        let (u, i, d) = (volume(&u), volume(&i), volume(&d));
        let tol = 1e-3 * (vs + vo);
        assert!(i > 0. && i < vs, "{i}");
        assert!((u + i - vs - vo).abs() <= tol, "union {u} + {i} vs {}", vs + vo);
        assert!((d - (vo - i)).abs() <= tol, "difference {d} vs {}", vo - i);
    }
}
