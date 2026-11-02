use num_bigint::BigInt;
use num_rational::BigRational as Q;
use nurbs_core::{
    original_graph_transition::{
        Budget, frame_distance_at_least, opposed_normal_chords_excluded, polynomial_support_frame,
    },
    surface::Surface,
};
fn plane(z: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., z], vec![0., 1., z]],
            vec![vec![1., 0., z], vec![1., 1., z]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn complete_planar_frame_has_oriented_exact_axis_normal() {
    let mut b = Budget::new(100_000).unwrap();
    assert_eq!(
        polynomial_support_frame(&plane(0.), false, &mut b)
            .unwrap()
            .normal(),
        Some([[0., 0.], [0., 0.], [1., 1.]])
    );
    assert_eq!(
        polynomial_support_frame(&plane(0.), true, &mut b)
            .unwrap()
            .normal(),
        Some([[0., 0.], [0., 0.], [-1., -1.]])
    );
    assert!(polynomial_support_frame(&plane(0.), false, &mut Budget::new(0).unwrap()).is_err());
}
#[test]
fn a_folded_planar_chart_does_not_inherit_a_corner_normal() {
    let s = Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..2)
            .map(|i| {
                [0.25, -0.25, 0.25]
                    .into_iter()
                    .map(|y| vec![i as f64, y, 0.])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    assert!(
        polynomial_support_frame(&s, false, &mut Budget::new(100_000).unwrap())
            .unwrap()
            .normal()
            .is_none()
    );
}
#[test]
fn normal_opposition_uses_unit_hemisphere_correlation_without_ignoring_self_pairs() {
    let up = [[0., 0.], [0., 0.], [1., 1.]];
    let down = [[0., 0.], [0., 0.], [-1., -1.]];
    assert!(opposed_normal_chords_excluded(up, up, 1e-6).unwrap());
    assert!(!opposed_normal_chords_excluded(up, down, 1e-6).unwrap());
    let wide = [[-1., 1.], [-1., 1.], [0.01, 1.]];
    assert!(opposed_normal_chords_excluded(wide, wide, 1e-6).unwrap());
    assert!(opposed_normal_chords_excluded(up, down, 0.5).is_err());
    let invalid = [[2., 3.], [0., 0.], [1., 1.]];
    assert!(opposed_normal_chords_excluded(invalid, up, 1e-6).is_err());
}
#[test]
fn exact_distance_accepts_equality_and_refuses_a_subfloating_increment() {
    let mut b = Budget::new(100_000).unwrap();
    let a = polynomial_support_frame(&plane(0.), false, &mut b).unwrap();
    let z = polynomial_support_frame(&plane(2.), true, &mut b).unwrap();
    let two = Q::from_integer(2.into());
    assert!(frame_distance_at_least(&a, &z, &two, &mut b).unwrap());
    let more = two + Q::new(1.into(), BigInt::from(1) << 100usize);
    assert!(!frame_distance_at_least(&a, &z, &more, &mut b).unwrap());
}
#[test]
fn exact_plane_inverse_preserves_non_dyadic_uv_and_refuses_off_support_points() {
    use nurbs_core::original_graph_transition::polynomial_plane_point;
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-1., -1., 3., 3.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![-1., -1., -4.], vec![-1., -1., 2.]],
            vec![vec![3., -1., -4.], vec![3., -1., 4.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let mut b = Budget::new(100_000).unwrap();
    let q = |n: i64| Q::from_integer(n.into());
    let p = polynomial_plane_point(&s, 1, [q(1), q(-1), q(-2)], &mut b)
        .unwrap()
        .unwrap();
    assert_eq!(p, [q(1), Q::new(2.into(), 7.into())]);
    assert!(
        polynomial_plane_point(&s, 1, [q(1), q(0), q(-2)], &mut b)
            .unwrap()
            .is_none()
    );
    assert!(
        polynomial_plane_point(&s, 1, [q(1), q(-1), q(4)], &mut b)
            .unwrap()
            .is_none()
    );
    assert!(
        polynomial_plane_point(&s, 1, [q(1), q(-1), q(-2)], &mut Budget::new(0).unwrap()).is_err()
    );
}
#[test]
fn strict_uv_membership_refuses_edge_touching_and_broken_loops() {
    use nurbs_core::original_graph_transition::convex_uv_strict_contains;
    let q = |n: i64| Q::from_integer(n.into());
    let vertices = [[q(0), q(0)], [q(1), q(0)], [q(1), q(1)], [q(0), q(1)]];
    let lines = (0..4)
        .map(|i| [vertices[i].clone(), vertices[(i + 1) % 4].clone()])
        .collect::<Vec<_>>();
    let mut b = Budget::new(100_000).unwrap();
    assert!(
        convex_uv_strict_contains(
            &lines,
            &[Q::new(1.into(), 2.into()), Q::new(1.into(), 2.into())],
            &mut b
        )
        .unwrap()
    );
    assert!(
        !convex_uv_strict_contains(&lines, &[q(0), Q::new(1.into(), 2.into())], &mut b).unwrap()
    );
    let mut bad = lines;
    bad[0][1][0] = q(2);
    assert!(convex_uv_strict_contains(&bad, &[q(0), q(0)], &mut b).is_err());
}
