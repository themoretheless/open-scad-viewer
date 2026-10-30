use super::*;
use cad_predicates::BezierIdentity as B;
fn partial() -> (Curve, Curve, Surface) {
    let s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![2., 2., 2., 3., 3., 4., 4., 5., 5., 5.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..7)
            .map(|i| vec![vec![i as f64, 0., 0.], vec![i as f64, 1., 0.]])
            .collect(),
        weights: (0..7)
            .map(|i| vec![if i % 2 == 0 { 1. } else { 2. }; 2])
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let c = Curve {
        degree: 2,
        knots: vec![3., 3., 3., 4., 4., 5., 5., 5.],
        control_points: s.control_points[2..].iter().map(|r| r[0].clone()).collect(),
        weights: s.weights[2..].iter().map(|r| r[0]).collect(),
        periodic: false,
    };
    let p = Curve::from_polyline(vec![vec![3., 0.], vec![5., 0.]]).unwrap();
    (c, p, s)
}
#[test]
fn original_c0_subborder_and_exact_integer_reversal() {
    let (c, p, s) = partial();
    for reverse in [false, true] {
        let c = if reverse {
            c.reverse().unwrap()
        } else {
            c.clone()
        };
        let r = verify_exact(&c, &p, &s, reverse, 100000).unwrap().unwrap();
        assert_eq!(r.outcome, B::Equal);
    }
    let mut altered = c.clone();
    altered.control_points[1][2] = 0_f64.next_up();
    assert!(
        !verify_exact(&altered, &p, &s, false, 100000)
            .unwrap()
            .is_some_and(|r| r.outcome == B::Equal)
    );
}
#[test]
fn signed_world_plane_keeps_complete_rational_multispan_basis() {
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-5., -5., -2., -2.],
        knots_v: vec![10., 10., 14., 14.],
        control_points: vec![
            vec![vec![5., 10., 0.], vec![5., 14., 0.]],
            vec![vec![2., 10., 0.], vec![2., 14., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 2., 2., 2.],
        control_points: vec![
            vec![2., 10., 0.],
            vec![2.5, 11., 0.],
            vec![3., 12., 0.],
            vec![4., 13., 0.],
            vec![5., 14., 0.],
        ],
        weights: vec![1., 2., 1., 2., 1.],
        periodic: false,
    };
    let mut p = c.clone();
    p.control_points = c.control_points.iter().map(|p| vec![-p[0], p[1]]).collect();
    for reverse in [false, true] {
        let p = if reverse {
            p.reverse().unwrap()
        } else {
            p.clone()
        };
        assert_eq!(
            verify_exact(&c, &p, &s, reverse, 100000)
                .unwrap()
                .unwrap()
                .outcome,
            B::Equal
        );
    }
    let mut shifted = p.clone();
    shifted.knots[3] = 1_f64.next_up();
    shifted.knots[4] = 1_f64.next_up();
    assert!(
        !verify_exact(&c, &shifted, &s, false, 100000)
            .unwrap()
            .is_some_and(|r| r.outcome == B::Equal)
    );
    let mut outside = c.clone();
    outside.control_points[0][0] = 1.;
    let mut outside_pc = outside.clone();
    outside_pc.control_points = outside
        .control_points
        .iter()
        .map(|p| vec![-p[0], p[1]])
        .collect();
    assert!(
        !verify_exact(&outside, &outside_pc, &s, false, 100000)
            .unwrap()
            .is_some_and(|r| r.outcome == B::Equal)
    );
}
#[test]
fn literal_constant_pole_and_zero_work_are_distinct() {
    let mut s = partial().2;
    for row in &mut s.control_points {
        for p in row {
            p[2] = 7.;
        }
    }
    s.control_points[0].fill(vec![0., 0., 7.]);
    let c = Curve::from_polyline(vec![vec![0., 0., 7.]; 2]).unwrap();
    let p = Curve::from_polyline(vec![vec![2., 0.], vec![2., 1.]]).unwrap();
    assert_eq!(
        verify_exact(&c, &p, &s, false, 100000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Equal
    );
    assert!(
        !verify_exact(&c, &p, &s, false, 0)
            .unwrap()
            .is_some_and(|r| r.outcome == B::Equal)
    );
    s.control_points[0][1][0] = 0_f64.next_up();
    assert!(
        !verify_exact(&c, &p, &s, false, 100000)
            .unwrap()
            .is_some_and(|r| r.outcome == B::Equal)
    );
}
