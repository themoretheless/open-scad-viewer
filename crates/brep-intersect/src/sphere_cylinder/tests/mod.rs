use super::super::test_utils::{rotated_translated, translated};
use brep_core::intersections::sphere_sphere::ARC_WEIGHT;
use super::*;

fn only_circles(
    report: &Report<SphereCylinderComponent>,
    count: usize,
) -> &Report<SphereCylinderComponent> {
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert!(report.unresolved.is_empty(), "{report:?}");
    assert!(!report.permits_topology_change());
    assert_eq!(report.components.len(), count, "{report:?}");
    report
}
type SphereCylinderCircle<'a> = (
    &'a Curve,
    [f64; 3],
    f64,
    [f64; 3],
    &'a [SpherePatchCircle],
    &'a [CylinderPatchCurve],
    f64,
);

fn circle_of(component: &SphereCylinderComponent) -> SphereCylinderCircle<'_> {
    let SphereCylinderComponent::Circle {
        curve,
        center,
        radius,
        normal,
        sphere_uv,
        cylinder_uv,
        max_sample_residual,
    } = component;
    (
        curve,
        *center,
        *radius,
        *normal,
        sphere_uv,
        cylinder_uv,
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
/// Radial distance from the z axis (canonical frames in these tests).
fn radial_z(point: [f64; 3]) -> f64 {
    point[0].hypot(point[1])
}
/// Worst residual of lifted UV arcs evaluated through their own surface,
/// against the sphere equation and a per-circle predicate.
fn uv_samples(
    sphere_model: &Model,
    cylinder_model: &Model,
    sphere_center: [f64; 3],
    sphere_radius: f64,
    sphere_uv: &[SpherePatchCircle],
    cylinder_uv: &[CylinderPatchCurve],
    cylinder_check: impl Fn([f64; 3]) -> f64,
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
                    .max(cylinder_check(p));
            }
        }
    }
    for lift in cylinder_uv {
        let surface = &cylinder_model.faces[lift.patch].surface;
        for arc in &lift.arcs {
            for k in 0..=8 {
                let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                assert!((0. ..=1.).contains(&uv[0]) && (0. ..=1.).contains(&uv[1]));
                let p = surface.evaluate(uv[0], uv[1]).unwrap().point;
                let p = [p[0], p[1], p[2]];
                worst = worst
                    .max(sphere_residual(p, sphere_center, sphere_radius))
                    .max(cylinder_check(p));
            }
        }
    }
    worst
}

#[test]
fn two_side_circles_match_the_sqrt_oracle() {
    // Cylinder R=2 spans z in 0..8; sphere r=3 centered on the axis at
    // the cylinder midpoint: side circles at z = 4 +- sqrt(9 - 4).
    let sphere = translated(&brep_core::analytic::sphere(3.).unwrap(), [0., 0., 4.]);
    let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
    let report = intersect_sphere_cylinder(&sphere, &cylinder, Options::default()).unwrap();
    let report = only_circles(&report, 2);
    let oracle = (9_f64 - 4.).sqrt();
    for (component, z) in report.components.iter().zip([4. - oracle, 4. + oracle]) {
        let (curve, center, radius, normal, sphere_uv, cylinder_uv, sampled) =
            circle_of(component);
        assert!((radius - 2.).abs() <= 1e-12, "{radius}");
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
                .max(sphere_residual(p, [0., 0., 4.], 3.))
                .max((radial_z(p) - 2.).abs());
        }
        assert!(worst <= 1e-12, "{worst}");
        assert!(sampled <= 1e-12, "{sampled}");
        // Side circles lift to iso-v lines on all four side patches.
        assert_eq!(cylinder_uv.len(), 4);
        let v0 = (z) / 8.;
        for lift in cylinder_uv {
            assert_eq!(lift.arcs.len(), 1);
            let arc = &lift.arcs[0];
            assert_eq!(arc.degree, 1);
            assert!((arc.control_points[0][1] - v0).abs() <= 1e-12);
            assert!((arc.control_points[1][1] - v0).abs() <= 1e-12);
        }
        assert!(!sphere_uv.is_empty());
        let uv_worst = uv_samples(
            &sphere,
            &cylinder,
            [0., 0., 4.],
            3.,
            sphere_uv,
            cylinder_uv,
            |p| (radial_z(p) - 2.).abs(),
        );
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }
}

