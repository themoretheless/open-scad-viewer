#[path = "support/exact_surface_derivative.rs"]
mod support;
use nurbs_core::{
    editing::exact_surface_derivative::{self as d, Axis, Refusal},
    surface::Surface,
};
use support::accepted;
#[test]
fn both_periodic_axes_keep_every_fundamental_cell_and_unwrapped_traces() {
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
    for axis in [Axis::U, Axis::V] {
        let out = accepted(s.clone(), axis);
        assert_eq!(out.len(), 9);
    }
}
#[test]
fn unclamped_quadratic_tensor_uses_actual_extended_knots() {
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
    for axis in [Axis::U, Axis::V] {
        assert_eq!(accepted(s.clone(), axis).len(), 9);
    }
}
#[test]
fn repeated_uv_knots_keep_distinct_one_sided_partial_traces() {
    let x = [0., 1., 2., 2.5, 4.];
    let y = [0., 1., 2., 3., 4.];
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
    let du = accepted(s.clone(), Axis::U);
    assert_eq!(du[0].evaluate(1., -0.5).unwrap().point, [2., 0., 2.]);
    assert_eq!(du[2].evaluate(1., -0.5).unwrap().point, [1., 0., 1.]);
    let dv = accepted(s, Axis::V);
    assert_eq!(dv[0].evaluate(0.5, 0.).unwrap().point, [0., 2., 2.]);
    assert_eq!(dv[1].evaluate(0.5, 0.).unwrap().point, [0., 1., 1.]);
}
#[test]
fn late_cell_storage_refusal_does_not_publish_an_earlier_success() {
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
    let before = s.clone();
    let first = Surface {
        knots_u: vec![0., 0., 1., 1.],
        control_points: s.control_points[..2].to_vec(),
        weights: s.weights[..2].to_vec(),
        ..s.clone()
    };
    let good = d::surface(&first, Axis::U, 1000000).unwrap();
    assert!(good.patches.is_some());
    let bad = d::surface(&s, Axis::U, 1000000).unwrap();
    assert_eq!(bad.refusal, Some(Refusal::NotRepresentable));
    assert!(bad.patches.is_none());
    assert!(bad.work_used > good.work_used);
    assert_eq!(s, before);
}
#[test]
fn existing_weight_conditioning_is_not_relaxed_for_exact_fields() {
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
    let r = d::surface(&s, Axis::U, 1000000).unwrap();
    assert!(r.patches.is_none());
    assert_eq!(r.refusal, Some(Refusal::InvalidOutput));
}
