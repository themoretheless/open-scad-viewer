use super::super::test_utils::{rotated_translated, translated};
use super::*;

fn only_lines(
    report: &Report<CylinderCylinderComponent>,
    count: usize,
) -> &Report<CylinderCylinderComponent> {
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert!(report.unresolved.is_empty(), "{report:?}");
    assert!(!report.permits_topology_change());
    assert_eq!(report.components.len(), count, "{report:?}");
    report
}
#[allow(clippy::type_complexity)]
fn line_of(
    component: &CylinderCylinderComponent,
) -> (
    &Curve,
    [f64; 3],
    [f64; 3],
    [f64; 3],
    Contact,
    &[CylinderPatchCurve],
    &[CylinderPatchCurve],
    f64,
) {
    let CylinderCylinderComponent::Line {
        curve,
        start,
        end,
        direction,
        contact,
        first_uv,
        second_uv,
        max_sample_residual,
    } = component;
    (
        curve,
        *start,
        *end,
        *direction,
        *contact,
        first_uv,
        second_uv,
        *max_sample_residual,
    )
}
/// Radial residual against a cylinder side given center/axis/radius.
fn side_residual(point: [f64; 3], center: [f64; 3], axis: [f64; 3], radius: f64) -> f64 {
    let rel = sub(point, center);
    let s = dot(rel, axis);
    let perp = sub(rel, axis.map(|x| x * s));
    (perp[0].hypot(perp[1]).hypot(perp[2]) - radius).abs()
}
/// Worst residual of lifted UV segments evaluated through their own side
/// surfaces against both cylinder side equations.
fn uv_samples(
    first_model: &Model,
    second_model: &Model,
    first_uv: &[CylinderPatchCurve],
    second_uv: &[CylinderPatchCurve],
    check: impl Fn([f64; 3]) -> f64,
) -> f64 {
    let mut worst = 0_f64;
    for (model, lifts) in [(first_model, first_uv), (second_model, second_uv)] {
        for lift in lifts {
            let surface = &model.faces[lift.patch].surface;
            for arc in &lift.arcs {
                assert_eq!(arc.degree, 1);
                for k in 0..=8 {
                    let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                    assert!((0. ..=1.).contains(&uv[0]) && (0. ..=1.).contains(&uv[1]));
                    let p = surface.evaluate(uv[0], uv[1]).unwrap().point;
                    worst = worst.max(check([p[0], p[1], p[2]]));
                }
            }
        }
    }
    worst
}

