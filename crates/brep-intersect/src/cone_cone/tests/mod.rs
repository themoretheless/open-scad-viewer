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
    report: &Report<ConeConeComponent>,
    count: usize,
) -> &Report<ConeConeComponent> {
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert!(report.unresolved.is_empty(), "{report:?}");
    assert!(!report.permits_topology_change());
    assert_eq!(report.components.len(), count, "{report:?}");
    report
}
#[allow(clippy::type_complexity)]
fn circle_of(
    component: &ConeConeComponent,
) -> (
    &Curve,
    [f64; 3],
    f64,
    [f64; 3],
    &[CylinderPatchCurve],
    &[CylinderPatchCurve],
    f64,
) {
    let ConeConeComponent::Circle {
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
/// Cone side profile residual of a canonical z-up cone.
fn side_residual_z(point: [f64; 3], r_bottom: f64, slope: f64) -> f64 {
    (point[0].hypot(point[1]) - (r_bottom + slope * point[2])).abs()
}
/// Worst residual of lifted UV arcs evaluated through their own surface
/// against both cones' side predicates.
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
fn different_tapers_cross_in_one_exact_circle() {
    // Cone1 r 1 -> 3 over z 0..6 (rho = 1 + s/3); cone2 r 4 -> 2 over
    // z 1..5 (rho = 4.5 - s/2). Root: (5/6) s = 3.5 — s* = 4.2 strictly
    // inside both ranges, radius 2.4, v = 0.7 on cone1 and 0.8 on cone2.
    let first = brep_core::analytic::frustum(1., 3., 6.).unwrap();
    let second = translated(&brep_core::analytic::frustum(4., 2., 4.).unwrap(), [0., 0., 1.]);
    let report = intersect_cone_cone(&first, &second, Options::default()).unwrap();
    let report = only_circles(&report, 1);
    let (curve, center, radius, normal, first_uv, second_uv, sampled) =
        circle_of(&report.components[0]);
    assert!((radius - 2.4).abs() <= 1e-12, "{radius}");
    assert!(
        sub(center, [0., 0., 4.2]).iter().all(|x| x.abs() <= 1e-12),
        "{center:?}"
    );
    assert!(normal[2] >= 1. - 1e-12, "{normal:?}");
    // Exact rational circle: four 90-degree arcs, weights cos(pi/4).
    assert_eq!(curve.degree, 2);
    assert_eq!(
        curve.knots,
        vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.]
    );
    assert_eq!(curve.control_points.len(), 9);
    // Sixteen samples satisfy both implicit side equations.
    let mut worst = 0_f64;
    for i in 0..16 {
        let p = point3(&curve.evaluate(i as f64 / 4.).unwrap().point);
        worst = worst
            .max(side_residual_z(p, 1., 1. / 3.))
            .max((p[0].hypot(p[1]) - (4.5 - p[2] / 2.)).abs())
            .max((p[2] - 4.2).abs());
    }
    assert!(worst <= 1e-12, "{worst}");
    assert!(sampled <= 1e-12, "{sampled}");
    // Iso-v lifts on all four side patches of both cones.
    assert_eq!(first_uv.len(), 4);
    assert_eq!(second_uv.len(), 4);
    for lift in first_uv {
        assert_eq!(lift.arcs.len(), 1);
        assert!((lift.arcs[0].control_points[0][1] - 0.7).abs() <= 1e-12);
    }
    for lift in second_uv {
        assert_eq!(lift.arcs.len(), 1);
        assert!((lift.arcs[0].control_points[0][1] - 0.8).abs() <= 1e-12);
    }
    let uv_worst = uv_samples(&first, &second, first_uv, second_uv, |p| {
        side_residual_z(p, 1., 1. / 3.).max((p[0].hypot(p[1]) - (4.5 - p[2] / 2.)).abs())
    });
    assert!(uv_worst <= 1e-9, "{uv_worst}");
}

#[test]
fn root_clipped_by_a_finite_height_resolves_empty() {
    // Cone1 r 3 -> 1 over z 0..6 (rho = 3 - s/3); cone2 r 4 -> 2 over
    // z 2..4 (rho = 6 - s). The linear root s* = 4.5 lies beyond cone2's
    // top ring: honestly absent, empty and resolved.
    let first = brep_core::analytic::frustum(3., 1., 6.).unwrap();
    let second = translated(&brep_core::analytic::frustum(4., 2., 2.).unwrap(), [0., 0., 2.]);
    let report = intersect_cone_cone(&first, &second, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
}

#[test]
fn clearly_separated_axially_resolves_empty() {
    // Cone2 provably above cone1 with a 0.5 gap between the facing ring
    // planes: no shared point is possible in any radius relation.
    let first = brep_core::analytic::frustum(1., 3., 6.).unwrap();
    let second = translated(
        &brep_core::analytic::frustum(1., 2., 3.).unwrap(),
        [0., 0., 6.5],
    );
    let report = intersect_cone_cone(&first, &second, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
}

#[test]
fn equal_taper_coincident_profiles_report_coincident_trim() {
    // Cone1 rho = 1 + s/3 over z 0..6; cone2 r 5/3 -> 11/3 over z 2..8
    // carries the same profile over the overlap z 2..6: coincident_trim,
    // never a surface component.
    let first = brep_core::analytic::frustum(1., 3., 6.).unwrap();
    let second = translated(
        &brep_core::analytic::frustum(5. / 3., 11. / 3., 6.).unwrap(),
        [0., 0., 2.],
    );
    let report = intersect_cone_cone(&first, &second, Options::default()).unwrap();
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
fn equal_taper_distinct_profiles_resolve_empty() {
    // Same taper 1/3, profiles 1/3 apart over the overlap z 2..6: one
    // side strictly inside the other, no contact, resolved empty.
    let first = brep_core::analytic::frustum(1., 3., 6.).unwrap();
    let second = translated(&brep_core::analytic::frustum(2., 4., 6.).unwrap(), [0., 0., 2.]);
    let report = intersect_cone_cone(&first, &second, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
}

#[test]
fn stacked_rim_plane_contact_stays_unresolved() {
    // Cone2 stacked base-to-top on cone1 with equal ring radii 3 at the
    // shared plane z=6: the coincident ring planes carry a rim/rim
    // tangent boundary contact — tangency_or_multiple_root, never a
    // guessed circle.
    let first = brep_core::analytic::frustum(1., 3., 6.).unwrap();
    let second = translated(&brep_core::analytic::frustum(3., 5., 6.).unwrap(), [0., 0., 6.]);
    let report = intersect_cone_cone(&first, &second, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::TangencyOrMultipleRoot
    );
    // Different ring radii at the shared plane (rim circle inside the
    // larger cap disk) are still a boundary contact: unresolved.
    let nested = translated(&brep_core::analytic::frustum(2., 4., 6.).unwrap(), [0., 0., 6.]);
    let report = intersect_cone_cone(&first, &nested, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::TangencyOrMultipleRoot
    );
}

#[test]
fn rim_on_side_contact_stays_unresolved() {
    // Cone2's bottom ring (radius 2 at z=3) lies exactly on cone1's side
    // (rho(3) = 2 for r 1 -> 3 over z 0..6): with the steeper taper 1/2
    // the side root sits on the overlap boundary — a rim contact, never
    // a guessed circle.
    let first = brep_core::analytic::frustum(1., 3., 6.).unwrap();
    let second = translated(&brep_core::analytic::frustum(2., 5., 6.).unwrap(), [0., 0., 3.]);
    let report = intersect_cone_cone(&first, &second, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::TangencyOrMultipleRoot
    );
}

#[test]
fn apex_meeting_stays_unresolved() {
    // Two true-apex cones apex-to-apex at the origin (cone2 flipped
    // down, anti-axial): the touching ring planes carry the apex contact
    // — tangency_or_multiple_root, never a point or circle.
    let first = brep_core::analytic::frustum(0., 3., 5.).unwrap();
    let second = flipped(&brep_core::analytic::frustum(0., 2., 4.).unwrap(), 0.);
    let report = intersect_cone_cone(&first, &second, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::TangencyOrMultipleRoot
    );
}

#[test]
fn anti_axial_pair_yields_the_exact_circle() {
    // Cone1 r 1 -> 3 over z 0..6 (rho = 1 + s/3); cone2 r 2 -> 4 over
    // its own height flipped down (rotation pi about X, then z -> 8 - z):
    // axis -z, own bottom ring (r=2) at z=8, rho = 2 + (8 - s)/3 over
    // s in [2,8]. Root: 2s/3 = 11/3 — s* = 5.5 strictly inside both
    // ranges, radius 17/6, v = 11/12 on cone1 and 5/12 on cone2.
    let first = brep_core::analytic::frustum(1., 3., 6.).unwrap();
    let second = flipped(&brep_core::analytic::frustum(2., 4., 6.).unwrap(), 8.);
    let report = intersect_cone_cone(&first, &second, Options::default()).unwrap();
    let report = only_circles(&report, 1);
    let (curve, center, radius, normal, first_uv, second_uv, sampled) =
        circle_of(&report.components[0]);
    assert!((radius - 17. / 6.).abs() <= 1e-12, "{radius}");
    assert!(
        sub(center, [0., 0., 5.5]).iter().all(|x| x.abs() <= 1e-12),
        "{center:?}"
    );
    assert!(normal[2] >= 1. - 1e-12, "{normal:?}");
    let mut worst = 0_f64;
    for i in 0..16 {
        let p = point3(&curve.evaluate(i as f64 / 4.).unwrap().point);
        worst = worst
            .max(side_residual_z(p, 1., 1. / 3.))
            .max((p[0].hypot(p[1]) - (2. + (8. - p[2]) / 3.)).abs());
    }
    assert!(worst <= 1e-12, "{worst}");
    assert!(sampled <= 1e-12);
    for lift in first_uv {
        assert!((lift.arcs[0].control_points[0][1] - 11. / 12.).abs() <= 1e-12);
    }
    for lift in second_uv {
        assert!((lift.arcs[0].control_points[0][1] - 5. / 12.).abs() <= 1e-12);
    }
    let uv_worst = uv_samples(&first, &second, first_uv, second_uv, |p| {
        side_residual_z(p, 1., 1. / 3.).max((p[0].hypot(p[1]) - (2. + (8. - p[2]) / 3.)).abs())
    });
    assert!(uv_worst <= 1e-9, "{uv_worst}");
}

#[test]
fn near_coaxial_bands_stay_unresolved() {
    let first = brep_core::analytic::frustum(1., 3., 6.).unwrap();
    // Recognition-scale perpendicular offset: near_coincidence, never
    // forced coaxial.
    let offset = translated(
        &brep_core::analytic::frustum(4., 2., 4.).unwrap(),
        [1e-10, 0., 1.],
    );
    let report = intersect_cone_cone(&first, &offset, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::NearCoincidence
    );
    // Recognition-scale tilt: near_coincidence as well.
    let tilted = rotated_translated(
        &brep_core::analytic::frustum(4., 2., 4.).unwrap(),
        1e-10,
        [0., 0., 1.],
    );
    let report = intersect_cone_cone(&first, &tilted, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::NearCoincidence
    );
    // Pure-rounding offsets snap: 1e-14 stays coaxial and resolves the
    // same circle as the exact pair.
    let snapped = translated(
        &brep_core::analytic::frustum(4., 2., 4.).unwrap(),
        [1e-14, 0., 1.],
    );
    let report = intersect_cone_cone(&first, &snapped, Options::default()).unwrap();
    only_circles(&report, 1);
}

#[test]
fn off_axis_and_non_parallel_pairs_are_unsupported_regions() {
    let first = brep_core::analytic::frustum(1., 3., 6.).unwrap();
    // Parallel but 0.5 off the axis: coaxial quartic, out of scope.
    let off = translated(
        &brep_core::analytic::frustum(4., 2., 4.).unwrap(),
        [0.5, 0., 1.],
    );
    // Tilted 0.5 rad: the general quartic, out of scope.
    let tilted = rotated_translated(
        &brep_core::analytic::frustum(4., 2., 4.).unwrap(),
        0.5,
        [0., 0., 1.],
    );
    for second in [&off, &tilted] {
        let report = intersect_cone_cone(&first, second, Options::default()).unwrap();
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
    let cone = brep_core::analytic::frustum(1., 3., 6.).unwrap();
    // Equal-radius frusta are cylinders (refused as cone operands), and
    // spheres, tubes, cuboids and tori are not the canonical cone.
    for (a, b) in [
        (
            brep_core::analytic::frustum(1., 3., 6.).unwrap(),
            brep_core::analytic::cylinder(1., 3.).unwrap(),
        ),
        (
            brep_core::analytic::frustum(1., 3., 6.).unwrap(),
            brep_core::analytic::sphere(2.).unwrap(),
        ),
        (
            brep_core::analytic::tube(2., 1., 3.).unwrap(),
            brep_core::analytic::frustum(1., 3., 6.).unwrap(),
        ),
        (
            brep_core::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap(),
            brep_core::analytic::frustum(1., 3., 6.).unwrap(),
        ),
        (
            brep_core::analytic::torus(3., 1.).unwrap(),
            brep_core::analytic::frustum(1., 3., 6.).unwrap(),
        ),
    ] {
        let report = intersect_cone_cone(&a, &b, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(report.unresolved.len(), 1);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::UnsupportedSurface
        );
    }
    // A structurally perturbed cone fails validation as a hard error.
    let mut perturbed = brep_core::analytic::frustum(1., 2., 3.).unwrap();
    perturbed.faces[0].surface.weights[1][0] = 0.5;
    assert!(intersect_cone_cone(&cone, &perturbed, Options::default()).is_err());
}

#[test]
fn rotated_translated_placement_keeps_the_exact_circle() {
    // The crossing pair of the first test rigidly placed about X by 0.5.
    let angle = 0.5;
    let offset = [0.3, -0.2, 1.1];
    let first = rotated_translated(
        &brep_core::analytic::frustum(1., 3., 6.).unwrap(),
        angle,
        offset,
    );
    let second = rotated_translated(
        &translated(&brep_core::analytic::frustum(4., 2., 4.).unwrap(), [0., 0., 1.]),
        angle,
        offset,
    );
    let report = intersect_cone_cone(&first, &second, Options::default()).unwrap();
    let report = only_circles(&report, 1);
    let (curve, center, radius, normal, first_uv, second_uv, sampled) =
        circle_of(&report.components[0]);
    // Independent binary64 oracle in the placed frame.
    let (sin, cos) = angle.sin_cos();
    let placed = |p: [f64; 3]| {
        [
            p[0] + offset[0],
            cos * p[1] - sin * p[2] + offset[1],
            sin * p[1] + cos * p[2] + offset[2],
        ]
    };
    let expected = placed([0., 0., 4.2]);
    let axis = sub(placed([0., 0., 1.]), placed([0., 0., 0.]));
    assert!((radius - 2.4).abs() <= 1e-12, "{radius}");
    assert!(
        sub(center, expected).iter().all(|x| x.abs() <= 1e-12),
        "{center:?}"
    );
    assert!(
        sub(normal, axis).iter().all(|x| x.abs() <= 1e-12),
        "{normal:?}"
    );
    let side = |p: [f64; 3], origin: [f64; 3], r_bottom: f64, slope: f64| {
        let rel = sub(p, origin);
        let a = dot(rel, axis);
        let perp = sub(rel, axis.map(|x| x * a));
        (norm3(perp) - (r_bottom + slope * a)).abs()
    };
    let origin1 = placed([0., 0., 0.]);
    let origin2 = placed([0., 0., 1.]);
    let mut worst = 0_f64;
    for i in 0..16 {
        let p = point3(&curve.evaluate(i as f64 / 4.).unwrap().point);
        worst = worst
            .max(side(p, origin1, 1., 1. / 3.))
            .max(side(p, origin2, 4., -0.5));
    }
    assert!(worst <= 1e-12, "{worst}");
    assert!(sampled <= 1e-12);
    assert_eq!(first_uv.len(), 4);
    assert_eq!(second_uv.len(), 4);
    let uv_worst = uv_samples(&first, &second, first_uv, second_uv, |p| {
        side(p, origin1, 1., 1. / 3.).max(side(p, origin2, 4., -0.5))
    });
    assert!(uv_worst <= 1e-9, "{uv_worst}");
}
