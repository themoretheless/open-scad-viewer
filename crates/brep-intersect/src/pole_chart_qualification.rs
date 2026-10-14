//! Independent world samples verify the typed source pullback on all pair kinds.
use super::*;
use brep_core::{Model, analytic};
fn lifts(model: &Model, curves: &[SpherePatchCircle], world: &Curve) {
    assert_eq!(curves.len(), 8);
    let sphere = sphere_sphere::recognize_any(model).unwrap().unwrap();
    for lift in curves {
        assert!(lift.arcs.is_empty());
        let implicit = lift.implicit.as_ref().unwrap();
        assert_eq!(
            implicit.source_surface.control_points,
            model.faces[lift.patch].surface.control_points
        );
        assert_eq!(
            implicit.source_surface.weights,
            model.faces[lift.patch].surface.weights
        );
    }
    let [start, end] = world.domain();
    for i in 0..129 {
        let p = world
            .evaluate(start + (end - start) * (i as f64 + 0.25) / 129.)
            .unwrap()
            .point;
        let p: [f64; 3] = p.try_into().unwrap();
        let mut represented = false;
        for lift in curves {
            let Some(uv) = sphere.invert_uv(lift.patch, p) else {
                continue;
            };
            let actual = model.faces[lift.patch]
                .surface
                .evaluate(uv[0].clamp(0., 1.), uv[1].clamp(0., 1.))
                .unwrap()
                .point;
            let error = (0..3)
                .map(|k| (actual[k] - p[k]).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(error < 1e-7, "patch={} t={i} error={error}", lift.patch);
            let s = lift.implicit.as_ref().unwrap();
            let residual = (0..3)
                .map(|k| s.normal[k] * (actual[k] - s.point[k]))
                .sum::<f64>()
                .abs();
            assert!(residual < 1e-7, "implicit plane residual={residual}");
            represented = true;
        }
        assert!(represented, "missing inverse at {p:?}");
    }
}
#[test]
fn pole_sphere_all_analytic_pair_kinds_preserve_source_uv_constraints() {
    let sphere = analytic::sphere_exact(2.).unwrap();
    assert!(
        sphere_sphere::recognize(&sphere).unwrap().is_none(),
        "Boolean stereo recognizer must stay layout-specific"
    );
    let second = crate::test_utils::translated(&sphere, [0., 0., 2.]);
    let pair = intersect_sphere_sphere(
        &sphere,
        &second,
        brep_core::intersections::Options::default(),
    )
    .unwrap();
    assert!(pair.unresolved.is_empty(), "{:?}", pair.unresolved);
    assert!(!pair.permits_topology_change());
    assert_eq!(pair.components.len(), 1);
    let SphereSphereComponent::Circle {
        curve,
        first_uv,
        second_uv,
        ..
    } = &pair.components[0];
    lifts(&sphere, first_uv, curve);
    lifts(&second, second_uv, curve);
    let plane = plane_sphere::plane_patch([-4., -4., 1.], [8., 0., 0.], [0., 8., 0.]);
    let pair = intersect_plane_sphere(&plane, &sphere, Options::default()).unwrap();
    assert!(pair.unresolved.is_empty(), "{:?}", pair.unresolved);
    assert!(!pair.permits_topology_change());
    assert_eq!(pair.components.len(), 1);
    let PlaneSphereComponent::Circle {
        curve, sphere_uv, ..
    } = &pair.components[0];
    lifts(&sphere, sphere_uv, curve);
    let cylinder = analytic::cylinder(1., 6.).unwrap();
    let centered = crate::test_utils::translated(&sphere, [0., 0., 3.]);
    let pair = intersect_sphere_cylinder(&centered, &cylinder, Options::default()).unwrap();
    assert!(pair.unresolved.is_empty(), "{:?}", pair.unresolved);
    assert!(!pair.permits_topology_change());
    assert_eq!(pair.components.len(), 2);
    for SphereCylinderComponent::Circle {
        curve, sphere_uv, ..
    } in &pair.components
    {
        lifts(&centered, sphere_uv, curve);
    }
    let cone = analytic::frustum(0.5, 2., 6.).unwrap();
    let pair = intersect_sphere_cone(&centered, &cone, Options::default()).unwrap();
    assert!(pair.unresolved.is_empty(), "{:?}", pair.unresolved);
    assert!(!pair.permits_topology_change());
    assert!(!pair.components.is_empty());
    for SphereConeComponent::Circle {
        curve, sphere_uv, ..
    } in &pair.components
    {
        lifts(&centered, sphere_uv, curve);
    }
    let torus = analytic::torus(3., 1.).unwrap();
    let torus_sphere = analytic::sphere_exact(2.5).unwrap();
    let pair = intersect_sphere_torus(&torus_sphere, &torus, Options::default()).unwrap();
    assert!(pair.unresolved.is_empty(), "{:?}", pair.unresolved);
    assert!(!pair.permits_topology_change());
    assert_eq!(pair.components.len(), 2);
    for SphereTorusComponent::Circle {
        curve, sphere_uv, ..
    } in &pair.components
    {
        lifts(&torus_sphere, sphere_uv, curve);
    }
}
#[test]
fn clipped_plane_lifts_keep_world_interval_constraints() {
    let sphere = analytic::sphere_exact(2.).unwrap();
    let plane = plane_sphere::plane_patch([0.2, -4., 1.], [3., 0., 0.], [0., 8., 0.]);
    let pair = intersect_plane_sphere(&plane, &sphere, Options::default()).unwrap();
    assert!(pair.unresolved.is_empty(), "{:?}", pair.unresolved);
    assert!(!pair.components.is_empty());
    for PlaneSphereComponent::Circle {
        curve, sphere_uv, ..
    } in &pair.components
    {
        lifts(&sphere, sphere_uv, curve);
        for lift in sphere_uv {
            let clip = lift.implicit.as_ref().unwrap().clip.as_ref().unwrap();
            assert!(clip.intervals.iter().all(|i| i[0] < i[1]));
        }
    }
}
#[test]
fn pole_tangency_coincidence_and_forged_controls_never_report_complete() {
    let sphere = analytic::sphere_exact(2.).unwrap();
    let tangent = crate::test_utils::translated(&sphere, [4., 0., 0.]);
    for other in [&sphere, &tangent] {
        let pair =
            intersect_sphere_sphere(&sphere, other, brep_core::intersections::Options::default())
                .unwrap();
        assert!(!pair.unresolved.is_empty());
        assert_eq!(
            pair.coverage,
            brep_core::intersections::Coverage::Incomplete
        );
        assert!(!pair.permits_topology_change());
    }
    let mut mutated = sphere.clone();
    mutated.faces[0].surface.control_points[1][1][2] += 0.01;
    let r = intersect_sphere_sphere(
        &mutated,
        &tangent,
        brep_core::intersections::Options::default(),
    )
    .unwrap();
    assert_eq!(
        r.unresolved[0].reason,
        brep_core::intersections::UnresolvedReason::UnsupportedSurface
    );
}
