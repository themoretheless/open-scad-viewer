use super::super::test_utils::{rotated_translated, translated};
use super::*;

fn only_circles(
    report: &Report<CylinderTorusComponent>,
    count: usize,
) -> &Report<CylinderTorusComponent> {
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert!(report.unresolved.is_empty(), "{report:?}");
    assert!(!report.permits_topology_change());
    assert_eq!(report.components.len(), count, "{report:?}");
    report
}
#[allow(clippy::type_complexity)]
fn circle_of(
    component: &CylinderTorusComponent,
) -> (
    &Curve,
    [f64; 3],
    f64,
    [f64; 3],
    &[CylinderPatchCurve],
    &[TorusPatchCurve],
    f64,
) {
    let CylinderTorusComponent::Circle {
        curve,
        center,
        radius,
        normal,
        cylinder_uv,
        torus_uv,
        max_sample_residual,
    } = component;
    (
        curve,
        *center,
        *radius,
        *normal,
        cylinder_uv,
        torus_uv,
        *max_sample_residual,
    )
}
/// Radial distance from the z axis (canonical frames in these tests).
fn radial_z(point: [f64; 3]) -> f64 {
    point[0].hypot(point[1])
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
/// Worst residual of lifted UV arcs evaluated through their own surface,
/// against a caller-supplied cylinder predicate and the torus equation.
fn uv_samples(
    cylinder_model: &Model,
    torus_model: &Model,
    cylinder_uv: &[CylinderPatchCurve],
    torus_uv: &[TorusPatchCurve],
    cylinder_check: impl Fn([f64; 3]) -> f64,
    torus_check: impl Fn([f64; 3]) -> f64,
) -> f64 {
    let mut worst = 0_f64;
    for lift in cylinder_uv {
        let surface = &cylinder_model.faces[lift.patch].surface;
        for arc in &lift.arcs {
            for k in 0..=8 {
                let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                assert!((0. ..=1.).contains(&uv[0]) && (0. ..=1.).contains(&uv[1]));
                let p = surface.evaluate(uv[0], uv[1]).unwrap().point;
                let p = [p[0], p[1], p[2]];
                worst = worst.max(cylinder_check(p)).max(torus_check(p));
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
                worst = worst.max(cylinder_check(p)).max(torus_check(p));
            }
        }
    }
    worst
}

#[test]
fn two_side_circles_match_the_sqrt_oracle() {
    // Torus R=3, r=1; cylinder R_c=2.2 spanning z in -4..4: a = -0.8,
    // side circles of radius 2.2 at z = +-sqrt(1 - 0.64) = +-0.6.
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let cylinder = translated(&brep_core::analytic::cylinder(2.2, 8.).unwrap(), [0., 0., -4.]);
    let report = intersect_cylinder_torus(&cylinder, &torus, Options::default()).unwrap();
    let report = only_circles(&report, 2);
    for (component, z) in report.components.iter().zip([-0.6, 0.6]) {
        let (curve, center, radius, normal, cylinder_uv, torus_uv, sampled) =
            circle_of(component);
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
                .max((radial_z(p) - 2.2).abs())
                .max(implicit(p, 3., 1.));
        }
        assert!(worst <= 1e-10, "{worst}");
        assert!(sampled <= 1e-10, "{sampled}");
        // Cylinder lift: the iso-v line at the exact height fraction on
        // all four side patches.
        assert_eq!(cylinder_uv.len(), 4, "{cylinder_uv:?}");
        let v0 = (z + 4.) / 8.;
        for lift in cylinder_uv {
            assert_eq!(lift.arcs.len(), 1);
            let arc = &lift.arcs[0];
            assert_eq!(arc.degree, 1);
            assert!((arc.control_points[0][1] - v0).abs() <= 1e-12, "{arc:?}");
            assert!((arc.control_points[1][1] - v0).abs() <= 1e-12, "{arc:?}");
            assert_eq!(arc.control_points[0][0], 0.);
            assert_eq!(arc.control_points[1][0], 1.);
        }
        // Torus lift: one iso-v line per revolution quadrant patch of the
        // profile row, at the exact rational-arc v parameter.
        assert_eq!(torus_uv.len(), 4, "{torus_uv:?}");
        let phi = z.atan2(2.2 - 3.).rem_euclid(TAU);
        let quadrant = (phi / std::f64::consts::FRAC_PI_2).floor() as usize;
        let vv = arc_parameter(phi - quadrant as f64 * std::f64::consts::FRAC_PI_2);
        for lift in torus_uv {
            assert_eq!(lift.arcs.len(), 1);
            let arc = &lift.arcs[0];
            assert_eq!(arc.degree, 1);
            assert!((arc.control_points[0][1] - vv).abs() <= 1e-12, "{arc:?}");
            assert!((arc.control_points[1][1] - vv).abs() <= 1e-12, "{arc:?}");
        }
        let uv_worst = uv_samples(
            &cylinder,
            &torus,
            cylinder_uv,
            torus_uv,
            |p| (radial_z(p) - 2.2).abs(),
            |p| implicit(p, 3., 1.),
        );
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }
}

