use brep_core::{Model, analysis::mass_properties, boolean};
use std::f64::consts::PI;

fn translate(mut model: Model, offset: [f64; 3]) -> Model {
    for vertex in &mut model.vertices {
        for i in 0..3 {
            vertex.point[i] += offset[i];
        }
    }
    for edge in &mut model.edges {
        for point in &mut edge.curve.control_points {
            for i in 0..3 {
                point[i] += offset[i];
            }
        }
    }
    for face in &mut model.faces {
        for point in face.surface.control_points.iter_mut().flatten() {
            for i in 0..3 {
                point[i] += offset[i];
            }
        }
    }
    model.rebuild_topology_ids();
    model.validate().unwrap();
    model
}
fn cylinder(radius: f64, low: f64, high: f64, x: f64) -> Model {
    translate(
        brep_core::cylinder(radius, high - low).unwrap(),
        [x, 0., low],
    )
}
fn volume(model: &Model) -> f64 {
    if model.is_empty() {
        0.
    } else {
        mass_properties(model, 1e-7, 800_000)
            .unwrap()
            .signed_volume_mm3
    }
}
fn near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < expected.abs().max(1.) * 2e-6,
        "{actual} != {expected}"
    );
}
fn check(model: &Model, expected: f64) {
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
    let actual = volume(model);
    near(actual, expected);
    if !model.is_empty() {
        assert!(actual > 0.);
        assert!(model.edges.iter().any(|e| e.curve.degree == 2));
    }
    let json = value_codec::to_string(model).unwrap();
    let decoded: Model = value_codec::from_str(&json).unwrap();
    decoded.validate().unwrap();
    assert_eq!(json, value_codec::to_string(&decoded).unwrap());
}
fn cap_area_at(model: &Model, z: f64) -> f64 {
    model
        .faces
        .iter()
        .filter(|f| f.surface.control_points.iter().flatten().all(|p| p[2] == z))
        .map(|face| {
            std::iter::once(&face.outer)
                .chain(&face.holes)
                .map(|&wire| {
                    let curves = model.loops[wire]
                        .coedges
                        .iter()
                        .map(|c| {
                            let mut curve = model.edges[c.edge].curve.clone();
                            if c.reversed {
                                curve = curve.reverse().unwrap();
                            }
                            for p in &mut curve.control_points {
                                p.pop();
                            }
                            curve
                        })
                        .collect::<Vec<_>>();
                    brep_core::planar_trim::signed_area(&curves, model.tolerance_mm).unwrap()
                })
                .sum::<f64>()
                .abs()
        })
        .sum()
}
fn check_circle_carriers(model: &Model) {
    for edge in model.edges.iter().filter(|e| e.curve.degree == 2) {
        let points = [0., 0.17, 0.5, 0.83, 1.].map(|t| edge.curve.evaluate(t).unwrap().point);
        assert!([0., 3.].iter().any(|&cx| {
            points
                .iter()
                .all(|p| ((p[0] - cx).powi(2) + p[1].powi(2) - 9.).abs() < 1e-9)
        }));
    }
    for face in model.faces.iter().filter(|f| f.surface.degree_u == 2) {
        for v in [0., 0.37, 1.] {
            let p = face.surface.evaluate(0.43, v).unwrap().point;
            assert!(
                [0., 3.]
                    .iter()
                    .any(|&cx| ((p[0] - cx).powi(2) + p[1].powi(2) - 9.).abs() < 1e-9)
            );
        }
    }
}
#[test]
fn unequal_height_transverse_circles_cover_all_four_operations() {
    let a = cylinder(3., 0., 5., 0.);
    let b = cylinder(3., 1., 3., 3.);
    let overlap = 6. * PI - 4.5 * 3f64.sqrt();
    for (operation, expected) in [
        ("intersection", 2. * overlap),
        ("union", 63. * PI - 2. * overlap),
        ("difference", 45. * PI - 2. * overlap),
        ("xor", 63. * PI - 4. * overlap),
    ] {
        let result = boolean(&a, &b, operation)
            .unwrap_or_else(|e| panic!("{operation}: {} {}", e.code, e.message));
        check(&result, expected);
        assert_eq!(result.bodies.len(), if operation == "xor" { 2 } else { 1 });
        check_circle_carriers(&result);
        let cap_areas = match operation {
            "intersection" => [0., overlap, overlap, 0.],
            "union" => [9. * PI, 9. * PI - overlap, 9. * PI - overlap, 9. * PI],
            "difference" => [9. * PI, overlap, overlap, 9. * PI],
            _ => [9. * PI; 4],
        };
        for (z, expected) in [0., 1., 3., 5.].into_iter().zip(cap_areas) {
            near(cap_area_at(&result, z), expected);
        }
        if operation != "difference" {
            near(volume(&boolean(&b, &a, operation).unwrap()), expected);
        }
    }
}

