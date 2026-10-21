use nurbs_core::{curve::Curve, curve_secant_trace};
#[test]
fn original_rational_curve_secants_cover_all_points_and_budget_refusals() {
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![2., 0., 0.], vec![2., 2., 0.], vec![0., 2., 0.]],
        weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
        periodic: false,
    };
    let saved = c.clone();
    for reverse in [false, true] {
        let out = curve_secant_trace::inspect(&c, [0.2, 0.7], reverse, 100000).unwrap();
        let ends = out.endpoints.unwrap();
        let error = out.secant_error_upper.unwrap();
        for i in 0..=100 {
            let x = i as f64 / 100.;
            let q = 0.2 + 0.5 * x;
            let point = c.evaluate(if reverse { 1. - q } else { q }).unwrap().point;
            let mut distance = 0_f64;
            let mut uncertainty = 0_f64;
            for k in 0..3 {
                let m = |j: usize| (ends[j][k][0] + ends[j][k][1]) * 0.5;
                distance += (point[k] - ((1. - x) * m(0) + x * m(1))).powi(2);
                uncertainty += ((ends[0][k][1] - ends[0][k][0]).max(ends[1][k][1] - ends[1][k][0])
                    * 0.5)
                    .powi(2);
            }
            assert!(distance.sqrt() <= error + uncertainty.sqrt() + 1e-12);
        }
        for limit in 0..out.cells {
            let partial = curve_secant_trace::inspect(&c, [0.2, 0.7], reverse, limit).unwrap();
            assert!(partial.endpoints.is_none() && partial.secant_error_upper.is_none());
            assert!(partial.cells <= limit);
        }
    }
    assert_eq!(c, saved);
}
#[test]
fn a_derivative_jump_grants_range_coverage_but_no_global_secant_error() {
    let c =
        Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.], vec![1., 1., 0.]]).unwrap();
    let out = curve_secant_trace::inspect(&c, [0., 1.], false, 100000).unwrap();
    assert!(out.secant_error_upper.is_none());
    assert_eq!(out.reason, "trace-C1-unproven");
    let bounds = curve_secant_trace::range_bounds(&c, [0., 1.], false, 100000)
        .unwrap()
        .bounds
        .unwrap();
    assert!(bounds[0][0] <= 0. && bounds[0][1] >= 1. && bounds[1][0] <= 0. && bounds[1][1] >= 1.);
    assert!(
        curve_secant_trace::inspect(&c, [0., 0.25], false, 100000)
            .unwrap()
            .secant_error_upper
            .is_some()
    );
}
