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
        "../../../tools/browser-qualification/joint-quadric-graph-fixtures.json"
    ))
    .unwrap()
}
#[test]
fn original_joint_nonplanar_polynomial_and_rational_maps_have_certified_uv() {
    for f in fixtures().into_iter().take(2) {
        for transpose in [false, true] {
            for rotated in [false, true] {
                for gauge in [1., 7.] {
                    let q = &f["query"];
                    let mut s: Surface = from_value(q["patch"].clone()).unwrap();
                    let mut c: Curve = from_value(q["curve"].clone()).unwrap();
                    let mut matrix: [[f64; 4]; 4] = from_value(q["quadric"].clone()).unwrap();
                    if transpose {
                        let (n, m) = (s.control_points.len(), s.control_points[0].len());
                        s.control_points = (0..m)
                            .map(|j| (0..n).map(|i| s.control_points[i][j].clone()).collect())
                            .collect();
                        s.weights = (0..m)
                            .map(|j| (0..n).map(|i| s.weights[i][j]).collect())
                            .collect();
                        std::mem::swap(&mut s.degree_u, &mut s.degree_v);
                    }
                    // Preserve natural domains when transposing the tensor.
                    if transpose {
                        s.knots_u = [vec![-2.; s.degree_u + 1], vec![2.; s.degree_u + 1]].concat();
                        s.knots_v = [vec![10.; s.degree_v + 1], vec![14.; s.degree_v + 1]].concat();
                    }
                    if rotated {
                        for p in s.control_points.iter_mut().flatten() {
                            p.swap(0, 2);
                        }
                        for p in &mut c.control_points {
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
                    let before = s.clone();
                    let r =
                        original_quadric_graph_contacts(&c, &s, matrix, &[], 1e-9, 10, 1_000_000)
                            .unwrap();
                    assert_eq!(before, s);
                    assert!(r.target_graph_proven && r.complete, "{}: {r:?}", f["name"]);
                    assert_eq!(
                        r.target_parameterization,
                        OriginalQuadricGraphParameterization::JointJacobian
                    );
                    assert_eq!(r.contacts.len(), 1);
                    for (axis, x) in [4., 0., 12.].into_iter().enumerate() {
                        let b = r.contacts[0].parameter_box[axis];
                        assert!(b[0] <= x && x <= b[1], "{b:?} excludes {x}");
                        assert!(b[1] - b[0] < 1e-9, "Unrefined inverse: {b:?}");
                    }
                }
            }
        }
    }
}
#[test]
fn folded_joint_map_and_wrong_original_boundary_never_certify() {
    let f = fixtures();
    let q = &f[2]["query"];
    let s: Surface = from_value(q["patch"].clone()).unwrap();
    let c: Curve = from_value(q["curve"].clone()).unwrap();
    let matrix = from_value(q["quadric"].clone()).unwrap();
    assert!(original_quadric_graph_contacts(&c, &s, matrix, &[], 1e-9, 10, 1_000_000).is_err());
    let q = &f[0]["query"];
    let mut s: Surface = from_value(q["patch"].clone()).unwrap();
    let c: Curve = from_value(q["curve"].clone()).unwrap();
    let matrix = from_value(q["quadric"].clone()).unwrap();
    s.control_points[0][1][0] = f64::from_bits(1);
    assert!(original_quadric_graph_contacts(&c, &s, matrix, &[], 1e-9, 10, 1_000_000).is_err());
}
#[test]
fn joint_chart_still_requires_original_membership_and_shared_predicate_budget() {
    let f = fixtures();
    let q = &f[0]["query"];
    let s: Surface = from_value(q["patch"].clone()).unwrap();
    let c: Curve = from_value(q["curve"].clone()).unwrap();
    let matrix = from_value(q["quadric"].clone()).unwrap();
    let mut wrong = s.clone();
    wrong.control_points[1][1][2] += 0.25;
    let r = original_quadric_graph_contacts(&c, &wrong, matrix, &[], 1e-9, 10, 1_000_000).unwrap();
    assert!(!r.target_graph_proven && r.carrier.is_none());
    let r = original_quadric_graph_contacts(&c, &s, matrix, &[], 1e-9, 10, 0).unwrap();
    assert!(!r.target_graph_proven && r.carrier.is_none());
    assert_eq!(r.exact_work, 0);
}

#[test]
fn joint_inverse_keeps_noncentral_contacts_edges_corners_and_exterior() {
    for f in fixtures().into_iter().take(2) {
        let q = &f["query"];
        let s: Surface = from_value(q["patch"].clone()).unwrap();
        let mut c: Curve = from_value(q["curve"].clone()).unwrap();
        let matrix = from_value(q["quadric"].clone()).unwrap();
        let l = f["scale"].as_f64().unwrap();
        for (a, b, u, v) in [
            (73. / 256., 201. / 256., -1., 13.),
            (201. / 256., 73. / 256., 1., 11.),
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
                assert!(b[1] - b[0] < 1e-9, "unrefined {b:?}");
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

#[test]
fn degree_eight_original_joint_map_retains_exact_unique_contact() {
    let f = fixtures();
    let q = &f[3]["query"];
    let s: Surface = from_value(q["patch"].clone()).unwrap();
    let c: Curve = from_value(q["curve"].clone()).unwrap();
    let matrix = from_value(q["quadric"].clone()).unwrap();
    let r = original_quadric_graph_contacts(&c, &s, matrix, &[], 1e-9, 10, 1_000_000).unwrap();
    assert!(r.complete && r.target_graph_proven, "{r:?}");
    assert_eq!(
        r.target_parameterization,
        OriginalQuadricGraphParameterization::JointJacobian
    );
    assert_eq!(r.contacts.len(), 1);
    for (axis, x) in [4., 0., 12.].into_iter().enumerate() {
        assert_eq!(r.contacts[0].parameter_box[axis], [x, x]);
    }
}
