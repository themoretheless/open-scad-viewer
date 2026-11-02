use nurbs_core::{
    curve::Curve,
    surface::Surface,
    surface_offset::contact_curve::lift::{self, FitLimits, FitStop},
};
fn cylinder() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [[1., 0.], [1., 1.], [0., 1.]]
            .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]])
            .to_vec(),
        weights: [1., std::f64::consts::FRAC_1_SQRT_2, 1.]
            .map(|w| vec![w; 2])
            .to_vec(),
        periodic_u: false,
        periodic_v: false,
    }
}
fn uv() -> Curve {
    Curve {
        degree: 1,
        knots: vec![2., 2., 7., 7.],
        control_points: vec![vec![0.1, 0.4], vec![0.4, 0.4]],
        weights: vec![1.; 2],
        periodic: false,
    }
}
#[test]
fn nonlinear_original_offset_lift_refines_actual_coefficients_and_retains_clock() {
    let s = cylinder();
    let u = uv();
    let saved = (s.clone(), u.clone());
    let l = FitLimits::default();
    let r = lift::fit(&u, &s, 0.1, 1e-7, None, l).unwrap();
    assert_eq!(r.stop, FitStop::Certified);
    let w = r.curve.as_ref().unwrap();
    assert!(w.degree > 1);
    assert_eq!(w.domain(), [2., 7.]);
    assert!(
        r.evaluations <= l.evaluations
            && r.coefficient_work <= l.coefficient_work
            && r.proof_cells <= l.proof_cells
    );
    assert!(
        lift::certify(
            w,
            &u,
            &s,
            0.1,
            1e-7,
            lift::Limits {
                cells: 100000,
                spans: 128
            }
        )
        .unwrap()
        .lift_correspondence_proven
    );
    assert_eq!((s, u), saved);
}
#[test]
fn forced_wrong_world_endpoints_or_any_shared_work_limit_never_publish_a_curve() {
    let s = cylinder();
    let u = uv();
    let bad = [[0., 0., 4.], [0., 0., 4.]];
    assert!(
        lift::fit(&u, &s, 0.1, 1e-7, Some(bad), FitLimits::default())
            .unwrap()
            .curve
            .is_none()
    );
    for kind in 0..4 {
        let mut l = FitLimits::default();
        match kind {
            0 => l.max_degree = 1,
            1 => l.evaluations = 1,
            2 => l.coefficient_work = 1,
            _ => l.proof_cells = 1,
        }
        assert!(
            lift::fit(&u, &s, 0.1, 1e-7, None, l)
                .unwrap()
                .curve
                .is_none()
        );
    }
}
