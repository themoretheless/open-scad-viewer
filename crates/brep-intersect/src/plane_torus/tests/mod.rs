use super::super::plane_sphere::plane_patch;
use brep_core::intersections::sphere_sphere::ARC_WEIGHT;
use super::super::test_utils::rotated_translated;
use super::*;

/// (curve, center, radius, full, plane_uv, torus_uv, sampled) — Copy view.
type CircleView<'a> = (
    &'a Curve,
    [f64; 3],
    f64,
    bool,
    &'a [PlanePatchCurve],
    &'a [TorusPatchCurve],
    f64,
);

/// Torus implicit residual (unscaled quartic) in the canonical frame.
fn implicit(point: [f64; 3], major: f64, minor: f64) -> f64 {
    let total = dot(point, point);
    let rho2 = point[0] * point[0] + point[1] * point[1];
    let base = total + major * major - minor * minor;
    (base * base - 4. * major * major * rho2).abs()
}
fn only_circles(report: &Report<PlaneTorusComponent>, count: usize) -> Vec<CircleView<'_>> {
    assert!(report.unresolved.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert!(!report.permits_topology_change());
    assert_eq!(report.components.len(), count, "{report:?}");
    report
        .components
        .iter()
        .map(|c| {
            let PlaneTorusComponent::Circle {
                curve,
                center,
                radius,
                full,
                plane_uv,
                torus_uv,
                max_sample_residual,
                ..
            } = c;
            (
                curve,
                *center,
                *radius,
                *full,
                plane_uv.as_slice(),
                torus_uv.as_slice(),
                *max_sample_residual,
            )
        })
        .collect()
}
/// Worst residual of lifted UV arcs on both operands, evaluated through
/// their own surfaces, against a caller-supplied implicit checker.
fn uv_samples(
    plane_model: &Model,
    torus_model: &Model,
    circles: &[CircleView<'_>],
    check: &dyn Fn([f64; 3]) -> f64,
) -> f64 {
    let mut worst = 0_f64;
    for (_, _, _, _, plane_uv, torus_uv, _) in circles {
        for lift in plane_uv.iter() {
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
        for lift in torus_uv.iter() {
            let surface = &torus_model.faces[lift.patch].surface;
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
    }
    worst
}

#[test]
fn perpendicular_section_matches_the_two_circle_oracle() {
    // Torus R=3, r=1; plane z = 0.4, patch [-5,5]^2: the exact parallels
    // of radii 3 +- sqrt(1 - 0.16), fully inside the patch.
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let plane = plane_patch([-5., -5., 0.4], [10., 0., 0.], [0., 10., 0.]);
    let report = intersect_plane_torus(&plane, &torus, Options::default()).unwrap();
    let circles = only_circles(&report, 2);
    let s = (1_f64 - 0.16).sqrt();
    let expected = [(3. + s, 0usize), (3. - s, 1)];
    for k in 0..2 {
        let (curve, center, radius, full, plane_uv, torus_uv, sampled) = circles[k];
        assert!(
            (radius - expected[k].0).abs() <= 1e-12,
            "{radius} vs {}",
            expected[k].0
        );
        assert!(
            sub(center, [0., 0., 0.4]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        assert!(full);
        // Exact rational circle: four 90-degree arcs, weights cos(pi/4).
        assert_eq!(curve.degree, 2);
        assert_eq!(
            curve.knots,
            vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.]
        );
        assert_eq!(curve.control_points.len(), 9);
        // Sixteen samples on both implicit equations.
        let mut worst = 0_f64;
        for i in 0..16 {
            let p = curve.evaluate(i as f64 / 4.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            worst = worst.max(implicit(p, 3., 1.)).max((p[2] - 0.4).abs());
        }
        assert!(worst <= 1e-10, "{worst}");
        assert!(sampled <= 1e-10, "{sampled}");
        // Torus UV: one iso-v line on each revolution quadrant patch of
        // the profile quadrant row, at the exact v parameter.
        assert_eq!(torus_uv.len(), 4, "{torus_uv:?}");
        let phi = 0.4_f64.atan2(if k == 0 { s } else { -s }).rem_euclid(TAU);
        let v0 = arc_parameter(phi - expected[k].1 as f64 * QUARTER);
        for lift in torus_uv {
            assert_eq!(lift.arcs.len(), 1);
            let arc = &lift.arcs[0];
            assert_eq!(arc.degree, 1);
            assert_eq!(arc.control_points[0], vec![0., v0]);
            assert_eq!(arc.control_points[1], vec![1., v0]);
        }
        // Plane UV: four exact ellipse arcs inside the unit square.
        assert_eq!(plane_uv.len(), 1);
        assert_eq!(plane_uv[0].arcs.len(), 4);
        for arc in &plane_uv[0].arcs {
            assert_eq!(arc.weights, vec![1., ARC_WEIGHT, 1.]);
        }
    }
    let uv_worst = uv_samples(&plane, &torus, &circles, &|p| {
        implicit(p, 3., 1.) / (4. * 64.) + (p[2] - 0.4).abs()
    });
    assert!(uv_worst <= 1e-9, "{uv_worst}");
    // Operand order is fixed: the torus as plane operand refuses.
    let swapped = intersect_plane_torus(&torus, &plane, Options::default()).unwrap();
    assert_eq!(
        swapped.unresolved[0].reason,
        UnresolvedReason::UnsupportedSurface
    );
}

#[test]
fn symmetry_plane_section_is_the_equator_pair() {
    // Plane z = 0: the exact equator circles R + r = 4 and R - r = 2,
    // landing exactly on profile quadrant seams (v = 0 on rows 0 and 2).
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let recognized = recognize_torus(&torus).unwrap().unwrap();
    let plane = plane_patch([-5., -5., 0.], [10., 0., 0.], [0., 10., 0.]);
    let report = intersect_plane_torus(&plane, &torus, Options::default()).unwrap();
    let circles = only_circles(&report, 2);
    assert!((circles[0].2 - 4.).abs() <= 1e-12, "{}", circles[0].2);
    assert!((circles[1].2 - 2.).abs() <= 1e-12, "{}", circles[1].2);
    for (k, row) in [0usize, 2].iter().enumerate() {
        assert_eq!(circles[k].5.len(), 4);
        for lift in circles[k].5 {
            assert_eq!(lift.arcs[0].control_points[0], vec![0., 0.]);
            assert_eq!(lift.arcs[0].control_points[1], vec![1., 0.]);
            assert!(recognized.patches[*row].contains(&lift.patch));
        }
    }
    let uv_worst = uv_samples(&plane, &torus, &circles, &|p| {
        implicit(p, 3., 1.) / (4. * 64.) + p[2].abs()
    });
    assert!(uv_worst <= 1e-9, "{uv_worst}");
    // The near-horn limit admitted by the constructor still resolves:
    // R - r = 1e-5 keeps the inner equator circle clearly above the band.
    let slim = brep_core::analytic::torus(1.00001, 1.).unwrap();
    let report = intersect_plane_torus(&plane, &slim, Options::default()).unwrap();
    let circles = only_circles(&report, 2);
    assert!((circles[0].2 - 2.00001).abs() <= 1e-12, "{}", circles[0].2);
    assert!((circles[1].2 - 1e-5).abs() <= 1e-12, "{}", circles[1].2);
    // Horn and spindle tori are refused by the canonical constructor:
    // the recognizer can never certify one.
    assert!(brep_core::analytic::torus(2., 2.).is_err());
    assert!(brep_core::analytic::torus(1., 2.).is_err());
}

#[test]
fn miss_tangency_and_band_classification() {
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let at = |z: f64| plane_patch([-5., -5., z], [10., 0., 0.], [0., 10., 0.]);
    // Miss: |h| = 2 > r, empty and resolved.
    let report = intersect_plane_torus(&at(2.), &torus, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    // Exact tangency and within the outward band: never a point/circle.
    for z in [1., 1. - 3e-14, -1.] {
        let report = intersect_plane_torus(&at(z), &torus, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{z} {report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
    }
    // Just clear of the band: a thin parallel pair resolves.
    let report = intersect_plane_torus(&at(1. - 1e-9), &torus, Options::default()).unwrap();
    let circles = only_circles(&report, 2);
    let h = 1. - 1e-9;
    let s = (1_f64 - h * h).sqrt();
    assert!(
        (circles[0].2 - (3. + s)).abs() <= 1e-12,
        "{} vs {}",
        circles[0].2,
        3. + s
    );
    assert!((circles[1].2 - (3. - s)).abs() <= 1e-12);
}

#[test]
fn through_axis_section_matches_the_meridian_pair() {
    // Plane y = 0 contains the Z axis: the exact meridian circles of
    // radius r = 1 centered at (+-3, 0, 0). The patch normal is
    // (0,-1,0), so m = axis x normal = (+1,0,0): side +1 sits at
    // revolution angle 0 (quadrant column 0), side -1 at pi (column 2).
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let recognized = recognize_torus(&torus).unwrap().unwrap();
    let plane = plane_patch([-5., 0., -5.], [10., 0., 0.], [0., 0., 10.]);
    let report = intersect_plane_torus(&plane, &torus, Options::default()).unwrap();
    let circles = only_circles(&report, 2);
    let mut signed = Vec::new();
    for (curve, center, radius, full, plane_uv, torus_uv, sampled) in &circles {
        assert!((radius - 1.).abs() <= 1e-12, "{radius}");
        assert!(
            center[1].abs() <= 1e-12 && center[2].abs() <= 1e-12,
            "{center:?}"
        );
        assert!((center[0].abs() - 3.).abs() <= 1e-12, "{center:?}");
        signed.push(center[0]);
        assert!(full);
        let mut worst = 0_f64;
        for i in 0..16 {
            let p = curve.evaluate(i as f64 / 4.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            worst = worst.max(implicit(p, 3., 1.)).max(p[1].abs());
        }
        assert!(worst <= 1e-10, "{worst}");
        assert!(*sampled <= 1e-10, "{sampled}");
        // Torus UV: one full iso-u line per profile quadrant patch of the
        // meridian's revolution column, exactly on the quadrant seam.
        assert_eq!(torus_uv.len(), 4, "{torus_uv:?}");
        let column = if center[0] > 0. { 0 } else { 2 };
        for lift in torus_uv.iter() {
            assert_eq!(lift.arcs.len(), 1);
            let arc = &lift.arcs[0];
            assert_eq!(arc.degree, 1);
            assert_eq!(arc.control_points[0], vec![0., 0.]);
            assert_eq!(arc.control_points[1], vec![0., 1.]);
            assert!(
                (0..4).any(|j| recognized.patches[j][column] == lift.patch),
                "{}",
                lift.patch
            );
        }
        assert_eq!(plane_uv[0].arcs.len(), 4);
    }
    assert!(signed[0] * signed[1] < 0., "{signed:?}");
    let uv_worst = uv_samples(&plane, &torus, &circles, &|p| {
        implicit(p, 3., 1.) / (4. * 64.) + p[1].abs()
    });
    assert!(uv_worst <= 1e-9, "{uv_worst}");
}

#[test]
fn offset_parallel_and_oblique_planes_are_honestly_refused() {
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    // Parallel to the axis at distance d: a Cassini-oval section, not a
    // circle pair — unsupported beyond the band, near_coincidence inside
    // the recognition-scale band, snapped only at pure rounding scale.
    let at = |y: f64| plane_patch([-5., y, -5.], [10., 0., 0.], [0., 0., 10.]);
    let refused = intersect_plane_torus(&at(0.5), &torus, Options::default()).unwrap();
    assert!(refused.components.is_empty());
    assert_eq!(
        refused.unresolved[0].reason,
        UnresolvedReason::UnsupportedSurface
    );
    let near = intersect_plane_torus(&at(1e-12), &torus, Options::default()).unwrap();
    assert!(near.components.is_empty());
    assert_eq!(near.unresolved[0].reason, UnresolvedReason::NearCoincidence);
    // Pure rounding-scale offset snaps to through-axis and resolves.
    let snapped = intersect_plane_torus(&at(1e-16), &torus, Options::default()).unwrap();
    assert_eq!(snapped.components.len(), 2, "{snapped:?}");
    // Oblique plane: the quartic with Villarceau degeneracies is out of
    // scope — unsupported, never a numerical fallback.
    let s = std::f64::consts::FRAC_1_SQRT_2;
    let oblique = plane_patch(
        [-5., -5. * s, -5. * s],
        [10., 0., 0.],
        [0., 10. * s, 10. * s],
    );
    let report = intersect_plane_torus(&oblique, &torus, Options::default()).unwrap();
    assert!(report.components.is_empty());
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::UnsupportedSurface
    );
    // Recognition-scale tilts off either axial configuration report
    // near_coincidence, never forced: 1 - cos(a) ~ a^2/2 <= 1e-9 off
    // perpendicular, sin(b) ~ b <= 1e-9 off parallel.
    let a: f64 = 1e-6;
    let tilted_perp = plane_patch(
        [-5., -5. * a.cos(), -5. * a.sin()],
        [10., 0., 0.],
        [0., 10. * a.cos(), 10. * a.sin()],
    );
    let report = intersect_plane_torus(&tilted_perp, &torus, Options::default()).unwrap();
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::NearCoincidence
    );
    let b: f64 = 1e-10;
    let tilted_par = plane_patch(
        [-5., -5. * b.sin(), -5.],
        [10., 0., 0.],
        [0., 10. * b.sin(), 10. * b.cos()],
    );
    let report = intersect_plane_torus(&tilted_par, &torus, Options::default()).unwrap();
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::NearCoincidence
    );
}

#[test]
fn domain_clipping_has_exact_trims_and_clipped_lifts() {
    // Plane z = 0.4, patch covering only x >= 0 (y in [-5, 5]): both
    // parallels clip to their x >= 0 half arcs with exact endpoints at
    // (0, +-rho, 0.4); the half circle covers revolution quadrants 3 and
    // 0 completely, so each torus lift spans u in [0, 1] on two patches.
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let plane = plane_patch([0., -5., 0.4], [5., 0., 0.], [0., 10., 0.]);
    let report = intersect_plane_torus(&plane, &torus, Options::default()).unwrap();
    let circles = only_circles(&report, 2);
    let s = (1_f64 - 0.16).sqrt();
    for (k, &(curve, _, radius, full, plane_uv, torus_uv, sampled)) in
        circles.iter().take(2).enumerate()
    {
        let rho = 3. + if k == 0 { s } else { -s };
        assert!((radius - rho).abs() <= 1e-12);
        assert!(!full);
        // Half circle: swept [3pi/2, 5pi/2] in the (x, y) basis, two
        // 90-degree pieces.
        assert_eq!(curve.knots, vec![0., 0., 0., 1., 1., 2., 2., 2.]);
        let start = curve.evaluate(0.).unwrap().point;
        let end = curve.evaluate(2.).unwrap().point;
        assert!(
            start[0].abs() <= 1e-12 && (start[1] + rho).abs() <= 1e-12,
            "{start:?}"
        );
        assert!(
            end[0].abs() <= 1e-12 && (end[1] - rho).abs() <= 1e-12,
            "{end:?}"
        );
        let mut worst = 0_f64;
        for i in 0..=16 {
            let p = curve.evaluate(i as f64 / 8.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            assert!(p[0] >= -1e-12, "{p:?}");
            worst = worst.max(implicit(p, 3., 1.)).max((p[2] - 0.4).abs());
        }
        assert!(worst <= 1e-10, "{worst}");
        assert!(sampled <= 1e-10, "{sampled}");
        // Torus UV: two patches (revolution quadrants 3 and 0), each a
        // full-sweep iso-v line.
        assert_eq!(torus_uv.len(), 2, "{torus_uv:?}");
        for lift in torus_uv {
            assert_eq!(lift.arcs.len(), 1);
            let arc = &lift.arcs[0];
            let u0 = arc.control_points[0][0];
            let u1 = arc.control_points[1][0];
            assert!((u0, u1) == (0., 1.), "{u0} {u1}");
            assert_eq!(arc.control_points[0][1], arc.control_points[1][1]);
        }
        // Plane UV: two exact clipped ellipse arcs.
        assert_eq!(plane_uv[0].arcs.len(), 2);
    }
    let uv_worst = uv_samples(&plane, &torus, &circles, &|p| {
        implicit(p, 3., 1.) / (4. * 64.) + (p[2] - 0.4).abs()
    });
    assert!(uv_worst <= 1e-9, "{uv_worst}");
    // A circle clearly outside the rectangle: empty and resolved.
    let outside = plane_patch([4.5, -5., 0.4], [5., 0., 0.], [0., 10., 0.]);
    let report = intersect_plane_torus(&outside, &torus, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    // The outer circle tangent to the patch edge x = 3 + s: tangency
    // region, the circle withheld, never guessed; the inner circle
    // (radius 3 - s, center x = 0 < 3 + s) is provably outside too.
    let tangent = plane_patch([3. + s, -5., 0.4], [5., 0., 0.], [0., 10., 0.]);
    let report = intersect_plane_torus(&tangent, &torus, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert!(report.components.is_empty(), "{report:?}");
    assert!(
        report
            .unresolved
            .iter()
            .any(|u| u.reason == UnresolvedReason::TangencyOrMultipleRoot)
    );
}

#[test]
fn rigid_placement_keeps_the_exact_circles() {
    let angle = 0.5;
    let offset = [0.3, -0.2, 1.1];
    let torus = rotated_translated(&brep_core::analytic::torus(3., 1.).unwrap(), angle, offset);
    let plane = rotated_translated(
        &plane_patch([-5., -5., 0.4], [10., 0., 0.], [0., 10., 0.]),
        angle,
        offset,
    );
    // The recognizer recovers the placed axis, center and radii.
    let recognized = recognize_torus(&torus).unwrap().unwrap();
    let (sin, cos) = angle.sin_cos();
    assert!(
        (recognized.major - 3.).abs() <= 1e-12,
        "{}",
        recognized.major
    );
    assert!(
        (recognized.minor - 1.).abs() <= 1e-12,
        "{}",
        recognized.minor
    );
    assert!(
        sub(recognized.axis, [0., -sin, cos])
            .iter()
            .all(|x| x.abs() <= 1e-12),
        "{:?}",
        recognized.axis
    );
    assert!(
        sub(recognized.center, offset)
            .iter()
            .all(|x| x.abs() <= 1e-12),
        "{:?}",
        recognized.center
    );
    let report = intersect_plane_torus(&plane, &torus, Options::default()).unwrap();
    let circles = only_circles(&report, 2);
    let s = (1_f64 - 0.16).sqrt();
    let placed = |p: [f64; 3]| {
        [
            p[0] + offset[0],
            cos * p[1] - sin * p[2] + offset[1],
            sin * p[1] + cos * p[2] + offset[2],
        ]
    };
    let placed_normal = sub(placed([0., 0., 2.4]), placed([0., 0., 1.4]));
    // Un-place a world point back into the canonical torus frame.
    let unplaced = |p: [f64; 3]| {
        [
            p[0] - offset[0],
            cos * (p[1] - offset[1]) + sin * (p[2] - offset[2]),
            -sin * (p[1] - offset[1]) + cos * (p[2] - offset[2]),
        ]
    };
    for (k, &(curve, center, radius, _, _, _, sampled)) in circles.iter().take(2).enumerate() {
        let rho = 3. + if k == 0 { s } else { -s };
        assert!((radius - rho).abs() <= 1e-12, "{radius}");
        assert!(
            sub(center, placed([0., 0., 0.4]))
                .iter()
                .all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        let mut worst = 0_f64;
        for i in 0..16 {
            let p = curve.evaluate(i as f64 / 4.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            worst = worst
                .max(implicit(unplaced(p), 3., 1.))
                .max(dot(placed_normal, sub(p, center)).abs());
        }
        assert!(worst <= 1e-10, "{worst}");
        assert!(sampled <= 1e-10, "{sampled}");
    }
    let placed_origin = placed([-5., -5., 0.4]);
    let uv_worst = uv_samples(&plane, &torus, &circles, &|p| {
        implicit(unplaced(p), 3., 1.) / (4. * 64.)
            + dot(placed_normal, sub(p, placed_origin)).abs()
    });
    assert!(uv_worst <= 1e-9, "{uv_worst}");
}

#[test]
fn noncanonical_operands_are_explicit_unsupported_regions() {
    let plane = plane_patch([-5., -5., 0.4], [10., 0., 0.], [0., 10., 0.]);
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    for (a, b) in [
        (plane.clone(), brep_core::analytic::cylinder(1., 2.).unwrap()),
        (plane.clone(), brep_core::analytic::sphere(2.).unwrap()),
        (
            plane.clone(),
            brep_core::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap(),
        ),
        (brep_core::analytic::cylinder(1., 2.).unwrap(), torus.clone()),
        (
            brep_core::cuboid([0., 0., 0.], [4., 4., 4.]).unwrap(),
            torus.clone(),
        ),
    ] {
        let report = intersect_plane_torus(&a, &b, Options::default()).unwrap();
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
    let skewed = plane_patch([-5., -5., 0.4], [10., 0., 0.], [3., 10., 0.]);
    let report = intersect_plane_torus(&skewed, &torus, Options::default()).unwrap();
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::UnsupportedSurface
    );
    // A structurally perturbed torus (one moved control point) fails the
    // exact certification and refuses.
    let mut perturbed = torus.clone();
    perturbed.faces[0].surface.control_points[1][1][0] += 1e-6;
    perturbed.rebuild_topology_ids();
    let report = intersect_plane_torus(&plane, &perturbed, Options::default()).unwrap();
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::UnsupportedSurface
    );
    // A valid canonical pair still resolves.
    let report = intersect_plane_torus(&plane, &torus, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
}