#[test]
fn two_parallel_cylinders_yield_two_exact_lines() {
    // r1=2 at the origin (z in 0..8), r2=3 offset by d=3.5 along x:
    // 1 = |r1-r2| < d < r1+r2 = 5, so the planar circle/circle section
    // gives two foot points at x = (d^2 + r1^2 - r2^2)/(2d), y = +-sqrt.
    let first = brep_core::analytic::cylinder(2., 8.).unwrap();
    let second = translated(&brep_core::analytic::cylinder(3., 8.).unwrap(), [3.5, 0., 0.]);
    let report = intersect_cylinder_cylinder(&first, &second, Options::default()).unwrap();
    let report = only_lines(&report, 2);
    let d = 3.5_f64;
    let x = (d * d + 4. - 9.) / (2. * d);
    let y = (4. - x * x).sqrt();
    for (component, sign) in report.components.iter().zip([-1., 1.]) {
        let (curve, start, end, direction, contact, first_uv, second_uv, sampled) =
            line_of(component);
        assert!(
            sub(start, [x, sign * y, 0.])
                .iter()
                .all(|v| v.abs() <= 1e-12),
            "{start:?}"
        );
        assert!(
            sub(end, [x, sign * y, 8.]).iter().all(|v| v.abs() <= 1e-12),
            "{end:?}"
        );
        assert!(direction[2].abs() >= 1. - 1e-12, "{direction:?}");
        assert_eq!(contact, Contact::Boundary);
        // Exact degree-1 line, unit knots and weights.
        assert_eq!(curve.degree, 1);
        assert_eq!(curve.knots, vec![0., 0., 1., 1.]);
        assert_eq!(curve.weights, vec![1., 1.]);
        assert_eq!(curve.control_points.len(), 2);
        // Nine samples satisfy both implicit cylinder equations.
        let mut worst = 0_f64;
        for k in 0..=8 {
            let p = curve.evaluate(k as f64 / 8.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            worst = worst
                .max((p[0].hypot(p[1]) - 2.).abs())
                .max(((p[0] - 3.5).hypot(p[1]) - 3.).abs());
        }
        assert!(worst <= 1e-12, "{worst}");
        assert!(sampled <= 1e-12, "{sampled}");
        // The rulings sit strictly inside quadrants: one iso-u segment on
        // one side patch per cylinder, v sweeping the full height.
        assert_eq!(first_uv.len(), 1);
        assert_eq!(second_uv.len(), 1);
        for lift in first_uv.iter().chain(second_uv) {
            assert_eq!(lift.arcs.len(), 1);
            let arc = &lift.arcs[0];
            assert!((arc.control_points[0][1]).abs() <= 1e-12);
            assert!((arc.control_points[1][1] - 1.).abs() <= 1e-12);
            assert_eq!(arc.control_points[0][0], arc.control_points[1][0]);
        }
        let uv_worst = uv_samples(&first, &second, first_uv, second_uv, |p| {
            (p[0].hypot(p[1]) - 2.)
                .abs()
                .max(((p[0] - 3.5).hypot(p[1]) - 3.).abs())
        });
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }
}

#[test]
fn seam_ruling_lifts_both_adjacent_side_patches() {
    // Place the second cylinder so that one ruling lands exactly on the
    // first cylinder's quadrant seam: offset along the direction at angle
    // -theta from the recognized frame x, where theta is the ruling angle.
    let first = brep_core::analytic::cylinder(2., 8.).unwrap();
    let rec = recognize_cylinder(&first).unwrap().unwrap();
    let d = 3.5_f64;
    let x = (d * d + 4. - 9.) / (2. * d);
    let y = (4. - x * x).sqrt();
    let (s, c) = (-y.atan2(x)).sin_cos();
    let u_world = [
        c * rec.frame[0][0] + s * rec.frame[1][0],
        c * rec.frame[0][1] + s * rec.frame[1][1],
        0.,
    ];
    let second = translated(
        &brep_core::analytic::cylinder(3., 8.).unwrap(),
        [d * u_world[0], d * u_world[1], 0.],
    );
    let report = intersect_cylinder_cylinder(&first, &second, Options::default()).unwrap();
    let report = only_lines(&report, 2);
    // The sign = +1 ruling sits on the first cylinder's seam at angle 0.
    let seam = line_of(&report.components[1]);
    assert_eq!(seam.5.len(), 2, "{:?}", seam.5);
    let us: Vec<f64> = seam
        .5
        .iter()
        .map(|lift| lift.arcs[0].control_points[0][0])
        .collect();
    assert!(us.contains(&0.) && us.contains(&1.), "{us:?}");
    assert_ne!(seam.5[0].patch, seam.5[1].patch);
    // The other ruling is strictly inside a quadrant: one patch.
    let plain = line_of(&report.components[0]);
    assert_eq!(plain.5.len(), 1);
    // Both seam lifts evaluate onto both cylinder side equations.
    let foot = [
        rec.center[0] + 2. * rec.frame[0][0],
        rec.center[1] + 2. * rec.frame[0][1],
        rec.center[2] + 2. * rec.frame[0][2],
    ];
    let second_center = [d * u_world[0], d * u_world[1], 4.];
    assert!(
        side_residual(foot, rec.center, rec.axis, 2.) <= 1e-12,
        "{foot:?}"
    );
    assert!(
        side_residual(foot, second_center, [0., 0., 1.], 3.) <= 1e-12,
        "{foot:?}"
    );
    for component in &report.components {
        let (_, _, _, _, _, first_uv, second_uv, _) = line_of(component);
        let uv_worst = uv_samples(&first, &second, first_uv, second_uv, |p| {
            side_residual(p, rec.center, rec.axis, 2.).max(side_residual(
                p,
                second_center,
                [0., 0., 1.],
                3.,
            ))
        });
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }
}

#[test]
fn finite_heights_clip_or_eliminate_the_lines() {
    let first = brep_core::analytic::cylinder(2., 8.).unwrap();
    // Partial overlap: the second cylinder spans z in 5..9, so the clip
    // interval is [1, 4] about the first center plane — lines z in 5..8.
    let partial = translated(&brep_core::analytic::cylinder(3., 4.).unwrap(), [3.5, 0., 5.]);
    let report = intersect_cylinder_cylinder(&first, &partial, Options::default()).unwrap();
    let report = only_lines(&report, 2);
    let d = 3.5_f64;
    let x = (d * d + 4. - 9.) / (2. * d);
    let y = (4. - x * x).sqrt();
    for (component, sign) in report.components.iter().zip([-1., 1.]) {
        let (_, start, end, _, _, _, second_uv, _) = line_of(component);
        assert!(
            sub(start, [x, sign * y, 5.])
                .iter()
                .all(|v| v.abs() <= 1e-12),
            "{start:?}"
        );
        assert!(
            sub(end, [x, sign * y, 8.]).iter().all(|v| v.abs() <= 1e-12),
            "{end:?}"
        );
        // The second lift spans v in 0..3/4 of its own height.
        let arc = &second_uv[0].arcs[0];
        assert!((arc.control_points[0][1]).abs() <= 1e-12);
        assert!((arc.control_points[1][1] - 0.75).abs() <= 1e-12);
    }
    // Disjoint heights: the clip interval is empty beyond the band.
    let disjoint = translated(&brep_core::analytic::cylinder(3., 4.).unwrap(), [3.5, 0., 9.]);
    let report = intersect_cylinder_cylinder(&first, &disjoint, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    // Heights touching at one cap plane: a band-thin clip degenerates.
    let touching = translated(&brep_core::analytic::cylinder(3., 4.).unwrap(), [3.5, 0., 8.]);
    let report = intersect_cylinder_cylinder(&first, &touching, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::TangencyOrMultipleRoot
    );
}

#[test]
fn external_and_internal_tangencies_stay_unresolved() {
    let first = brep_core::analytic::cylinder(2., 8.).unwrap();
    // External tangency: d == r1 + r2.
    let external = translated(&brep_core::analytic::cylinder(3., 8.).unwrap(), [5., 0., 0.]);
    // Internal tangency: d == |r1 - r2|.
    let internal = translated(&brep_core::analytic::cylinder(5., 8.).unwrap(), [3., 0., 0.]);
    for second in [&external, &internal] {
        let report = intersect_cylinder_cylinder(&first, second, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
        assert!(!report.permits_topology_change());
    }
    // Just clear of the external band: separate, empty and resolved.
    let clear = translated(
        &brep_core::analytic::cylinder(3., 8.).unwrap(),
        [5. + 1e-9, 0., 0.],
    );
    let report = intersect_cylinder_cylinder(&first, &clear, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    // Just across: the transverse two-line case.
    let across = translated(
        &brep_core::analytic::cylinder(3., 8.).unwrap(),
        [5. - 1e-9, 0., 0.],
    );
    let report = intersect_cylinder_cylinder(&first, &across, Options::default()).unwrap();
    only_lines(&report, 2);
    // Radially nested without contact: d < |r1 - r2| provably.
    let nested = translated(&brep_core::analytic::cylinder(5., 8.).unwrap(), [2., 0., 0.]);
    let report = intersect_cylinder_cylinder(&first, &nested, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
}

#[test]
fn coaxial_equal_radii_report_the_coincident_band() {
    let first = brep_core::analytic::cylinder(2., 8.).unwrap();
    // Exact equal radii, overlapping heights: coincident side surface.
    let same = brep_core::analytic::cylinder(2., 8.).unwrap();
    // Radii equal within the outward band: inseparable from coincidence.
    let near = brep_core::analytic::cylinder(2. + 2e-15, 8.).unwrap();
    for second in [&same, &near] {
        let report = intersect_cylinder_cylinder(&first, second, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::CoincidentTrim
        );
        assert!(!report.permits_topology_change());
    }
    // Stacked equal-radius cylinders sharing the cap plane z = 8: the
    // coincident rim circle is a tangency contact, never a guessed circle.
    let stacked = translated(&brep_core::analytic::cylinder(2., 4.).unwrap(), [0., 0., 8.]);
    let report = intersect_cylinder_cylinder(&first, &stacked, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::TangencyOrMultipleRoot
    );
    // Equal radii, heights disjoint beyond the band: no contact.
    let apart = translated(&brep_core::analytic::cylinder(2., 4.).unwrap(), [0., 0., 9.]);
    let report = intersect_cylinder_cylinder(&first, &apart, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
}

#[test]
fn coaxial_unequal_radii_resolve_empty_unless_cap_planes_coincide() {
    let outer = brep_core::analytic::cylinder(2., 8.).unwrap();
    // Nested coaxial cylinders with no shared cap plane (the inner spans
    // z in 2..6): the sides never meet and no rim lies on the other side.
    let inner = translated(&brep_core::analytic::cylinder(1., 4.).unwrap(), [0., 0., 2.]);
    let report = intersect_cylinder_cylinder(&outer, &inner, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    // Same-height nested cylinders share both cap disks over the smaller
    // radius: an honest coincident-trim region, not resolved empty.
    let shared_caps = brep_core::analytic::cylinder(1., 8.).unwrap();
    let report = intersect_cylinder_cylinder(&outer, &shared_caps, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::CoincidentTrim
    );
    // Stacked cylinders with different radii share the smaller cap disk:
    // an honest coincident-trim region, not a resolved-empty report.
    let stacked = translated(&brep_core::analytic::cylinder(1., 4.).unwrap(), [0., 0., 8.]);
    let report = intersect_cylinder_cylinder(&outer, &stacked, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::CoincidentTrim
    );
}

#[test]
fn near_parallel_within_the_band_stays_unresolved_not_forced() {
    let first = brep_core::analytic::cylinder(2., 8.).unwrap();
    let tilted = rotated_translated(
        &brep_core::analytic::cylinder(3., 8.).unwrap(),
        1e-10,
        [3.5, 0., 0.],
    );
    let report = intersect_cylinder_cylinder(&first, &tilted, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::NearCoincidence
    );
    // Clearly non-parallel: the general quartic is out of scope.
    let skew = rotated_translated(
        &brep_core::analytic::cylinder(3., 8.).unwrap(),
        0.3,
        [3.5, 0., 0.],
    );
    let report = intersect_cylinder_cylinder(&first, &skew, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.unresolved.len(), 1);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::UnsupportedSurface
    );
    assert_eq!(
        report.unresolved[0].parameter_box,
        vec![0., 1., 0., 1., 0., 1., 0., 1.]
    );
    // A near-coaxial offset inside the recognition band: near_coincidence.
    let near = translated(&brep_core::analytic::cylinder(2., 8.).unwrap(), [1e-10, 0., 0.]);
    let report = intersect_cylinder_cylinder(&first, &near, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::NearCoincidence
    );
}

#[test]
fn rotated_translated_placement_keeps_the_exact_lines() {
    // Rigid placement of the whole parallel configuration about X by 0.5.
    let angle = 0.5;
    let offset = [0.3, -0.2, 1.1];
    let first = rotated_translated(&brep_core::analytic::cylinder(2., 8.).unwrap(), angle, offset);
    let second = rotated_translated(
        &translated(&brep_core::analytic::cylinder(3., 8.).unwrap(), [3.5, 0., 0.]),
        angle,
        offset,
    );
    let report = intersect_cylinder_cylinder(&first, &second, Options::default()).unwrap();
    let report = only_lines(&report, 2);
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
    let first_center = placed([0., 0., 4.]);
    let second_center = placed([3.5, 0., 4.]);
    let d = 3.5_f64;
    let x = (d * d + 4. - 9.) / (2. * d);
    let y = (4. - x * x).sqrt();
    for (component, sign) in report.components.iter().zip([-1., 1.]) {
        let (curve, start, end, direction, _, first_uv, second_uv, sampled) =
            line_of(component);
        assert!(
            sub(start, placed([x, sign * y, 0.]))
                .iter()
                .all(|v| v.abs() <= 1e-12),
            "{start:?}"
        );
        assert!(
            sub(end, placed([x, sign * y, 8.]))
                .iter()
                .all(|v| v.abs() <= 1e-12),
            "{end:?}"
        );
        assert!(
            sub(direction, axis).iter().all(|v| v.abs() <= 1e-12),
            "{direction:?}"
        );
        let mut worst = 0_f64;
        for k in 0..=8 {
            let p = curve.evaluate(k as f64 / 8.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            worst = worst
                .max(side_residual(p, first_center, axis, 2.))
                .max(side_residual(p, second_center, axis, 3.));
        }
        assert!(worst <= 1e-12, "{worst}");
        assert!(sampled <= 1e-12);
        let uv_worst = uv_samples(&first, &second, first_uv, second_uv, |p| {
            side_residual(p, first_center, axis, 2.).max(side_residual(
                p,
                second_center,
                axis,
                3.,
            ))
        });
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }
}

#[test]
fn noncanonical_operands_are_explicit_unsupported_regions() {
    let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
    // Frusta, tubes, spheres and cuboids are not the canonical cylinder.
    for (a, b) in [
        (
            brep_core::analytic::frustum(1., 2., 3.).unwrap(),
            brep_core::analytic::cylinder(2., 8.).unwrap(),
        ),
        (
            brep_core::analytic::tube(2., 1., 3.).unwrap(),
            brep_core::analytic::cylinder(2., 8.).unwrap(),
        ),
        (
            brep_core::analytic::cylinder(2., 8.).unwrap(),
            brep_core::analytic::sphere(2.).unwrap(),
        ),
        (
            brep_core::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap(),
            brep_core::analytic::cylinder(2., 8.).unwrap(),
        ),
    ] {
        let report = intersect_cylinder_cylinder(&a, &b, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(report.unresolved.len(), 1);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::UnsupportedSurface
        );
    }
    // A canonical pair still resolves.
    let second = translated(&brep_core::analytic::cylinder(3., 8.).unwrap(), [3.5, 0., 0.]);
    let report = intersect_cylinder_cylinder(&cylinder, &second, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    assert_eq!(report.components.len(), 2);
    // A structurally perturbed cylinder fails validation as a hard error.
    let mut perturbed = brep_core::analytic::cylinder(1., 2.).unwrap();
    perturbed.faces[0].surface.weights[1][0] = 0.5;
    assert!(intersect_cylinder_cylinder(&cylinder, &perturbed, Options::default()).is_err());
}
