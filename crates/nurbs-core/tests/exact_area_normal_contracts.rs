use nurbs_core::{
    editing::exact_surface_derivative::{self as d, Refusal},
    surface::Surface,
};
fn domains(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 6., 6.],
        knots_v: vec![-3., -3., 5., 5.],
        control_points: vec![
            vec![vec![0., 0., 1.], vec![0., 4., 1.]],
            vec![vec![3., 0., 1.], vec![3., 4., 1.]],
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
    let r = d::area_normal(&s, 1000000).unwrap();
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
                let e = s.evaluate(u, v).unwrap();
                let actual = p.evaluate(u, v).unwrap().point;
                if let Some((a, b)) = e.first_derivatives() {
                    let expected = cross(a, b);
                    for k in 0..3 {
                        assert!((actual[k] - expected[k]).abs() < 1e-9);
                    }
                } else {
                    assert!(
                        p.control_points
                            .iter()
                            .flatten()
                            .flatten()
                            .all(|&x| x == 0.)
                    );
                }
            }
        }
    }
    #[cfg(feature = "codec")]
    println!(
        "exact-area-normal-oracle={}",
        value_codec::json!({"source":s,"patches":out})
    );
    out
}
#[test]
fn plane_field_is_oriented_area_density_not_a_unit_normal() {
    let out = accepted(plane());
    assert_eq!(out[0].control_points, vec![vec![vec![0., 0., 0.375]; 2]; 2]);
    assert_eq!(out[0].weights, vec![vec![1.; 2]; 2]);
}
#[test]
fn curved_rational_field_keeps_exact_tensor_weights_and_absolute_clocks() {
    let out = accepted(rational());
    assert_eq!(out.len(), 1);
    assert_eq!(domains(&out[0]), [[2., 6.], [-3., 5.]]);
    assert!(out[0].degree_u >= 2 && out[0].degree_v >= 2);
}
#[test]
fn transposing_parameters_reverses_the_oriented_field() {
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
            for k in 0..3 {
                assert!((x[k] + y[k]).abs() < 1e-11);
            }
        }
    }
}
#[test]
fn nonuniform_uv_cells_keep_all_original_polynomial_area_vectors() {
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
                assert_eq!(p.evaluate(u, v).unwrap().point, [-v, -u, 1.]);
            }
        }
    }
}
#[test]
fn degenerate_source_has_a_zero_field_without_a_regularity_claim() {
    let mut s = plane();
    s.control_points = vec![vec![vec![1., 2., 3.]; 2]; 2];
    s.weights = vec![vec![1., 2.], vec![2., 4.]];
    let out = accepted(s);
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
fn periodic_aliases_keep_the_complete_fundamental_cell_cover() {
    let a = [[0., 0., 0.], [2., 0., 0.], [0., 2., 0.], [0., 0., 0.]];
    let b = [[0., 0., 0.], [0., 0., 2.], [1., 1., 1.], [0., 0., 0.]];
    let weights = [1., 2., 4., 1.];
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
        weights: weights
            .iter()
            .map(|a| weights.iter().map(|b| a * b).collect())
            .collect(),
        periodic_u: true,
        periodic_v: true,
    };
    assert_eq!(accepted(s).len(), 9);
}
#[test]
fn unclamped_quadratic_axes_use_original_extended_knots() {
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
fn repeated_knots_preserve_distinct_one_sided_area_densities() {
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
        [-108., -216., 36.]
    );
    assert_eq!(out[2].evaluate(1., -0.5).unwrap().point, [-54., -108., 18.]);
    assert_eq!(out[0].evaluate(0.5, 0.).unwrap().point, [-216., -108., 36.]);
    assert_eq!(out[1].evaluate(0.5, 0.).unwrap().point, [-108., -54., 18.]);
}
#[test]
fn late_storage_failure_discards_earlier_field_patches() {
    let x = [0., 2., 3.];
    let weights = [1., 2., 3.];
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 2., 2.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: x
            .iter()
            .map(|&x| vec![vec![x, 0., 0.], vec![x, 1., 0.]])
            .collect(),
        weights: weights.iter().map(|&w| vec![w, w]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let first = Surface {
        knots_u: vec![0., 0., 1., 1.],
        control_points: s.control_points[..2].to_vec(),
        weights: s.weights[..2].to_vec(),
        ..s.clone()
    };
    assert!(d::area_normal(&first, 1000000).unwrap().patches.is_some());
    let r = d::area_normal(&s, 1000000).unwrap();
    assert!(r.patches.is_none());
    assert_eq!(r.refusal, Some(Refusal::NotRepresentable));
}
#[test]
fn original_numeric_caps_and_invalid_requests_are_not_relaxed() {
    for n in [0, 1000001, u64::MAX] {
        assert!(d::area_normal(&plane(), n).is_err());
    }
    let mut s = plane();
    s.weights[0][0] = 0.;
    assert!(d::area_normal(&s, 1000000).is_err());
    let mut s = plane();
    s.control_points[1][0][0] = f64::NAN;
    assert!(d::area_normal(&s, 1000000).is_err());
}
#[test]
fn all_cell_products_cancellation_storage_and_proof_share_one_work_ledger() {
    let s = rational();
    let r = d::area_normal(&s, 1000000).unwrap();
    assert!(r.patches.is_some(), "{:?}", r.refusal);
    let n = r.work_used;
    assert!(d::area_normal(&s, n).unwrap().patches.is_some());
    let r = d::area_normal(&s, n - 1).unwrap();
    assert!(r.patches.is_none());
    assert_eq!(r.refusal, Some(Refusal::ResourceLimit));
    assert!(r.work_used < n);
}
#[test]
fn genuine_high_degree_area_density_refuses_the_existing_output_limit() {
    let mut s = Surface {
        degree_u: 13,
        degree_v: 1,
        knots_u: [vec![0.; 14], vec![1.; 14]].concat(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..14)
            .map(|i| {
                vec![
                    vec![if i == 13 { 1. } else { 0. }, 0., 0.],
                    vec![if i == 13 { 1. } else { 0. }, 1., 0.],
                ]
            })
            .collect(),
        weights: vec![vec![1.; 2]; 14],
        periodic_u: false,
        periodic_v: false,
    };
    s.weights[13] = vec![2., 2.];
    let r = d::area_normal(&s, 1000000).unwrap();
    assert!(r.patches.is_none());
    assert_eq!(r.refusal, Some(Refusal::OutputDegreeLimit));
}
#[test]
fn area_output_coordinates_and_weight_limits_are_not_relaxed() {
    let mut s = plane();
    s.control_points[1][0][0] = 1e9;
    s.control_points[1][1][0] = 1e9;
    s.control_points[0][1][1] = 1e9;
    s.control_points[1][1][1] = 1e9;
    s.validate().unwrap();
    assert_eq!(
        d::area_normal(&s, 1000000).unwrap().refusal,
        Some(Refusal::InvalidOutput)
    );
    let tiny = 2f64.powi(-30);
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![tiny, 0., 0.], vec![tiny, 1., 0.]],
        ],
        weights: vec![vec![tiny, tiny], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    s.validate().unwrap();
    let r = d::area_normal(&s, 1000000).unwrap();
    assert!(r.patches.is_none());
    assert_eq!(r.refusal, Some(Refusal::InvalidOutput));
}
