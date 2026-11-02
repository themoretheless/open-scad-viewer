use nurbs_core::{
    editing::exact_surface_knot_insertion::{Refusal, insert},
    surface::{Axis, Surface},
};
fn patch() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-2., -2., 6., 6.],
        knots_v: vec![3., 3., 7., 7.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 3., 1.]],
            vec![vec![3., 0., 2.], vec![3., 3., 4.]],
        ],
        weights: vec![vec![1., 3.], vec![2., 6.]],
        periodic_u: false,
        periodic_v: false,
    }
}
fn dump(s: &Surface) -> String {
    format!(
        "{{\"degreeU\":{},\"degreeV\":{},\"knotsU\":{:?},\"knotsV\":{:?},\"controlPoints\":{:?},\"weights\":{:?}}}",
        s.degree_u, s.degree_v, s.knots_u, s.knots_v, s.control_points, s.weights
    )
}
fn accepted(s: &Surface, axis: Axis, knot: f64, count: usize) -> Surface {
    let r = insert(s, axis, knot, count, 1_000_000).unwrap();
    assert_eq!(r.refusal, None);
    assert_eq!(r.inserted, count);
    assert!(r.work_used <= 1_000_000);
    assert_eq!(
        r.rows_verified,
        match axis {
            Axis::U => s.control_points[0].len(),
            Axis::V => s.control_points.len(),
        }
    );
    println!(
        "surface-insertion-oracle={{\"source\":{},\"output\":{}}}",
        dump(s),
        dump(&r.surface)
    );
    r.surface
}
#[test]
fn rational_net_has_one_global_scale_not_one_per_row() {
    let mut s = patch();
    // All newly interpolated Cartesian coordinates are exactly representable.
    s.control_points[1][0] = vec![3., 0., 3.];
    s.control_points[1][1] = vec![3., 3., 4.];
    s.control_points[0][1][2] = 1.;
    let u = accepted(&s, Axis::U, 2., 1);
    let _v = accepted(&s, Axis::V, 5., 1);
    assert_eq!(
        u.weights,
        vec![vec![0.25, 0.75], vec![0.375, 1.125], vec![0.5, 1.5]]
    );
    for i in 0..=8 {
        for j in 0..=8 {
            let (a, b) = (-2. + i as f64, 3. + j as f64 / 2.);
            let p = s.evaluate(a, b).unwrap().point;
            let q = u.evaluate(a, b).unwrap().point;
            for d in 0..3 {
                assert!((p[d] - q[d]).abs() < 1e-12);
            }
        }
    }
}
#[test]
fn both_axes_keep_original_uv_and_cross_terms() {
    let mut s = patch();
    s.weights = vec![vec![1.; 2]; 2];
    let u = accepted(&s, Axis::U, 2., 1);
    let v = accepted(&u, Axis::V, 5., 1);
    assert_eq!(v.knots_u, vec![-2., -2., 2., 6., 6.]);
    assert_eq!(v.knots_v, vec![3., 3., 5., 7., 7.]);
    assert_eq!(v.control_points[1][1], vec![1.5, 1.5, 1.75]);
}
#[test]
fn late_unrepresentable_row_rolls_back_entire_surface() {
    let mut s = patch();
    s.control_points[1][0] = vec![3., 0., 3.];
    s.control_points[1][1][0] = 2.;
    let r = insert(&s, Axis::U, 2., 1, 1_000_000).unwrap();
    assert_eq!(r.surface, s);
    assert_eq!(r.inserted, 0);
    assert_eq!(
        r.refusal,
        Some(Refusal::Arithmetic(
            nurbs_core::editing::exact_knot_insertion::Refusal::NotRepresentable
        ))
    );
}
#[test]
fn batch_and_existing_knots_use_one_ledger() {
    let s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                vec![
                    vec![i as f64, 0., (i % 2) as f64],
                    vec![i as f64, 2., (i % 2) as f64 + 1.],
                ]
            })
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let a = accepted(&s, Axis::U, 0.5, 2);
    let b = accepted(&s, Axis::U, 0.5, 1);
    let c = accepted(&b, Axis::U, 0.5, 1);
    assert_eq!(a, c);
    let full = insert(&s, Axis::U, 0.5, 2, 1_000_000).unwrap();
    for budget in [1, 100, full.work_used - 1] {
        let r = insert(&s, Axis::U, 0.5, 2, budget).unwrap();
        assert_eq!(r.inserted, 0);
        assert_eq!(r.surface, s);
        assert!(r.work_used <= budget);
    }
    assert_eq!(
        insert(&s, Axis::U, 0.5, 2, full.work_used).unwrap().surface,
        a
    );
}
#[test]
fn untouched_periodic_axis_keeps_seam_storage() {
    let mut s = patch();
    s.weights = vec![vec![1.; 5]; 2];
    s.degree_v = 2;
    s.knots_v = vec![-2., -1., 0., 1., 2., 3., 4., 5.];
    s.periodic_v = true;
    s.control_points = (0..2)
        .map(|i| {
            [0., 2., -2., 0., 2.]
                .iter()
                .map(|&v| vec![i as f64 * 2., v, 0.])
                .collect()
        })
        .collect();
    s.validate().unwrap();
    let r = accepted(&s, Axis::U, 2., 1);
    assert!(r.periodic_v);
    assert_eq!(r.knots_v, s.knots_v);
    assert!(insert(&s, Axis::V, 1.5, 1, 1_000_000).is_err());
}
#[test]
fn invalid_tail_requests_and_control_cap() {
    let s = patch();
    for (k, n, b) in [
        (-2., 1, 1000),
        (6., 1, 1000),
        (f64::NAN, 1, 1000),
        (2., 0, 1000),
        (2., usize::MAX, 1000),
        (2., 1, 0),
        (2., 1, 1_000_001),
    ] {
        assert!(insert(&s, Axis::U, k, n, b).is_err());
    }
    let mut bad = s.clone();
    bad.weights[1][1] = f64::NAN;
    assert!(insert(&bad, Axis::U, 2., 1, 1_000_000).is_err());
    let mut cap = s;
    cap.control_points = (0..256)
        .map(|i| vec![vec![i as f64, 0., 0.], vec![i as f64, 1., 0.]])
        .collect();
    cap.weights = vec![vec![1.; 2]; 256];
    cap.knots_u = [
        vec![0., 0.],
        (1..255).map(|i| i as f64).collect(),
        vec![255., 255.],
    ]
    .concat();
    let r = insert(&cap, Axis::U, 0.5, 1, 1_000_000).unwrap();
    assert_eq!(r.surface, cap);
    assert_eq!(r.inserted, 0);
    assert_eq!(r.work_used, 0);
}