#[test]
fn one_circle_survives_clipping_by_the_finite_height() {
    // Sphere r=3 centered at the bottom cap plane: upper side circle at
    // z = sqrt(5) inside, lower one below the cylinder; the bottom cap
    // plane circle has radius 3 > R, so it lies outside the cap disk.
    let sphere = brep_core::analytic::sphere(3.).unwrap();
    let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
    let report = intersect_sphere_cylinder(&sphere, &cylinder, Options::default()).unwrap();
    let report = only_circles(&report, 1);
    let (curve, center, radius, _, _, cylinder_uv, sampled) = circle_of(&report.components[0]);
    assert!((radius - 2.).abs() <= 1e-12);
    assert!(
        sub(center, [0., 0., 5_f64.sqrt()])
            .iter()
            .all(|x| x.abs() <= 1e-12)
    );
    let mut worst = 0_f64;
    for i in 0..16 {
        let p = curve.evaluate(i as f64 / 4.).unwrap().point;
        let p = [p[0], p[1], p[2]];
        worst = worst
            .max(sphere_residual(p, [0., 0., 0.], 3.))
            .max((radial_z(p) - 2.).abs());
    }
    assert!(worst <= 1e-12, "{worst}");
    assert!(sampled <= 1e-12);
    assert_eq!(cylinder_uv.len(), 4);
}

#[test]
fn small_sphere_poking_through_a_cap_yields_a_cap_circle() {
    // r=1.5 < R=2, center 0.5 below the top cap plane z=8: circle of
    // radius sqrt(1.5^2 - 0.5^2) = sqrt(2) in the cap plane, inside the
    // disk; no side contact is possible.
    let sphere = translated(&brep_core::analytic::sphere(1.5).unwrap(), [0., 0., 7.5]);
    let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
    let report = intersect_sphere_cylinder(&sphere, &cylinder, Options::default()).unwrap();
    let report = only_circles(&report, 1);
    let (curve, center, radius, _, sphere_uv, cylinder_uv, sampled) =
        circle_of(&report.components[0]);
    let oracle = 2_f64.sqrt();
    assert!((radius - oracle).abs() <= 1e-12, "{radius}");
    assert!(
        sub(center, [0., 0., 8.]).iter().all(|x| x.abs() <= 1e-12),
        "{center:?}"
    );
    let mut worst = 0_f64;
    for i in 0..16 {
        let p = curve.evaluate(i as f64 / 4.).unwrap().point;
        let p = [p[0], p[1], p[2]];
        worst = worst
            .max(sphere_residual(p, [0., 0., 7.5], 1.5))
            .max((p[2] - 8.).abs());
    }
    assert!(worst <= 1e-12, "{worst}");
    assert!(sampled <= 1e-12);
    // Cap lift: one face, four exact 90-degree arcs of the UV circle of
    // radius sqrt(2)/(2R) = sqrt(2)/4 about [1/2, 1/2].
    assert_eq!(cylinder_uv.len(), 1);
    let lift = &cylinder_uv[0];
    assert_eq!(lift.arcs.len(), 4);
    for arc in &lift.arcs {
        assert_eq!(arc.degree, 2);
        assert_eq!(arc.weights, vec![1., ARC_WEIGHT, 1.]);
        let endpoint = &arc.control_points[0];
        let uvr = (endpoint[0] - 0.5).hypot(endpoint[1] - 0.5);
        assert!((uvr - oracle / 4.).abs() <= 1e-12, "{uvr}");
    }
    let uv_worst = uv_samples(
        &sphere,
        &cylinder,
        [0., 0., 7.5],
        1.5,
        sphere_uv,
        cylinder_uv,
        |p| (p[2] - 8.).abs(),
    );
    assert!(uv_worst <= 1e-9, "{uv_worst}");
}

