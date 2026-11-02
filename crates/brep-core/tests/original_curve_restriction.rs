use brep_core::profile_certificate::original_restriction as r;
use nurbs_core::curve::Curve;
fn q(n: i64, d: i64) -> [String; 2] {
    [n.to_string(), d.to_string()]
}
#[test]
fn nonbinary_source_interval_and_point_remain_exact_rational() {
    let source = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![1., 0.]],
        weights: vec![1., 3.],
        periodic: false,
    };
    let restriction = r::restrict(&source, [q(1, 3), q(2, 3)]).unwrap();
    assert_eq!(restriction.source(), &source);
    assert_eq!(
        restriction.evaluate_natural(q(1, 3)).unwrap(),
        [q(3, 5), q(0, 1)]
    );
    assert_eq!(
        restriction.evaluate_natural(q(2, 3)).unwrap(),
        [q(6, 7), q(0, 1)]
    );
    restriction.reverify().unwrap();
    assert!(restriction.evaluate_natural(q(0, 1)).is_err());
    let reversed = r::restrict(&source, [q(2, 3), q(1, 3)]).unwrap();
    assert_eq!(
        reversed.evaluate_natural(q(1, 2)).unwrap(),
        [q(3, 4), q(0, 1)]
    );
    assert_eq!(
        restriction.spans()[0]
            .homogeneous_controls()
            .iter()
            .rev()
            .cloned()
            .collect::<Vec<_>>(),
        reversed.spans()[0].homogeneous_controls()
    );
}
#[test]
fn original_quadratic_support_is_not_replaced_by_rounded_trim_controls() {
    let source = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![4., 0.], vec![5., 4.], vec![4., 8.]],
        weights: vec![1., 0.7, 1.],
        periodic: false,
    };
    let restriction = r::restrict_binary64(&source, [0., 0.75]).unwrap();
    restriction.reverify().unwrap();
    let midpoint = restriction.evaluate_natural(q(1, 2)).unwrap();
    // The source weight is the EXACT binary64 value of 0.7, not decimal 7/10.
    assert_eq!(midpoint[1], q(4, 1));
    assert_ne!(midpoint[0], q(75, 17));
    assert_eq!(restriction.source().control_points, source.control_points);
    assert!(r::restrict(&source, [q(-1, 1), q(1, 1)]).is_err());
    assert!(r::restrict(&source, [q(1, 2), q(1, 2)]).is_err());
}

#[test]
fn original_internal_knot_clock_is_retained_and_discontinuous_source_refuses() {
    let source = Curve {
        degree: 1,
        knots: vec![2., 2., 3., 5., 5.],
        control_points: vec![vec![0., 0.], vec![1., 1.], vec![3., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let restriction = r::restrict(&source, [q(5, 2), q(4, 1)]).unwrap();
    assert_eq!(restriction.spans().len(), 2);
    assert_eq!(restriction.spans()[0].parameter_range(), [q(5, 2), q(3, 1)]);
    assert_eq!(restriction.spans()[1].parameter_range(), [q(3, 1), q(4, 1)]);
    assert_eq!(
        restriction.evaluate_natural(q(3, 1)).unwrap(),
        [q(1, 1), q(1, 1)]
    );
    assert_eq!(
        restriction.evaluate_natural(q(4, 1)).unwrap(),
        [q(2, 1), q(1, 1)]
    );
    restriction.reverify().unwrap();
    let broken = Curve {
        degree: 1,
        knots: vec![0., 0., 0.5, 0.5, 1., 1.],
        control_points: vec![vec![0., 0.], vec![1., 0.], vec![2., 0.], vec![3., 0.]],
        weights: vec![1.; 4],
        periodic: false,
    };
    assert!(r::restrict_binary64(&broken, [0., 1.]).is_err());
}
