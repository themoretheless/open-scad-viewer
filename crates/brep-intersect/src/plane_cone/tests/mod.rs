use super::super::plane_sphere::plane_patch;
use super::super::test_utils::rotated_translated;
use super::*;



/// Worst residual of the UV lifts evaluated through their own surfaces
/// against the cone side and plane equations.
fn lift_worst(
    plane_model: &Model,
    cone_model: &Model,
    plane_uv: &[PlanePatchCurve],
    cone_uv: &[CylinderPatchCurve],
    cone_def: ([f64; 3], [f64; 3], f64, f64),
    plane_def: ([f64; 3], [f64; 3]),
) -> f64 {
    let check = |p: [f64; 3]| {
        let rel = sub(p, cone_def.0);
        let a = dot(rel, cone_def.1);
        let perp = sub(rel, cone_def.1.map(|x| x * a));
        (norm(perp) - (cone_def.2 + cone_def.3 * a))
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
    for lift in cone_uv {
        let surface = &cone_model.faces[lift.patch].surface;
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
fn perpendicular_circle_interpolates_radius_between_rings() {
    // Frustum r 3 -> 1 over z 0..5, plane z = 2.5 patch [-4,4]^2: the
    // exact circle of the linearly interpolated radius 2 at mid-height.
    let plane = plane_patch([-4., -4., 2.5], [8., 0., 0.], [0., 8., 0.]);
    let cone = brep_core::analytic::frustum(3., 1., 5.).unwrap();
    let report = intersect_plane_cone(&plane, &cone, Options::default()).unwrap();
    assert!(report.unresolved.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    assert!(!report.permits_topology_change());
    let [
        PlaneConeComponent::Circle {
            curve,
            center,
            radius,
            normal,
            full,
            plane_uv,
            cone_uv,
            max_sample_residual,
        },
    ] = &report.components[..]
    else {
        panic!("expected one circle component: {report:?}")
    };
    assert!(*full);
    assert!((*radius - 2.).abs() <= 1e-12, "{radius}");
    assert!(
        sub(*center, [0., 0., 2.5]).iter().all(|x| x.abs() <= 1e-12),
        "{center:?}"
    );
    assert_eq!(*normal, [0., 0., 1.]);
    assert_eq!(
        curve.knots,
        vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.]
    );
    assert_eq!(curve.weights.len(), 9);
    assert!((curve.weights[1] - std::f64::consts::FRAC_1_SQRT_2).abs() <= 1e-15);
    assert!(*max_sample_residual <= 1e-12, "{max_sample_residual}");
    assert_eq!(plane_uv[0].arcs.len(), 4);
    // Cone UV: iso-v rulings at mid-height on all four side patches.
    assert_eq!(cone_uv.len(), 4);
    for lift in cone_uv {
        assert_eq!(lift.arcs.len(), 1);
        let arc = &lift.arcs[0];
        assert_eq!(arc.degree, 1);
        assert_eq!(arc.control_points[0], vec![0., 0.5]);
        assert_eq!(arc.control_points[1], vec![1., 0.5]);
    }
    let worst = lift_worst(
        &plane,
        &cone,
        plane_uv,
        cone_uv,
        ([0., 0., 0.], [0., 0., 1.], 3., -0.4),
        ([0., 0., 1.], [-4., -4., 2.5]),
    );
    assert!(worst <= 1e-9, "{worst}");
}

#[test]
fn perpendicular_clipped_circle_has_exact_trims() {
    // Plane z = 2.5, patch x in [0, 4], y in [-4, 4]: the radius-2 circle
    // clipped to its x >= 0 half, swept [3pi/2, 5pi/2].
    let plane = plane_patch([0., -4., 2.5], [4., 0., 0.], [0., 8., 0.]);
    let cone = brep_core::analytic::frustum(3., 1., 5.).unwrap();
    let report = intersect_plane_cone(&plane, &cone, Options::default()).unwrap();
    assert!(report.unresolved.is_empty(), "{report:?}");
    let [
        PlaneConeComponent::Circle {
            curve,
            center,
            full,
            plane_uv,
            cone_uv,
            max_sample_residual,
            ..
        },
    ] = &report.components[..]
    else {
        panic!("expected one clipped circle: {report:?}")
    };
    assert!(!*full);
    assert!(
        sub(*center, [0., 0., 2.5]).iter().all(|x| x.abs() <= 1e-12),
        "{center:?}"
    );
    assert_eq!(curve.knots, vec![0., 0., 0., 1., 1., 2., 2., 2.]);
    let start = point3(&curve.evaluate(0.).unwrap().point);
    let end = point3(&curve.evaluate(2.).unwrap().point);
    assert!(
        start[0].abs() <= 1e-12 && (start[1] + 2.).abs() <= 1e-12,
        "{start:?}"
    );
    assert!((start[2] - 2.5).abs() <= 1e-12);
    assert!(
        end[0].abs() <= 1e-12 && (end[1] - 2.).abs() <= 1e-12,
        "{end:?}"
    );
    assert!((end[2] - 2.5).abs() <= 1e-12);
    assert!(*max_sample_residual <= 1e-12, "{max_sample_residual}");
    // Plane UV: two arcs meeting at the phi = 0 seam (1/2, 1/2); the free
    // endpoints sit exactly on u = 0 at v = 1/4 and v = 3/4.
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
    assert!((edge_v[0] - 0.25).abs() <= 1e-12, "{edge_v:?}");
    assert!((edge_v[1] - 0.75).abs() <= 1e-12, "{edge_v:?}");
    assert_eq!(seam.len(), 2, "{seam:?}");
    for p in &seam {
        assert!((p[0] - 0.5).abs() <= 1e-12, "{p:?}");
        assert!((p[1] - 0.5).abs() <= 1e-12, "{p:?}");
    }
    assert_eq!(cone_uv.len(), 2, "{cone_uv:?}");
    for lift in cone_uv {
        for arc in &lift.arcs {
            assert_eq!(arc.degree, 1);
            assert!((arc.control_points[0][1] - 0.5).abs() <= 1e-12);
            assert!((arc.control_points[1][1] - 0.5).abs() <= 1e-12);
        }
    }
    let worst = lift_worst(
        &plane,
        &cone,
        plane_uv,
        cone_uv,
        ([0., 0., 0.], [0., 0., 1.], 3., -0.4),
        ([0., 0., 1.], [0., -4., 2.5]),
    );
    assert!(worst <= 1e-9, "{worst}");
}

#[test]
fn ring_planes_apex_level_and_beyond_classification() {
    let cone = brep_core::analytic::frustum(3., 1., 5.).unwrap();
    let at = |z: f64| plane_patch([-4., -4., z], [8., 0., 0.], [0., 8., 0.]);
    // Plane coincident with a ring plane: coincident cap/rim region,
    // never a guessed circle.
    for z in [0., 5.] {
        let report = intersect_plane_cone(&at(z), &cone, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{z} {report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::CoincidentTrim
        );
    }
    // Provably beyond both rings: empty and resolved.
    for z in [6., -1., 5. + 1e-9, -1e-9] {
        let report = intersect_plane_cone(&at(z), &cone, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{z} {report:?}"
        );
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
    }
    // Just clear of the top ring band below: a full circle near the rim,
    // radius 1 + 0.4e-9 by linear interpolation.
    let report = intersect_plane_cone(&at(5. - 1e-9), &cone, Options::default()).unwrap();
    let [
        PlaneConeComponent::Circle {
            center,
            radius,
            full,
            ..
        },
    ] = &report.components[..]
    else {
        panic!("expected one circle: {report:?}")
    };
    assert!(*full);
    assert!((radius - (1. + 0.4e-9)).abs() <= 1e-12, "{radius}");
    assert!((center[2] - (5. - 1e-9)).abs() <= 1e-12, "{center:?}");
    // Apex cone (r_top == 0): the apex-level perpendicular plane is a
    // tangency, never a point component; a mid section is the circle.
    let apex_cone = brep_core::analytic::frustum(3., 0., 5.).unwrap();
    let report = intersect_plane_cone(&at(5.), &apex_cone, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::TangencyOrMultipleRoot
    );
    let report = intersect_plane_cone(&at(2.5), &apex_cone, Options::default()).unwrap();
    let [
        PlaneConeComponent::Circle {
            radius, cone_uv, ..
        },
    ] = &report.components[..]
    else {
        panic!("expected one circle on the apex cone: {report:?}")
    };
    assert!((*radius - 1.5).abs() <= 1e-12, "{radius}");
    assert_eq!(cone_uv.len(), 4);
}

#[test]
fn through_axis_plane_yields_two_exact_rulings() {
    // Frustum r 3 -> 1 over z 0..5 (apex at z = 7.5), plane y = 0
    // through the axis: the two exact rulings (+-3, 0, 0) -> (+-1, 0, 5).
    let plane = plane_patch([-5., 0., -2.], [10., 0., 0.], [0., 0., 9.]);
    let cone = brep_core::analytic::frustum(3., 1., 5.).unwrap();
    let report = intersect_plane_cone(&plane, &cone, Options::default()).unwrap();
    assert!(report.unresolved.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    assert_eq!(report.components.len(), 2, "{report:?}");
    let wlen = 1.16_f64.sqrt();
    let mut seen: Vec<f64> = Vec::new();
    for component in &report.components {
        let PlaneConeComponent::Line {
            curve,
            start,
            end,
            direction,
            contact,
            plane_uv,
            cone_uv,
            max_sample_residual,
        } = component
        else {
            panic!("expected line components: {report:?}")
        };
        assert_eq!(*contact, Contact::Boundary);
        assert_eq!(curve.degree, 1);
        assert_eq!(curve.knots, vec![0., 0., 1., 1.]);
        assert!((start[0].abs() - 3.).abs() <= 1e-12, "{start:?}");
        assert!(
            start[1].abs() <= 1e-12 && start[2].abs() <= 1e-12,
            "{start:?}"
        );
        assert!((end[0].abs() - 1.).abs() <= 1e-12, "{end:?}");
        assert!(
            end[1].abs() <= 1e-12 && (end[2] - 5.).abs() <= 1e-12,
            "{end:?}"
        );
        assert_eq!(start[0].signum(), end[0].signum());
        // Unit ruling direction (-/+0.4, 0, 1) / sqrt(1.16).
        assert!(
            (direction[0].abs() - 0.4 / wlen).abs() <= 1e-12,
            "{direction:?}"
        );
        assert!((direction[2] - 1. / wlen).abs() <= 1e-12, "{direction:?}");
        seen.push(start[0]);
        assert!(*max_sample_residual <= 1e-12, "{max_sample_residual}");
        assert!(!plane_uv.is_empty() && !cone_uv.is_empty());
        // Cone UV: iso-u rulings spanning the full height fraction 0..1.
        for lift in cone_uv {
            for arc in &lift.arcs {
                assert_eq!(arc.degree, 1);
                assert!(arc.control_points[0][1].abs() <= 1e-12, "{arc:?}");
                assert!((arc.control_points[1][1] - 1.).abs() <= 1e-12, "{arc:?}");
            }
        }
        let worst = lift_worst(
            &plane,
            &cone,
            plane_uv,
            cone_uv,
            ([0., 0., 0.], [0., 0., 1.], 3., -0.4),
            ([0., 1., 0.], [-5., 0., -2.]),
        );
        assert!(worst <= 1e-9, "{worst}");
    }
    seen.sort_by(f64::total_cmp);
    assert!(seen[0] < 0. && seen[1] > 0., "{seen:?}");
    // Apex cone: the rulings meet at the apex (0, 0, 5) on the model.
    let apex_cone = brep_core::analytic::frustum(3., 0., 5.).unwrap();
    let report = intersect_plane_cone(&plane, &apex_cone, Options::default()).unwrap();
    assert!(report.unresolved.is_empty(), "{report:?}");
    assert_eq!(report.components.len(), 2, "{report:?}");
    for component in &report.components {
        let PlaneConeComponent::Line { start, end, .. } = component else {
            panic!("expected lines: {report:?}")
        };
        assert!(
            (start[0].abs() - 3.).abs() <= 1e-12 && start[2].abs() <= 1e-12,
            "{start:?}"
        );
        assert!(end[0].abs() <= 1e-12 && end[1].abs() <= 1e-12, "{end:?}");
        assert!((end[2] - 5.).abs() <= 1e-12, "{end:?}");
    }
}

#[test]
fn parallel_offset_plane_yields_exact_hyperbola() {
    // Frustum r 3 -> 1 over z 0..5 (slope -0.4, apex z = 7.5), plane
    // y = 0.5 parallel to the axis: the hyperbola 0.16 x^2 - X^2 = 0.25
    // in apex coordinates (x axial from the apex, X the world x), with
    // a = 1.25, b = 0.5 and asymptote half-angle atan(0.4) = alpha.
    let plane = plane_patch([-4., 0.5, -1.], [8., 0., 0.], [0., 0., 7.]);
    let cone = brep_core::analytic::frustum(3., 1., 5.).unwrap();
    let report = intersect_plane_cone(&plane, &cone, Options::default()).unwrap();
    assert!(report.unresolved.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    // The height rings clip the one nappe arm into the two arcs above
    // and below the symmetry plane.
    assert_eq!(report.components.len(), 2, "{report:?}");
    let mut signs: Vec<f64> = Vec::new();
    for component in &report.components {
        let PlaneConeComponent::Hyperbola {
            curve,
            center,
            semi_transverse,
            semi_conjugate,
            transverse,
            conjugate,
            normal,
            plane_uv,
            cone_uv,
            max_sample_residual,
        } = component
        else {
            panic!("expected hyperbola components: {report:?}")
        };
        assert!(
            (*semi_transverse - 1.25).abs() <= 1e-12,
            "{semi_transverse}"
        );
        assert!((*semi_conjugate - 0.5).abs() <= 1e-12, "{semi_conjugate}");
        // Asymptote slope b / a is exactly the cone slope magnitude.
        assert!((semi_conjugate / semi_transverse - 0.4).abs() <= 1e-12);
        assert!(
            sub(*center, [0., 0.5, 7.5])
                .iter()
                .all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        assert!(
            sub(*transverse, [0., 0., -1.])
                .iter()
                .all(|x| x.abs() <= 1e-12)
        );
        assert!((conjugate[0].abs() - 1.).abs() <= 1e-12, "{conjugate:?}");
        assert!(conjugate[1].abs() <= 1e-12 && conjugate[2].abs() <= 1e-12);
        assert!((normal[1].abs() - 1.).abs() <= 1e-12, "{normal:?}");
        assert!(cone_uv.is_none());
        assert!(*max_sample_residual <= 1e-12, "{max_sample_residual}");
        // Arc endpoints sit exactly on the ring planes: |X| = sqrt(8.75)
        // at z = 0 and sqrt(0.75) at z = 5.
        let start = point3(&curve.evaluate(0.).unwrap().point);
        let end = point3(&curve.evaluate(1.).unwrap().point);
        for p in [start, end] {
            assert!((p[1] - 0.5).abs() <= 1e-12, "{p:?}");
            let on_bottom =
                p[2].abs() <= 1e-12 && (p[0].abs() - 8.75_f64.sqrt()).abs() <= 1e-12;
            let on_top =
                (p[2] - 5.).abs() <= 1e-12 && (p[0].abs() - 0.75_f64.sqrt()).abs() <= 1e-12;
            assert!(on_bottom || on_top, "{p:?}");
        }
        // One arc carries the X > 0 sweep, the other X < 0.
        assert_eq!(start[0].signum(), end[0].signum(), "{start:?} {end:?}");
        signs.push(start[0].signum());
        // The plane UV lift evaluates through the patch onto both
        // implicit equations.
        let empty: Vec<CylinderPatchCurve> = Vec::new();
        let worst = lift_worst(
            &plane,
            &cone,
            plane_uv,
            &empty,
            ([0., 0., 0.], [0., 0., 1.], 3., -0.4),
            ([0., 1., 0.], [-4., 0.5, -1.]),
        );
        assert!(worst <= 1e-9, "{worst}");
    }
    signs.sort_by(f64::total_cmp);
    assert!(signs[0] < 0. && signs[1] > 0., "{signs:?}");
}

#[test]
fn oblique_ellipse_matches_the_independent_vertex_oracle() {
    // Frustum r 3 -> 1 over z 0..5; plane normal (0, 0.6, 0.8) through
    // (0, 0, 2.5) (offset 2.0): |axis.n| = 0.8 > sin(alpha) = 0.4/sqrt(1.16)
    // — an ellipse. Independent oracle: the two vertices are the
    // intersections of the plane with the cone generators in the
    // symmetry plane x = 0, y = +-(3 - 0.4 z): (0, 20/7, 5/14) and
    // (0, -20/13, 95/26), so the center is their midpoint and the
    // semi-major is half their distance, 250/91 exactly.
    let v1 = [0., 20. / 7., 5. / 14.];
    let v2 = [0., -20. / 13., 95. / 26.];
    let mid = [0., (v1[1] + v2[1]) / 2., (v1[2] + v2[2]) / 2.];
    // Patch exactly through the oracle center: u along world x, v along
    // the in-plane axis projection (0, -0.8, 0.6), half-extent 4.
    let e1p = [0., -0.8, 0.6];
    let origin = [mid[0] - 4., mid[1] - 4. * e1p[1], mid[2] - 4. * e1p[2]];
    let plane = plane_patch(origin, [8., 0., 0.], e1p.map(|x| x * 8.));
    let cone = brep_core::analytic::frustum(3., 1., 5.).unwrap();
    let report = intersect_plane_cone(&plane, &cone, Options::default()).unwrap();
    assert!(report.unresolved.is_empty(), "{report:?}");
    let [
        PlaneConeComponent::Ellipse {
            curve,
            center,
            semi_major,
            semi_minor,
            major,
            minor,
            normal,
            full,
            plane_uv,
            cone_uv,
            max_sample_residual,
        },
    ] = &report.components[..]
    else {
        panic!("expected one ellipse: {report:?}")
    };
    assert!(*full);
    let v1 = [0., 20. / 7., 5. / 14.];
    let v2 = [0., -20. / 13., 95. / 26.];
    let mid = [0., (v1[1] + v2[1]) / 2., (v1[2] + v2[2]) / 2.];
    assert!(
        sub(*center, mid).iter().all(|x| x.abs() <= 1e-12),
        "{center:?} vs {mid:?}"
    );
    assert!((*semi_major - 250. / 91.).abs() <= 1e-12, "{semi_major}");
    assert!((*semi_minor - 2.096570).abs() <= 1e-5, "{semi_minor}");
    assert!(
        sub(*major, [0., -0.8, 0.6])
            .iter()
            .all(|x| x.abs() <= 1e-12),
        "{major:?}"
    );
    assert!(minor[0].abs() >= 1. - 1e-12, "{minor:?}");
    assert!(minor[1].abs() <= 1e-12 && minor[2].abs() <= 1e-12);
    assert!(normal[0].abs() <= 1e-12, "{normal:?}");
    assert!((normal[1].abs() - 0.6).abs() <= 1e-12 && (normal[2].abs() - 0.8).abs() <= 1e-12);
    assert!(cone_uv.is_none());
    assert!(*max_sample_residual <= 1e-12, "{max_sample_residual}");
    // The curve's phi = 0 / pi endpoints are the oracle vertices.
    let p0 = point3(&curve.evaluate(0.).unwrap().point);
    let p2 = point3(&curve.evaluate(2.).unwrap().point);
    assert!(
        sub(p0, v2).iter().all(|x| x.abs() <= 1e-12),
        "{p0:?} vs {v2:?}"
    );
    assert!(
        sub(p2, v1).iter().all(|x| x.abs() <= 1e-12),
        "{p2:?} vs {v1:?}"
    );
    let empty: Vec<CylinderPatchCurve> = Vec::new();
    let worst = lift_worst(
        &plane,
        &cone,
        plane_uv,
        &empty,
        ([0., 0., 0.], [0., 0., 1.], 3., -0.4),
        ([0., 0.6, 0.8], origin),
    );
    assert!(worst <= 1e-9, "{worst}");
}

#[test]
fn ruling_parallel_plane_yields_exact_parabola() {
    // Frustum r 3 -> 1 over z 0..5: sin(alpha) = 2/sqrt(29). The plane
    // built exactly parallel to a ruling (normal (0, cos, sin) with
    // sin = 2/sqrt(29)) through (0, 0, 2) cuts the exact parabola with
    // focal length 1.1 sin(alpha), all weights 1, clipped by the bottom
    // ring plane.
    let sigma = 0.4 / 1.16_f64.sqrt();
    let s = (1. - sigma * sigma).sqrt();
    let normal = [0., s, sigma];
    let offset = 2. * sigma;
    // Patch exactly through (0, 0, 2) (the plane offset 2 sigma holds by
    // construction): u along world x, v along the in-plane axis
    // projection, half-extents 6 and 7 covering the whole clipped arc.
    let e1 = [0., -sigma, s];
    let origin = [-6., 7. * sigma, 2. - 7. * s];
    let plane = plane_patch(origin, [12., 0., 0.], e1.map(|x| x * 14.));
    let cone = brep_core::analytic::frustum(3., 1., 5.).unwrap();
    let report = intersect_plane_cone(&plane, &cone, Options::default()).unwrap();
    assert!(report.unresolved.is_empty(), "{report:?}");
    let [
        PlaneConeComponent::Parabola {
            curve,
            vertex,
            direction,
            focal_length,
            normal: n,
            plane_uv,
            cone_uv,
            max_sample_residual,
        },
    ] = &report.components[..]
    else {
        panic!("expected one parabola: {report:?}")
    };
    assert_eq!(curve.degree, 2);
    assert_eq!(curve.knots, vec![0., 0., 0., 1., 1., 1.]);
    assert!(
        curve.weights.iter().all(|w| *w == 1.),
        "{:?}",
        curve.weights
    );
    // Exact focal-length relation: |L| = 0.8 |d0|, |d0| = 5.5 sigma.
    assert!(
        (*focal_length - 1.1 * sigma).abs() <= 1e-12,
        "{focal_length}"
    );
    assert!(
        sub(*vertex, [0., -1.0998, 4.7504])
            .iter()
            .all(|x| x.abs() <= 1e-3),
        "{vertex:?}"
    );
    assert!(
        sub(*direction, [0., sigma, -s])
            .iter()
            .all(|x| x.abs() <= 1e-12),
        "{direction:?}"
    );
    assert!(n[0].abs() <= 1e-12, "{n:?}");
    assert!((n[1].abs() - s).abs() <= 1e-12 && (n[2].abs() - sigma).abs() <= 1e-12);
    assert!(cone_uv.is_none());
    assert!(*max_sample_residual <= 1e-12, "{max_sample_residual}");
    // Both clip endpoints sit exactly on the bottom ring plane z = 0
    // with the ring radius 3.
    for t in [0., 1.] {
        let p = point3(&curve.evaluate(t).unwrap().point);
        assert!(p[2].abs() <= 1e-12, "{p:?}");
        assert!((p[0].hypot(p[1]) - 3.).abs() <= 1e-9, "{p:?}");
    }
    // The plane offset is exactly n.(0,0,2).
    let d0 = offset - dot(normal, [0., 0., 7.5]);
    assert!((d0 + 5.5 * sigma).abs() <= 1e-12, "{d0}");
    let empty: Vec<CylinderPatchCurve> = Vec::new();
    let worst = lift_worst(
        &plane,
        &cone,
        plane_uv,
        &empty,
        ([0., 0., 0.], [0., 0., 1.], 3., -0.4),
        (normal, origin),
    );
    assert!(worst <= 1e-9, "{worst}");
}

#[test]
fn parabola_threshold_band_and_apex_contact_stay_unresolved() {
    let cone = brep_core::analytic::frustum(3., 1., 5.).unwrap();
    let sigma = 0.4 / 1.16_f64.sqrt();
    let s = (1. - sigma * sigma).sqrt();
    // Recognition-scale tilt off the ruling-parallel angle: the
    // degenerate threshold is never a guessed conic. The patch plane
    // carries the tilted normal through (0, 0, 2).
    let delta = 5e-10_f64;
    let (sd, cd) = delta.sin_cos();
    let e1t = [0., -(sigma * cd + s * sd), s * cd - sigma * sd];
    let origin = [-6., -7. * e1t[1], 2. - 7. * e1t[2]];
    let plane = plane_patch(origin, [12., 0., 0.], e1t.map(|x| x * 14.));
    let report = intersect_plane_cone(&plane, &cone, Options::default()).unwrap();
    assert!(report.components.is_empty(), "{report:?}");
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::TangencyOrMultipleRoot
    );
    // Oblique planes through the apex (ellipse angle and hyperbola
    // angle): the apex is a multiple-root contact, never guessed. The
    // patch spans the plane through the apex along e1 = (1,0,0) (both
    // normals have a zero x component) and e2 = normal x e1.
    let apex = [0., 0., 7.5];
    for normal in [[0., 0.6, 0.8], [0., 0.96_f64.sqrt(), 0.2]] {
        let e1 = [1., 0., 0.];
        let e2 = cross(normal, e1);
        let e2 = e2.map(|x| x / norm(e2));
        let origin = sub(sub(apex, e1.map(|x| x * 6.)), e2.map(|x| x * 8.));
        let plane = plane_patch(origin, e1.map(|x| x * 12.), e2.map(|x| x * 16.));
        let report = intersect_plane_cone(&plane, &cone, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{normal:?} {report:?}");
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
    }
}

#[test]
fn misses_resolve_empty() {
    let cone = brep_core::analytic::frustum(3., 1., 5.).unwrap();
    // Perpendicular plane beyond the top ring.
    let plane = plane_patch([-4., -4., 7.], [8., 0., 0.], [0., 8., 0.]);
    let report = intersect_plane_cone(&plane, &cone, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    // Oblique plane whose ellipse lives entirely below the bottom ring.
    let plane = plane_patch([-30., 20., -20.], [60., 0., 0.], [0., -48., 36.]);
    // normal of that patch is (0, 0.6, 0.8)-ish; ensure a clear miss:
    // n.p = 0.6*20 + 0.8*(-20) = -4 at the origin — far below the cone.
    let report = intersect_plane_cone(&plane, &cone, Options::default()).unwrap();
    assert!(
        report.components.is_empty() && report.unresolved.is_empty(),
        "{report:?}"
    );
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
}

#[test]
fn rotated_translated_placement_keeps_the_exact_ellipse() {
    // The ellipse configuration rotated 0.5 rad about X and translated:
    // same section against the transformed oracle.
    let angle = 0.5_f64;
    let offset = [10., -7., 3.];
    let (sa, ca) = angle.sin_cos();
    let rot = |p: [f64; 3]| {
        [
            p[0] + offset[0],
            ca * p[1] - sa * p[2] + offset[1],
            sa * p[1] + ca * p[2] + offset[2],
        ]
    };
    // Exact pre-rotation patch through the oracle center, then rotated.
    let mid0 = [0., (20. / 7. - 20. / 13.) / 2., (5. / 14. + 95. / 26.) / 2.];
    let e1p = [0., -0.8, 0.6];
    let origin0 = [mid0[0] - 4., mid0[1] - 4. * e1p[1], mid0[2] - 4. * e1p[2]];
    let plane = plane_patch(
        rot(origin0),
        sub(rot([origin0[0] + 8., origin0[1], origin0[2]]), rot(origin0)),
        sub(
            rot([
                origin0[0],
                origin0[1] + 8. * e1p[1],
                origin0[2] + 8. * e1p[2],
            ]),
            rot(origin0),
        ),
    );
    let cone = rotated_translated(
        &brep_core::analytic::frustum(3., 1., 5.).unwrap(),
        angle,
        offset,
    );
    let report = intersect_plane_cone(&plane, &cone, Options::default()).unwrap();
    assert!(report.unresolved.is_empty(), "{report:?}");
    let [
        PlaneConeComponent::Ellipse {
            center,
            semi_major,
            major,
            full,
            cone_uv,
            max_sample_residual,
            ..
        },
    ] = &report.components[..]
    else {
        panic!("expected one placed ellipse: {report:?}")
    };
    assert!(*full);
    assert!(cone_uv.is_none());
    let mid = rot([0., (20. / 7. - 20. / 13.) / 2., (5. / 14. + 95. / 26.) / 2.]);
    assert!(
        sub(*center, mid).iter().all(|x| x.abs() <= 1e-9),
        "{center:?} vs {mid:?}"
    );
    assert!((*semi_major - 250. / 91.).abs() <= 1e-9, "{semi_major}");
    let major_expected = sub(rot([0., -0.8, 0.6]), rot([0., 0., 0.]));
    assert!(
        sub(*major, major_expected).iter().all(|x| x.abs() <= 1e-9),
        "{major:?} vs {major_expected:?}"
    );
    assert!(*max_sample_residual <= 1e-9, "{max_sample_residual}");
}

#[test]
fn non_canonical_operands_are_explicit_refusals() {
    let plane = plane_patch([-4., -4., 2.5], [8., 0., 0.], [0., 8., 0.]);
    // An equal-radius frustum is a cylinder: refused here (the
    // plane/cylinder cell owns it).
    let cylinder = brep_core::analytic::cylinder(2., 5.).unwrap();
    let sphere = brep_core::analytic::sphere(2.).unwrap();
    let tube = brep_core::analytic::tube(2., 1., 5.).unwrap();
    let frustum = brep_core::analytic::frustum(3., 1., 5.).unwrap();
    for second in [&cylinder, &sphere, &tube] {
        let report = intersect_plane_cone(&plane, second, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
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
    // A solid as the plane operand: fixed order, explicit refusal.
    let report = intersect_plane_cone(&frustum, &frustum, Options::default()).unwrap();
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::UnsupportedSurface
    );
    // Skewed non-rectangular patch: not the canonical planar patch.
    let skewed = plane_patch([-4., -4., 2.5], [8., 0., 0.], [3., 8., 0.]);
    let report = intersect_plane_cone(&skewed, &frustum, Options::default()).unwrap();
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::UnsupportedSurface
    );
}
