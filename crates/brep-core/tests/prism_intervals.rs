use brep_core::{Model, analysis::mass_properties, boolean};
fn translate(mut model: Model, delta: [f64; 3]) -> Model {
    for v in &mut model.vertices {
        for i in 0..3 {
            v.point[i] += delta[i];
        }
    }
    for e in &mut model.edges {
        for p in &mut e.curve.control_points {
            for i in 0..3 {
                p[i] += delta[i];
            }
        }
    }
    for f in &mut model.faces {
        for p in f.surface.control_points.iter_mut().flatten() {
            for i in 0..3 {
                p[i] += delta[i];
            }
        }
    }
    model.validate().unwrap();
    model
}
fn cylinder(r: f64, interval: [f64; 2]) -> Model {
    translate(
        brep_core::cylinder(r, interval[1] - interval[0]).unwrap(),
        [0., 0., interval[0]],
    )
}
fn intervals(model: &Model) -> Vec<[f64; 2]> {
    let mut result = Vec::new();
    for body in &model.bodies {
        let mut low = f64::INFINITY;
        let mut high = f64::NEG_INFINITY;
        for usage in &model.shells[body.outer_shell].faces {
            for p in model.faces[usage.face]
                .surface
                .control_points
                .iter()
                .flatten()
            {
                low = low.min(p[2]);
                high = high.max(p[2]);
            }
        }
        result.push([low, high]);
    }
    result.sort_by(|a, b| a[0].total_cmp(&b[0]));
    result
}
fn check(model: &Model, expected: &[[f64; 2]], area: f64) {
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(intervals(model), expected);
    if expected.is_empty() {
        assert!(model.is_empty());
        return;
    }
    let h = expected.iter().map(|i| i[1] - i[0]).sum::<f64>();
    let mass = mass_properties(model, 1e-7, 400_000).unwrap();
    assert!(mass.signed_volume_mm3 > 0.);
    assert!((mass.signed_volume_mm3 - area * h).abs() < area * h * 1e-6);
    assert!(
        model.edges.iter().any(
            |e| e.curve.degree == 2 && e.curve.weights.iter().any(|w| *w != e.curve.weights[0])
        )
    );
    let encoded = value_codec::to_string(model).unwrap();
    let decoded: Model = value_codec::from_str(&encoded).unwrap();
    decoded.validate().unwrap();
    assert_eq!(model.1.faces, decoded.1.faces);
}
#[test]
fn interval_subtraction_covers_empty_one_and_two_surviving_bodies() {
    let a = cylinder(3., [0., 10.]);
    for (cut, expected) in [
        ([-2., 12.], vec![]),
        ([-2., 4.], vec![[4., 10.]]),
        ([6., 12.], vec![[0., 6.]]),
        ([3., 7.], vec![[0., 3.], [7., 10.]]),
    ] {
        for radius in [3., 4.] {
            let result = boolean(&a, &cylinder(radius, cut), "difference").unwrap();
            check(&result, &expected, 9. * std::f64::consts::PI);
            if expected.iter().any(|i| i[0] == 0.) {
                assert!(
                    result.1.faces.contains(&a.1.faces[4]),
                    "Surviving bottom cap ID must persist"
                );
            }
            if expected.iter().any(|i| i[1] == 10.) {
                assert!(
                    result.1.faces.contains(&a.1.faces[5]),
                    "Surviving top cap ID must persist"
                );
            }
        }
    }
}
#[test]
fn equal_profile_xor_and_union_regularize_touching_cap_intervals() {
    for (a, b, exclusive, joined) in [
        ([0., 5.], [0., 3.], vec![[3., 5.]], vec![[0., 5.]]),
        (
            [0., 10.],
            [3., 7.],
            vec![[0., 3.], [7., 10.]],
            vec![[0., 10.]],
        ),
        ([0., 5.], [3., 8.], vec![[0., 3.], [5., 8.]], vec![[0., 8.]]),
        ([0., 2.], [2., 5.], vec![[0., 5.]], vec![[0., 5.]]),
        (
            [0., 2.],
            [4., 6.],
            vec![[0., 2.], [4., 6.]],
            vec![[0., 2.], [4., 6.]],
        ),
        ([0., 5.], [0., 5.], vec![], vec![[0., 5.]]),
    ] {
        let a = cylinder(3., a);
        let b = cylinder(3., b);
        check(
            &boolean(&a, &b, "xor").unwrap(),
            &exclusive,
            9. * std::f64::consts::PI,
        );
        check(
            &boolean(&a, &b, "union").unwrap(),
            &joined,
            9. * std::f64::consts::PI,
        );
    }
}
#[test]
fn annular_profiles_keep_holes_in_both_interval_components() {
    let a = brep_core::tube(3., 1., 10.).unwrap();
    let b = cylinder(4., [3., 7.]);
    let result = boolean(&a, &b, "difference").unwrap();
    check(&result, &[[0., 3.], [7., 10.]], 8. * std::f64::consts::PI);
    assert_eq!(
        result.faces.iter().filter(|f| !f.holes.is_empty()).count(),
        4
    );
}
#[test]
fn disjoint_profile_difference_is_identity_and_genuine_steps_have_native_boundaries() {
    let a = cylinder(3., [0., 5.]);
    for height in [[1., 3.], [0., 5.], [-1., 6.]] {
        let touching = translate(cylinder(1., height), [4., 0., 0.]);
        let result = boolean(&a, &touching, "difference").unwrap();
        assert_eq!(
            value_codec::to_string(&a).unwrap(),
            value_codec::to_string(&result).unwrap()
        );
    }
    let small = cylinder(1., [0., 5.]);
    let annulus = translate(brep_core::tube(3., 2., 2.).unwrap(), [0., 0., 1.]);
    let result = boolean(&small, &annulus, "difference").unwrap();
    assert_eq!(
        value_codec::to_string(&small).unwrap(),
        value_codec::to_string(&result).unwrap()
    );
    let partial = translate(cylinder(3., [1., 3.]), [3., 0., 0.]);
    let overlap_volume = 2. * (6. * std::f64::consts::PI - 4.5 * 3f64.sqrt());
    for (operation, expected) in [
        ("difference", 45. * std::f64::consts::PI - overlap_volume),
        ("union", 63. * std::f64::consts::PI - overlap_volume),
    ] {
        let result = boolean(&a, &partial, operation).unwrap();
        assert_eq!(result.validate().unwrap().boundary_edge_count, 0);
        let mass = mass_properties(&result, 1e-7, 400_000).unwrap();
        assert!((mass.signed_volume_mm3 - expected).abs() < expected * 1e-6);
        assert!(
            result
                .faces
                .iter()
                .any(|f| f.surface.degree_u == 2 || f.surface.degree_v == 2)
        );
    }
}
#[test]
fn rotated_interval_components_preserve_volume_and_outward_orientation() {
    let rotate = |mut model: Model| {
        let (s, c) = 0.73f64.sin_cos();
        let map = |p: [f64; 3]| {
            [
                c * p[0] + s * p[2] + 7.,
                p[1] - 9.,
                -s * p[0] + c * p[2] + 11.,
            ]
        };
        for v in &mut model.vertices {
            v.point = map(v.point);
        }
        for e in &mut model.edges {
            for p in &mut e.curve.control_points {
                *p = map([p[0], p[1], p[2]]).to_vec();
            }
        }
        for f in &mut model.faces {
            for p in f.surface.control_points.iter_mut().flatten() {
                *p = map([p[0], p[1], p[2]]).to_vec();
            }
        }
        model.validate().unwrap();
        model
    };
    for operation in ["difference", "xor"] {
        let a = rotate(cylinder(3., [0., 10.]));
        let b = rotate(cylinder(3., [3., 7.]));
        let result = boolean(&a, &b, operation).unwrap();
        assert_eq!(result.bodies.len(), 2);
        result.validate().unwrap();
        let mass = mass_properties(&result, 1e-7, 400_000).unwrap();
        assert!((mass.signed_volume_mm3 - 54. * std::f64::consts::PI).abs() < 1e-5);
    }
}
