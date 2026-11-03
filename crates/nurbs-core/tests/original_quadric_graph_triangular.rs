use nurbs_core::{
    curve::Curve,
    intersection::certified_curve_surface::{
        original_quadric_graph_contacts, OriginalQuadricGraphParameterization,
    },
    surface::Surface,
};
fn target() -> (Surface, [[f64; 4]; 4], Curve) {
    (
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![-2., -2., -2., 2., 2., 2.],
            knots_v: vec![10., 10., 14., 14.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 3., 0.]],
                vec![vec![1.5, 0., 0.], vec![1., 3., 1.]],
                vec![vec![3., 0., 0.], vec![3., 3., 3.]],
            ],
            weights: vec![vec![1., 1.], vec![1., 1.5], vec![1., 2.]],
            periodic_u: false,
            periodic_v: false,
        },
        [
            [0., 0.5, 0., 0.],
            [0.5, 0., 0., 0.],
            [0., 0., 0., -1.5],
            [0., 0., -1.5, 0.],
        ],
        Curve {
            degree: 1,
            knots: vec![2., 2., 6., 6.],
            control_points: vec![vec![1.5, 1.5, -0.25], vec![1.5, 1.5, 1.75]],
            weights: vec![1.; 2],
            periodic: false,
        },
    )
}
#[test]
fn nonplanar_joint_projection_has_original_natural_uv_in_both_directions() {
    for transpose in [false, true] {
        for rotated in [false, true] {
            for gauge in [1., 7.] {
                let (mut s, mut q, mut c) = target();
                if transpose {
                    s.control_points = (0..2)
                        .map(|j| (0..3).map(|i| s.control_points[i][j].clone()).collect())
                        .collect();
                    s.weights = (0..2)
                        .map(|j| (0..3).map(|i| s.weights[i][j]).collect())
                        .collect();
                    std::mem::swap(&mut s.degree_u, &mut s.degree_v);
                    std::mem::swap(&mut s.knots_u, &mut s.knots_v);
                }
                if rotated {
                    for p in s.control_points.iter_mut().flatten() {
                        p.swap(0, 2);
                    }
                    for p in &mut c.control_points {
                        p.swap(0, 2);
                    }
                    let old = q;
                    let perm = [2, 1, 0, 3];
                    q = std::array::from_fn(|i| std::array::from_fn(|j| old[perm[i]][perm[j]]));
                }
                for w in s.weights.iter_mut().flatten() {
                    *w *= gauge;
                }
                let before = s.clone();
                let r =
                    original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 1_000_000).unwrap();
                assert_eq!(s, before);
                assert!(r.complete && r.target_graph_proven, "{r:?}");
                assert_eq!(
                    r.target_parameterization,
                    OriginalQuadricGraphParameterization::TriangularRational
                );
                assert_eq!(r.contacts.len(), 1);
                let expected = if transpose {
                    [4., 11.6, 0.]
                } else {
                    [4., 0., 11.6]
                };
                for (axis, x) in expected.into_iter().enumerate() {
                    let b = r.contacts[0].parameter_box[axis];
                    assert!(b[0] <= x && x <= b[1], "{b:?} excludes {x}");
                    assert!(b[1] - b[0] < 1e-12);
                }
            }
        }
    }
}
#[test]
fn original_cross_identity_and_quadric_membership_are_both_required() {
    let (s, q, c) = target();
    let mut drift = s.clone();
    drift.control_points[1][1][0] = f64::from_bits(1f64.to_bits() + 1);
    let r = original_quadric_graph_contacts(&c, &drift, q, &[], 1e-9, 10, 1_000_000).unwrap();
    assert_eq!(r.target_parameterization,OriginalQuadricGraphParameterization::JointJacobian);
    assert_eq!(r.target_membership.outcome,cad_predicates::BezierIdentity::Different);
    assert!(!r.target_graph_proven && !r.complete && r.contacts.is_empty() && r.carrier.is_none());
    let mut wrong = s.clone();
    wrong.control_points[1][1][2] = 1.25;
    let r = original_quadric_graph_contacts(&c, &wrong, q, &[], 1e-9, 10, 1_000_000).unwrap();
    assert!(!r.target_graph_proven && r.carrier.is_none());
    let r = original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 0).unwrap();
    assert!(!r.target_graph_proven && r.carrier.is_none());
    assert_eq!(r.exact_work, 0);
}

#[test]
fn joint_projection_preserves_all_edges_corners_and_excludes_exterior() {
    for (x, y, u, v) in [
        (0., 0., -2., 10.),
        (3., 0., 2., 10.),
        (0., 3., -2., 14.),
        (3., 3., 2., 14.),
        (0., 1.5, -2., 12.),
        (3., 1.5, 2., 10. + 4. / 3.),
        (1.5, 0., 0., 10.),
        (1.5, 3., 0., 14.),
    ] {
        let (s, q, mut c) = target();
        let z = x * y / 3.;
        c.control_points = vec![vec![x, y, z - 1.], vec![x, y, z + 1.]];
        let r = original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 1_000_000).unwrap();
        assert!(r.complete && r.target_graph_proven, "{r:?}");
        assert_eq!(r.contacts.len(), 1);
        for (axis, x) in [4., u, v].into_iter().enumerate() {
            let b = r.contacts[0].parameter_box[axis];
            assert!(b[0] <= x && x <= b[1], "{b:?} excludes {x}");
        }
    }
    for (x, y) in [(-1., 1.5), (4., 1.5), (1.5, -1.), (1.5, 4.)] {
        let (s, q, mut c) = target();
        let z = x * y / 3.;
        c.control_points = vec![vec![x, y, z - 1.], vec![x, y, z + 1.]];
        let r = original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 1_000_000).unwrap();
        assert!(
            r.complete && r.target_graph_proven && r.contacts.is_empty(),
            "{r:?}"
        );
    }
}
