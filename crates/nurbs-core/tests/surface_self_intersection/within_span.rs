use super::*;
use nurbs_core::{curve::Curve, intersection::intersect_self_curve_certified_report as audit};
pub(super) fn loop_surface() -> Surface {
    Surface {
        degree_u: 3,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [
            [9. / 16., -9. / 32.],
            [-7. / 16., 13. / 32.],
            [-7. / 16., -13. / 32.],
            [9. / 16., 9. / 32.],
        ]
        .into_iter()
        .map(|[x, y]| vec![vec![x, y, 0.], vec![x, y, 2.]])
        .collect(),
        weights: vec![vec![1.; 2]; 4],
        periodic_u: false,
        periodic_v: false,
    }
}
fn curve(s: &Surface) -> Curve {
    Curve {
        degree: s.degree_u,
        knots: s.knots_u.clone(),
        control_points: s.control_points.iter().map(|r| r[0].clone()).collect(),
        weights: vec![1.; s.control_points.len()],
        periodic: false,
    }
}
#[test]
fn single_cubic_span_loop_has_certified_distinct_preimages() {
    let r = surface_self_intersection::inspect(&loop_surface(), 10000).unwrap();
    assert_eq!(r.status, Status::Present);
    let c = r.certified_crossing.unwrap();
    for (i, t) in [0.25, 0.75].into_iter().enumerate() {
        assert!(
            c.parameter_boxes[i][0][0] <= t && t <= c.parameter_boxes[i][0][1],
            "{c:?}"
        );
    }
    for i in 0..2 {
        assert!(c.position_bounds[i][0] <= 0. && 0. <= c.position_bounds[i][1]);
    }
}
#[test]
fn local_derivative_proofs_exclude_the_diagonal_of_a_simple_curve() {
    let r = audit(&curve(&graph()), 1e-7, 100).unwrap();
    assert!(r.complete);
    assert!(r.crossings.is_empty());
    assert!(r.unresolved.is_empty());
}
#[test]
fn within_span_budget_exhaustion_is_not_injectivity() {
    let r = audit(&curve(&loop_surface()), 1e-7, 1).unwrap();
    assert!(!r.complete);
    assert_eq!(r.cells, 1);
    assert!(r.crossings.is_empty());
    assert!(!r.unresolved.is_empty());
}
#[test]
fn stationary_fold_cannot_be_dismissed_as_the_trivial_diagonal() {
    let mut s = graph();
    s.control_points[2] = s.control_points[0].clone();
    let r = audit(&curve(&s), 1e-7, 100).unwrap();
    assert!(!r.complete);
    assert!(r.crossings.is_empty());
    assert!(!r.unresolved.is_empty());
}

#[test]
fn whole_planar_loop_coverage_can_be_complete() {
    let r = audit(&curve(&loop_surface()), 1e-7, 10000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.crossings.len(), 1);
    assert!(r.unresolved.is_empty());
}
#[test]
fn injective_arc_can_cover_its_diagonal_without_a_global_strict_coordinate() {
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let r = audit(&c, 1e-7, 10000).unwrap();
    assert!(r.complete, "{r:?}");
    assert!(r.crossings.is_empty());
    assert!(r.cells > 1);
}
#[test]
fn nonunit_single_span_loop_preserves_normalized_source_boxes() {
    let mut s = loop_surface();
    s.knots_u = vec![-2., -2., -2., -2., 6., 6., 6., 6.];
    let r = surface_self_intersection::inspect(&s, 10000).unwrap();
    assert_eq!(r.status, Status::Present);
    let c = r.certified_crossing.unwrap();
    for (i, t) in [0.25, 0.75].into_iter().enumerate() {
        assert!(c.parameter_boxes[i][0][0] <= t && t <= c.parameter_boxes[i][0][1]);
    }
}
#[test]
fn rational_weights_do_not_require_an_exact_parametric_root() {
    let mut s = loop_surface();
    s.weights = [1., 1. + 1. / 1024., 1. - 1. / 1024., 1.]
        .into_iter()
        .map(|w| vec![w; 2])
        .collect();
    let r = surface_self_intersection::inspect(&s, 10000).unwrap();
    assert_eq!(r.status, Status::Present);
    assert!(r.certified_crossing.is_some());
    assert!(r.exact_witness.is_none());
}

#[test]
fn single_span_loop_is_certified_on_v_axis_too() {
    let mut s = loop_surface();
    s.control_points = (0..2)
        .map(|j| (0..4).map(|i| s.control_points[i][j].clone()).collect())
        .collect();
    s.weights = (0..2)
        .map(|j| (0..4).map(|i| s.weights[i][j]).collect())
        .collect();
    std::mem::swap(&mut s.degree_u, &mut s.degree_v);
    std::mem::swap(&mut s.knots_u, &mut s.knots_v);
    let r = surface_self_intersection::inspect(&s, 10000).unwrap();
    assert_eq!(r.status, Status::Present);
    let c = r.certified_crossing.unwrap();
    for (i, t) in [0.25, 0.75].into_iter().enumerate() {
        assert_eq!(c.parameter_boxes[i][0], [0.5, 0.5]);
        assert!(c.parameter_boxes[i][1][0] <= t && t <= c.parameter_boxes[i][1][1]);
    }
}
