use nurbs_core::{
    curve::Curve,
    trim_domain::{exact_loop_joins, exact_trimmed_loop_joins as joins},
};
fn line(a: [f64; 2], b: [f64; 2], d: [f64; 2], w: [f64; 2]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![d[0], d[0], d[1], d[1]],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: w.to_vec(),
        periodic: false,
    }
}
fn square() -> (Vec<Curve>, Vec<[f64; 2]>) {
    let curves = vec![
        line([-1., 0.], [3., 0.], [2., 6.], [1.; 2]),
        line([1., -1.], [1., 3.], [2., 6.], [1.; 2]),
        line([2., 1.], [-2., 1.], [2., 6.], [1.; 2]),
        line([0., 2.], [0., -2.], [2., 6.], [1.; 2]),
    ];
    (curves, vec![[3., 4.]; 4])
}
#[test]
fn declared_uv_intervals_close_even_when_untrimmed_definitions_do_not() {
    let (mut c, mut ranges) = square();
    let before = c.clone();
    assert_eq!(exact_loop_joins(&c).unwrap(), Some(false));
    let r = joins(&c, &ranges, 100000).unwrap();
    assert_eq!(r.closed, Some(true));
    assert!(r.joins.iter().all(|j| *j == Some(true)));
    assert!(r.work_used > 0);
    assert_eq!(joins(&c, &ranges, r.work_used - 1).unwrap().closed, None);
    assert_eq!(c, before);
    for i in [1, 3] {
        c[i].control_points.reverse();
        ranges[i] = [5., 4.];
    }
    assert_eq!(joins(&c, &ranges, 100000).unwrap().closed, Some(true));
}
#[test]
fn rational_non_dyadic_root_ends_match_adjacent_original_endpoint_controls() {
    let mut c = vec![
        line([0., 0.], [9., 0.], [-2., 4.], [1., 4.]),
        line([8., 0.], [8., 2.], [0., 1.], [1.; 2]),
        line([8., 2.], [6., 2.], [0., 1.], [1.; 2]),
        line([6., 2.], [6., 0.], [0., 1.], [1.; 2]),
    ];
    let ranges = [[0., 2.], [0., 1.], [0., 1.], [0., 1.]];
    assert_eq!(joins(&c, &ranges, 100000).unwrap().closed, Some(true));
    c[1].control_points[0][0] = 8f64.next_up();
    let r = joins(&c, &ranges, 100000).unwrap();
    assert_eq!(r.closed, Some(false));
    assert_eq!(r.joins[0], Some(false));
}
#[test]
fn exhausted_work_leaves_unchecked_joins_visible_and_invalid_ranges_refuse() {
    let (c, ranges) = square();
    let r = joins(&c, &ranges, 1).unwrap();
    assert_eq!(r.closed, None);
    assert!(r.joins.iter().any(Option::is_none));
    assert!(r.work_used <= 1);
    assert_eq!(joins(&c, &ranges, 0).unwrap().closed, None);
    let mut bad = ranges.clone();
    bad[0] = [1., 4.];
    assert!(joins(&c, &bad, 100000).is_err());
    assert!(joins(&c, &ranges[..3], 100000).is_err());
}

fn rational_multispan() -> Curve {
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 0.5, 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![21., 42.], vec![63., 42.], vec![84., 0.]],
        weights: vec![1., 2., 2., 1.],
        periodic: false,
    }
}
#[test]
fn interior_multispan_trim_ends_close_with_original_control_endpoints() {
    let a = rational_multispan();
    let before = a.clone();
    let c = vec![a.clone(), line([60., 36.], [24., 36.], [0., 1.], [1.; 2])];
    let ranges = [[0.25, 0.75], [0., 1.]];
    let r = joins(&c, &ranges, 1000000).unwrap();
    assert_eq!(r.closed, Some(true));
    assert!(r.joins.iter().all(|j| *j == Some(true)));
    assert_eq!(joins(&c, &ranges, r.work_used - 1).unwrap().closed, None);
    let reverse = vec![a.clone(), line([24., 36.], [60., 36.], [0., 1.], [1.; 2])];
    assert_eq!(
        joins(&reverse, &[[0.75, 0.25], [0., 1.]], 1000000)
            .unwrap()
            .closed,
        Some(true)
    );
    let bad = vec![
        a.clone(),
        line([60f64.next_up(), 36.], [24., 36.], [0., 1.], [1.; 2]),
    ];
    assert_eq!(joins(&bad, &ranges, 1000000).unwrap().closed, Some(false));
    assert_eq!(a, before);
}
#[test]
fn both_sides_of_an_original_c1_knot_must_match_the_same_trim_endpoint() {
    let c = vec![
        rational_multispan(),
        line([42., 42.], [24., 36.], [0., 1.], [1.; 2]),
    ];
    assert_eq!(
        joins(&c, &[[0.25, 0.5], [0., 1.]], 1000000).unwrap().closed,
        Some(true)
    );
}
#[test]
fn c0_shared_knot_closes_but_a_discontinuous_knot_does_not_supply_an_endpoint() {
    let c0 = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
        control_points: vec![
            vec![0., 0.],
            vec![2., 4.],
            vec![4., 4.],
            vec![6., 4.],
            vec![8., 0.],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    let closing = line([4., 4.], [2., 3.], [0., 1.], [1.; 2]);
    assert_eq!(
        joins(&[c0, closing.clone()], &[[0.25, 0.5], [0., 1.]], 1000000)
            .unwrap()
            .closed,
        Some(true)
    );
    let jump = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 0.5, 0.5, 0.5, 1., 1., 1.],
        control_points: vec![
            vec![0., 0.],
            vec![2., 4.],
            vec![4., 4.],
            vec![5., 4.],
            vec![6., 4.],
            vec![8., 0.],
        ],
        weights: vec![1.; 6],
        periodic: false,
    };
    assert!(
        matches!(joins(&[jump,closing],&[[0.25,0.5],[0.,1.]],1000000),
        Err(e) if e.code=="NURBS_INVALID_INPUT")
    );
}

#[test]
fn literal_multispan_endpoints_do_not_require_an_unneeded_algebraic_arena() {
    let c = Curve {
        degree: 1,
        knots: [
            vec![0., 0.],
            (1..254).map(|i| i as f64).collect(),
            vec![254., 254.],
        ]
        .concat(),
        control_points: (0..255).map(|i| vec![(i % 2) as f64, 0.]).collect(),
        weights: vec![1.; 255],
        periodic: false,
    };
    let curves = vec![c; 129];
    let ranges = vec![[0., 254.]; 129];
    let r = joins(&curves, &ranges, 0).unwrap();
    assert_eq!(r.closed, Some(true));
    assert_eq!(r.work_used, 0);
}
