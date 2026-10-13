use super::super::plane_sphere::plane_patch;
use super::super::test_utils::rotated_translated;
use super::*;



/// Worst residual of the UV lifts evaluated through their own surfaces
/// against the cylinder side and plane equations.
fn lift_worst(
    plane_model: &Model,
    cylinder_model: &Model,
    plane_uv: &[PlanePatchCurve],
    cylinder_uv: &[CylinderPatchCurve],
    cylinder_def: ([f64; 3], [f64; 3], f64),
    plane_def: ([f64; 3], [f64; 3]),
) -> f64 {
    let check = |p: [f64; 3]| {
        let rel = sub(p, cylinder_def.0);
        let a = dot(rel, cylinder_def.1);
        let perp = sub(rel, cylinder_def.1.map(|x| x * a));
        (perp[0].hypot(perp[1]).hypot(perp[2]) - cylinder_def.2)
            .abs()
            .max(dot(plane_def.0, sub(p, plane_def.1)).abs())
    };
    let mut worst = 0_f64;
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
    for lift in cylinder_uv {
        let surface = &cylinder_model.faces[lift.patch].surface;
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
    worst
}

#[test]
fn perpendicular_full_circle_has_exact_lifts() {
    // Plane z = 4, patch [-3,3]^2; cylinder r = 2, z in [0, 8]: the exact
    // circle of radius 2 at mid-height, fully inside the patch.
    let plane = plane_patch([-3., -3., 4.], [6., 0., 0.], [0., 6., 0.]);
    let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
    let report = intersect_plane_cylinder(&plane, &cylinder, Options::default()).unwrap();
    assert!(report.unresolved.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    assert!(!report.permits_topology_change());
    let [
        PlaneCylinderComponent::Circle {
            curve,
            center,
            radius,
            normal,
            full,
            plane_uv,
            cylinder_uv,
            max_sample_residual,
        },
    ] = &report.components[..]
    else {
        panic!("expected one circle component: {report:?}")
    };
    assert!(*full);
    assert!((*radius - 2.).abs() <= 1e-12, "{radius}");
    assert!(
        sub(*center, [0., 0., 4.]).iter().all(|x| x.abs() <= 1e-12),
        "{center:?}"
    );
    assert_eq!(*normal, [0., 0., 1.]);
    // Four exact 90-degree arcs with the unit-circle weights.
    assert_eq!(
        curve.knots,
        vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.]
    );
    assert_eq!(curve.weights.len(), 9);
    assert!((curve.weights[1] - std::f64::consts::FRAC_1_SQRT_2).abs() <= 1e-15);
    assert!(*max_sample_residual <= 1e-12, "{max_sample_residual}");
    // Plane UV: the exact ellipse (a circle in an isotropic patch frame).
    assert_eq!(plane_uv[0].arcs.len(), 4);
    // Cylinder UV: iso-v rulings at mid-height on all four side patches.
    assert_eq!(cylinder_uv.len(), 4);
    for lift in cylinder_uv {
        assert_eq!(lift.arcs.len(), 1);
        let arc = &lift.arcs[0];
        assert_eq!(arc.degree, 1);
        assert_eq!(arc.control_points[0], vec![0., 0.5]);
        assert_eq!(arc.control_points[1], vec![1., 0.5]);
    }
    let worst = lift_worst(
        &plane,
        &cylinder,
        plane_uv,
        cylinder_uv,
        ([0., 0., 4.], [0., 0., 1.], 2.),
        ([0., 0., 1.], [-3., -3., 4.]),
    );
    assert!(worst <= 1e-9, "{worst}");
}

#[test]
fn perpendicular_clipped_circle_has_exact_trims() {
    // Plane z = 4, patch x in [0, 3], y in [-3, 3]: the circle of radius 2
    // clipped to its x >= 0 half, swept [3pi/2, 5pi/2], endpoints
    // (0, -+2, 4).
    let plane = plane_patch([0., -3., 4.], [3., 0., 0.], [0., 6., 0.]);
    let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
    let report = intersect_plane_cylinder(&plane, &cylinder, Options::default()).unwrap();
    assert!(report.unresolved.is_empty(), "{report:?}");
    let [
        PlaneCylinderComponent::Circle {
            curve,
            center,
            full,
            plane_uv,
            cylinder_uv,
            max_sample_residual,
            ..
        },
    ] = &report.components[..]
    else {
        panic!("expected one clipped circle: {report:?}")
    };
    assert!(!*full);
    assert!(
        sub(*center, [0., 0., 4.]).iter().all(|x| x.abs() <= 1e-12),
        "{center:?}"
    );
    assert_eq!(curve.knots, vec![0., 0., 0., 1., 1., 2., 2., 2.]);
    let start = point3(&curve.evaluate(0.).unwrap().point);
    let end = point3(&curve.evaluate(2.).unwrap().point);
    assert!(
        start[0].abs() <= 1e-12 && (start[1] + 2.).abs() <= 1e-12,
        "{start:?}"
    );
    assert!((start[2] - 4.).abs() <= 1e-12);
    assert!(
        end[0].abs() <= 1e-12 && (end[1] - 2.).abs() <= 1e-12,
        "{end:?}"
    );
    assert!((end[2] - 4.).abs() <= 1e-12);
    assert!(*max_sample_residual <= 1e-12, "{max_sample_residual}");
    // Plane UV: two arcs meeting at the phi = 0 seam (2/3, 1/2); the free
    // endpoints sit exactly on u = 0 at v = 1/6 and v = 5/6.
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
    assert_eq!(edge_v.len(), 2, "{edge_v:?}");
    assert!((edge_v[0] - 1. / 6.).abs() <= 1e-12, "{edge_v:?}");
    assert!((edge_v[1] - 5. / 6.).abs() <= 1e-12, "{edge_v:?}");
    assert_eq!(seam.len(), 2, "{seam:?}");
    for p in &seam {
        assert!((p[0] - 2. / 3.).abs() <= 1e-12, "{p:?}");
        assert!((p[1] - 0.5).abs() <= 1e-12, "{p:?}");
    }
    // Cylinder UV: iso-v sub-u arcs at mid-height on the two side patches
    // the half circle sweeps (quadrants 3 and 0).
    assert_eq!(cylinder_uv.len(), 2, "{cylinder_uv:?}");
    for lift in cylinder_uv {
        for arc in &lift.arcs {
            assert_eq!(arc.degree, 1);
            assert!((arc.control_points[0][1] - 0.5).abs() <= 1e-12);
            assert!((arc.control_points[1][1] - 0.5).abs() <= 1e-12);
        }
    }
    let worst = lift_worst(
        &plane,
        &cylinder,
        plane_uv,
        cylinder_uv,
        ([0., 0., 4.], [0., 0., 1.], 2.),
        ([0., 0., 1.], [0., -3., 4.]),
    );
    assert!(worst <= 1e-9, "{worst}");
}

#[test]
fn cap_plane_coincidence_and_beyond_classification() {
    let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
    let at = |z: f64| plane_patch([-3., -3., z], [6., 0., 0.], [0., 6., 0.]);
    // Plane coincident with the top cap plane: a coincident cap/rim
    // region, never a guessed circle.
    let report = intersect_plane_cylinder(&at(8.), &cylinder, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::CoincidentTrim
    );
    // Provably beyond the cap: empty and resolved.
    for z in [9., 8. + 1e-9] {
        let report = intersect_plane_cylinder(&at(z), &cylinder, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{z} {report:?}"
        );
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
    }
    // Just clear of the cap band below: a full circle near the rim.
    let report =
        intersect_plane_cylinder(&at(8. - 1e-9), &cylinder, Options::default()).unwrap();
    let [PlaneCylinderComponent::Circle { center, full, .. }] = &report.components[..] else {
        panic!("expected one circle: {report:?}")
    };
    assert!(*full);
    assert!((center[2] - (8. - 1e-9)).abs() <= 1e-12, "{center:?}");
}

#[test]
fn parallel_plane_yields_two_exact_rulings() {
    // Plane x = 1, patch y in [-3, 3], z in [0, 10]: the two exact
    // rulings at (1, +-sqrt(3), z), z in [0, 8] (clipped by the caps).
    let plane = plane_patch([1., -3., 0.], [0., 6., 0.], [0., 0., 10.]);
    let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
    let report = intersect_plane_cylinder(&plane, &cylinder, Options::default()).unwrap();
    assert!(report.unresolved.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    assert_eq!(report.components.len(), 2, "{report:?}");
    let h = 3_f64.sqrt();
    let mut seen: Vec<f64> = Vec::new();
    for component in &report.components {
        let PlaneCylinderComponent::Line {
            curve,
            start,
            end,
            direction,
            contact,
            plane_uv,
            cylinder_uv,
            max_sample_residual,
        } = component
        else {
            panic!("expected line components: {report:?}")
        };
        assert_eq!(*direction, [0., 0., 1.]);
        assert_eq!(*contact, Contact::Boundary);
        assert!(
            (start[0] - 1.).abs() <= 1e-12 && start[2].abs() <= 1e-12,
            "{start:?}"
        );
        assert!(
            (end[0] - 1.).abs() <= 1e-12 && (end[2] - 8.).abs() <= 1e-12,
            "{end:?}"
        );
        assert!((start[1].abs() - h).abs() <= 1e-12, "{start:?}");
        assert_eq!(start[1], end[1]);
        seen.push(start[1]);
        assert_eq!(curve.degree, 1);
        assert_eq!(curve.knots, vec![0., 0., 1., 1.]);
        assert!(*max_sample_residual <= 1e-12, "{max_sample_residual}");
        // Plane UV: a degree-1 segment at constant u, v from 0 to 0.8.
        assert_eq!(plane_uv[0].arcs.len(), 1);
        let arc = &plane_uv[0].arcs[0];
        assert_eq!(arc.degree, 1);
        let u_const = (3. + start[1]) / 6.;
        assert!(
            (arc.control_points[0][0] - u_const).abs() <= 1e-12,
            "{arc:?}"
        );
        assert!(
            (arc.control_points[1][0] - u_const).abs() <= 1e-12,
            "{arc:?}"
        );
        assert!(arc.control_points[0][1].abs() <= 1e-12, "{arc:?}");
        assert!((arc.control_points[1][1] - 0.8).abs() <= 1e-12, "{arc:?}");
        assert!(!cylinder_uv.is_empty());
        let worst = lift_worst(
            &plane,
            &cylinder,
            plane_uv,
            cylinder_uv,
            ([0., 0., 4.], [0., 0., 1.], 2.),
            ([1., 0., 0.], [1., -3., 0.]),
        );
        assert!(worst <= 1e-9, "{worst}");
    }
    seen.sort_by(f64::total_cmp);
    assert!(seen[0] < 0. && seen[1] > 0., "{seen:?}");
}

#[test]
fn parallel_miss_tangency_and_band_classification() {
    let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
    let at = |x: f64| plane_patch([x, -3., 0.], [0., 6., 0.], [0., 0., 10.]);
    // Miss: d = 3 > r, empty and resolved.
    let report = intersect_plane_cylinder(&at(3.), &cylinder, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    // Tangency and within the band around it: never a guessed line.
    for x in [2., 2. - 3e-14] {
        let report = intersect_plane_cylinder(&at(x), &cylinder, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{x} {report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
    }
    // Just clear of the band outside: empty resolved.
    let report =
        intersect_plane_cylinder(&at(2. + 1e-9), &cylinder, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    // Just clear inside: two rulings near the tangent line.
    let report =
        intersect_plane_cylinder(&at(2. - 1e-9), &cylinder, Options::default()).unwrap();
    assert_eq!(report.components.len(), 2, "{report:?}");
    let h = (4_f64 - (2. - 1e-9) * (2. - 1e-9)).sqrt();
    for component in &report.components {
        let PlaneCylinderComponent::Line { start, .. } = component else {
            panic!("expected lines: {report:?}")
        };
        // h^2 = r^2 - d^2 loses digits to cancellation: compare relative.
        assert!((start[1].abs() - h).abs() <= 1e-6 * h, "{start:?} vs {h}");
    }
}

#[test]
fn parallel_boundary_line_is_a_tangency_region() {
    // Plane x = 1 with the patch edge exactly on y = sqrt(3): one ruling
    // runs along the patch boundary within the band (tangency region),
    // the other is resolved with exact trims.
    let plane = plane_patch([1., -3., 0.], [0., 3. + 3_f64.sqrt(), 0.], [0., 0., 10.]);
    let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
    let report = intersect_plane_cylinder(&plane, &cylinder, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(report.unresolved.len(), 1, "{report:?}");
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::TangencyOrMultipleRoot
    );
    let [PlaneCylinderComponent::Line { start, end, .. }] = &report.components[..] else {
        panic!("expected one resolved line: {report:?}")
    };
    let h = 3_f64.sqrt();
    assert!(
        (start[0] - 1.).abs() <= 1e-12 && (start[1] + h).abs() <= 1e-12,
        "{start:?}"
    );
    assert!(
        start[2].abs() <= 1e-12 && (end[2] - 8.).abs() <= 1e-12,
        "{end:?}"
    );
}

/// Oblique fixture: plane through the cylinder center (0, 0, 4) with
/// normal (0, -sin(beta), cos(beta)), u along x, v in the yz-plane.
fn oblique_plane(beta: f64, half_u: f64, half_v: f64) -> Model {
    let (s, c) = beta.sin_cos();
    plane_patch(
        [-half_u, -half_v * c, 4. - half_v * s],
        [2. * half_u, 0., 0.],
        [0., 2. * half_v * c, 2. * half_v * s],
    )
}

#[test]
fn oblique_full_ellipse_matches_the_exact_oracle() {
    // beta = 30 degrees: semi-major 4/sqrt(3) along (0, cos, sin),
    // semi-minor 2 along (-1, 0, 0); the ellipse clears both caps and
    // the patch rectangle.
    let beta = std::f64::consts::PI / 6.;
    let (s, c) = beta.sin_cos();
    let plane = oblique_plane(beta, 6., 8.);
    let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
    let report = intersect_plane_cylinder(&plane, &cylinder, Options::default()).unwrap();
    assert!(report.unresolved.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    let [
        PlaneCylinderComponent::Ellipse {
            curve,
            center,
            semi_major,
            semi_minor,
            major,
            minor,
            normal,
            full,
            plane_uv,
            cylinder_uv,
            max_sample_residual,
        },
    ] = &report.components[..]
    else {
        panic!("expected one ellipse: {report:?}")
    };
    assert!(*full);
    assert!(cylinder_uv.is_none(), "cylinder side lift is out of scope");
    assert!((semi_major - 2. / c).abs() <= 1e-12, "{semi_major}");
    assert!((semi_minor - 2.).abs() <= 1e-12, "{semi_minor}");
    assert!(
        sub(*center, [0., 0., 4.]).iter().all(|x| x.abs() <= 1e-12),
        "{center:?}"
    );
    assert!(
        sub(*major, [0., c, s]).iter().all(|x| x.abs() <= 1e-12),
        "{major:?}"
    );
    assert!(
        sub(*minor, [-1., 0., 0.]).iter().all(|x| x.abs() <= 1e-12),
        "{minor:?}"
    );
    assert!(
        sub(*normal, [0., -s, c]).iter().all(|x| x.abs() <= 1e-12),
        "{normal:?}"
    );
    // phi = 0 is the major-axis endpoint (0, 2, 4 + 2/sqrt(3)).
    let p0 = point3(&curve.evaluate(0.).unwrap().point);
    let oracle = [0., 2., 4. + 2. / 3_f64.sqrt()];
    assert!(sub(p0, oracle).iter().all(|x| x.abs() <= 1e-12), "{p0:?}");
    assert!(*max_sample_residual <= 1e-12, "{max_sample_residual}");
    let worst = lift_worst(
        &plane,
        &cylinder,
        plane_uv,
        &[],
        ([0., 0., 4.], [0., 0., 1.], 2.),
        ([0., -s, c], [-6., -8. * c, 4. - 8. * s]),
    );
    assert!(worst <= 1e-9, "{worst}");
}

#[test]
fn oblique_clipped_ellipse_has_exact_cap_trims() {
    // beta = 75 degrees: the axial semi-extent 2*tan(beta) exceeds the
    // half height, so the ellipse is clipped by both cap planes into two
    // arcs whose endpoints sit exactly on z = 0 and z = 8.
    let beta = 5. * std::f64::consts::PI / 12.;
    let plane = oblique_plane(beta, 6., 8.);
    let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
    let report = intersect_plane_cylinder(&plane, &cylinder, Options::default()).unwrap();
    assert!(report.unresolved.is_empty(), "{report:?}");
    assert_eq!(report.components.len(), 2, "{report:?}");
    let mut z_levels: Vec<f64> = Vec::new();
    for component in &report.components {
        let PlaneCylinderComponent::Ellipse {
            curve,
            full,
            cylinder_uv,
            max_sample_residual,
            ..
        } = component
        else {
            panic!("expected ellipse arcs: {report:?}")
        };
        assert!(!*full);
        assert!(cylinder_uv.is_none());
        assert!(*max_sample_residual <= 1e-12, "{max_sample_residual}");
        let [lo, hi] = curve.domain();
        let start = point3(&curve.evaluate(lo).unwrap().point);
        let end = point3(&curve.evaluate(hi).unwrap().point);
        for p in [start, end] {
            // Every trim endpoint lies on a cap plane.
            assert!(p[2].abs() <= 1e-12 || (p[2] - 8.).abs() <= 1e-12, "{p:?}");
            z_levels.push(p[2]);
            // ... and on the cylinder side.
            assert!((p[0].hypot(p[1]) - 2.).abs() <= 1e-12, "{p:?}");
        }
    }
    // One arc touches the top cap at both ends, the other the bottom.
    let tops = z_levels.iter().filter(|&&z| z > 4.).count();
    assert_eq!(tops, 2, "{z_levels:?}");
}

#[test]
fn oblique_cap_tangency_stays_unresolved() {
    // tan(beta) = 2: the axial semi-extent equals the half height, so the
    // ellipse touches both cap planes exactly — a tangency region, never
    // a guessed ellipse.
    let beta = 2_f64.atan();
    let plane = oblique_plane(beta, 6., 8.);
    let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
    let report = intersect_plane_cylinder(&plane, &cylinder, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::TangencyOrMultipleRoot
    );
}

#[test]
fn near_coincident_tilts_are_never_forced() {
    let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
    // Recognition-scale tilt off perpendicular (about X by 1e-6).
    let a = 1e-6_f64;
    let (sa, ca) = a.sin_cos();
    let near_perp = plane_patch(
        [-3., -3. * ca, 4. - 3. * sa],
        [6., 0., 0.],
        [0., 6. * ca, 6. * sa],
    );
    let report = intersect_plane_cylinder(&near_perp, &cylinder, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::NearCoincidence
    );
    // Recognition-scale tilt off parallel: normal (cos b, 0, sin b) with
    // sin(b) between the rounding snap and the recognition scale.
    let b = 5e-10_f64;
    let (sb, cb) = b.sin_cos();
    let near_par = plane_patch(
        [cb, -3., sb - 5. * sb],
        [0., 6., 0.],
        [-10. * sb, 0., 10. * cb],
    );
    let report = intersect_plane_cylinder(&near_par, &cylinder, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::NearCoincidence
    );
}

#[test]
fn noncanonical_operands_are_explicit_unsupported_regions() {
    let plane = plane_patch([-3., -3., 4.], [6., 0., 0.], [0., 6., 0.]);
    let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
    for (a, b) in [
        (
            plane.clone(),
            brep_core::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap(),
        ),
        (plane.clone(), brep_core::analytic::sphere(2.).unwrap()),
        // Operand order is fixed: the cylinder as plane operand refuses.
        (cylinder.clone(), plane.clone()),
        (
            brep_core::cuboid([0., 0., 0.], [4., 4., 1.]).unwrap(),
            cylinder.clone(),
        ),
    ] {
        let report = intersect_plane_cylinder(&a, &b, Options::default()).unwrap();
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
    let skewed = plane_patch([-3., -3., 4.], [6., 0., 0.], [3., 6., 0.]);
    let report = intersect_plane_cylinder(&skewed, &cylinder, Options::default()).unwrap();
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::UnsupportedSurface
    );
    // A structurally perturbed patch (non-unit weight) fails validation.
    let mut perturbed = plane.clone();
    perturbed.faces[0].surface.weights[0][0] = 2.;
    assert!(intersect_plane_cylinder(&perturbed, &cylinder, Options::default()).is_err());
    // A valid canonical pair still resolves.
    let report = intersect_plane_cylinder(&plane, &cylinder, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    assert!(!report.components.is_empty(), "{report:?}");
}

#[test]
fn rigid_placement_keeps_the_exact_ellipse() {
    let angle = 0.5;
    let offset = [0.3, -0.2, 1.1];
    let beta = std::f64::consts::PI / 6.;
    let (s, c) = beta.sin_cos();
    let plane = rotated_translated(&oblique_plane(beta, 6., 8.), angle, offset);
    let cylinder =
        rotated_translated(&brep_core::analytic::cylinder(2., 8.).unwrap(), angle, offset);
    let report = intersect_plane_cylinder(&plane, &cylinder, Options::default()).unwrap();
    assert!(report.unresolved.is_empty(), "{report:?}");
    let [
        PlaneCylinderComponent::Ellipse {
            curve,
            center,
            semi_major,
            semi_minor,
            major,
            minor,
            normal,
            full,
            plane_uv,
            cylinder_uv,
            max_sample_residual,
        },
    ] = &report.components[..]
    else {
        panic!("expected one ellipse: {report:?}")
    };
    assert!(*full);
    assert!(cylinder_uv.is_none());
    let (sa, ca) = angle.sin_cos();
    let placed = |p: [f64; 3]| {
        [
            p[0] + offset[0],
            ca * p[1] - sa * p[2] + offset[1],
            sa * p[1] + ca * p[2] + offset[2],
        ]
    };
    let placed_dir = |p: [f64; 3]| sub(placed(p), placed([0., 0., 0.]));
    assert!((semi_major - 2. / c).abs() <= 1e-12, "{semi_major}");
    assert!((semi_minor - 2.).abs() <= 1e-12, "{semi_minor}");
    assert!(
        sub(*center, placed([0., 0., 4.]))
            .iter()
            .all(|x| x.abs() <= 1e-12),
        "{center:?}"
    );
    assert!(
        sub(*major, placed_dir([0., c, s]))
            .iter()
            .all(|x| x.abs() <= 1e-12),
        "{major:?}"
    );
    assert!(
        sub(*minor, placed_dir([-1., 0., 0.]))
            .iter()
            .all(|x| x.abs() <= 1e-12),
        "{minor:?}"
    );
    assert!(
        sub(*normal, placed_dir([0., -s, c]))
            .iter()
            .all(|x| x.abs() <= 1e-12),
        "{normal:?}"
    );
    let p0 = point3(&curve.evaluate(0.).unwrap().point);
    let oracle = placed([0., 2., 4. + 2. / 3_f64.sqrt()]);
    assert!(sub(p0, oracle).iter().all(|x| x.abs() <= 1e-12), "{p0:?}");
    assert!(*max_sample_residual <= 1e-12, "{max_sample_residual}");
    let worst = lift_worst(
        &plane,
        &cylinder,
        plane_uv,
        &[],
        (placed([0., 0., 4.]), placed_dir([0., 0., 1.]), 2.),
        (placed_dir([0., -s, c]), placed([-6., -8. * c, 4. - 8. * s])),
    );
    assert!(worst <= 1e-9, "{worst}");
}
