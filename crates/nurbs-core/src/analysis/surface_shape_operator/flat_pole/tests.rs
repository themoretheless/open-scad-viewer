use super::*;
fn source(rational: bool) -> Surface {
    Surface {
        degree_u: 3,
        degree_v: 1,
        knots_u: [vec![0.; 4], vec![1.; 4]].concat(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..4)
            .map(|i| {
                [0., 1.]
                    .into_iter()
                    .map(|v| {
                        vec![
                            i as f64 / 3. * (1. - v),
                            i as f64 / 3. * v,
                            if i == 3 { 0.125 } else { 0. },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![if rational { vec![1., 2.] } else { vec![1., 1.] }; 4],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn exact_factored_original_poles_have_uniform_flat_normal_and_tensor_limits() {
    for rational in [false, true] {
        for end in [false, true] {
            let mut s = source(rational);
            if end {
                s.control_points.reverse();
                s.weights.reverse();
            }
            let before = s.clone();
            let proof = inspect_original_flat_pole(&s, end, 2, 0.125, 128).unwrap();
            assert!(
                proof.report.proven,
                "rational={rational} end={end}: {}",
                proof.report.reason
            );
            let c = proof.certificate.unwrap();
            assert_eq!(c.normal(), [0., 0., if end { -1. } else { 1. }]);
            assert_eq!(c.pole(), [0.; 3]);
            assert!(c.normal_coefficient().is_finite() && c.tensor_coefficient().is_finite());
            for u in [0.125_f64, 0.0625, 0.03125] {
                let d = (1. + 18. * 0.125_f64.powi(2) * u.powi(4)).sqrt();
                let normal = (2. - 2. / d).sqrt();
                let tensor = 1.5 * u / d.powi(3);
                assert!(normal <= u * u * c.normal_coefficient());
                assert!(tensor <= u * c.tensor_coefficient());
            }
            c.reverify().unwrap();
            assert_eq!(s, before);
        }
    }
}
#[test]
fn subnormal_height_or_pole_drifts_do_not_become_literal_zero_factors() {
    let mut s = source(false);
    s.control_points[2][0][2] = 0_f64.next_up();
    let r = inspect_original_flat_pole(&s, false, 2, 0.125, 128).unwrap();
    assert!(!r.report.proven && r.certificate.is_none() && r.report.cells == 0);
    let mut s = source(false);
    s.control_points[0][1][0] = 0_f64.next_up();
    let r = inspect_original_flat_pole(&s, false, 2, 0.125, 128).unwrap();
    assert!(!r.report.proven && r.certificate.is_none());
}
#[test]
fn an_actual_fold_in_the_requested_original_band_is_not_hidden_by_regular_limits() {
    let mut s = source(false);
    let radius = [0., 1. / 3., 0., -1.];
    for (row, r) in s.control_points.iter_mut().zip(radius) {
        row[0] = vec![r, 0., 0.];
        row[1] = vec![0., r, 0.];
    }
    let folded = inspect_original_flat_pole(&s, false, 2, 0.5, 64).unwrap();
    assert!(!folded.report.proven && folded.certificate.is_none());
    let regular = inspect_original_flat_pole(&s, false, 2, 0.125, 128).unwrap();
    assert!(regular.report.proven, "{}", regular.report.reason);
}
#[test]
fn missing_work_or_invalid_query_never_mints_a_pole_certificate() {
    let s = source(false);
    let r = inspect_original_flat_pole(&s, false, 2, 0.125, 0).unwrap();
    assert!(!r.report.proven && r.report.cells == 0 && r.certificate.is_none());
    assert!(inspect_original_flat_pole(&s, false, 3, 0.125, 128).is_err());
    assert!(inspect_original_flat_pole(&s, false, 2, 0., 128).is_err());
    assert!(inspect_original_flat_pole(&s, false, 2, f64::NAN, 128).is_err());
    assert!(inspect_original_flat_pole(&s, false, 2, 0.125, 100001).is_err());
}

#[test]
fn a_winding_fan_cannot_mint_a_boundary_pole_quotient_from_flat_limits() {
    let points = [[1., 0.], [-1., 2.], [-1., -2.], [1., 0.]];
    let s = Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: [vec![0.; 4], vec![1.; 4]].concat(),
        knots_v: [vec![0.; 4], vec![1.; 4]].concat(),
        control_points: (0..4)
            .map(|i| {
                points
                    .iter()
                    .map(|p| vec![i as f64 / 3. * p[0], i as f64 / 3. * p[1], 0.])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    };
    let proof = inspect_original_flat_pole(&s, false, 2, 0.125, 128).unwrap();
    assert!(!proof.report.quotient_proven && proof.certificate.is_none());
    if let Some(angle) = proof.report.fan_angle_upper {
        assert!(angle > 4.);
    }
}