#[test]
fn unequal_footprints_meeting_on_a_cap_regularize_without_an_internal_face() {
    let a = cylinder(3., 0., 2., 0.);
    let b = cylinder(3., 2., 5., 3.);
    let lens = 6. * PI - 4.5 * 3f64.sqrt();
    for operation in ["union", "xor"] {
        let result = boolean(&a, &b, operation).unwrap();
        check(&result, 45. * PI);
        assert_eq!(result.bodies.len(), 1);
        near(cap_area_at(&result, 2.), 18. * PI - 2. * lens);
    }
    assert!(boolean(&a, &b, "intersection").unwrap().is_empty());
    check(&boolean(&a, &b, "difference").unwrap(), 18. * PI);
}
#[test]
fn transverse_step_reuses_with_a_different_footprint_after_roundtrip() {
    let a = cylinder(3., 0., 5., 0.);
    let b = cylinder(3., 1., 3., 3.);
    let union = boolean(&a, &b, "union").unwrap();
    let decoded: Model = value_codec::from_str(&value_codec::to_string(&union).unwrap()).unwrap();
    let result = boolean(&decoded, &b, "intersection").unwrap();
    check(&result, 18. * PI);
    let trimmed = boolean(&decoded, &b, "difference").unwrap();
    check(&trimmed, 45. * PI - 2. * (6. * PI - 4.5 * 3f64.sqrt()));
}
#[test]
fn blind_circular_pocket_has_real_step_caps_and_reuses_after_roundtrip() {
    let a = cylinder(3., 0., 10., 0.);
    let cutter = cylinder(1., 4., 12., 0.);
    let pocket = boolean(&a, &cutter, "difference").unwrap();
    check(&pocket, 84. * PI);
    assert_eq!(pocket.bodies.len(), 1);
    assert!(pocket.faces.iter().any(|f| {
        f.surface
            .control_points
            .iter()
            .flatten()
            .all(|p| p[2] == 4.)
    }));
    let decoded: Model = value_codec::from_str(&value_codec::to_string(&pocket).unwrap()).unwrap();
    let further = boolean(&decoded, &cylinder(1., 2., 5., 0.), "difference").unwrap();
    check(&further, 82. * PI);
    let restored = boolean(&pocket, &cylinder(1., 4., 10., 0.), "union").unwrap();
    check(&restored, 90. * PI);
}
#[test]
fn enclosed_cavity_and_annular_steps_keep_shell_and_hole_ownership() {
    let a = cylinder(3., 0., 10., 0.);
    let cavity = cylinder(1., 3., 7., 0.);
    let result = boolean(&a, &cavity, "difference").unwrap();
    check(&result, 86. * PI);
    assert_eq!(result.bodies.len(), 1);
    assert_eq!(result.bodies[0].inner_shells.len(), 1);
    let annulus = translate(brep_core::tube(4., 2., 4.).unwrap(), [0., 0., 3.]);
    let result = boolean(&a, &annulus, "union").unwrap();
    check(&result, 118. * PI);
}
#[test]
fn circle_line_steps_preserve_analytic_carrier_and_regularized_empty() {
    let a = cylinder(3., 0., 6., 0.);
    let b = brep_core::cuboid([0., -4., 2.], [4., 4., 8.]).unwrap();
    for (operation, expected) in [
        ("difference", 36. * PI),
        ("intersection", 18. * PI),
        ("union", 36. * PI + 192.),
        ("xor", 18. * PI + 192.),
    ] {
        check(
            &boolean(&a, &b, operation)
                .unwrap_or_else(|e| panic!("{operation}: {} {}", e.code, e.message)),
            expected,
        );
    }
    let stepped = boolean(&a, &cylinder(1., 3., 8., 0.), "difference").unwrap();
    assert!(
        boolean(&stepped, &stepped, "difference")
            .unwrap()
            .is_empty()
    );
}

#[test]
fn unresolved_slabs_and_nonprismatic_curves_refuse_without_changing_operands() {
    let a = cylinder(3., 0., 5., 0.);
    let near = cylinder(3., 1e-6, 3., 3.);
    let original = value_codec::to_string(&a).unwrap();
    let failure = boolean(&a, &near, "union").unwrap_err();
    assert_eq!(failure.code, "BREP_UNSUPPORTED_OPERATION");
    assert!(failure.message.contains("Z slab"));
    assert_eq!(original, value_codec::to_string(&a).unwrap());
    assert!(boolean(&a, &brep_core::frustum(3., 2., 4.).unwrap(), "union").is_err());
}

#[test]
fn reversed_outer_inner_shell_roles_are_refused_on_reuse() {
    let mut invalid = boolean(
        &cylinder(3., 0., 10., 0.),
        &cylinder(1., 3., 7., 0.),
        "difference",
    )
    .unwrap();
    let outer = invalid.bodies[0].outer_shell;
    invalid.bodies[0].outer_shell = invalid.bodies[0].inner_shells[0];
    invalid.bodies[0].inner_shells[0] = outer;
    invalid.rebuild_topology_ids();
    invalid.validate().unwrap(); // Combinatorial validation alone cannot infer shell roles.
    let before = value_codec::to_string(&invalid).unwrap();
    let error = boolean(&invalid, &cylinder(1., 0., 5., 2.), "difference").unwrap_err();
    assert_eq!(error.code, "BREP_UNSUPPORTED_OPERATION");
    assert!(error.message.contains("shell role"));
    assert_eq!(before, value_codec::to_string(&invalid).unwrap());
}
