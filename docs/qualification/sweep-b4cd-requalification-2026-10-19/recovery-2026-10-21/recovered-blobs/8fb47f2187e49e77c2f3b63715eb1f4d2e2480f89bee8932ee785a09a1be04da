use brep_core::{Model, analysis::mass_properties, boolean};
fn transform(model: &Model, angles: [f64; 3], translation: [f64; 3], reflection: bool) -> Model {
    let apply = |mut p: [f64; 3]| {
        if reflection {
            p[0] = -p[0];
        }
        let (sx, cx) = angles[0].sin_cos();
        let (sy, cy) = angles[1].sin_cos();
        let (sz, cz) = angles[2].sin_cos();
        let p = [p[0], cx * p[1] - sx * p[2], sx * p[1] + cx * p[2]];
        let p = [cy * p[0] + sy * p[2], p[1], -sy * p[0] + cy * p[2]];
        [
            cz * p[0] - sz * p[1] + translation[0],
            sz * p[0] + cz * p[1] + translation[1],
            p[2] + translation[2],
        ]
    };
    let mut result = model.clone();
    for v in &mut result.vertices {
        v.point = apply(v.point);
    }
    for e in &mut result.edges {
        for p in &mut e.curve.control_points {
            *p = apply([p[0], p[1], p[2]]).to_vec();
        }
    }
    for f in &mut result.faces {
        for p in f.surface.control_points.iter_mut().flatten() {
            *p = apply([p[0], p[1], p[2]]).to_vec();
        }
    }
    if reflection {
        for shell in &mut result.shells {
            for usage in &mut shell.faces {
                usage.reversed ^= true;
            }
        }
    }
    result.validate().unwrap();
    result
}
fn volume(model: &Model, expected: f64) {
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
    let mass = mass_properties(model, 1e-7, 400_000).unwrap();
    assert!(mass.signed_volume_mm3 > 0., "Outward orientation lost");
    assert!(
        (mass.signed_volume_mm3 - expected).abs() < expected * 1e-6,
        "Expected {expected}, got {}",
        mass.signed_volume_mm3
    );
}
#[test]
fn public_cylinder_booleans_preserve_equal_heights_after_rigid_pose_and_reflection() {
    let a = brep_core::cylinder(3., 5.).unwrap();
    let b = transform(&a, [0.; 3], [3., 0., 0.], false);
    let intersection = 6. * std::f64::consts::PI - 4.5 * 3f64.sqrt();
    for (angles, reflection) in [
        ([0.; 3], false),
        ([0.2, 0.7, 0.4], false),
        ([1.1, -0.4, 2.1], true),
    ] {
        let a = transform(&a, angles, [7., -9., 11.], reflection);
        let b = transform(&b, angles, [7., -9., 11.], reflection);
        for (operation, area) in [
            ("union", 18. * std::f64::consts::PI - intersection),
            ("intersection", intersection),
            ("difference", 9. * std::f64::consts::PI - intersection),
        ] {
            let result = boolean(&a, &b, operation)
                .unwrap_or_else(|e| panic!("{angles:?}, mirror {reflection}, {operation}: {e:?}"));
            volume(&result, area * 5.);
        }
    }
}
#[test]
fn public_cylinder_box_through_cut_and_differing_height_intersection_work_in_arbitrary_axes() {
    let cylinder = brep_core::cylinder(3., 5.).unwrap();
    let cutter = brep_core::cuboid([0., -4., -2.], [4., 4., 8.]).unwrap();
    let other = transform(
        &brep_core::cylinder(3., 6.).unwrap(),
        [0.; 3],
        [3., 0., 2.],
        false,
    );
    for reflection in [false, true] {
        let pose = [0.7, 1.2, -0.4];
        let translation = [-3., 7., 9.];
        let a = transform(&cylinder, pose, translation, reflection);
        let b = transform(&cutter, pose, translation, reflection);
        volume(
            &boolean(&a, &b, "difference").unwrap(),
            22.5 * std::f64::consts::PI,
        );
        let b = transform(&other, pose, translation, reflection);
        volume(
            &boolean(&a, &b, "intersection").unwrap(),
            3. * (6. * std::f64::consts::PI - 4.5 * 3f64.sqrt()),
        );
    }
}
