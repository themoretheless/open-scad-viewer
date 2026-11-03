use nurbs_core::{
    editing::exact_surface_derivative::{self as d, Refusal},
    surface::Surface,
};
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn domains(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 6., 6.],
        knots_v: vec![-3., -3., 5., 5.],
        control_points: vec![
            vec![vec![0., 0., 1.], vec![2., 4., 1.]],
            vec![vec![3., 0., 1.], vec![5., 4., 1.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn rational() -> Surface {
    let mut s = plane();
    s.control_points = vec![
        vec![vec![0., 0., 0.], vec![0., 6., 0.]],
        vec![vec![6., 0., 0.], vec![6., 6., 18.]],
    ];
    s.weights = vec![vec![1., 2.], vec![2., 4.]];
    s
}
fn accepted(s: Surface) -> Vec<Surface> {
    let before = s.clone();
    let r = d::metric(&s, 1000000).unwrap();
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
        let [u, v] = domains(p);
        for i in 1..4 {
            for j in 1..4 {
                let u = u[0] + (u[1] - u[0]) * i as f64 / 4.;
                let v = v[0] + (v[1] - v[0]) * j as f64 / 4.;
                if let Some((a, b)) = s.evaluate(u, v).unwrap().first_derivatives() {
                    let expected = [dot(a, a), dot(a, b), dot(b, b)];
                    let actual = p.evaluate(u, v).unwrap().point;
                    for k in 0..3 {
                        assert!((actual[k] - expected[k]).abs() < 1e-8);
                    }
                }
            }
        }
    }
    #[cfg(feature = "codec")]
    println!(
        "exact-metric-field-oracle={}",
        value_codec::json!({"source":s,"patches":out})
    );
    out
}
#[test]
fn oblique_plane_keeps_physical_clocks_and_single_not_doubled_f() {
    let out = accepted(plane());
    assert_eq!(
        out[0].control_points,
        vec![vec![vec![0.5625, 0.1875, 0.3125]; 2]; 2]
    );
    let m = out[0].control_points[0][0].clone();
    assert_eq!(m[0] * m[2] - m[1] * m[1], 0.375f64.powi(2));
}
#[test]
fn curved_rational_metric_has_a_shared_full_tensor_denominator() {
    let out = accepted(rational());
    assert_eq!((out[0].degree_u, out[0].degree_v), (4, 4));
    assert!(out[0].control_points.iter().flatten().any(|p| p[1] != 0.));
}
#[test]
fn parameter_swap_exchanges_diagonal_fields_but_not_the_cross_coefficient() {
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
    let a = accepted(s);
    let b = accepted(t);
    for u in [2., 3., 6.] {
        for v in [-3., 0., 5.] {
            let x = a[0].evaluate(u, v).unwrap().point;
            let y = b[0].evaluate(v, u).unwrap().point;
            for (i, j) in [(0, 2), (1, 1), (2, 0)] {
                assert!((x[i] - y[j]).abs() < 1e-10);
            }
        }
    }
}
#[test]
fn parameter_reversal_changes_the_cross_sign_without_normalizing() {
    let mut s = plane();
    s.control_points.reverse();
    s.weights.reverse();
    let out = accepted(s);
    assert_eq!(
        out[0].control_points,
        vec![vec![vec![0.5625, -0.1875, 0.3125]; 2]; 2]
    );
}
#[test]
fn nonuniform_multispan_tensor_preserves_each_original_uv_cell() {
    let gu = [0., 0.5, 2., 3.];
    let gv = [-2., -1., 1., 2.];
    let s = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 3., 3., 3.],
        knots_v: vec![-2., -2., -2., 0., 2., 2., 2.],
        control_points: gu
            .iter()
            .map(|&u| gv.iter().map(|&v| vec![u, v, u * v]).collect())
            .collect(),
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    };
    let out = accepted(s);
    assert_eq!(out.len(), 4);
    for p in out {
        let [u, v] = domains(&p);
        for u in u {
            for v in v {
                assert_eq!(
                    p.evaluate(u, v).unwrap().point,
                    [1. + v * v, u * v, 1. + u * u]
                );
            }
        }
    }
}
#[test]
fn singular_and_zero_metrics_are_geometry_not_regularity_certificates() {
    let mut s = plane();
    s.control_points = vec![vec![vec![1., 2., 3.]; 2]; 2];
    s.weights = vec![vec![1., 2.], vec![2., 8.]];
    let out = accepted(s);
    assert!(
        out[0]
            .control_points
            .iter()
            .flatten()
            .flatten()
            .all(|&x| x == 0.)
    );
    let mut s = plane();
    s.control_points = vec![vec![vec![0., 0., 0.]; 2], vec![vec![1., 0., 0.]; 2]];
    let out = accepted(s);
    assert_eq!(
        out[0].control_points,
        vec![vec![vec![0.0625, 0., 0.]; 2]; 2]
    );
}
#[test]
fn mixed_variable_weights_can_encode_a_constant_metric_without_separability() {
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
    let out = accepted(s);
    assert_eq!(
        out[0].control_points,
        vec![vec![vec![56.25, 0., 14.0625]; 2]; 2]
    );
}
#[test]
fn high_source_degree_can_still_give_a_constant_low_degree_metric() {
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
    let out = accepted(s);
    assert_eq!((out[0].degree_u, out[0].degree_v), (1, 1));
    assert_eq!(
        out[0].control_points,
        vec![vec![vec![156.25, 0., 1.]; 2]; 2]
    );
}
#[test]
fn periodic_axes_keep_all_fundamental_cells() {
    let a = [[0., 0., 0.], [6., 0., 0.], [0., 6., 0.], [0., 0., 0.]];
    let b = [[0., 0., 0.], [0., 0., 6.], [3., 3., 3.], [0., 0., 0.]];
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
    assert_eq!(accepted(s).len(), 9);
}
#[test]
fn unclamped_tensor_uses_actual_extended_basis() {
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
    assert_eq!(accepted(s).len(), 9);
}
#[test]
fn repeated_knots_keep_both_one_sided_metric_traces() {
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
    let out = accepted(s);
    assert_eq!(
        out[0].evaluate(1., -0.5).unwrap().point,
        [360., 648., 1332.]
    );
    assert_eq!(out[2].evaluate(1., -0.5).unwrap().point, [90., 324., 1332.]);
    assert_eq!(out[0].evaluate(0.5, 0.).unwrap().point, [1332., 648., 360.]);
    assert_eq!(out[1].evaluate(0.5, 0.).unwrap().point, [1332., 324., 90.]);
}
#[test]
fn late_exact_storage_refusal_never_publishes_earlier_cells() {
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
    assert!(d::metric(&first, 1000000).unwrap().patches.is_some());
    let r = d::metric(&s, 1000000).unwrap();
    assert!(r.patches.is_none());
    assert_eq!(r.refusal, Some(Refusal::NotRepresentable));
}
#[test]
fn one_ledger_spans_products_cancellation_storage_and_actual_output_proof() {
    let s = rational();
    let full = d::metric(&s, 1000000).unwrap();
    assert!(full.patches.is_some(), "{:?}", full.refusal);
    let n = full.work_used;
    assert!(d::metric(&s, n).unwrap().patches.is_some());
    let r = d::metric(&s, n - 1).unwrap();
    assert!(r.patches.is_none());
    assert_eq!(r.refusal, Some(Refusal::ResourceLimit));
    assert!(r.work_used < n);
}
#[test]
fn output_degree_and_coordinate_limits_are_not_relaxed() {
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
        d::metric(&s, 1000000).unwrap().refusal,
        Some(Refusal::OutputDegreeLimit)
    );
    let mut s = plane();
    s.control_points = vec![
        vec![vec![0., 0., 0.], vec![0., 1e9, 0.]],
        vec![vec![1e9, 0., 0.], vec![1e9, 1e9, 0.]],
    ];
    s.validate().unwrap();
    assert_eq!(
        d::metric(&s, 1000000).unwrap().refusal,
        Some(Refusal::InvalidOutput)
    );
}
#[test]
fn invalid_source_or_budget_is_an_error() {
    for n in [0, 1000001, u64::MAX] {
        assert!(d::metric(&plane(), n).is_err());
    }
    let mut s = plane();
    s.weights[0][0] = 0.;
    assert!(d::metric(&s, 1000000).is_err());
    let mut s = plane();
    s.control_points[0][0][0] = f64::NAN;
    assert!(d::metric(&s, 1000000).is_err());
}
