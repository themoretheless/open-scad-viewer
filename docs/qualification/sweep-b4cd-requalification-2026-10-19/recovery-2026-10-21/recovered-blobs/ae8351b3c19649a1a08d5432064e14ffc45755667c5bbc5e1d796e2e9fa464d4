use nurbs_core::{
    curve::Curve,
    surface::{Axis, Surface},
    surface_normal_trace,
};
fn quarter() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![3., 0., 4.], vec![3., 8., 4.]],
            vec![vec![3., 0., 1.], vec![3., 8., 1.]],
            vec![vec![0., 0., 1.], vec![0., 8., 1.]],
        ],
        weights: vec![
            vec![1.; 2],
            vec![std::f64::consts::FRAC_1_SQRT_2; 2],
            vec![1.; 2],
        ],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn original_rational_normals_have_whole_interval_secant_error_and_reversed_coverage() {
    let s = quarter();
    let pc = Curve::from_polyline(vec![vec![0.2, 0.25], vec![0.8, 0.75]]).unwrap();
    let before = (s.clone(), pc.clone());
    let mut cells = 0;
    for reverse in [false, true] {
        for i in 0..16 {
            let q = [i as f64 / 16., (i + 1) as f64 / 16.];
            let out = surface_normal_trace::inspect(&pc, &s, q, reverse, 100000 - cells).unwrap();
            cells += out.cells;
            let endpoints = out.endpoints.unwrap_or_else(|| panic!("{out:?}"));
            let error = out.secant_error_upper.unwrap();
            for step in 0..=16 {
                let x = step as f64 / 16.;
                let t = q[0] + x * (q[1] - q[0]);
                let uv = pc.evaluate(if reverse { 1. - t } else { t }).unwrap().point;
                let normal = s.evaluate(uv[0], uv[1]).unwrap().unit_normal().unwrap();
                // Numerical reference checks the bound; sampling grants none.
                let mut squared = 0.;
                let mut endpoint_uncertainty = 0.;
                for k in 0..3 {
                    let center = |j: usize| (endpoints[j][k][0] + endpoints[j][k][1]) * 0.5;
                    let delta = normal[k] - ((1. - x) * center(0) + x * center(1));
                    squared += delta * delta;
                    endpoint_uncertainty += ((endpoints[0][k][1] - endpoints[0][k][0])
                        .max(endpoints[1][k][1] - endpoints[1][k][0])
                        * 0.5)
                        .powi(2);
                }
                assert!(
                    squared.sqrt() <= error + endpoint_uncertainty.sqrt() + 1e-12,
                    "{out:?}"
                );
            }
        }
    }
    assert!(cells <= 100000);
    let coarse = surface_normal_trace::inspect(&pc, &s, [0.25, 0.5], false, 100000).unwrap();
    let fine = surface_normal_trace::inspect(&pc, &s, [0.25, 0.375], false, 100000).unwrap();
    assert!(fine.secant_error_upper.unwrap() < coarse.secant_error_upper.unwrap());
    assert_eq!((s, pc), before);
}
#[test]
fn normal_bound_refuses_missing_work_basis_regularity_and_chart_membership() {
    let s = quarter();
    let pc = Curve::from_polyline(vec![vec![0.2, 0.25], vec![0.8, 0.75]]).unwrap();
    let zero = surface_normal_trace::inspect(&pc, &s, [0., 1.], false, 0).unwrap();
    assert!(zero.secant_error_upper.is_none() && zero.endpoints.is_none());
    let inserted = nurbs_core::surface_edit::insert(&s, Axis::U, 0.5, 1).unwrap();
    let out = surface_normal_trace::inspect(&pc, &inserted, [0., 1.], false, 100000).unwrap();
    assert_eq!(out.reason, "surface-trace-C2-unproven");
    assert!(out.secant_error_upper.is_none());
    assert!(
        surface_normal_trace::inspect(&pc, &inserted, [0., 0.125], false, 100000)
            .unwrap()
            .secant_error_upper
            .is_some()
    );
    let outside = Curve::from_polyline(vec![vec![0.2, 0.25], vec![1.2, 0.75]]).unwrap();
    assert!(surface_normal_trace::inspect(&outside, &s, [0., 1.], false, 100000).is_err());
    let mut folded = s.clone();
    folded.weights = vec![vec![1.; 2]; 3];
    for (i, x) in [0., 1., 0.].into_iter().enumerate() {
        folded.control_points[i] = vec![vec![x, 0., 0.], vec![x, 8., 0.]];
    }
    let out = surface_normal_trace::inspect(&pc, &folded, [0., 1.], false, 100000).unwrap();
    assert_eq!(out.reason, "whole-trace-regularity-unproven");
    assert!(out.secant_error_upper.is_none() && out.second_derivative_bounds.is_none());
}