#[test]
fn height_clipping_drops_the_lower_side_circle() {
    // Torus R=3, r=1 at the origin; cylinder R_c=2.2 spanning z in
    // 0.3..8.3: the z=-0.6 side circle is provably beyond the bottom
    // cap, the z=+0.6 circle survives, and the bottom cap plane at
    // h_c=0.3 cuts the torus at radii 3 +- sqrt(1 - 0.09) — the inner
    // circle (2.0460608...) sits inside the cap disk, the outer one is
    // beyond it.
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let cylinder = translated(&brep_core::analytic::cylinder(2.2, 8.).unwrap(), [0., 0., 0.3]);
    let report = intersect_cylinder_torus(&cylinder, &torus, Options::default()).unwrap();
    let report = only_circles(&report, 2);
    // Sorted by torus-axial height: the cap circle at z=0.3, then the
    // side circle at z=0.6. No component at z=-0.6.
    let s = (1_f64 - 0.09).sqrt();
    let (curve, center, radius, _, cylinder_uv, torus_uv, sampled) =
        circle_of(&report.components[0]);
    assert!((radius - (3. - s)).abs() <= 1e-12, "{radius}");
    assert!(
        sub(center, [0., 0., 0.3]).iter().all(|x| x.abs() <= 1e-12),
        "{center:?}"
    );
    let mut worst = 0_f64;
    for i in 0..16 {
        let p = curve.evaluate(i as f64 / 4.).unwrap().point;
        let p = [p[0], p[1], p[2]];
        worst = worst.max((p[2] - 0.3).abs()).max(implicit(p, 3., 1.));
    }
    assert!(worst <= 1e-10, "{worst}");
    assert!(sampled <= 1e-10, "{sampled}");
    // Cap lift: one face, four exact 90-degree arcs of the UV circle of
    // radius rho/(2R_c) about [1/2, 1/2].
    assert_eq!(cylinder_uv.len(), 1);
    let lift = &cylinder_uv[0];
    assert_eq!(lift.arcs.len(), 4);
    for arc in &lift.arcs {
        assert_eq!(arc.degree, 2);
        assert_eq!(arc.weights, vec![1., ARC_WEIGHT, 1.]);
        let uvr = (arc.control_points[0][0] - 0.5).hypot(arc.control_points[0][1] - 0.5);
        assert!((uvr - (3. - s) / 4.4).abs() <= 1e-12, "{uvr}");
    }
    assert_eq!(torus_uv.len(), 4);
    let uv_worst = uv_samples(
        &cylinder,
        &torus,
        cylinder_uv,
        torus_uv,
        |p| (p[2] - 0.3).abs(),
        |p| implicit(p, 3., 1.),
    );
    assert!(uv_worst <= 1e-9, "{uv_worst}");
    let (_, center, radius, _, cylinder_uv, _, sampled) = circle_of(&report.components[1]);
    assert!((radius - 2.2).abs() <= 1e-12, "{radius}");
    assert!(
        sub(center, [0., 0., 0.6]).iter().all(|x| x.abs() <= 1e-12),
        "{center:?}"
    );
    assert_eq!(cylinder_uv.len(), 4);
    assert!(sampled <= 1e-10, "{sampled}");
}

