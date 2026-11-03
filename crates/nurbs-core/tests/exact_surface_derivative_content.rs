use nurbs_core::{
    editing::exact_surface_derivative::{self as d, Axis, Refusal},
    surface::Surface,
};
fn hidden() -> Surface {
    let x = [0., 0., 20., 30., 0., 20., 60.];
    let weights = [5., 2.5, 1., 0.5, 1., 2.5, 5.];
    Surface {
        degree_u: 6,
        degree_v: 1,
        knots_u: [vec![0.; 7], vec![1.; 7]].concat(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: x
            .iter()
            .map(|&x| vec![vec![x, 0., 0.], vec![x, 1., 0.]])
            .collect(),
        weights: weights.iter().map(|&w| vec![w, w]).collect(),
        periodic_u: false,
        periodic_v: false,
    }
}
fn tensor_hidden() -> Surface {
    let x = [0., 0., 20., 30., 0., 20., 60.];
    let z = [0., 2., 4., 3., 2., 4., 6.];
    let weights = [5., 2.5, 1., 0.5, 1., 2.5, 5.];
    Surface {
        degree_u: 6,
        degree_v: 6,
        knots_u: [vec![0.; 7], vec![1.; 7]].concat(),
        knots_v: [vec![0.; 7], vec![1.; 7]].concat(),
        control_points: (0..7)
            .map(|i| (0..7).map(|j| vec![x[i], x[j], z[i] * z[j]]).collect())
            .collect(),
        weights: weights
            .iter()
            .map(|a| weights.iter().map(|b| a * b).collect())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    }
}
fn accepted(source: &Surface, axis: Axis) -> Vec<Surface> {
    let before = source.clone();
    let r = d::surface(source, axis, 1000000).unwrap();
    assert!(r.refusal.is_none(), "{:?},work={}", r.refusal, r.work_used);
    let out = r.patches.unwrap();
    assert_eq!(*source, before);
    #[cfg(feature = "codec")]
    println!(
        "exact-surface-content-oracle={}",
        value_codec::json!({"axis":match axis{Axis::U=>"u",Axis::V=>"v"},"source":source,"patches":out})
    );
    out
}
#[test]
fn positive_rational_source_with_hidden_polynomial_partial_no_longer_refuses() {
    let out = accepted(&hidden(), Axis::U);
    assert_eq!(out.len(), 1);
    assert_eq!((out[0].degree_u, out[0].degree_v), (1, 1));
    assert_eq!(out[0].weights, vec![vec![1.; 2]; 2]);
    assert_eq!(
        out[0].control_points,
        vec![vec![vec![0., 0., 0.]; 2], vec![vec![120., 0., 0.]; 2]]
    );
}
#[test]
fn transposed_source_cancels_v_content_with_original_clocks() {
    let s = hidden();
    let t = Surface {
        degree_u: s.degree_v,
        degree_v: s.degree_u,
        knots_u: s.knots_v.clone(),
        knots_v: s.knots_u.clone(),
        control_points: (0..2)
            .map(|j| (0..7).map(|i| s.control_points[i][j].clone()).collect())
            .collect(),
        weights: (0..2)
            .map(|j| (0..7).map(|i| s.weights[i][j]).collect())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let out = accepted(&t, Axis::V);
    assert_eq!((out[0].degree_u, out[0].degree_v), (1, 1));
    for u in [0., 0.5, 1.] {
        for v in [0., 0.5, 1.] {
            assert_eq!(out[0].evaluate(u, v).unwrap().point, [120. * v, 0., 0.]);
        }
    }
}
#[test]
fn both_axis_contents_reduce_without_independent_row_gauges() {
    let s = tensor_hidden();
    for axis in [Axis::U, Axis::V] {
        let out = accepted(&s, axis);
        assert_eq!((out[0].degree_u, out[0].degree_v), (1, 1));
        assert!(out[0].weights.iter().flatten().all(|&w| w == 1.));
        for u in [0., 0.25, 0.5, 1.] {
            for v in [0., 0.25, 0.5, 1.] {
                let expected = match axis {
                    Axis::U => [120. * u, 0., 36. * v],
                    Axis::V => [0., 120. * v, 36. * u],
                };
                assert_eq!(out[0].evaluate(u, v).unwrap().point, expected);
            }
        }
    }
}
#[test]
fn cancellation_and_actual_output_proof_share_the_work_boundary() {
    let s = hidden();
    let report = d::surface(&s, Axis::U, 1000000).unwrap();
    assert!(report.patches.is_some(), "{:?}", report.refusal);
    let n = report.work_used;
    assert!(d::surface(&s, Axis::U, n).unwrap().patches.is_some());
    let short = d::surface(&s, Axis::U, n - 1).unwrap();
    assert_eq!(short.refusal, Some(Refusal::ResourceLimit));
    assert!(short.patches.is_none());
    assert!(short.work_used < n);
}
