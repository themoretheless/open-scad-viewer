//! Full original cells and joins of the new profile; no body qualification.
use nurbs_core::{
    analysis::bezier_curvature_bound, construct::curves::bounded_corner, curve::Curve,
    surface::Surface, surface_g2_pcurves as g2, surface_join::Decision,
};

#[test]
fn matched_original_profiles_keep_the_cap_and_complete_g2_at_both_tolerances() {
    for tolerance in [1e-7, 2.5e-8] {
        for radius in [0.1, 0.2, 0.35] {
            let proposal = bounded_corner::propose(radius, 10., tolerance).unwrap();
            let c = proposal.composite_matched_g2().unwrap();
            let before = c.clone();
            assert_eq!(c.degree, 5);
            assert_eq!(
                c.control_points.first(),
                proposal.parts[0].control_points.first()
            );
            assert_eq!(
                c.control_points.last(),
                proposal.parts[2].control_points.last()
            );
            assert!(c.control_points[..3].iter().all(|p| p[0] == 0.));
            assert!(c.control_points[13..].iter().all(|p| p[1] == 0.));
            for start in [0, 5, 10] {
                let cell = Curve {
                    degree: 5,
                    knots: [vec![0.; 6], vec![1.; 6]].concat(),
                    control_points: c.control_points[start..start + 6].to_vec(),
                    weights: c.weights[start..start + 6].to_vec(),
                    periodic: false,
                };
                let report = bezier_curvature_bound::inspect(&cell, 10., 4096, 1000000).unwrap();
                let certificate = report.certificate.unwrap_or_else(|| {
                    panic!(
                        "radius={radius} tolerance={tolerance} cell={start}: {:?}",
                        report.decision
                    )
                });
                assert!(std::ptr::eq(certificate.source(), &cell));
                assert_eq!(certificate.maximum(), 10.);
                assert!(
                    bezier_curvature_bound::inspect(&cell, 10., 0, 1000000)
                        .unwrap()
                        .certificate
                        .is_none()
                );
            }
            let s = Surface {
                degree_u: 5,
                degree_v: 1,
                knots_u: c.knots.clone(),
                knots_v: vec![0., 0., 1., 1.],
                control_points: c
                    .control_points
                    .iter()
                    .map(|p| vec![p.clone(), vec![p[0], p[1], 1.]])
                    .collect(),
                weights: c.weights.iter().map(|w| vec![*w; 2]).collect(),
                periodic_u: false,
                periodic_v: false,
            };
            for knot in [c.knots[6], c.knots[11]] {
                for report in [
                    g2::inspect_original_span_join(&s, knot, 1e-5, 1e-4, 6000).unwrap(),
                    g2::inspect_original_geometric_span_join(&s, knot, 1e-5, 1e-4, 6000).unwrap(),
                ] {
                    assert_eq!(
                        report.decision,
                        Decision::WithinTolerance,
                        "r={radius} tol={tolerance} knot={knot}: normal={:?} tensor={:?}",
                        report.normal,
                        report.tensor
                    );
                    assert!(report.regularity_proven && report.pending == 0);
                }
            }
            assert_eq!(c, before);
        }
    }
}

#[test]
fn invalid_authored_parts_do_not_panic_or_grant_a_profile() {
    let mut proposal = bounded_corner::propose(0.1, 10., 1e-7).unwrap();
    proposal.parts[0] = Curve::from_polyline(vec![vec![0.; 3], vec![1.; 3]]).unwrap();
    assert!(proposal.composite_matched_g2().is_err());
}

#[test]
fn single_quartic_has_no_internal_knots_and_proves_cap_and_flat_endpoint_g2() {
    use nurbs_core::curve_join::{self, Endpoint, Tolerances};
    for tolerance in [1e-7, 2.5e-8] {
        for radius in [0.1, 0.2, 0.35] {
            let proposal = bounded_corner::propose(radius, 10., tolerance).unwrap();
            let c = proposal.quartic_g2().unwrap();
            assert_eq!(c.degree, 4);
            assert_eq!(c.control_points.len(), 5);
            assert_eq!(c.knots, vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.]);
            let report = bezier_curvature_bound::inspect(&c, 10., 4096, 1000000).unwrap();
            let certificate = report
                .certificate
                .unwrap_or_else(|| panic!("r={radius} tol={tolerance}: {:?}", report.decision));
            assert!(std::ptr::eq(certificate.source(), &c));
            let y = c.control_points[0][1];
            let a = Curve::from_polyline(vec![vec![0., y + 1., 0.], vec![0., y, 0.]]).unwrap();
            let b = Curve::from_polyline(vec![vec![y, 0., 0.], vec![y + 1., 0., 0.]]).unwrap();
            for (first, fe, second, se) in [
                (&a, Endpoint::End, &c, Endpoint::Start),
                (&c, Endpoint::End, &b, Endpoint::Start),
            ] {
                let report = curve_join::inspect(
                    first,
                    fe,
                    second,
                    se,
                    Tolerances {
                        position: 1e-7,
                        tangent: 1e-5,
                        curvature: 1e-4,
                    },
                )
                .unwrap();
                assert_eq!(report.g2, curve_join::Decision::WithinTolerance);
            }
            if radius == 0.1 {
                let rejected = bezier_curvature_bound::inspect(&c, 9., 4096, 1000000).unwrap();
                assert!(rejected.certificate.is_none());
                assert_eq!(
                    rejected.decision.outcome,
                    cad_predicates::CurvatureBoundOutcome::Exceeds
                );
                assert_eq!(rejected.decision.witness_parameter, Some(0.5));
                assert!(
                    bezier_curvature_bound::inspect(&c, 9., 1, 1000000)
                        .unwrap()
                        .certificate
                        .is_none()
                );
            }
        }
    }
}
