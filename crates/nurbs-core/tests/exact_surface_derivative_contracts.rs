#[path = "support/exact_surface_derivative.rs"]
mod support;
use nurbs_core::{
    editing::exact_surface_derivative::{self as d, Axis, Refusal},
    surface::Surface,
};
use support::accepted;
fn rational() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 6., 6.],
        knots_v: vec![-3., -3., 5., 5.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 3., 0.]],
            vec![vec![3., 0., 0.], vec![3., 3., 3.]],
        ],
        weights: vec![vec![1., 2.], vec![2., 8.]],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn nonseparable_rational_weights_have_one_tensor_gauge_and_original_clocks() {
    for axis in [Axis::U, Axis::V] {
        let out = accepted(rational(), axis);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].degree_u, 2);
        assert_eq!(out[0].degree_v, 2);
    }
}
#[test]
fn both_nonuniform_axes_cover_every_source_cell_without_reclocking() {
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
    for axis in [Axis::U, Axis::V] {
        let out = accepted(s.clone(), axis);
        assert_eq!(out.len(), 4);
        assert!(out.iter().all(|p| p.degree_u == 1 && p.degree_v == 1));
    }
}
#[test]
fn weighted_nonuniform_multispan_denominator_is_extracted_before_differentiation() {
    let x = [12., 8., 4., 3.];
    let weights = [2., 3., 6., 8.];
    let s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 3., 3., 3.],
        knots_v: vec![-1., -1., 3., 3.],
        control_points: x
            .iter()
            .map(|&x| vec![vec![x, 0., 0.], vec![x, 2., 0.]])
            .collect(),
        weights: weights.iter().map(|&w| vec![w, w]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let du = accepted(s.clone(), Axis::U);
    assert_eq!(du.len(), 2);
    assert_eq!(du[0].control_points[0][0], vec![-12., 0., 0.]);
    assert_eq!(du[1].control_points[2][1], vec![-0.75, 0., 0.]);
    let dv = accepted(s, Axis::V);
    assert!(dv.iter().all(|p| {
        p.degree_u == 1
            && p.degree_v == 1
            && p.control_points
                .iter()
                .flatten()
                .all(|p| p == &vec![0., 0.5, 0.])
    }));
}
#[test]
fn high_source_degree_can_produce_a_proven_constant_field_within_output_bounds() {
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
    for axis in [Axis::U, Axis::V] {
        let out = accepted(s.clone(), axis);
        assert_eq!(out[0].degree_u, 1);
        assert_eq!(out[0].degree_v, 1);
    }
}
#[test]
fn tensor_axis_transpose_keeps_physical_partial_fields() {
    let s = rational();
    let mut t = s.clone();
    t.degree_u = s.degree_v;
    t.degree_v = s.degree_u;
    t.knots_u = s.knots_v.clone();
    t.knots_v = s.knots_u.clone();
    t.control_points = (0..2)
        .map(|j| (0..2).map(|i| s.control_points[i][j].clone()).collect())
        .collect();
    t.weights = (0..2)
        .map(|j| (0..2).map(|i| s.weights[i][j]).collect())
        .collect();
    let a = accepted(s, Axis::U);
    let b = accepted(t, Axis::V);
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
fn exact_storage_failure_is_atomic() {
    let mut s = rational();
    s.weights = vec![vec![1., 1.], vec![3., 3.]];
    s.control_points[1][0][0] = 1.;
    s.control_points[1][1][0] = 1.;
    let r = d::surface(&s, Axis::U, 1000000).unwrap();
    assert!(r.patches.is_none());
    assert_eq!(r.refusal, Some(Refusal::NotRepresentable));
}
#[test]
fn output_degree_limit_does_not_hide_a_constant_transverse_partial() {
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
    let r = d::surface(&s, Axis::U, 1000000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::OutputDegreeLimit));
    assert!(r.patches.is_none());
    let out = accepted(s, Axis::V);
    assert!(
        out[0]
            .control_points
            .iter()
            .flatten()
            .all(|p| p == &vec![0., 1., 0.])
    );
}
#[test]
fn one_budget_covers_all_cells_storage_and_fresh_output_identity() {
    let s = rational();
    let n = d::surface(&s, Axis::U, 1000000).unwrap().work_used;
    assert!(d::surface(&s, Axis::U, n).unwrap().patches.is_some());
    for limit in [1, n - 1] {
        let r = d::surface(&s, Axis::U, limit).unwrap();
        assert_eq!(r.refusal, Some(Refusal::ResourceLimit));
        assert!(r.patches.is_none());
        assert!(r.work_used <= limit);
    }
}
#[test]
fn invalid_input_and_work_are_errors() {
    for n in [0, 1000001, u64::MAX] {
        assert!(d::surface(&rational(), Axis::U, n).is_err());
    }
    let mut s = rational();
    s.weights[1][1] = 0.;
    assert!(d::surface(&s, Axis::U, 1000000).is_err());
    let mut s = rational();
    s.control_points[0][0][0] = f64::NAN;
    assert!(d::surface(&s, Axis::V, 1000000).is_err());
}
