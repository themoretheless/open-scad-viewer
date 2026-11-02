use nurbs_core::{curve::Curve, curve_surface_agreement as a, surface::Surface};
fn support() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        vec![
                            i as f64 / 2.,
                            j as f64 / 2.,
                            if i == 2 { 0.25 } else { 0. } + if j == 2 { 0.125 } else { 0. },
                        ]
                    })
                    .collect()
            })
            .collect(),
    }
}
fn line() -> Curve {
    Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: vec![vec![0.125, 0.25], vec![0.875, 0.75]],
        weights: vec![1.; 2],
        periodic: false,
    }
}
#[test]
fn straight_trace_cancels_exact_high_powers_before_float_conversion() {
    let s = support();
    let p = line();
    let before = (s.clone(), p.clone());
    let r = a::lift(&p, &s, None, 1e-12, 100_000).unwrap();
    let c = r.curve.unwrap();
    assert_eq!(c.degree, 2);
    assert_eq!(c.domain(), p.domain());
    let pulled = nurbs_core::curve_pullback::project(
        &c,
        &s,
        nurbs_core::curve_pullback::Limits {
            tolerance: 1e-12,
            max_segments: 256,
            max_iterations: 32,
            agreement_cells: 100_000,
        },
    )
    .unwrap()
    .pcurve
    .unwrap();
    assert_eq!(pulled.degree, 1);
    assert_eq!(
        a::verify_exact(&c, &pulled, &s, false, 1_000_000)
            .unwrap()
            .unwrap()
            .outcome,
        cad_predicates::BezierIdentity::Equal
    );
    let exact = a::verify_exact(&c, &p, &s, false, 1_000_000)
        .unwrap()
        .unwrap();
    assert_eq!(exact.outcome, cad_predicates::BezierIdentity::Equal);
    assert_eq!(r.evidence.unwrap().status, a::Status::WithinTolerance);
    assert_eq!((s, p), before);
}
#[test]
fn reversal_and_owned_endpoint_edits_receive_fresh_original_admission() {
    let s = support();
    let p = line().reverse().unwrap();
    let r = a::lift(&p, &s, None, 1e-12, 100_000).unwrap();
    let c = r.curve.unwrap();
    assert_eq!(
        a::verify_exact(&c, &p, &s, false, 1_000_000)
            .unwrap()
            .unwrap()
            .outcome,
        cad_predicates::BezierIdentity::Equal
    );
    let d = p.domain();
    let mut ends: [[f64; 3]; 2] = [
        s.evaluate(
            p.evaluate(d[0]).unwrap().point[0],
            p.evaluate(d[0]).unwrap().point[1],
        )
        .unwrap()
        .point
        .try_into()
        .unwrap(),
        s.evaluate(
            p.evaluate(d[1]).unwrap().point[0],
            p.evaluate(d[1]).unwrap().point[1],
        )
        .unwrap()
        .point
        .try_into()
        .unwrap(),
    ];
    ends[1][2] += 0.01;
    assert!(
        a::lift(&p, &s, Some(ends), 1e-10, 100_000)
            .unwrap()
            .curve
            .is_none()
    );
    assert!(a::lift(&p, &s, None, 1e-10, 0).unwrap().curve.is_none());
}
#[test]
fn a_small_nonzero_fourth_power_is_not_cancelled_as_roundoff() {
    let mut s = support();
    s.control_points[1][1][2] += 2f64.powi(-30);
    let p = line();
    let c = a::lift(&p, &s, None, 1e-10, 100_000)
        .unwrap()
        .curve
        .unwrap();
    assert_eq!(c.degree, 4);
    assert_eq!(
        a::verify(&c, &p, &s, false, 1e-10, 100_000).unwrap().status,
        a::Status::WithinTolerance
    );
}

#[test]
fn nonlinear_chart_progression_is_not_flattened_with_a_tolerance() {
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![0.125, 0.25, 0.],
            vec![0.5 + 2f64.powi(-30), 0.5, 0.],
            vec![0.875, 0.75, 0.],
        ],
        weights: vec![1.; 3],
        periodic: false,
    };
    let p = nurbs_core::curve_pullback::project(
        &c,
        &s,
        nurbs_core::curve_pullback::Limits {
            tolerance: 1e-12,
            max_segments: 256,
            max_iterations: 32,
            agreement_cells: 100_000,
        },
    )
    .unwrap()
    .pcurve
    .unwrap();
    assert_eq!(p.degree, 2);
    assert_eq!(
        a::verify_exact(&c, &p, &s, false, 1_000_000)
            .unwrap()
            .unwrap()
            .outcome,
        cad_predicates::BezierIdentity::Equal
    );
}
