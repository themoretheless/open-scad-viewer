use nurbs_core::{
    editing::exact_surface_derivative::{self as d, Refusal},
    surface::{Axis, Surface},
};
fn domains(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
fn rational() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 6., 6.],
        knots_v: vec![-3., -3., 5., 5.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 2., 0.]],
            vec![vec![2., 0., 0.], vec![2., 2., 2.]],
        ],
        weights: vec![vec![1., 2.], vec![2., 4.]],
        periodic_u: false,
        periodic_v: false,
    }
}
fn cubic() -> Surface {
    Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![2., 2., 2., 2., 6., 6., 6., 6.],
        knots_v: vec![-3., -3., -3., -3., 5., 5., 5., 5.],
        control_points: (0..4)
            .map(|i| {
                (0..4)
                    .map(|j| {
                        vec![
                            if i == 3 { 6. } else { 0. },
                            if j == 3 { 6. } else { 0. },
                            (i * (i - 1usize.min(i)) * j + i * j * (j - 1usize.min(j))) as f64,
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    }
}
fn accepted(s: Surface, orders: [usize; 2]) -> Vec<Surface> {
    let before = s.clone();
    let r = d::mixed_partial(&s, orders, 1000000).unwrap();
    assert!(r.refusal.is_none(), "{:?},work={}", r.refusal, r.work_used);
    let out = r.patches.unwrap();
    assert_eq!(s, before);
    let us = (s.degree_u..s.control_points.len())
        .filter(|&i| s.knots_u[i] < s.knots_u[i + 1])
        .collect::<Vec<_>>();
    let vs = (s.degree_v..s.control_points[0].len())
        .filter(|&j| s.knots_v[j] < s.knots_v[j + 1])
        .collect::<Vec<_>>();
    assert_eq!(out.len(), us.len() * vs.len());
    for (n, p) in out.iter().enumerate() {
        p.validate().unwrap();
        assert!(!p.periodic_u && !p.periodic_v);
        assert_eq!(
            domains(p),
            [
                [s.knots_u[us[n / vs.len()]], s.knots_u[us[n / vs.len()] + 1]],
                [s.knots_v[vs[n % vs.len()]], s.knots_v[vs[n % vs.len()] + 1]]
            ]
        );
        if orders.iter().sum::<usize>() == 2 {
            let [u, v] = domains(p);
            for i in 1..4 {
                for j in 1..4 {
                    let u = u[0] + (u[1] - u[0]) * i as f64 / 4.;
                    let v = v[0] + (v[1] - v[0]) * j as f64 / 4.;
                    if let Some((uu, uv, vv)) = s.evaluate(u, v).unwrap().second_derivatives() {
                        let expected = match orders {
                            [2, 0] => uu,
                            [1, 1] => uv,
                            [0, 2] => vv,
                            _ => unreachable!(),
                        };
                        let actual = p.evaluate(u, v).unwrap().point;
                        for k in 0..3 {
                            assert!((expected[k] - actual[k]).abs() < 1e-9);
                        }
                    }
                }
            }
        }
    }
    #[cfg(feature = "codec")]
    println!(
        "exact-mixed-partial-oracle={}",
        value_codec::json!({"orders":orders,"source":s,"patches":out})
    );
    out
}
#[test]
fn cubic_second_and_third_fields_preserve_physical_parameter_scales() {
    let s = cubic();
    for orders in [[2, 0], [1, 1], [0, 2], [3, 0], [2, 1], [1, 2], [0, 3]] {
        let out = accepted(s.clone(), orders);
        for u in [2., 3., 6.] {
            for v in [-3., 0., 5.] {
                let t = (u - 2.) / 4.;
                let q = (v + 3.) / 8.;
                let expected = match orders {
                    [2, 0] => [2.25 * t, 0., 2.25 * q],
                    [1, 1] => [0., 0., 1.125 * (t + q)],
                    [0, 2] => [0., 0.5625 * q, 0.5625 * t],
                    [3, 0] => [0.5625, 0., 0.],
                    [2, 1] => [0., 0., 0.28125],
                    [1, 2] => [0., 0., 0.140625],
                    [0, 3] => [0., 0.0703125, 0.],
                    _ => unreachable!(),
                };
                assert_eq!(out[0].evaluate(u, v).unwrap().point, expected);
            }
        }
    }
}
#[test]
fn rational_order_above_source_degree_is_not_fabricated_zero() {
    let s = rational();
    for orders in [[2, 0], [1, 1], [0, 2], [3, 0], [2, 1], [1, 2], [0, 3]] {
        let out = accepted(s.clone(), orders);
        assert!(
            out[0]
                .control_points
                .iter()
                .flatten()
                .flatten()
                .any(|&x| x != 0.)
        );
    }
    let third = d::mixed_partial(&s, [3, 0], 1000000)
        .unwrap()
        .patches
        .unwrap();
    assert_eq!(third[0].evaluate(2., -3.).unwrap().point, [0.375, 0., 0.]);
}
#[test]
fn first_order_matches_existing_constructor_literal_outputs() {
    let s = rational();
    for (orders, axis) in [([1, 0], Axis::U), ([0, 1], Axis::V)] {
        assert_eq!(
            accepted(s.clone(), orders),
            d::surface(&s, axis, 1000000).unwrap().patches.unwrap()
        );
    }
}
#[test]
fn transposed_mixed_orders_keep_the_same_physical_vector_field() {
    let s = rational();
    let t = Surface {
        degree_u: s.degree_v,
        degree_v: s.degree_u,
        knots_u: s.knots_v.clone(),
        knots_v: s.knots_u.clone(),
        control_points: (0..2)
            .map(|j| (0..2).map(|i| s.control_points[i][j].clone()).collect())
            .collect(),
        weights: (0..2)
            .map(|j| (0..2).map(|i| s.weights[i][j]).collect())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let a = accepted(s, [2, 1]);
    let b = accepted(t, [1, 2]);
    for u in [2., 3., 6.] {
        for v in [-3., 0., 5.] {
            let x = a[0].evaluate(u, v).unwrap().point;
            let y = b[0].evaluate(v, u).unwrap().point;
            for k in 0..3 {
                assert!((x[k] - y[k]).abs() < 1e-12);
            }
        }
    }
}
#[test]
fn nonuniform_cubic_tensor_keeps_every_original_span() {
    let u = [0., 0., 0., 0., 1., 3., 3., 3., 3.];
    let v = [-2., -2., -2., -2., 0., 2., 2., 2., 2.];
    let s = Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: u.to_vec(),
        knots_v: v.to_vec(),
        control_points: (0..5)
            .map(|i| {
                (0..5)
                    .map(|j| {
                        let a = &u[i + 1..i + 4];
                        let b = &v[j + 1..j + 4];
                        let su = a.iter().sum::<f64>();
                        let sv = b.iter().sum::<f64>();
                        let pu = a[0] * a[1] + a[0] * a[2] + a[1] * a[2];
                        let pv = b[0] * b[1] + b[0] * b[2] + b[1] * b[2];
                        vec![
                            6. * a.iter().product::<f64>(),
                            6. * b.iter().product::<f64>(),
                            2. * (pu * sv + su * pv),
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 5]; 5],
        periodic_u: false,
        periodic_v: false,
    };
    for orders in [[2, 1], [1, 1]] {
        let out = accepted(s.clone(), orders);
        assert_eq!(out.len(), 4);
        for p in out {
            let [u, v] = domains(&p);
            for u in u {
                for v in v {
                    assert_eq!(
                        p.evaluate(u, v).unwrap().point,
                        if orders == [2, 1] {
                            [0., 0., 36.]
                        } else {
                            [0., 0., 36. * (u + v)]
                        }
                    );
                }
            }
        }
    }
}
#[test]
fn unclamped_tensor_has_complete_fundamental_domain_cover() {
    let g = [-0.5, 0.5, 1.5, 2.5, 3.5];
    let s = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![-2., -1., 0., 1., 2., 3., 4., 5.],
        knots_v: vec![-2., -1., 0., 1., 2., 3., 4., 5.],
        control_points: g
            .iter()
            .map(|&u| g.iter().map(|&v| vec![u, v, u * v]).collect())
            .collect(),
        weights: vec![vec![1.; 5]; 5],
        periodic_u: false,
        periodic_v: false,
    };
    let out = accepted(s, [1, 1]);
    assert_eq!(out.len(), 9);
    assert!(out.iter().all(|p| {
        p.control_points
            .iter()
            .flatten()
            .all(|p| p == &vec![0., 0., 1.])
    }));
}
#[test]
fn periodic_rational_aliases_keep_third_and_zero_cross_fields() {
    let a = [[0., 0., 0.], [2., 0., 0.], [0., 2., 0.], [0., 0., 0.]];
    let b = [[0., 0., 0.], [0., 0., 2.], [1., 1., 1.], [0., 0., 0.]];
    let w = [1., 2., 4., 1.];
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-1., 0., 1., 2., 3., 4.],
        knots_v: vec![-1., 0., 1., 2., 3., 4.],
        control_points: a
            .iter()
            .map(|a| {
                b.iter()
                    .map(|b| (0..3).map(|k| a[k] + b[k]).collect())
                    .collect()
            })
            .collect(),
        weights: w
            .iter()
            .map(|a| w.iter().map(|b| a * b).collect())
            .collect(),
        periodic_u: true,
        periodic_v: true,
    };
    assert_eq!(accepted(s.clone(), [3, 0]).len(), 9);
    let out = accepted(s, [2, 1]);
    assert_eq!(out.len(), 9);
    assert!(out.iter().all(|p| {
        p.control_points
            .iter()
            .flatten()
            .flatten()
            .all(|&x| x == 0.)
    }));
}
#[test]
fn repeated_knots_preserve_one_sided_second_and_cross_traces() {
    let x = [0., 3., 6., 7.5, 12.];
    let y = [0., 3., 6., 9., 12.];
    let s = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 2., 2., 2.],
        knots_v: vec![-1., -1., -1., 0., 0., 2., 2., 2.],
        control_points: x
            .iter()
            .map(|&x| y.iter().map(|&y| vec![x, y, x * y]).collect())
            .collect(),
        weights: vec![vec![1.; 5]; 5],
        periodic_u: false,
        periodic_v: false,
    };
    let uu = accepted(s.clone(), [2, 0]);
    assert_eq!(uu[0].evaluate(1., -0.5).unwrap().point, [0., 0., 0.]);
    assert_eq!(uu[2].evaluate(1., -0.5).unwrap().point, [6., 0., 18.]);
    let uv = accepted(s, [1, 1]);
    assert_eq!(uv[0].evaluate(0.5, 0.).unwrap().point, [0., 0., 36.]);
    assert_eq!(uv[1].evaluate(0.5, 0.).unwrap().point, [0., 0., 18.]);
}
#[test]
fn mixed_variable_affine_source_has_zero_higher_fields() {
    let s = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![-2., -2., -2., 2., 2., 2.],
        knots_v: vec![1., 1., 1., 9., 9., 9.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        let i = i as f64;
                        let j = j as f64;
                        vec![
                            15. * i * (2. * i + j) / (2. + i + j),
                            15. * j * (2. * j + i) / (2. + i + j),
                            3.,
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: (0..3)
            .map(|i| (0..3).map(|j| (2 + i + j) as f64).collect())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let out = accepted(s, [2, 1]);
    assert!(
        out[0]
            .control_points
            .iter()
            .flatten()
            .flatten()
            .all(|&x| x == 0.)
    );
}
#[test]
fn high_source_degree_does_not_force_a_high_degree_zero_field() {
    let s = Surface {
        degree_u: 25,
        degree_v: 1,
        knots_u: [vec![0.; 26], vec![2.; 26]].concat(),
        knots_v: vec![-2., -2., 2., 2.],
        control_points: (0..26)
            .map(|i| vec![vec![i as f64, 0., 1.], vec![i as f64, 4., 1.]])
            .collect(),
        weights: vec![vec![1.; 2]; 26],
        periodic_u: false,
        periodic_v: false,
    };
    let out = accepted(s, [3, 0]);
    assert_eq!((out[0].degree_u, out[0].degree_v), (1, 1));
    assert!(
        out[0]
            .control_points
            .iter()
            .flatten()
            .flatten()
            .all(|&x| x == 0.)
    );
}
#[test]
fn late_storage_failure_discards_previously_constructible_fields() {
    let x = [0., 2., 3.];
    let w = [1., 2., 3.];
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 2., 2.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: x
            .iter()
            .map(|&x| vec![vec![x, 0., 0.], vec![x, 1., 0.]])
            .collect(),
        weights: w.iter().map(|&w| vec![w, w]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let first = Surface {
        knots_u: vec![0., 0., 1., 1.],
        control_points: s.control_points[..2].to_vec(),
        weights: s.weights[..2].to_vec(),
        ..s.clone()
    };
    assert!(
        d::mixed_partial(&first, [2, 0], 1000000)
            .unwrap()
            .patches
            .is_some()
    );
    let r = d::mixed_partial(&s, [2, 0], 1000000).unwrap();
    assert!(r.patches.is_none());
    assert_eq!(r.refusal, Some(Refusal::NotRepresentable));
}
#[test]
fn differentiation_reduction_storage_and_proof_share_one_budget() {
    let s = rational();
    let full = d::mixed_partial(&s, [2, 1], 1000000).unwrap();
    assert!(full.patches.is_some());
    let n = full.work_used;
    assert!(d::mixed_partial(&s, [2, 1], n).unwrap().patches.is_some());
    let short = d::mixed_partial(&s, [2, 1], n - 1).unwrap();
    assert_eq!(short.refusal, Some(Refusal::ResourceLimit));
    assert!(short.patches.is_none());
    assert!(short.work_used < n);
}
#[test]
fn output_degree_and_coordinate_limits_are_unchanged() {
    let mut s = Surface {
        degree_u: 7,
        degree_v: 1,
        knots_u: [vec![0.; 8], vec![1.; 8]].concat(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..8)
            .map(|i| {
                vec![
                    vec![if i == 7 { 1. } else { 0. }, 0., 0.],
                    vec![if i == 7 { 1. } else { 0. }, 1., 0.],
                ]
            })
            .collect(),
        weights: vec![vec![1.; 2]; 8],
        periodic_u: false,
        periodic_v: false,
    };
    s.weights[7] = vec![2., 2.];
    assert_eq!(
        d::mixed_partial(&s, [3, 0], 1000000).unwrap().refusal,
        Some(Refusal::OutputDegreeLimit)
    );
    let s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 0.5, 0.5, 0.5],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [0., 0., 1e9]
            .iter()
            .map(|&x| vec![vec![x, 0., 0.], vec![x, 1., 0.]])
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    s.validate().unwrap();
    assert_eq!(
        d::mixed_partial(&s, [2, 0], 1000000).unwrap().refusal,
        Some(Refusal::InvalidOutput)
    );
}
#[test]
fn invalid_orders_sources_and_work_do_not_start_an_unbounded_loop() {
    for orders in [
        [0, 0],
        [4, 0],
        [2, 2],
        [usize::MAX, 1],
        [usize::MAX, usize::MAX],
    ] {
        assert!(d::mixed_partial(&rational(), orders, 1000000).is_err());
    }
    for n in [0, 1000001, u64::MAX] {
        assert!(d::mixed_partial(&rational(), [2, 0], n).is_err());
    }
    let mut s = rational();
    s.weights[0][0] = 0.;
    assert!(d::mixed_partial(&s, [1, 1], 1000000).is_err());
    let mut s = rational();
    s.control_points[0][0][0] = f64::NAN;
    assert!(d::mixed_partial(&s, [3, 0], 1000000).is_err());
}