#[test]
fn torus_poking_through_both_caps_yields_cap_circle_pairs() {
    // Torus R=3, r=1; short wide cylinder R_c=4.5 spanning -0.25..0.25:
    // |a| = 1.5 > 1 — no side contact; each cap plane at h_c = +-0.25
    // cuts the torus at 3 +- sqrt(1 - 0.0625), both inside the disk:
    // four cap circles.
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let cylinder = translated(
        &brep_core::analytic::cylinder(4.5, 0.5).unwrap(),
        [0., 0., -0.25],
    );
    let report = intersect_cylinder_torus(&cylinder, &torus, Options::default()).unwrap();
    let report = only_circles(&report, 4);
    let s = (1_f64 - 0.0625).sqrt();
    let expected = [
        (-0.25, 3. - s),
        (-0.25, 3. + s),
        (0.25, 3. - s),
        (0.25, 3. + s),
    ];
    for (component, &(z, rho)) in report.components.iter().zip(&expected) {
        let (curve, center, radius, _, cylinder_uv, torus_uv, sampled) = circle_of(component);
        assert!((radius - rho).abs() <= 1e-12, "{radius} vs {rho}");
        assert!(
            sub(center, [0., 0., z]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        let mut worst = 0_f64;
        for i in 0..16 {
            let p = curve.evaluate(i as f64 / 4.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            worst = worst.max((p[2] - z).abs()).max(implicit(p, 3., 1.));
        }
        assert!(worst <= 1e-10, "{worst}");
        assert!(sampled <= 1e-10, "{sampled}");
        assert_eq!(cylinder_uv.len(), 1);
        assert_eq!(cylinder_uv[0].arcs.len(), 4);
        // Torus lift at the exact profile angle atan2(z, rho - R).
        assert_eq!(torus_uv.len(), 4);
        let phi = z.atan2(rho - 3.).rem_euclid(TAU);
        let quadrant = (phi / std::f64::consts::FRAC_PI_2).floor() as usize;
        let vv = arc_parameter(phi - quadrant as f64 * std::f64::consts::FRAC_PI_2);
        for lift in torus_uv {
            let arc = &lift.arcs[0];
            assert!((arc.control_points[0][1] - vv).abs() <= 1e-12, "{arc:?}");
        }
        let uv_worst = uv_samples(
            &cylinder,
            &torus,
            cylinder_uv,
            torus_uv,
            |p| (p[2] - z).abs(),
            |p| implicit(p, 3., 1.),
        );
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }
}

#[test]
fn meridian_tangency_stays_a_tangency_region() {
    // R_c == R - r = 2 (inner equator) and R_c == R + r = 4 (outer
    // equator): the line rho = R_c is tangent to the meridian circle at
    // z == 0 — the one configuration where the roots would coincide.
    // Never a guessed circle.
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    for r_c in [2., 2. + 2e-15, 2. - 2e-15, 4., 4. + 2e-15, 4. - 2e-15] {
        let cylinder = translated(&brep_core::analytic::cylinder(r_c, 8.).unwrap(), [0., 0., -4.]);
        let report = intersect_cylinder_torus(&cylinder, &torus, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
        assert!(!report.permits_topology_change());
    }
    // Just clear of the band on the outside: provable miss, resolved.
    let clear = translated(
        &brep_core::analytic::cylinder(2. - 1e-9, 8.).unwrap(),
        [0., 0., -4.],
    );
    let report = intersect_cylinder_torus(&clear, &torus, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    // Just across: two small transverse circles around the inner equator.
    let across = translated(
        &brep_core::analytic::cylinder(2. + 1e-9, 8.).unwrap(),
        [0., 0., -4.],
    );
    let report = intersect_cylinder_torus(&across, &torus, Options::default()).unwrap();
    only_circles(&report, 2);
}

#[test]
fn zero_circle_configurations_resolve_empty() {
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    // Thin cylinder in the hole: the line rho = 1 misses the meridian
    // circle, caps far above the tube.
    let thin = translated(&brep_core::analytic::cylinder(1., 8.).unwrap(), [0., 0., -4.]);
    // Short narrow cylinder around the center plane: no side contact,
    // and both cap circles (radii 3 +- sqrt(0.75)) lie outside the
    // R_c = 1.5 disks.
    let narrow = translated(&brep_core::analytic::cylinder(1.5, 1.).unwrap(), [0., 0., -0.5]);
    // Huge cylinder swallowing the whole torus.
    let huge = translated(
        &brep_core::analytic::cylinder(20., 40.).unwrap(),
        [0., 0., -20.],
    );
    for cylinder in [&thin, &narrow, &huge] {
        let report = intersect_cylinder_torus(cylinder, &torus, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
    }
}

#[test]
fn rim_tangency_stays_unresolved() {
    // R_c = 2.5, half height sqrt(0.75): the side circles at
    // z = +-sqrt(1 - 0.25) land exactly on the cap planes, and the cap
    // circle of radius R + |a| = 2.5 equals R_c — rim tangencies, never
    // guessed circles. The outer cap circles (radius 3.5) are absent.
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let half = 0.75_f64.sqrt();
    let cylinder = translated(
        &brep_core::analytic::cylinder(2.5, 2. * half).unwrap(),
        [0., 0., -half],
    );
    let report = intersect_cylinder_torus(&cylinder, &torus, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::TangencyOrMultipleRoot
    );
}

#[test]
fn cap_plane_tube_tangency_stays_unresolved() {
    // R_c = 5 (no side contact: |a| = 2 > 1), half height 1: both cap
    // planes sit at |h_c| == r — tangent to the tube, never guessed.
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let cylinder = translated(&brep_core::analytic::cylinder(5., 2.).unwrap(), [0., 0., -1.]);
    let report = intersect_cylinder_torus(&cylinder, &torus, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::TangencyOrMultipleRoot
    );
    // Just clear of the band on the outside: both caps beyond the tube,
    // resolved empty.
    let clear = translated(
        &brep_core::analytic::cylinder(5., 2.).unwrap(),
        [0., 0., 2. + 1e-9],
    );
    let report = intersect_cylinder_torus(&clear, &torus, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    // Just across: the top cap clears the tube (h_c = 1 + 1e-9) and the
    // bottom cap at h_c = -(1 - 1e-9) cuts two small cap circles.
    let across = translated(&brep_core::analytic::cylinder(5., 2.).unwrap(), [0., 0., 1e-9]);
    let report = intersect_cylinder_torus(&across, &torus, Options::default()).unwrap();
    only_circles(&report, 2);
}

#[test]
fn near_coaxial_bands_stay_unresolved_not_forced() {
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let base = translated(&brep_core::analytic::cylinder(2.2, 8.).unwrap(), [0., 0., -4.]);
    // Center-line offset inside the recognition band: near_coincidence.
    let near = translated(&base, [1e-10, 0., 0.]);
    let report = intersect_cylinder_torus(&near, &torus, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::NearCoincidence
    );
    // Recognition-scale tilt of the cylinder axis: near_coincidence.
    let (sin, cos) = 1e-10_f64.sin_cos();
    let tilted = brep_core::transform::affine(
        &base,
        [
            [cos, 0., sin, 0.],
            [0., 1., 0., 0.],
            [-sin, 0., cos, 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let report = intersect_cylinder_torus(&tilted, &torus, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::NearCoincidence
    );
}

#[test]
fn clearly_off_axis_or_tilted_pairs_are_unsupported_regions() {
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let base = translated(&brep_core::analytic::cylinder(2.2, 8.).unwrap(), [0., 0., -4.]);
    let off = translated(&base, [0.5, 0., 0.]);
    let (sin, cos) = 0.3_f64.sin_cos();
    let tilted = brep_core::transform::affine(
        &base,
        [
            [cos, 0., sin, 0.],
            [0., 1., 0., 0.],
            [-sin, 0., cos, 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    for cylinder in [&off, &tilted] {
        let report = intersect_cylinder_torus(cylinder, &torus, Options::default()).unwrap();
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
    let cylinder = translated(&brep_core::analytic::cylinder(2.2, 8.).unwrap(), [0., 0., -4.]);
    // Spheres, cuboids, tori-as-first-operand and swapped operand order
    // are not the canonical pair.
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
            brep_core::analytic::torus(2., 1.).unwrap(),
            brep_core::analytic::torus(3., 1.).unwrap(),
        ),
        (
            brep_core::analytic::torus(3., 1.).unwrap(),
            brep_core::analytic::cylinder(2.2, 8.).unwrap(),
        ),
        (
            brep_core::analytic::cylinder(2.2, 8.).unwrap(),
            brep_core::analytic::sphere(2.).unwrap(),
        ),
    ] {
        let report = intersect_cylinder_torus(&a, &b, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(report.unresolved.len(), 1);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::UnsupportedSurface
        );
    }
    // A canonical pair still resolves.
    let report = intersect_cylinder_torus(&cylinder, &torus, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    assert_eq!(report.components.len(), 2);
    // A structurally perturbed torus (one moved control point) fails the
    // exact certification and is an unsupported region, never a fallback.
    let mut perturbed = torus.clone();
    perturbed.faces[0].surface.control_points[1][1][0] += 1e-6;
    perturbed.rebuild_topology_ids();
    let report = intersect_cylinder_torus(&cylinder, &perturbed, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::UnsupportedSurface
    );
}

#[test]
fn rotated_translated_placement_keeps_the_exact_circles() {
    // Rigid placement of the whole coaxial configuration about X by 0.5.
    let angle = 0.5;
    let offset = [0.3, -0.2, 1.1];
    let torus = rotated_translated(&brep_core::analytic::torus(3., 1.).unwrap(), angle, offset);
    let cylinder = rotated_translated(
        &translated(&brep_core::analytic::cylinder(2.2, 8.).unwrap(), [0., 0., -4.]),
        angle,
        offset,
    );
    let report = intersect_cylinder_torus(&cylinder, &torus, Options::default()).unwrap();
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
    for (component, z) in report.components.iter().zip([-0.6, 0.6]) {
        let (curve, center, radius, normal, cylinder_uv, torus_uv, sampled) =
            circle_of(component);
        let expected = placed([0., 0., z]);
        assert!((radius - 2.2).abs() <= 1e-12, "{radius}");
        assert!(
            sub(center, expected).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        assert!(
            sub(normal, axis).iter().all(|x| x.abs() <= 1e-12),
            "{normal:?}"
        );
        let placed_check = |p: [f64; 3]| {
            let rel = sub(p, placed([0., 0., 0.]));
            let a = dot(rel, axis);
            let perp = sub(rel, axis.map(|x| x * a));
            let rho_p = perp[0].hypot(perp[1]).hypot(perp[2]);
            let base = rho_p * rho_p + a * a + 9. - 1.;
            (
                (rho_p - 2.2).abs(),
                (base * base - 36. * rho_p * rho_p).abs(),
            )
        };
        let mut worst = 0_f64;
        for i in 0..16 {
            let p = curve.evaluate(i as f64 / 4.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            let (c_r, t_r) = placed_check(p);
            worst = worst.max(c_r).max(t_r);
        }
        assert!(worst <= 1e-9, "{worst}");
        assert!(sampled <= 1e-10, "{sampled}");
        assert_eq!(cylinder_uv.len(), 4);
        assert_eq!(torus_uv.len(), 4);
        let uv_worst = uv_samples(
            &cylinder,
            &torus,
            cylinder_uv,
            torus_uv,
            |p| placed_check(p).0,
            |p| placed_check(p).1,
        );
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }
}
