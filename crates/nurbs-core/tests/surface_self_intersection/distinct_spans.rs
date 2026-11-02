use super::*;
use nurbs_core::{
    curve::Curve,
    intersection::{
        intersect_curve_curve_certified_report as pair_report,
        intersect_distinct_spans_certified_report as self_report,
    },
};
fn surface() -> Surface {
    let mut s = nonlinear_crossing_strip();
    s.knots_u = vec![0., 0., 0., 1., 2., 3., 3., 3.];
    s
}
fn curve(s: &Surface) -> Curve {
    Curve {
        degree: s.degree_u,
        knots: s.knots_u.clone(),
        control_points: s.control_points.iter().map(|row| row[0].clone()).collect(),
        weights: s.weights.iter().map(|row| row[0]).collect(),
        periodic: false,
    }
}
#[test]
fn general_original_bspline_spans_do_not_need_bezier_extraction() {
    let s = surface();
    let r = surface_self_intersection::inspect(&s, 10000).unwrap();
    assert_eq!(r.status, Status::Present);
    let c = r.certified_crossing.unwrap();
    let t = (1. + 6_f64.sqrt()) / 5.;
    let x = (11. - 4. * 6_f64.sqrt()) / 25.;
    let uv = [(2. - (2. - 2. * x).sqrt()) / 3., (2. + t) / 3.];
    for (i, t) in uv.into_iter().enumerate() {
        assert!(
            c.parameter_boxes[i][0][0] <= t && t <= c.parameter_boxes[i][0][1],
            "{c:?}"
        );
    }
    for i in 0..2 {
        assert!(c.position_bounds[i][0] <= x && x <= c.position_bounds[i][1]);
    }
    assert!(c.cells <= 10000);
}
#[test]
fn distinct_span_mode_does_not_take_the_identical_curve_shortcut() {
    let c = curve(&surface());
    let pair = pair_report(&c, &c, 1e-7, 10000).unwrap();
    assert!(pair.identical_parameter_overlap);
    let distinct = self_report(&c, 1e-7, 10000).unwrap();
    assert!(!distinct.identical_parameter_overlap);
    assert!(!distinct.crossings.is_empty());
    for crossing in distinct.crossings {
        assert!(crossing.parameters[0][1] < crossing.parameters[1][0]);
    }
}
#[test]
fn distinct_original_pairs_keep_resource_exhaustion_explicit() {
    let c = curve(&surface());
    let r = self_report(&c, 1e-7, 1).unwrap();
    assert!(!r.complete);
    assert!(r.crossings.is_empty());
    assert!(!r.unresolved.is_empty());
    assert_eq!(r.cells, 1);
}
#[test]
fn empty_distinct_span_coverage_does_not_audit_single_span_loops() {
    let c = curve(&graph());
    let r = self_report(&c, 1e-7, 100).unwrap();
    assert!(r.complete);
    assert!(r.crossings.is_empty());
    assert!(!r.identical_parameter_overlap);
}

#[test]
fn general_source_knots_are_kept_on_v_axis_too() {
    let mut s = surface();
    s.control_points = (0..2)
        .map(|j| (0..5).map(|i| s.control_points[i][j].clone()).collect())
        .collect();
    s.weights = (0..2)
        .map(|j| (0..5).map(|i| s.weights[i][j]).collect())
        .collect();
    std::mem::swap(&mut s.degree_u, &mut s.degree_v);
    std::mem::swap(&mut s.knots_u, &mut s.knots_v);
    let r = surface_self_intersection::inspect(&s, 10000).unwrap();
    assert_eq!(r.status, Status::Present);
    let c = r.certified_crossing.unwrap();
    assert!(c.parameter_boxes[0][1][1] < 1. / 3.);
    assert!(c.parameter_boxes[1][1][0] > 2. / 3.);
}
#[test]
fn initial_pair_cap_is_explicit_without_claiming_complete_coverage() {
    let c = Curve::from_polyline(
        (0..100)
            .map(|i| vec![i as f64, (i % 2) as f64, 0.])
            .collect(),
    )
    .unwrap();
    let r = self_report(&c, 1e-7, 100).unwrap();
    assert!(!r.complete);
    assert_eq!(r.cells, 0);
    assert_eq!(r.unresolved.len(), 1);
    assert!(r.crossings.is_empty());
}
