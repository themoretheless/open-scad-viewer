use super::*;
use nurbs_core::intersection::intersect_surface_iso_certified_report as audit;
fn source() -> Surface {
    let mut s = nonlinear_crossing_strip();
    s.degree_v = 2;
    s.knots_v = vec![0., 0., 0., 1., 1., 1.];
    s.control_points = s
        .control_points
        .iter()
        .map(|row| {
            let x = row[0][0];
            let y = row[0][1];
            [0., 1., 2.].into_iter().map(|z| vec![x, y, z]).collect()
        })
        .collect();
    s.weights = vec![vec![1., 2., 2.]; 5];
    s
}
#[test]
fn rational_iso_is_proved_on_original_surface_bounds() {
    let r = audit(&source(), 0, 1e-7, 10000).unwrap();
    assert!(
        !r.crossings.is_empty(),
        "cells={}, unresolved={:?}",
        r.cells,
        r.unresolved
    );
}

#[test]
fn all_cartesian_iso_controls_can_remain_exact_rational_values() {
    let mut s = source();
    for row in &mut s.control_points {
        for (j, p) in row.iter_mut().enumerate() {
            p[0] += j as f64;
            p[1] += j as f64;
        }
    }
    let r = audit(&s, 0, 1e-7, 10000).unwrap();
    assert!(!r.crossings.is_empty(), "{r:?}");
    let c = &r.crossings[0];
    for (i, x) in [79. / 63., 79. / 63., 8. / 7.].into_iter().enumerate() {
        assert!(c.position_bounds[i][0] <= x && x <= c.position_bounds[i][1]);
    }
    assert!(!r.complete);
    assert!(!r.unresolved.is_empty());
}
#[test]
fn original_surface_bounds_work_on_v_axis_without_float_controls() {
    let mut s = source();
    s.control_points = (0..3)
        .map(|j| (0..5).map(|i| s.control_points[i][j].clone()).collect())
        .collect();
    s.weights = (0..3)
        .map(|j| (0..5).map(|i| s.weights[i][j]).collect())
        .collect();
    std::mem::swap(&mut s.degree_u, &mut s.degree_v);
    std::mem::swap(&mut s.knots_u, &mut s.knots_v);
    let r = audit(&s, 1, 1e-7, 10000).unwrap();
    assert!(!r.crossings.is_empty());
    let c = &r.crossings[0];
    assert!(c.position_bounds[2][0] <= 8. / 7. && 8. / 7. <= c.position_bounds[2][1]);
}
#[test]
fn original_iso_budget_and_axis_validation_remain_explicit() {
    let s = source();
    let r = audit(&s, 0, 1e-7, 1).unwrap();
    assert!(!r.complete);
    assert!(r.crossings.is_empty());
    assert_eq!(r.cells, 1);
    assert!(audit(&s, 2, 1e-7, 100).is_err());
    assert!(audit(&s, 0, 1e-7, 0).is_err());
}

#[test]
fn oblique_monotonic_projection_excludes_only_the_shared_join() {
    let c = nurbs_core::curve::Curve::from_polyline(vec![
        vec![0., 0., 0.],
        vec![2., 2., 0.],
        vec![2., -2., 0.],
    ])
    .unwrap();
    let r = nurbs_core::intersection::intersect_self_curve_certified_report(&c, 1e-7, 100).unwrap();
    assert!(r.complete, "{r:?}");
    assert!(r.crossings.is_empty());
}
#[test]
fn reversed_tangents_at_a_join_do_not_prove_injectivity() {
    let c = nurbs_core::curve::Curve::from_polyline(vec![
        vec![0., 0., 0.],
        vec![3., 0., 0.],
        vec![1., 0., 0.],
    ])
    .unwrap();
    let r = nurbs_core::intersection::intersect_self_curve_certified_report(&c, 1e-7, 100).unwrap();
    assert!(!r.complete);
    assert!(!r.unresolved.is_empty());
}
