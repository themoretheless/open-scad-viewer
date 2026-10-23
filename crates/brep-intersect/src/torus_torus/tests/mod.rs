use super::super::test_utils::{rotated_translated, translated};
use super::*;

/// Rotate pi about X then lift along Z: y -> -y, z -> z0 - z (axis flips).
fn flipped(model: &Model, z0: f64) -> Model {
    brep_core::transform::affine(
        model,
        [
            [1., 0., 0., 0.],
            [0., -1., 0., 0.],
            [0., 0., -1., z0],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap()
}
fn only_circles(
    report: &Report<TorusTorusComponent>,
    count: usize,
) -> &Report<TorusTorusComponent> {
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert!(report.unresolved.is_empty(), "{report:?}");
    assert!(!report.permits_topology_change());
    assert_eq!(report.components.len(), count, "{report:?}");
    report
}
#[allow(clippy::type_complexity)]
fn circle_of(
    component: &TorusTorusComponent,
) -> (
    &Curve,
    [f64; 3],
    f64,
    [f64; 3],
    &[TorusPatchCurve],
    &[TorusPatchCurve],
    f64,
) {
    let TorusTorusComponent::Circle {
        curve,
        center,
        radius,
        normal,
        first_uv,
        second_uv,
        max_sample_residual,
    } = component;
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
/// Torus implicit residual (unscaled quartic) in the canonical z-up frame.
fn implicit(point: [f64; 3], major: f64, minor: f64) -> f64 {
    let total = dot(point, point);
    let rho2 = point[0] * point[0] + point[1] * point[1];
    let base = total + major * major - minor * minor;
    (base * base - 4. * major * major * rho2).abs()
}
/// Exact rational 90-degree-arc parameter (the plane_torus map).
fn arc_parameter(local: f64) -> f64 {
    let t = (local / 2.).tan();
    let s = std::f64::consts::SQRT_2 * t / (1. - t);
    s / (1. + s)
}
/// Meridian circle/circle oracle: roots of (rho-R1)^2 + z^2 = r1^2
/// against (rho-R2)^2 + (z-h)^2 = r2^2 as (rho, z) pairs.
fn meridian_roots(r1: f64, big_r1: f64, r2: f64, big_r2: f64, h: f64) -> [(f64, f64); 2] {
    let dr = big_r2 - big_r1;
    let d = dr.hypot(h);
    let a = (d * d + r1 * r1 - r2 * r2) / (2. * d);
    let l = (r1 * r1 - a * a).sqrt();
    let root = |sign: f64| {
        (
            big_r1 + (a * dr - sign * l * h) / d,
            (a * h + sign * l * dr) / d,
        )
    };
    [root(1.), root(-1.)]
}
/// Worst residual of lifted UV arcs evaluated through their own surface,
/// against both caller-supplied torus predicates.
fn uv_samples(
    first_model: &Model,
    second_model: &Model,
    first_uv: &[TorusPatchCurve],
    second_uv: &[TorusPatchCurve],
    first_check: impl Fn([f64; 3]) -> f64,
    second_check: impl Fn([f64; 3]) -> f64,
) -> f64 {
    let mut worst = 0_f64;
    for (model, lifts) in [(first_model, first_uv), (second_model, second_uv)] {
        for lift in lifts {
            let surface = &model.faces[lift.patch].surface;
            for arc in &lift.arcs {
                for k in 0..=8 {
                    let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                    assert!((0. ..=1.).contains(&uv[0]) && (0. ..=1.).contains(&uv[1]));
                    let p = surface.evaluate(uv[0], uv[1]).unwrap().point;
                    let p = [p[0], p[1], p[2]];
                    worst = worst.max(first_check(p)).max(second_check(p));
                }
            }
        }
    }
    worst
}

#[test]
fn two_circle_pair_matches_the_circle_circle_oracle() {
    // Torus R=3, r=1 at the origin against torus R=3, r=sqrt(5.2) lifted
    // to z=3: the meridian centers (3,0) and (3,3) sit at d=3, a=0.8,
    // l=0.6 — the exact circle pair of radii 2.4 and 3.6, both at
    // z=0.8, sorted by height then radius.
    let first = brep_core::analytic::torus(3., 1.).unwrap();
    let second = translated(
        &brep_core::analytic::torus(3., 5.2_f64.sqrt()).unwrap(),
        [0., 0., 3.],
    );
    let report = intersect_torus_torus(&first, &second, Options::default()).unwrap();
    let report = only_circles(&report, 2);
    for (component, rho) in report.components.iter().zip([2.4, 3.6]) {
        let (curve, center, radius, normal, first_uv, second_uv, sampled) =
            circle_of(component);
        assert!((radius - rho).abs() <= 1e-12, "{radius}");
        assert!(
            sub(center, [0., 0., 0.8]).iter().all(|x| x.abs() <= 1e-12),
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
        // Sixteen samples satisfy both torus implicit equations.
        let mut worst = 0_f64;
        for i in 0..16 {
            let p = curve.evaluate(i as f64 / 4.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            worst = worst.max(implicit(p, 3., 1.)).max(implicit(
                sub(p, [0., 0., 3.]),
                3.,
                5.2_f64.sqrt(),
            ));
        }
        assert!(worst <= 1e-10, "{worst}");
        assert!(sampled <= 1e-10, "{sampled}");
        // First-torus lift: one iso-v line per revolution quadrant patch
        // of the profile row, at the exact rational-arc v parameter.
        assert_eq!(first_uv.len(), 4, "{first_uv:?}");
        let phi = 0.8_f64.atan2(rho - 3.).rem_euclid(TAU);
        let quadrant = (phi / std::f64::consts::FRAC_PI_2).floor() as usize;
        let v0 = arc_parameter(phi - quadrant as f64 * std::f64::consts::FRAC_PI_2);
        for lift in first_uv {
            assert_eq!(lift.arcs.len(), 1);
            let arc = &lift.arcs[0];
            assert_eq!(arc.degree, 1);
            assert!((arc.control_points[0][1] - v0).abs() <= 1e-12, "{arc:?}");
            assert!((arc.control_points[1][1] - v0).abs() <= 1e-12, "{arc:?}");
            assert_eq!(arc.control_points[0][0], 0.);
            assert_eq!(arc.control_points[1][0], 1.);
        }
        // Second-torus lift at the exact profile angle atan2(z-3, rho-3).
        assert_eq!(second_uv.len(), 4, "{second_uv:?}");
        let phi2 = (0.8_f64 - 3.).atan2(rho - 3.).rem_euclid(TAU);
        let quadrant2 = (phi2 / std::f64::consts::FRAC_PI_2).floor() as usize;
        let v2 = arc_parameter(phi2 - quadrant2 as f64 * std::f64::consts::FRAC_PI_2);
        for lift in second_uv {
            assert_eq!(lift.arcs.len(), 1);
            let arc = &lift.arcs[0];
            assert_eq!(arc.degree, 1);
            assert!((arc.control_points[0][1] - v2).abs() <= 1e-12, "{arc:?}");
            assert!((arc.control_points[1][1] - v2).abs() <= 1e-12, "{arc:?}");
        }
        let uv_worst = uv_samples(
            &first,
            &second,
            first_uv,
            second_uv,
            |p| implicit(p, 3., 1.),
            |p| implicit(sub(p, [0., 0., 3.]), 3., 5.2_f64.sqrt()),
        );
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }
}

#[test]
fn equal_minor_radii_pair_matches_the_oracle() {
    // Torus R=3, r=1 at the origin against torus R=3, r=1 lifted to
    // z=1: d=1, a=1/2, l=sqrt(3)/2 — the exact circles of radii
    // 3 -+ sqrt(3)/2, both at z=1/2.
    let first = brep_core::analytic::torus(3., 1.).unwrap();
    let second = translated(&brep_core::analytic::torus(3., 1.).unwrap(), [0., 0., 1.]);
    let report = intersect_torus_torus(&first, &second, Options::default()).unwrap();
    let report = only_circles(&report, 2);
    let root = 0.75_f64.sqrt();
    for (component, rho) in report.components.iter().zip([3. - root, 3. + root]) {
        let (curve, center, radius, _, first_uv, second_uv, sampled) = circle_of(component);
        assert!((radius - rho).abs() <= 1e-12, "{radius} vs {rho}");
        assert!(
            sub(center, [0., 0., 0.5]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        let mut worst = 0_f64;
        for i in 0..16 {
            let p = curve.evaluate(i as f64 / 4.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            worst = worst
                .max(implicit(p, 3., 1.))
                .max(implicit(sub(p, [0., 0., 1.]), 3., 1.));
        }
        assert!(worst <= 1e-10, "{worst}");
        assert!(sampled <= 1e-10, "{sampled}");
        assert_eq!(first_uv.len(), 4);
        assert_eq!(second_uv.len(), 4);
        let uv_worst = uv_samples(
            &first,
            &second,
            first_uv,
            second_uv,
            |p| implicit(p, 3., 1.),
            |p| implicit(sub(p, [0., 0., 1.]), 3., 1.),
        );
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }
}

#[test]
fn generic_configuration_matches_the_meridian_oracle() {
    // Torus R=3, r=1 at the origin against torus R=2.5, r=1.2 lifted to
    // z=0.7: both roots land strictly inside both meridian circles and
    // revolve into two circles.
    let first = brep_core::analytic::torus(3., 1.).unwrap();
    let second = translated(&brep_core::analytic::torus(2.5, 1.2).unwrap(), [0., 0., 0.7]);
    let report = intersect_torus_torus(&first, &second, Options::default()).unwrap();
    let report = only_circles(&report, 2);
    let mut roots = meridian_roots(1., 3., 1.2, 2.5, 0.7);
    roots.sort_by(|x, y| x.1.total_cmp(&y.1).then(x.0.total_cmp(&y.0)));
    for (component, (rho, z)) in report.components.iter().zip(roots) {
        let (_curve, center, radius, _, first_uv, second_uv, sampled) = circle_of(component);
        assert!((radius - rho).abs() <= 1e-12, "{radius} vs {rho}");
        assert!(
            sub(center, [0., 0., z]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?} vs {z}"
        );
        assert!(sampled <= 1e-10, "{sampled}");
        assert_eq!(first_uv.len(), 4);
        assert_eq!(second_uv.len(), 4);
        let uv_worst = uv_samples(
            &first,
            &second,
            first_uv,
            second_uv,
            |p| implicit(p, 3., 1.),
            |p| implicit(sub(p, [0., 0., 0.7]), 2.5, 1.2),
        );
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }
}

#[test]
fn anti_axial_second_torus_keeps_the_exact_circles() {
    // The two-circle configuration with the second torus flipped
    // (rotation pi about X, then z -> 6 - z): its center plane stays at
    // z=3 but its axis is -z. The surface is symmetric, so the exact
    // circles are unchanged and the second lift follows its own frame.
    let first = brep_core::analytic::torus(3., 1.).unwrap();
    let second = flipped(
        &translated(
            &brep_core::analytic::torus(3., 5.2_f64.sqrt()).unwrap(),
            [0., 0., 3.],
        ),
        6.,
    );
    let report = intersect_torus_torus(&first, &second, Options::default()).unwrap();
    let report = only_circles(&report, 2);
    for (component, rho) in report.components.iter().zip([2.4, 3.6]) {
        let (_, center, radius, _, first_uv, second_uv, sampled) = circle_of(component);
        assert!((radius - rho).abs() <= 1e-12, "{radius}");
        assert!(
            sub(center, [0., 0., 0.8]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        assert!(sampled <= 1e-10, "{sampled}");
        assert_eq!(first_uv.len(), 4);
        // Flipped lift: profile angle atan2(-(z-3), rho-3) in its frame.
        assert_eq!(second_uv.len(), 4);
        let phi2 = (3. - 0.8_f64).atan2(rho - 3.).rem_euclid(TAU);
        let quadrant2 = (phi2 / std::f64::consts::FRAC_PI_2).floor() as usize;
        let v2 = arc_parameter(phi2 - quadrant2 as f64 * std::f64::consts::FRAC_PI_2);
        for lift in second_uv {
            let arc = &lift.arcs[0];
            assert!((arc.control_points[0][1] - v2).abs() <= 1e-12, "{arc:?}");
        }
        let uv_worst = uv_samples(
            &first,
            &second,
            first_uv,
            second_uv,
            |p| implicit(p, 3., 1.),
            |p| implicit(sub(p, [0., 0., 3.]), 3., 5.2_f64.sqrt()),
        );
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }
}

#[test]
fn external_meridian_tangency_stays_a_tangency_region() {
    // Torus R=3, r=1 at the origin against torus R=3, r=1 at z=2: the
    // meridian circles touch externally (d = 2 = r_1 + r_2) at rho=3,
    // z=1 — the contact revolves into a circle but is never guessed.
    let first = brep_core::analytic::torus(3., 1.).unwrap();
    for dz in [0., 2e-15, -2e-15] {
        let second = translated(&brep_core::analytic::torus(3., 1.).unwrap(), [0., 0., 2. + dz]);
        let report = intersect_torus_torus(&first, &second, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
        assert!(!report.permits_topology_change());
    }
    // Just clear of the band: provable miss, resolved.
    let clear = translated(
        &brep_core::analytic::torus(3., 1.).unwrap(),
        [0., 0., 2. + 1e-9],
    );
    let report = intersect_torus_torus(&first, &clear, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    // Just across: two small transverse circles around the touch point.
    let across = translated(
        &brep_core::analytic::torus(3., 1.).unwrap(),
        [0., 0., 2. - 1e-9],
    );
    let report = intersect_torus_torus(&first, &across, Options::default()).unwrap();
    only_circles(&report, 2);
}

#[test]
fn internal_meridian_tangency_stays_a_tangency_region() {
    // Torus R=3, r=1 at the origin against torus R=3, r=2 at z=1: the
    // meridian circles touch internally (d = 1 = |r_1 - r_2|) at rho=3,
    // z=-1 — never a guessed circle.
    let first = brep_core::analytic::torus(3., 1.).unwrap();
    for dr in [0., 2e-15, -2e-15] {
        let second = translated(&brep_core::analytic::torus(3., 2. + dr).unwrap(), [0., 0., 1.]);
        let report = intersect_torus_torus(&first, &second, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
    }
    // Just larger: the second meridian circle swallows the first,
    // provably contained, resolved empty.
    let swallow = translated(
        &brep_core::analytic::torus(3., 2. + 1e-9).unwrap(),
        [0., 0., 1.],
    );
    let report = intersect_torus_torus(&first, &swallow, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    // Just smaller: two transverse circles around the touch point.
    let across = translated(
        &brep_core::analytic::torus(3., 2. - 1e-9).unwrap(),
        [0., 0., 1.],
    );
    let report = intersect_torus_torus(&first, &across, Options::default()).unwrap();
    only_circles(&report, 2);
}

#[test]
fn coincident_tori_report_coincident_trim() {
    // The identical torus twice: equal meridian circles within the band
    // (R_1 == R_2, h == 0, r_1 == r_2) — coincident surfaces, never a
    // guessed curve.
    let first = brep_core::analytic::torus(3., 1.).unwrap();
    let second = brep_core::analytic::torus(3., 1.).unwrap();
    let report = intersect_torus_torus(&first, &second, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(report.unresolved.len(), 1);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::CoincidentTrim
    );
    assert_eq!(
        report.unresolved[0].parameter_box,
        vec![0., 1., 0., 1., 0., 1., 0., 1.]
    );
    assert!(!report.permits_topology_change());
}

#[test]
fn nested_and_separate_pairs_resolve_empty() {
    let first = brep_core::analytic::torus(3., 1.).unwrap();
    // Concentric within the band with a provably different minor radius:
    // certified containment through the d <= band branch.
    let concentric = brep_core::analytic::torus(3., 0.5).unwrap();
    // Nested without contact: R=3.2, r=2.5 contains the first tube
    // (d = 0.2, d + r_1 = 1.2 < r_2 = 2.5).
    let nested = brep_core::analytic::torus(3.2, 2.5).unwrap();
    // The first torus nested inside the second's tube without contact:
    // R=2.7, r=0.4 (d = 0.3, d + r_2 = 0.7 < r_1 = 1).
    let inner = brep_core::analytic::torus(2.7, 0.4).unwrap();
    // Far along the axis, no reach back.
    let far = translated(&brep_core::analytic::torus(3., 1.).unwrap(), [0., 0., 10.]);
    for second in [&concentric, &nested, &inner, &far] {
        let report = intersect_torus_torus(&first, second, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
    }
}

#[test]
fn near_coaxial_bands_stay_unresolved_not_forced() {
    let first = brep_core::analytic::torus(3., 1.).unwrap();
    let base = translated(
        &brep_core::analytic::torus(3., 5.2_f64.sqrt()).unwrap(),
        [0., 0., 3.],
    );
    // Center-line offset inside the recognition band: near_coincidence.
    let near = translated(&base, [1e-10, 0., 0.]);
    let report = intersect_torus_torus(&first, &near, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::NearCoincidence
    );
    // Recognition-scale tilt of the second axis: near_coincidence.
    let (sin, cos) = 1e-10_f64.sin_cos();
    let tilted = brep_core::transform::affine(
        &base,
        [
            [cos, 0., sin, 0.],
            [0., 1., 0., 0.],
            [-sin, 0., cos, 3. - 3. * cos],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let report = intersect_torus_torus(&first, &tilted, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::NearCoincidence
    );
}

#[test]
fn clearly_off_axis_or_tilted_pairs_are_unsupported_regions() {
    let first = brep_core::analytic::torus(3., 1.).unwrap();
    let base = translated(
        &brep_core::analytic::torus(3., 5.2_f64.sqrt()).unwrap(),
        [0., 0., 3.],
    );
    let off = translated(&base, [0.5, 0., 0.]);
    let (sin, cos) = 0.3_f64.sin_cos();
    let tilted = brep_core::transform::affine(
        &base,
        [
            [cos, 0., sin, -3. * sin],
            [0., 1., 0., 0.],
            [-sin, 0., cos, 3. - 3. * cos],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    for second in [&off, &tilted] {
        let report = intersect_torus_torus(&first, second, Options::default()).unwrap();
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

#[test]
fn noncanonical_operands_are_explicit_unsupported_regions() {
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let other = translated(
        &brep_core::analytic::torus(3., 5.2_f64.sqrt()).unwrap(),
        [0., 0., 3.],
    );
    // Spheres, cuboids, cylinders and frusta are not the canonical
    // operand on either side.
    for (a, b) in [
        (
            brep_core::analytic::sphere(2.).unwrap(),
            brep_core::analytic::torus(3., 1.).unwrap(),
        ),
        (
            brep_core::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap(),
            brep_core::analytic::torus(3., 1.).unwrap(),
        ),
        (
            brep_core::analytic::torus(3., 1.).unwrap(),
            brep_core::analytic::cylinder(2.2, 8.).unwrap(),
        ),
        (
            brep_core::analytic::torus(3., 1.).unwrap(),
            brep_core::analytic::frustum(1., 3., 6.).unwrap(),
        ),
        (
            brep_core::analytic::frustum(1., 3., 6.).unwrap(),
            brep_core::analytic::frustum(1., 3., 6.).unwrap(),
        ),
    ] {
        let report = intersect_torus_torus(&a, &b, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(report.unresolved.len(), 1);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::UnsupportedSurface
        );
    }
    // A canonical pair still resolves.
    let report = intersect_torus_torus(&torus, &other, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    assert_eq!(report.components.len(), 2);
    // A structurally perturbed torus (one moved control point) fails the
    // exact certification and is an unsupported region, never a fallback.
    let mut perturbed = other.clone();
    perturbed.faces[0].surface.control_points[1][1][0] += 1e-6;
    perturbed.rebuild_topology_ids();
    let report = intersect_torus_torus(&torus, &perturbed, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::UnsupportedSurface
    );
}

#[test]
fn rotated_translated_placement_keeps_the_exact_circles() {
    // Rigid placement of the whole coaxial two-circle configuration
    // about X by 0.5.
    let angle = 0.5;
    let offset = [0.3, -0.2, 1.1];
    let first = rotated_translated(&brep_core::analytic::torus(3., 1.).unwrap(), angle, offset);
    let second = rotated_translated(
        &translated(
            &brep_core::analytic::torus(3., 5.2_f64.sqrt()).unwrap(),
            [0., 0., 3.],
        ),
        angle,
        offset,
    );
    let report = intersect_torus_torus(&first, &second, Options::default()).unwrap();
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
    let axis = sub(placed([0., 0., 1.]), placed([0., 0., 0.]));
    let placed_implicit = |p: [f64; 3], plane_z: f64| {
        // First torus (plane_z = 0): R=3, r^2=1; second (plane_z = 3): r^2=5.2.
        let minor2 = if plane_z == 0. { 1. } else { 5.2 };
        let rel = sub(p, placed([0., 0., plane_z]));
        let a = dot(rel, axis);
        let perp = sub(rel, axis.map(|x| x * a));
        let rho_p = perp[0].hypot(perp[1]).hypot(perp[2]);
        let base = rho_p * rho_p + a * a + 9. - minor2;
        (base * base - 36. * rho_p * rho_p).abs()
    };
    for expected_rho in [2.4, 3.6] {
        // Placement rounding may swap the equal-height circles at the
        // binary64 level; match by radius instead of position.
        let component = report
            .components
            .iter()
            .find(|c| {
                let TorusTorusComponent::Circle { radius, .. } = c;
                (radius - expected_rho).abs() <= 1e-12
            })
            .unwrap_or_else(|| panic!("no circle of radius {expected_rho}"));
        let (curve, center, _radius, normal, first_uv, second_uv, sampled) =
            circle_of(component);
        let expected = placed([0., 0., 0.8]);
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
            worst = worst
                .max(placed_implicit(p, 0.))
                .max(placed_implicit(p, 3.));
        }
        assert!(worst <= 1e-9, "{worst}");
        assert!(sampled <= 1e-10, "{sampled}");
        assert_eq!(first_uv.len(), 4);
        assert_eq!(second_uv.len(), 4);
        let uv_worst = uv_samples(
            &first,
            &second,
            first_uv,
            second_uv,
            |p| placed_implicit(p, 0.),
            |p| placed_implicit(p, 3.),
        );
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }
}
