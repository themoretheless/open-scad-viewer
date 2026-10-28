use super::*;
fn bowl(sign: f64) -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![2., 2., 2., 5., 5., 5.],
        knots_v: vec![-4., -4., -4., 8., 8., 8.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        vec![
                            i as f64 / 2.,
                            sign * (i * j) as f64 / 4.,
                            if i == 2 {
                                [1., 1. + sign * 0.25, 3. + sign * 0.5][j]
                            } else {
                                0.
                            },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn original_natural_axes_nonunit_domains_rational_lines_and_reversed_maps_agree_exactly() {
    let a = bowl(1.);
    let pa = Curve::from_polyline(vec![vec![2., -4.], vec![5., -4.]]).unwrap();
    for transpose in [false, true] {
        for upper in [false, true] {
            for reverse in [false, true] {
                let mut b = bowl(-1.);
                if upper {
                    for row in &mut b.control_points {
                        row.reverse();
                    }
                    for row in &mut b.weights {
                        row.reverse();
                    }
                }
                if transpose {
                    b = view(&b, Boundary::VMin);
                }
                let fixed = if transpose { 0 } else { 1 };
                let varying = 1 - fixed;
                let domain = [
                    [b.knots_u[b.degree_u], b.knots_u[b.control_points.len()]],
                    [b.knots_v[b.degree_v], b.knots_v[b.control_points[0].len()]],
                ];
                let mut first = vec![0.; 2];
                let mut last = first.clone();
                first[fixed] = domain[fixed][usize::from(upper)];
                last[fixed] = first[fixed];
                first[varying] = domain[varying][usize::from(reverse)];
                last[varying] = domain[varying][usize::from(!reverse)];
                let mut pb = Curve::from_polyline(vec![first, last]).unwrap();
                pb.weights = vec![1., 2.];
                let saved = b.clone();
                let maps = [[1., 1.], [1., 2.]];
                let r = prepare_exact_meridian_jets(
                    [&a, &b],
                    [&pa, &pb],
                    [false, reverse],
                    maps,
                    2,
                    100000000,
                )
                .unwrap();
                let first = prepare_exact_meridian_tangency(
                    [&a, &b],
                    [&pa, &pb],
                    [false, reverse],
                    maps,
                    2,
                    100000000,
                )
                .unwrap();
                assert!(first.certificate.is_some(), "{}", first.reason);
                let proof = r.certificate.expect(r.reason);
                assert!(std::ptr::eq(proof.surfaces()[1], &b));
                assert!(std::ptr::eq(proof.curves()[1], &pb));
                assert_eq!(proof.forward_weights(), maps);
                assert_eq!(proof.reversed(), [false, reverse]);
                assert_eq!(proof.dropped_axis(), 2);
                assert_eq!(b, saved);
                let short = prepare_exact_meridian_jets(
                    [&a, &b],
                    [&pa, &pb],
                    [false, reverse],
                    maps,
                    2,
                    r.work - 1,
                )
                .unwrap();
                assert!(short.certificate.is_none());
            }
        }
    }
}
#[test]
fn unsupported_truncated_layout_and_insufficient_exact_work_do_not_admit() {
    let a = bowl(1.);
    let b = bowl(-1.);
    let p = Curve::from_polyline(vec![vec![2., -4.], vec![5., -4.]]).unwrap();
    let short =
        prepare_exact_meridian_jets([&a, &b], [&p, &p], [false; 2], [[1.; 2]; 2], 2, 1).unwrap();
    assert!(short.certificate.is_none());
    let truncated = Curve::from_polyline(vec![vec![2., -4.], vec![4., -4.]]).unwrap();
    let r = prepare_exact_meridian_jets(
        [&a, &b],
        [&p, &truncated],
        [false; 2],
        [[1.; 2]; 2],
        2,
        1000000,
    )
    .unwrap();
    assert!(r.certificate.is_none());
    assert_eq!(r.work, 0);
}
#[test]
fn first_order_token_owns_original_sources_without_granting_second_order() {
    let a = bowl(1.);
    let mut b = a.clone();
    b.control_points[2][2][2] += 0.125;
    let p = Curve::from_polyline(vec![vec![2., -4.], vec![5., -4.]]).unwrap();
    let saved = b.clone();
    let r =
        prepare_exact_meridian_tangency([&a, &b], [&p, &p], [false; 2], [[1.; 2]; 2], 2, 100000000)
            .unwrap();
    let proof = r.certificate.expect(r.reason);
    assert!(std::ptr::eq(proof.surfaces()[1], &b));
    assert_eq!(proof.dropped_axis(), 2);
    let second =
        prepare_exact_meridian_jets([&a, &b], [&p, &p], [false; 2], [[1.; 2]; 2], 2, 100000000)
            .unwrap();
    assert!(second.certificate.is_none());
    assert!(matches!(second.outcome, BezierIdentity::Different));
    let short = prepare_exact_meridian_tangency(
        [&a, &b],
        [&p, &p],
        [false; 2],
        [[1.; 2]; 2],
        2,
        r.work - 1,
    )
    .unwrap();
    assert!(short.certificate.is_none());
    assert_eq!(b, saved);
    let truncated = Curve::from_polyline(vec![vec![2., -4.], vec![4., -4.]]).unwrap();
    let r = prepare_exact_meridian_tangency(
        [&a, &b],
        [&p, &truncated],
        [false; 2],
        [[1.; 2]; 2],
        2,
        100000000,
    )
    .unwrap();
    assert!(r.certificate.is_none());
}