#[test]
fn zero_circle_configurations_resolve_empty() {
    let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
    // Small sphere strictly inside.
    let inside = translated(&brep_core::analytic::sphere(1.).unwrap(), [0., 0., 4.]);
    // Sphere beyond the top cap, no reach back.
    let beyond = translated(&brep_core::analytic::sphere(1.5).unwrap(), [0., 0., 11.]);
    // Large sphere swallowing the whole cylinder (side crossings beyond
    // the height, cap circles outside the disks).
    let swallow = translated(&brep_core::analytic::sphere(20.).unwrap(), [0., 0., 4.]);
    for sphere in [&inside, &beyond, &swallow] {
        let report = intersect_sphere_cylinder(sphere, &cylinder, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
    }
}

#[test]
fn equal_radii_report_the_coincident_band() {
    let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
    // Exact r == R: tangent equator circle — never a guessed component.
    let exact = translated(&brep_core::analytic::sphere(2.).unwrap(), [0., 0., 4.]);
    // Within the outward band: inseparable from coincidence.
    let near = brep_core::analytic::sphere(2. + 2e-15).unwrap();
    let near = translated(&near, [0., 0., 4.]);
    // r == R with the sphere reaching back over the top cap: the radius
    // ambiguity still forbids certifying a cap circle.
    let reaching = translated(&brep_core::analytic::sphere(2.).unwrap(), [0., 0., 9.]);
    for sphere in [&exact, &near, &reaching] {
        let report = intersect_sphere_cylinder(sphere, &cylinder, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(report.unresolved.len(), 1);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::CoincidentTrim
        );
        assert!(!report.permits_topology_change());
    }
}

#[test]
fn cap_plane_touch_stays_a_tangency_region() {
    let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
    // r=1.5 sphere tangent to the top cap plane z=8 from inside.
    let sphere = translated(&brep_core::analytic::sphere(1.5).unwrap(), [0., 0., 6.5]);
    let report = intersect_sphere_cylinder(&sphere, &cylinder, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::TangencyOrMultipleRoot
    );
    // Just clear of the band: strictly inside, empty and resolved.
    let clear = translated(&brep_core::analytic::sphere(1.5).unwrap(), [0., 0., 6.5 - 1e-9]);
    let report = intersect_sphere_cylinder(&clear, &cylinder, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    // Just across: a small transverse cap circle.
    let across = translated(&brep_core::analytic::sphere(1.5).unwrap(), [0., 0., 6.5 + 1e-9]);
    let report = intersect_sphere_cylinder(&across, &cylinder, Options::default()).unwrap();
    only_circles(&report, 1);
}

#[test]
fn rim_tangency_stays_unresolved() {
    // sqrt(r^2 - R^2) == h/2 exactly: both side crossings land on the cap
    // planes — the rim tangency is never a guessed circle.
    let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
    let r = (4_f64 + 16.).sqrt();
    let sphere = translated(&brep_core::analytic::sphere(r).unwrap(), [0., 0., 4.]);
    let report = intersect_sphere_cylinder(&sphere, &cylinder, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::TangencyOrMultipleRoot
    );
}

#[test]
fn near_axial_offset_within_the_band_stays_unresolved() {
    let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
    let sphere = translated(&brep_core::analytic::sphere(3.).unwrap(), [1e-10, 0., 4.]);
    let report = intersect_sphere_cylinder(&sphere, &cylinder, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::NearCoincidence
    );
}

#[test]
fn clearly_off_axis_pairs_are_unsupported_regions() {
    let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
    let sphere = translated(&brep_core::analytic::sphere(3.).unwrap(), [0.5, 0., 4.]);
    let report = intersect_sphere_cylinder(&sphere, &cylinder, Options::default()).unwrap();
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

#[test]
fn noncanonical_operands_are_explicit_unsupported_regions() {
    let sphere = brep_core::analytic::sphere(2.).unwrap();
    // Frustum and tube are not the canonical cylinder; cuboids and tori
    // are neither canonical operand.
    for (a, b) in [
        (
            brep_core::analytic::sphere(2.).unwrap(),
            brep_core::analytic::frustum(1., 2., 3.).unwrap(),
        ),
        (
            brep_core::analytic::sphere(2.).unwrap(),
            brep_core::analytic::tube(2., 1., 3.).unwrap(),
        ),
        (
            brep_core::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap(),
            brep_core::analytic::cylinder(1., 2.).unwrap(),
        ),
        (
            brep_core::analytic::torus(3., 1.).unwrap(),
            brep_core::analytic::cylinder(1., 2.).unwrap(),
        ),
    ] {
        let report = intersect_sphere_cylinder(&a, &b, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(report.unresolved.len(), 1);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::UnsupportedSurface
        );
    }
    // A canonical pair still resolves: sphere r=2 at the bottom ring of
    // cylinder(1, 4) crosses the side once at z = 4 - sqrt(3).
    let tall = brep_core::analytic::cylinder(1., 4.).unwrap();
    let report = intersect_sphere_cylinder(&sphere, &tall, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    assert_eq!(report.components.len(), 1);
    // A structurally perturbed cylinder fails validation as a hard error.
    let mut perturbed = brep_core::analytic::cylinder(1., 2.).unwrap();
    perturbed.faces[0].surface.weights[1][0] = 0.5;
    assert!(intersect_sphere_cylinder(&sphere, &perturbed, Options::default()).is_err());
}

#[test]
fn rotated_translated_placement_keeps_the_exact_circles() {
    // Rigid placement of the whole axial configuration about X by 0.5.
    let angle = 0.5;
    let offset = [0.3, -0.2, 1.1];
    let sphere = rotated_translated(
        &translated(&brep_core::analytic::sphere(3.).unwrap(), [0., 0., 4.]),
        angle,
        offset,
    );
    let cylinder =
        rotated_translated(&brep_core::analytic::cylinder(2., 8.).unwrap(), angle, offset);
    let report = intersect_sphere_cylinder(&sphere, &cylinder, Options::default()).unwrap();
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
    let sphere_center = placed([0., 0., 4.]);
    let axis = {
        let a = placed([0., 0., 1.]);
        let b = placed([0., 0., 0.]);
        sub(a, b)
    };
    let oracle = (9_f64 - 4.).sqrt();
    for (component, z) in report.components.iter().zip([4. - oracle, 4. + oracle]) {
        let (curve, center, radius, normal, sphere_uv, cylinder_uv, sampled) =
            circle_of(component);
        let expected = placed([0., 0., z]);
        assert!((radius - 2.).abs() <= 1e-12, "{radius}");
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
            worst = worst.max(sphere_residual(p, sphere_center, 3.)).max({
                let rel = sub(p, expected);
                let a = dot(rel, axis);
                let perp = sub(rel, axis.map(|x| x * a));
                (perp[0].hypot(perp[1]).hypot(perp[2]) - 2.).abs()
            });
        }
        assert!(worst <= 1e-12, "{worst}");
        assert!(sampled <= 1e-12);
        assert_eq!(cylinder_uv.len(), 4);
        assert!(!sphere_uv.is_empty());
        let axis = normal;
        let uv_worst = uv_samples(
            &sphere,
            &cylinder,
            sphere_center,
            3.,
            sphere_uv,
            cylinder_uv,
            |p| {
                let rel = sub(p, center);
                let a = dot(rel, axis);
                let perp = sub(rel, axis.map(|x| x * a));
                (perp[0].hypot(perp[1]).hypot(perp[2]) - 2.).abs()
            },
        );
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }
}

#[test]
fn swapped_cap_slots_and_bottom_cap_circle() {
    // Sphere poking through the bottom cap from below: circle on the
    // bottom cap face, axial slot 0.
    let sphere = translated(&brep_core::analytic::sphere(1.5).unwrap(), [0., 0., 0.5]);
    let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
    let report = intersect_sphere_cylinder(&sphere, &cylinder, Options::default()).unwrap();
    let report = only_circles(&report, 1);
    let (_, center, radius, _, _, cylinder_uv, _) = circle_of(&report.components[0]);
    assert!((radius - 2_f64.sqrt()).abs() <= 1e-12);
    assert!(sub(center, [0., 0., 0.]).iter().all(|x| x.abs() <= 1e-12));
    assert_eq!(cylinder_uv.len(), 1);
    // The bottom cap is a different face than the top cap; its lift still
    // evaluates onto the sphere and the cap plane.
    let lift = &cylinder_uv[0];
    let surface = &cylinder.faces[lift.patch].surface;
    for arc in &lift.arcs {
        for k in 0..=8 {
            let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
            let p = surface.evaluate(uv[0], uv[1]).unwrap().point;
            let p = [p[0], p[1], p[2]];
            assert!(sphere_residual(p, [0., 0., 0.5], 1.5) <= 1e-9);
            assert!((p[2]).abs() <= 1e-9);
        }
    }
}
