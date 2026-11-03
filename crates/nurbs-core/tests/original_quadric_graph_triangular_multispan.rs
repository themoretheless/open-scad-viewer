use nurbs_core::{
    curve::Curve,
    intersection::certified_curve_surface::{
        original_quadric_graph_contacts, OriginalQuadricGraphParameterization,
    },
    surface::Surface,
};
fn target(x: f64, y: f64) -> (Surface, [[f64; 4]; 4], Curve) {
    let l = 1680.;
    let a = [0., 1., 2., 4.];
    let c = [0., 0.25, 0.75, 1.];
    let weights = (0..4)
        .map(|i| {
            a.iter()
                .map(|a| [1., 1. + a / 4., 1. + 3. * a / 4., 1. + a][i])
                .collect()
        })
        .collect();
    let control_points = (0..4)
        .map(|i| {
            a.iter()
                .enumerate()
                .map(|(j, a)| {
                    let x = [0., l / (4. + a), l * (3. + 2. * a) / (4. + 3. * a), l][i];
                    vec![x, l * c[j], x * c[j]]
                })
                .collect()
        })
        .collect();
    let s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![-2., -2., -2., 0., 2., 2., 2.],
        knots_v: vec![10., 10., 11., 13., 14., 14.],
        control_points,
        weights,
        periodic_u: false,
        periodic_v: false,
    };
    let q = [
        [0., 0.5, 0., 0.],
        [0.5, 0., 0., 0.],
        [0., 0., 0., -l / 2.],
        [0., 0., -l / 2., 0.],
    ];
    let z = x * y / l;
    let c = Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: vec![vec![x, y, z - 1.], vec![x, y, z + 1.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    (s, q, c)
}
#[test]
fn original_joint_multispan_target_preserves_internal_knots_and_conditional_uv() {
    for (x, y, u, v) in [
        (840., 840., 0., 11. + 6. / 7.),
        (420., 840., -1., 11. + 10. / 11.),
        (840., 420., 0., 11.),
    ] {
        for transpose in [false, true] {
            for rotated in [false, true] {
                for gauge in [1., 7.] {
                    let (mut s, mut q, mut c) = target(x, y);
                    if transpose {
                        s.control_points = (0..4)
                            .map(|j| (0..4).map(|i| s.control_points[i][j].clone()).collect())
                            .collect();
                        s.weights = (0..4)
                            .map(|j| (0..4).map(|i| s.weights[i][j]).collect())
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
                    let r = original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 1_000_000)
                        .unwrap();
                    assert_eq!(before, s);
                    assert!(r.complete && r.target_graph_proven, "{r:?}");
                    assert_eq!(
                        r.target_parameterization,
                        OriginalQuadricGraphParameterization::TriangularRational
                    );
                    assert_eq!(r.contacts.len(), 1);
                    let expected = if transpose { [4., v, u] } else { [4., u, v] };
                    for (axis, x) in expected.into_iter().enumerate() {
                        let b = r.contacts[0].parameter_box[axis];
                        assert!(b[0] <= x && x <= b[1], "{b:?} excludes {x}");
                        assert!(b[1] - b[0] < 1e-11);
                    }
                }
            }
        }
    }
}
#[test]
fn joint_multispan_original_identity_membership_and_predicate_budget_are_required() {
    let (s, q, c) = target(840., 840.);
    let mut drift = s.clone();
    drift.control_points[1][1][0] = f64::from_bits(drift.control_points[1][1][0].to_bits() + 1);
    assert!(original_quadric_graph_contacts(&c, &drift, q, &[], 1e-9, 10, 1_000_000).is_err());
    let mut wrong = s.clone();
    wrong.control_points[1][1][2] += 0.25;
    let r = original_quadric_graph_contacts(&c, &wrong, q, &[], 1e-9, 10, 1_000_000).unwrap();
    assert!(!r.target_graph_proven && r.carrier.is_none());
    let r = original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 0).unwrap();
    assert!(!r.target_graph_proven && r.carrier.is_none());
    assert_eq!(r.exact_work, 0);
}
#[test]
fn original_joint_multispan_edges_corners_and_exterior_are_classified() {
    for (x, y, u, v) in [
        (0., 0., -2., 10.),
        (1680., 0., 2., 10.),
        (0., 1680., -2., 14.),
        (1680., 1680., 2., 14.),
        (0., 840., -2., 12.),
        (1680., 840., 2., 11.8),
        (840., 0., 0., 10.),
        (840., 1680., 0., 14.),
    ] {
        let (s, q, c) = target(x, y);
        let r = original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 1_000_000).unwrap();
        assert!(r.complete && r.target_graph_proven, "{r:?}");
        assert_eq!(r.contacts.len(), 1);
        for (axis, x) in [4., u, v].into_iter().enumerate() {
            let b = r.contacts[0].parameter_box[axis];
            assert!(b[0] <= x && x <= b[1], "{b:?} excludes {x}");
        }
    }
    for (x, y) in [(-2., 840.), (1682., 840.), (840., -2.), (840., 1682.)] {
        let (s, q, c) = target(x, y);
        let r = original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 1_000_000).unwrap();
        assert!(
            r.complete && r.target_graph_proven && r.contacts.is_empty(),
            "{r:?}"
        );
    }
}
