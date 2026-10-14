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
    report: &Report<ConeTorusComponent>,
    count: usize,
) -> &Report<ConeTorusComponent> {
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert!(report.unresolved.is_empty(), "{report:?}");
    assert!(!report.permits_topology_change());
    assert_eq!(report.components.len(), count, "{report:?}");
    report
}
#[allow(clippy::type_complexity)]
fn circle_of(
    component: &ConeTorusComponent,
) -> (
    &Curve,
    [f64; 3],
    f64,
    [f64; 3],
    &[CylinderPatchCurve],
    &[TorusPatchCurve],
    f64,
) {
    let ConeTorusComponent::Circle {
        curve,
        center,
        radius,
        normal,
        cone_uv,
        torus_uv,
        max_sample_residual,
    } = component;
    (
        curve,
        *center,
        *radius,
        *normal,
        cone_uv,
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
/// Cone side profile residual of the canonical z-up frames used here.
fn side_residual_z(point: [f64; 3], r_bottom: f64, slope: f64, z0: f64) -> f64 {
    (point[0].hypot(point[1]) - (r_bottom + slope * (point[2] - z0))).abs()
}
/// Exact rational 90-degree-arc parameter (the plane_torus map).
fn arc_parameter(local: f64) -> f64 {
    let t = (local / 2.).tan();
    let s = std::f64::consts::SQRT_2 * t / (1. - t);
    s / (1. + s)
}
/// Worst residual of lifted UV arcs evaluated through their own surface,
/// against a caller-supplied cone predicate and the torus equation.
fn uv_samples(
    cone_model: &Model,
    torus_model: &Model,
    cone_uv: &[CylinderPatchCurve],
    torus_uv: &[TorusPatchCurve],
    cone_check: impl Fn([f64; 3]) -> f64,
    torus_check: impl Fn([f64; 3]) -> f64,
) -> f64 {
    let mut worst = 0_f64;
    for lift in cone_uv {
        let surface = &cone_model.faces[lift.patch].surface;
        for arc in &lift.arcs {
            for k in 0..=8 {
                let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                assert!((0. ..=1.).contains(&uv[0]) && (0. ..=1.).contains(&uv[1]));
                let p = surface.evaluate(uv[0], uv[1]).unwrap().point;
                let p = [p[0], p[1], p[2]];
                worst = worst.max(cone_check(p)).max(torus_check(p));
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
                worst = worst.max(cone_check(p)).max(torus_check(p));
            }
        }
    }
    worst
}

#[test]
fn two_side_circles_match_the_quadratic_oracle() {
    // Torus R=3, r=1 at the origin; frustum r 1 -> 3 over z 0..6
    // (slope 1/3) with its bottom ring at z=-3: the meridian quadratic
    // is 5 t^2 - 33 t + 54 = 0 with the exact roots t = 3 and t = 3.6 —
    // two side circles (rho=2, z=0) and (rho=2.2, z=0.6), both cap
    // planes at |h|=3 clear of the tube.
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let cone = translated(
        &brep_core::analytic::frustum(1., 3., 6.).unwrap(),
        [0., 0., -3.],
    );
    let report = intersect_cone_torus(&cone, &torus, Options::default()).unwrap();
    let report = only_circles(&report, 2);
    for (component, (rho, z)) in report.components.iter().zip([(2., 0.), (2.2, 0.6)]) {
        let (curve, center, radius, normal, cone_uv, torus_uv, sampled) = circle_of(component);
        assert!((radius - rho).abs() <= 1e-12, "{radius}");
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
        // Sixteen samples satisfy the cone side profile and the torus
        // implicit equation.
        let mut worst = 0_f64;
        for i in 0..16 {
            let p = curve.evaluate(i as f64 / 4.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            worst = worst
                .max(side_residual_z(p, 1., 1. / 3., -3.))
                .max(implicit(p, 3., 1.));
        }
        assert!(worst <= 1e-10, "{worst}");
        assert!(sampled <= 1e-10, "{sampled}");
        // Cone lift: the iso-v line at the exact height fraction on all
        // four side patches (v = t / 6, t = z + 3).
        assert_eq!(cone_uv.len(), 4, "{cone_uv:?}");
        let v0 = (z + 3.) / 6.;
        for lift in cone_uv {
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
        let phi = z.atan2(rho - 3.).rem_euclid(TAU);
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
            &cone,
            &torus,
            cone_uv,
            torus_uv,
            |p| side_residual_z(p, 1., 1. / 3., -3.),
            |p| implicit(p, 3., 1.),
        );
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }
}

#[test]
fn height_clipping_drops_the_upper_side_circle() {
    // Frustum r 1 -> 3 over z 0..6 at the origin; torus R=3, r=1 at
    // z=6.5: the meridian quadratic has the exact roots
    // t = (129 +- 3 sqrt(39)) / 20 — the lower root t1 = 5.5133...
    // survives, the upper t2 = 7.3867... is provably beyond the top
    // ring. The top cap plane at h_c = -0.5 cuts the tube at
    // 3 +- sqrt(0.75): the inner circle (2.1339...) sits inside the
    // r_top=3 disk, the outer one is beyond it. Sorted by torus-axial
    // height: the side circle (z = -(1 + 3 sqrt(39))/20), then the cap
    // circle (z = -1/2).
    let cone = brep_core::analytic::frustum(1., 3., 6.).unwrap();
    let torus = translated(&brep_core::analytic::torus(3., 1.).unwrap(), [0., 0., 6.5]);
    let report = intersect_cone_torus(&cone, &torus, Options::default()).unwrap();
    let report = only_circles(&report, 2);
    let root39 = 39_f64.sqrt();
    let t1 = (129. - 3. * root39) / 20.;
    let rho1 = 1. + t1 / 3.;
    let (curve, center, radius, _, cone_uv, torus_uv, sampled) =
        circle_of(&report.components[0]);
    assert!((radius - rho1).abs() <= 1e-12, "{radius}");
    assert!(
        sub(center, [0., 0., t1]).iter().all(|x| x.abs() <= 1e-12),
        "{center:?}"
    );
    let mut worst = 0_f64;
    for i in 0..16 {
        let p = curve.evaluate(i as f64 / 4.).unwrap().point;
        let p = [p[0], p[1], p[2]];
        worst = worst.max(side_residual_z(p, 1., 1. / 3., 0.)).max(implicit(
            sub(p, [0., 0., 6.5]),
            3.,
            1.,
        ));
    }
    assert!(worst <= 1e-10, "{worst}");
    assert!(sampled <= 1e-10, "{sampled}");
    assert_eq!(cone_uv.len(), 4);
    let v0 = t1 / 6.;
    for lift in cone_uv {
        assert!((lift.arcs[0].control_points[0][1] - v0).abs() <= 1e-12);
    }
    assert_eq!(torus_uv.len(), 4);
    let uv_worst = uv_samples(
        &cone,
        &torus,
        cone_uv,
        torus_uv,
        |p| side_residual_z(p, 1., 1. / 3., 0.),
        |p| implicit(sub(p, [0., 0., 6.5]), 3., 1.),
    );
    assert!(uv_worst <= 1e-9, "{uv_worst}");
    // Cap circle: radius 3 - sqrt(0.75) in the top cap plane z=6, the
    // exact four-arc UV circle of radius rho/(2 r_top) about [1/2, 1/2].
    let inner = 3. - 0.75_f64.sqrt();
    let (curve, center, radius, _, cone_uv, torus_uv, sampled) =
        circle_of(&report.components[1]);
    assert!((radius - inner).abs() <= 1e-12, "{radius}");
    assert!(
        sub(center, [0., 0., 6.]).iter().all(|x| x.abs() <= 1e-12),
        "{center:?}"
    );
    let mut worst = 0_f64;
    for i in 0..16 {
        let p = curve.evaluate(i as f64 / 4.).unwrap().point;
        let p = [p[0], p[1], p[2]];
        worst = worst
            .max((p[2] - 6.).abs())
            .max(implicit(sub(p, [0., 0., 6.5]), 3., 1.));
    }
    assert!(worst <= 1e-10, "{worst}");
    assert!(sampled <= 1e-10, "{sampled}");
    assert_eq!(cone_uv.len(), 1);
    let lift = &cone_uv[0];
    assert_eq!(lift.arcs.len(), 4);
    for arc in &lift.arcs {
        assert_eq!(arc.degree, 2);
        assert_eq!(arc.weights, vec![1., ARC_WEIGHT, 1.]);
        let uvr = (arc.control_points[0][0] - 0.5).hypot(arc.control_points[0][1] - 0.5);
        assert!((uvr - inner / 6.).abs() <= 1e-12, "{uvr}");
    }
    assert_eq!(torus_uv.len(), 4);
    let uv_worst = uv_samples(
        &cone,
        &torus,
        cone_uv,
        torus_uv,
        |p| (p[2] - 6.).abs(),
        |p| implicit(sub(p, [0., 0., 6.5]), 3., 1.),
    );
    assert!(uv_worst <= 1e-9, "{uv_worst}");
}

#[test]
fn cap_planes_cut_the_tube_circle_pair() {
    // Frustum r 2 -> 4 over z 0..6 at the origin; torus R=3, r=1 at
    // z=6.5: the side line misses the meridian circle (distance
    // 3.5/sqrt(10) = 1.1068... > 1), and the top cap plane at h_c = -0.5
    // cuts the two circles 3 +- sqrt(0.75), both inside the r_top=4
    // disk. The bottom cap at h_c = -6.5 clears the tube.
    let cone = brep_core::analytic::frustum(2., 4., 6.).unwrap();
    let torus = translated(&brep_core::analytic::torus(3., 1.).unwrap(), [0., 0., 6.5]);
    let report = intersect_cone_torus(&cone, &torus, Options::default()).unwrap();
    let report = only_circles(&report, 2);
    let root = 0.75_f64.sqrt();
    for (component, rho) in report.components.iter().zip([3. - root, 3. + root]) {
        let (curve, center, radius, _, cone_uv, torus_uv, sampled) = circle_of(component);
        assert!((radius - rho).abs() <= 1e-12, "{radius} vs {rho}");
        assert!(
            sub(center, [0., 0., 6.]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        let mut worst = 0_f64;
        for i in 0..16 {
            let p = curve.evaluate(i as f64 / 4.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            worst = worst
                .max((p[2] - 6.).abs())
                .max(implicit(sub(p, [0., 0., 6.5]), 3., 1.));
        }
        assert!(worst <= 1e-10, "{worst}");
        assert!(sampled <= 1e-10, "{sampled}");
        assert_eq!(cone_uv.len(), 1);
        assert_eq!(cone_uv[0].arcs.len(), 4);
        // Torus lift at the exact profile angle atan2(z, rho - R).
        assert_eq!(torus_uv.len(), 4);
        let phi = (-0.5_f64).atan2(rho - 3.).rem_euclid(TAU);
        let quadrant = (phi / std::f64::consts::FRAC_PI_2).floor() as usize;
        let vv = arc_parameter(phi - quadrant as f64 * std::f64::consts::FRAC_PI_2);
        for lift in torus_uv {
            let arc = &lift.arcs[0];
            assert!((arc.control_points[0][1] - vv).abs() <= 1e-12, "{arc:?}");
        }
        let uv_worst = uv_samples(
            &cone,
            &torus,
            cone_uv,
            torus_uv,
            |p| (p[2] - 6.).abs(),
            |p| implicit(sub(p, [0., 0., 6.5]), 3., 1.),
        );
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }
}

#[test]
fn anti_axial_orientation_yields_the_exact_circles() {
    // The two-circle configuration with the cone flipped (rotation pi
    // about X, then z -> -z): the cone axis is -z, its own bottom ring
    // (r=1) at z=3, rho = 1 + (3 - z)/3 for z in -3..3. Meridian roots
    // t = 3 and 3.6 in the cone's own axial parameter map to the circles
    // (rho=2, z=0) and (rho=2.2, z=-0.6) — sorted z=-0.6 first.
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let cone = flipped(
        &translated(
            &brep_core::analytic::frustum(1., 3., 6.).unwrap(),
            [0., 0., -3.],
        ),
        0.,
    );
    let report = intersect_cone_torus(&cone, &torus, Options::default()).unwrap();
    let report = only_circles(&report, 2);
    for (component, (rho, z)) in report.components.iter().zip([(2.2, -0.6), (2., 0.)]) {
        let (curve, center, radius, normal, cone_uv, torus_uv, sampled) = circle_of(component);
        assert!((radius - rho).abs() <= 1e-12, "{radius} vs {rho}");
        assert!(
            sub(center, [0., 0., z]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        assert!(normal[2] <= -1. + 1e-12, "{normal:?}");
        let mut worst = 0_f64;
        for i in 0..16 {
            let p = curve.evaluate(i as f64 / 4.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            worst = worst
                .max((radial_z(p) - (1. + (3. - p[2]) / 3.)).abs())
                .max(implicit(p, 3., 1.));
        }
        assert!(worst <= 1e-10, "{worst}");
        assert!(sampled <= 1e-10, "{sampled}");
        assert_eq!(cone_uv.len(), 4);
        assert_eq!(torus_uv.len(), 4);
        let uv_worst = uv_samples(
            &cone,
            &torus,
            cone_uv,
            torus_uv,
            |p| (radial_z(p) - (1. + (3. - p[2]) / 3.)).abs(),
            |p| implicit(p, 3., 1.),
        );
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }
}

#[test]
fn meridian_tangency_stays_a_tangency_region() {
    // Frustum r 1 -> 3 over z 0..6 at the origin; torus at z = 6 - sqrt(10):
    // the side line is tangent to the meridian circle (distance
    // |2 + z_b/3| * 3/sqrt(10) == 1 with foot t = (20/3 - sqrt(10)) 0.9
    // inside the height range) — a double root, never a guessed circle.
    let tangent_z = 6. - 10_f64.sqrt();
    for dz in [0., 2e-15, -2e-15] {
        let cone = brep_core::analytic::frustum(1., 3., 6.).unwrap();
        let torus = translated(
            &brep_core::analytic::torus(3., 1.).unwrap(),
            [0., 0., tangent_z + dz],
        );
        let report = intersect_cone_torus(&cone, &torus, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
        assert!(!report.permits_topology_change());
    }
    // Just clear of the band on the outside: provable miss, resolved.
    let cone = brep_core::analytic::frustum(1., 3., 6.).unwrap();
    let clear = translated(
        &brep_core::analytic::torus(3., 1.).unwrap(),
        [0., 0., tangent_z - 1e-9],
    );
    let report = intersect_cone_torus(&cone, &clear, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    // Just across: two small transverse side circles around the foot.
    let across = translated(
        &brep_core::analytic::torus(3., 1.).unwrap(),
        [0., 0., tangent_z + 1e-9],
    );
    let report = intersect_cone_torus(&cone, &across, Options::default()).unwrap();
    only_circles(&report, 2);
}

#[test]
fn rim_contact_stays_unresolved() {
    // Frustum r 2 -> 4 over z 0..6 at the origin against the torus
    // R=3, r=1 at the origin: the meridian roots are t=0 and t=0.6 —
    // the t=0 root sits on the bottom ring plane (rim contact), and the
    // bottom rim circle (rho=2, z=0) lies exactly on the torus. The
    // t=0.6 root is a clean transverse side circle and still reports.
    let cone = brep_core::analytic::frustum(2., 4., 6.).unwrap();
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let report = intersect_cone_torus(&cone, &torus, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::TangencyOrMultipleRoot
    );
    let report = only_circles_len1(&report);
    let (_, center, radius, _, cone_uv, _, sampled) = circle_of(&report.components[0]);
    assert!((radius - 2.2).abs() <= 1e-12, "{radius}");
    assert!(
        sub(center, [0., 0., 0.6]).iter().all(|x| x.abs() <= 1e-12),
        "{center:?}"
    );
    assert_eq!(cone_uv.len(), 4);
    assert!(sampled <= 1e-10, "{sampled}");
}
fn only_circles_len1(report: &Report<ConeTorusComponent>) -> &Report<ConeTorusComponent> {
    assert_eq!(report.components.len(), 1, "{report:?}");
    assert!(!report.permits_topology_change());
    report
}

#[test]
fn cap_plane_tube_tangency_stays_unresolved() {
    // Frustum r 1 -> 3 over z 0..6 with its bottom ring plane at z=1:
    // the bottom cap plane sits at |h_c| == r — tangent to the tube,
    // never a guessed circle. The side line misses the meridian circle
    // (distance 7/sqrt(10) > 1).
    let cone = translated(&brep_core::analytic::frustum(1., 3., 6.).unwrap(), [0., 0., 1.]);
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let report = intersect_cone_torus(&cone, &torus, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::TangencyOrMultipleRoot
    );
    // Just clear of the band on the outside: the bottom cap plane at
    // h_c = 1 + 1e-9 clears the tube, resolved empty.
    let clear = translated(
        &brep_core::analytic::frustum(1., 3., 6.).unwrap(),
        [0., 0., 1. + 1e-9],
    );
    let report = intersect_cone_torus(&clear, &torus, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    // Just across: the bottom cap at h_c = 1 - 1e-9 crosses the tube,
    // but both circle radii 3 +- sqrt(1 - h_c^2) (about 3 +- 4.5e-5) lie
    // provably outside the r_bottom=1 disk — resolved empty.
    let across = translated(
        &brep_core::analytic::frustum(1., 3., 6.).unwrap(),
        [0., 0., 1. - 1e-9],
    );
    let report = intersect_cone_torus(&across, &torus, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
}

#[test]
fn zero_circle_configurations_resolve_empty() {
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    // Frustum r 1 -> 3 over z 0..6 with its bottom ring at the torus
    // center plane: the side line misses the meridian circle (distance
    // 6/sqrt(10) > 1), the bottom cap circles (radii 2 and 4) lie
    // outside the r_bottom=1 disk, the top cap clears the tube.
    let flat = brep_core::analytic::frustum(1., 3., 6.).unwrap();
    // Small frustum around the center plane in the torus hole: the side
    // line misses the meridian circle, both bottom cap circle radii lie
    // outside the r_bottom=0.5 disk, the top cap clears the tube.
    let thin = translated(
        &brep_core::analytic::frustum(0.5, 1.5, 2.).unwrap(),
        [0., 0., -0.5],
    );
    // Frustum far beyond the tube.
    let far = translated(
        &brep_core::analytic::frustum(1., 3., 6.).unwrap(),
        [0., 0., 100.],
    );
    for cone in [&flat, &thin, &far] {
        let report = intersect_cone_torus(cone, &torus, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
    }
}

#[test]
fn near_coaxial_bands_stay_unresolved_not_forced() {
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let base = translated(
        &brep_core::analytic::frustum(1., 3., 6.).unwrap(),
        [0., 0., -3.],
    );
    // Center-line offset inside the recognition band: near_coincidence.
    let near = translated(&base, [1e-10, 0., 0.]);
    let report = intersect_cone_torus(&near, &torus, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::NearCoincidence
    );
    // Recognition-scale tilt of the cone axis: near_coincidence.
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
    let report = intersect_cone_torus(&tilted, &torus, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::NearCoincidence
    );
}

#[test]
fn clearly_off_axis_or_tilted_pairs_are_unsupported_regions() {
    let torus = brep_core::analytic::torus(3., 1.).unwrap();
    let base = translated(
        &brep_core::analytic::frustum(1., 3., 6.).unwrap(),
        [0., 0., -3.],
    );
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
    for cone in [&off, &tilted] {
        let report = intersect_cone_torus(cone, &torus, Options::default()).unwrap();
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
    let cone = translated(
        &brep_core::analytic::frustum(1., 3., 6.).unwrap(),
        [0., 0., -3.],
    );
    // Spheres, cuboids, cylinders (an equal-radius frustum is refused as
    // a cone operand), tori-as-first-operand and swapped operand order
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
            brep_core::analytic::cylinder(2.2, 8.).unwrap(),
            brep_core::analytic::torus(3., 1.).unwrap(),
        ),
        (
            brep_core::analytic::torus(2., 1.).unwrap(),
            brep_core::analytic::torus(3., 1.).unwrap(),
        ),
        (
            brep_core::analytic::torus(3., 1.).unwrap(),
            brep_core::analytic::frustum(1., 3., 6.).unwrap(),
        ),
        (
            brep_core::analytic::frustum(1., 3., 6.).unwrap(),
            brep_core::analytic::sphere(2.).unwrap(),
        ),
    ] {
        let report = intersect_cone_torus(&a, &b, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(report.unresolved.len(), 1);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::UnsupportedSurface
        );
    }
    // A canonical pair still resolves.
    let report = intersect_cone_torus(&cone, &torus, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    assert_eq!(report.components.len(), 2);
    // A structurally perturbed cone (one moved control point) breaks the
    // pcurve/edge agreement and fails validation as a hard error — never
    // a numerical fallback.
    let mut perturbed = cone.clone();
    perturbed.faces[0].surface.control_points[1][1][0] += 1e-6;
    perturbed.rebuild_topology_ids();
    assert!(intersect_cone_torus(&perturbed, &torus, Options::default()).is_err());
    // A structurally perturbed torus is an explicit unsupported region.
    let mut perturbed_torus = torus.clone();
    perturbed_torus.faces[0].surface.control_points[1][1][0] += 1e-6;
    perturbed_torus.rebuild_topology_ids();
    let report = intersect_cone_torus(&cone, &perturbed_torus, Options::default()).unwrap();
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
    let torus = rotated_translated(&brep_core::analytic::torus(3., 1.).unwrap(), angle, offset);
    let cone = rotated_translated(
        &translated(
            &brep_core::analytic::frustum(1., 3., 6.).unwrap(),
            [0., 0., -3.],
        ),
        angle,
        offset,
    );
    let report = intersect_cone_torus(&cone, &torus, Options::default()).unwrap();
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
    let base = placed([0., 0., -3.]);
    for (component, (rho, z)) in report.components.iter().zip([(2., 0.), (2.2, 0.6)]) {
        let (curve, center, radius, normal, cone_uv, torus_uv, sampled) = circle_of(component);
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
        let placed_check = |p: [f64; 3]| {
            let rel = sub(p, placed([0., 0., 0.]));
            let a = dot(rel, axis);
            let perp = sub(rel, axis.map(|x| x * a));
            let rho_p = perp[0].hypot(perp[1]).hypot(perp[2]);
            let base_t = rho_p * rho_p + a * a + 9. - 1.;
            let cone_rel = sub(p, base);
            let cone_a = dot(cone_rel, axis);
            let cone_perp = sub(cone_rel, axis.map(|x| x * cone_a));
            let cone_rho = cone_perp[0].hypot(cone_perp[1]).hypot(cone_perp[2]);
            (
                (cone_rho - (1. + cone_a / 3.)).abs(),
                (base_t * base_t - 36. * rho_p * rho_p).abs(),
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
        assert_eq!(cone_uv.len(), 4);
        assert_eq!(torus_uv.len(), 4);
        let uv_worst = uv_samples(
            &cone,
            &torus,
            cone_uv,
            torus_uv,
            |p| placed_check(p).0,
            |p| placed_check(p).1,
        );
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }
}
