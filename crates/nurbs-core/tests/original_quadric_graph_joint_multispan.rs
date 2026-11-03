use nurbs_core::{
    curve::Curve,
    intersection::certified_curve_surface::{
        original_quadric_graph_contacts, OriginalQuadricGraphParameterization,
    },
    surface::Surface,
};
use value_codec::{from_str, from_value, Value};
fn fixtures() -> Vec<Value> {
    from_str(include_str!(
        "../../../tools/browser-qualification/joint-multispan-quadric-graph-fixtures.json"
    ))
    .unwrap()
}
#[test]
fn joint_original_multispan_has_global_uv_at_knots_and_inside_cells() {
    for f in fixtures().into_iter().take(2) {
        for transpose in [false, true] {
            for rotated in [false, true] {
                for gauge in [1., 7.] {
                    let q = &f["query"];
                    let mut s: Surface = from_value(q["patch"].clone()).unwrap();
                    let mut c: Curve = from_value(q["curve"].clone()).unwrap();
                    let mut matrix: [[f64; 4]; 4] = from_value(q["quadric"].clone()).unwrap();
                    let l = f["scale"].as_f64().unwrap();
                    if transpose {
                        let (n, m) = (s.control_points.len(), s.control_points[0].len());
                        s.control_points = (0..m)
                            .map(|j| (0..n).map(|i| s.control_points[i][j].clone()).collect())
                            .collect();
                        s.weights = (0..m)
                            .map(|j| (0..n).map(|i| s.weights[i][j]).collect())
                            .collect();
                        std::mem::swap(&mut s.degree_u, &mut s.degree_v);
                        std::mem::swap(&mut s.knots_u, &mut s.knots_v);
                    }
                    if rotated {
                        for p in s.control_points.iter_mut().flatten() {
                            p.swap(0, 2);
                        }
                        let old = matrix;
                        let perm = [2, 1, 0, 3];
                        matrix =
                            std::array::from_fn(|i| std::array::from_fn(|j| old[perm[i]][perm[j]]));
                    }
                    for w in s.weights.iter_mut().flatten() {
                        *w *= gauge;
                    }
                    for (a, b, u, v) in [
                        (9. / 16., 9. / 16., 0., 12.),
                        (73. / 256., 201. / 256., -1., 13.),
                    ] {
                        let (x, y) = (l * a, l * b);
                        let z = x * y / l;
                        c.control_points = vec![vec![x, y, z - 1.], vec![x, y, z + 1.]];
                        if rotated {
                            for p in &mut c.control_points {
                                p.swap(0, 2);
                            }
                        }
                        let before = s.clone();
                        let r = original_quadric_graph_contacts(
                            &c,
                            &s,
                            matrix,
                            &[],
                            1e-9,
                            10,
                            1_000_000,
                        )
                        .unwrap();
                        assert_eq!(before, s);
                        assert!(r.complete && r.target_graph_proven, "{r:?}");
                        assert_eq!(
                            r.target_parameterization,
                            OriginalQuadricGraphParameterization::JointJacobian
                        );
                        assert_eq!(r.contacts.len(), 1);
                        let expected = if transpose { [4., v, u] } else { [4., u, v] };
                        for (axis, x) in expected.into_iter().enumerate() {
                            let b = r.contacts[0].parameter_box[axis];
                            assert!(b[0] <= x && x <= b[1], "{b:?} excludes {x}");
                            assert!(b[1] - b[0] < 1e-9);
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn folded_and_discontinuous_original_joint_charts_refuse() {
    for f in fixtures().into_iter().skip(2).take(2) {
        let q = &f["query"];
        let s: Surface = from_value(q["patch"].clone()).unwrap();
        let c: Curve = from_value(q["curve"].clone()).unwrap();
        let matrix = from_value(q["quadric"].clone()).unwrap();
        assert!(original_quadric_graph_contacts(&c, &s, matrix, &[], 1e-9, 10, 1_000_000).is_err());
    }
}
#[test]
fn original_multispan_joint_membership_and_budget_are_not_replaced_by_chart_proof() {
    let f = fixtures();
    let q = &f[0]["query"];
    let s: Surface = from_value(q["patch"].clone()).unwrap();
    let c: Curve = from_value(q["curve"].clone()).unwrap();
    let matrix = from_value(q["quadric"].clone()).unwrap();
    let mut wrong = s.clone();
    wrong.control_points[1][1][2] += 0.25;
    let r = original_quadric_graph_contacts(&c, &wrong, matrix, &[], 1e-9, 10, 1_000_000).unwrap();
    assert!(!r.target_graph_proven && r.contacts.is_empty() && r.carrier.is_none());
    let r = original_quadric_graph_contacts(&c, &s, matrix, &[], 1e-9, 10, 0).unwrap();
    assert!(!r.target_graph_proven && r.carrier.is_none());
    assert_eq!(r.exact_work, 0);
}

#[test]
fn genuine_c0_joint_chart_has_unique_contact_on_both_sides_and_on_the_knot() {
    let f = fixtures();
    let q = &f[4]["query"];
    let s: Surface = from_value(q["patch"].clone()).unwrap();
    let mut c: Curve = from_value(q["curve"].clone()).unwrap();
    let matrix = from_value(q["quadric"].clone()).unwrap();
    let l = f[4]["scale"].as_f64().unwrap();
    for (a, b, u, v) in [
        (0.5, 0.5, 0., 12.),
        (67. / 256., 195. / 256., -1., 13.),
        (195. / 256., 67. / 256., 1., 11.),
    ] {
        let (x, y) = (l * a, l * b);
        let z = x * y / l;
        c.control_points = vec![vec![x, y, z - 1.], vec![x, y, z + 1.]];
        let r = original_quadric_graph_contacts(&c, &s, matrix, &[], 1e-9, 10, 1_000_000).unwrap();
        assert!(r.complete && r.target_graph_proven, "{r:?}");
        assert_eq!(r.contacts.len(), 1);
        for (axis, x) in [4., u, v].into_iter().enumerate() {
            let b = r.contacts[0].parameter_box[axis];
            assert!(b[0] <= x && x <= b[1], "{b:?} excludes {x}");
            assert!(b[1] - b[0] < 1e-9);
        }
    }
}

#[test]
fn original_multispan_joint_outer_boundary_and_exterior_are_exactly_classified() {
    for index in [0, 1, 4] {
        let f = fixtures();
        let q = &f[index]["query"];
        let s: Surface = from_value(q["patch"].clone()).unwrap();
        let mut c: Curve = from_value(q["curve"].clone()).unwrap();
        let matrix = from_value(q["quadric"].clone()).unwrap();
        let l = f[index]["scale"].as_f64().unwrap();
        for (a, b, u, v) in [
            (0., 0., -2., 10.),
            (1., 0., 2., 10.),
            (0., 1., -2., 14.),
            (1., 1., 2., 14.),
            (0., 0.5, -2., 12.),
            (1., 0.5, 2., 12.),
            (0.5, 0., 0., 10.),
            (0.5, 1., 0., 14.),
        ] {
            let (x, y) = (l * a, l * b);
            let z = x * y / l;
            c.control_points = vec![vec![x, y, z - 1.], vec![x, y, z + 1.]];
            let r =
                original_quadric_graph_contacts(&c, &s, matrix, &[], 1e-9, 10, 1_000_000).unwrap();
            assert!(r.complete && r.target_graph_proven, "{r:?}");
            assert_eq!(r.contacts.len(), 1);
            for (axis, x) in [4., u, v].into_iter().enumerate() {
                let b = r.contacts[0].parameter_box[axis];
                assert!(b[0] <= x && x <= b[1], "{b:?} excludes {x}");
            }
        }
        for (x, y) in [
            (-2., l / 2.),
            (l + 2., l / 2.),
            (l / 2., -2.),
            (l / 2., l + 2.),
        ] {
            let z = x * y / l;
            c.control_points = vec![vec![x, y, z - 1.], vec![x, y, z + 1.]];
            let r =
                original_quadric_graph_contacts(&c, &s, matrix, &[], 1e-9, 10, 1_000_000).unwrap();
            assert!(
                r.complete && r.target_graph_proven && r.contacts.is_empty(),
                "{r:?}"
            );
        }
    }
}
