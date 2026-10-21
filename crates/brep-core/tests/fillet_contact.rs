use brep_core::fillet_contact::{self, Status};
use nurbs_core::{
    offset::fillet_surfaces::{FilletOptions, RadiusLaw},
    surface::Surface,
};
fn plane(u: [f64; 3], v: [f64; 3]) -> Surface {
    let p = |a: f64, b: f64| (0..3).map(|k| u[k] * a + v[k] * b).collect::<Vec<_>>();
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![vec![p(0., 0.), p(0., 10.)], vec![p(10., 0.), p(10., 10.)]],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn curved_rational_contacts_certify_ball_centers_with_rotating_original_normals() {
    let w = std::f64::consts::FRAC_1_SQRT_2;
    let arc = |r: f64| nurbs_core::curve::Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![r, 0., 0.], vec![r, 0., -r], vec![0., 0., -r]],
        weights: vec![1., w, 1.],
        periodic: false,
    };
    let cylinder = |r: f64| {
        let c = arc(r);
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: c.knots.clone(),
            knots_v: vec![0., 0., 1., 1.],
            control_points: c
                .control_points
                .iter()
                .map(|p| vec![vec![p[0], -2., p[2]], vec![p[0], 2., p[2]]])
                .collect(),
            weights: c.weights.iter().map(|&w| vec![w; 2]).collect(),
            periodic_u: false,
            periodic_v: false,
        }
    };
    let a = cylinder(5.);
    let b = cylinder(3.);
    let section = [
        (5., 0., 1.),
        (5., 1., w),
        (4., 1., 1.),
        (3., 1., w),
        (3., 0., 1.),
    ];
    let radial = [[1., 0.], [1., -1.], [0., -1.]];
    // An authored half-torus is a genuine rolling-ball skin. Its G2 against
    // these cylinders is a separate claim; this test qualifies the radius law.
    let skin = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
        control_points: radial
            .iter()
            .map(|p| {
                section
                    .iter()
                    .map(|&(r, y, _)| vec![r * p[0], y, r * p[1]])
                    .collect()
            })
            .collect(),
        weights: [1., w, 1.]
            .iter()
            .map(|&u| section.iter().map(|&(_, _, v)| u * v).collect())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let contacts: Vec<_> = [(5., 0., &a), (3., 1., &b)]
        .iter()
        .map(|&(radius, v, support)| {
            let curve = arc(radius);
            let support_pcurve =
                nurbs_core::curve::Curve::from_polyline(vec![vec![0., 0.5], vec![1., 0.5]])
                    .unwrap();
            let fillet_pcurve =
                nurbs_core::curve::Curve::from_polyline(vec![vec![0., v], vec![1., v]]).unwrap();
            let positions = nurbs_core::surface_g2_pcurves::verify_positions(
                &curve,
                [&support_pcurve, &fillet_pcurve],
                [support, &skin],
                [false; 2],
                1e-6,
                100000,
                1000000,
            )
            .unwrap();
            assert!(positions.proven);
            fillet_contact::Contact {
                curve,
                support_pcurve,
                fillet_pcurve,
                positions,
            }
        })
        .collect();
    let before = (a.clone(), b.clone(), skin.clone());
    let result = brep_core::fillet_radius::inspect(
        [&a, &b],
        &skin,
        [&contacts[0], &contacts[1]],
        &RadiusLaw::Constant(1.),
        [-1., 1.],
        1e-6,
        100000,
        1000000,
    )
    .unwrap();
    assert_eq!(
        result.decision,
        brep_core::fillet_radius::Decision::WithinTolerance,
        "reason={} cells={} bound={:?}",
        result.reason,
        result.cells,
        result.maximum_center_gap_mm
    );
    assert_eq!(result.reason, "complete-original-normal-secant-bound");
    eprintln!(
        "curved-contact-radius cells={} exact={} upper_mm={}",
        result.cells,
        result.exact_work,
        result.maximum_center_gap_mm.unwrap()[1]
    );
    assert!(result.maximum_center_gap_mm.unwrap()[1] <= 1e-6);
    assert!(result.cells <= 100000 && result.exact_work <= 1000000);
    let wrong = brep_core::fillet_radius::inspect(
        [&a, &b],
        &skin,
        [&contacts[0], &contacts[1]],
        &RadiusLaw::Constant(2.),
        [-1., 1.],
        1e-6,
        100000,
        1000000,
    )
    .unwrap();
    assert_eq!(
        wrong.decision,
        brep_core::fillet_radius::Decision::ExceedsTolerance
    );
    assert!(wrong.violation_witness.unwrap()[1] > 1e-6);
    let partial = brep_core::fillet_radius::inspect(
        [&a, &b],
        &skin,
        [&contacts[0], &contacts[1]],
        &RadiusLaw::Constant(1.),
        [-1., 1.],
        1e-6,
        200,
        1000000,
    )
    .unwrap();
    assert_eq!(
        partial.decision,
        brep_core::fillet_radius::Decision::Unresolved
    );
    assert!(partial.maximum_center_gap_mm.is_none());
    assert!(partial.cells <= 200);
    assert_eq!((a, b, skin), before);
}
#[test]
fn variable_nominal_radii_build_c2_skins_with_whole_interval_plane_contacts_and_g2() {
    let a = plane([1., 0., 0.], [0., 1., 0.]);
    let b = plane([0., 1., 0.], [0., 0., 1.]);
    let before = [a.clone(), b.clone()];
    let options = FilletOptions {
        max_sections: 5,
        ..Default::default()
    };
    for (start, end) in [(0.5, 1.5), (1.5, 0.5), (0.1, 0.2)] {
        let law = RadiusLaw::Linear { start, end };
        let report =
            fillet_contact::build_variable_g2([&a, &b], law, &options, 1e-6, 100000, 1000000)
                .unwrap();
        assert_eq!(
            report.status,
            Status::VerifiedContacts,
            "{:?}",
            report.fillet.failure
        );
        assert!(report.skin_edit_error_upper.unwrap() <= 0.25e-6);
        assert!((report.fillet.spine.first().unwrap().radius - start).abs() < 1e-12);
        assert!((report.fillet.spine.last().unwrap().radius - end).abs() < 1e-12);
        let skin = report.fillet.surface.as_ref().unwrap();
        let mut cells = report.cells;
        let mut exact = report.exact_work;
        for (side, contact) in report.contacts.as_ref().unwrap().iter().enumerate() {
            let proof = nurbs_core::surface_g2_pcurves::inspect(
                &contact.curve,
                [&contact.support_pcurve, &contact.fillet_pcurve],
                [if side == 0 { &a } else { &b }, skin],
                [false, false],
                1e-6,
                1e-5,
                1e-4,
                (100000 - cells) / (2 - side),
                1000000 - exact,
            )
            .unwrap();
            cells += proof.geometric.cells;
            exact += proof.positions.exact_work;
            assert!(proof.positions.proven);
            assert_eq!(
                proof.geometric.decision,
                nurbs_core::surface_join::Decision::WithinTolerance,
                "{:?}",
                proof.geometric
            );
        }
        let contacts = report.contacts.as_ref().unwrap();
        let certified = brep_core::fillet_radius::inspect(
            [&a, &b],
            skin,
            [&contacts[0], &contacts[1]],
            &RadiusLaw::Linear { start, end },
            [options.side_a, options.side_b],
            1e-6,
            100000 - cells,
            1000000 - exact,
        )
        .unwrap();
        cells += certified.cells;
        exact += certified.exact_work;
        assert_eq!(
            certified.decision,
            brep_core::fillet_radius::Decision::WithinTolerance,
            "bounds={:?} cells={} positions={} exact={} bases={:?}",
            certified.maximum_center_gap_mm,
            certified.cells,
            certified.positions.len(),
            certified.exact_work,
            contacts
                .iter()
                .map(|c| (&c.curve.knots, &c.curve.weights))
                .collect::<Vec<_>>()
        );
        assert!(certified.maximum_center_gap_mm.unwrap()[1] <= 1e-6);
        let wrong = brep_core::fillet_radius::inspect(
            [&a, &b],
            skin,
            [&contacts[0], &contacts[1]],
            &RadiusLaw::Constant(2.),
            [options.side_a, options.side_b],
            1e-6,
            100000,
            1000000,
        )
        .unwrap();
        assert_eq!(
            wrong.decision,
            brep_core::fillet_radius::Decision::ExceedsTolerance
        );
        assert!(wrong.maximum_center_gap_mm.unwrap()[0] > 1e-6);
        let unknown = brep_core::fillet_radius::inspect(
            [&a, &b],
            skin,
            [&contacts[0], &contacts[1]],
            &RadiusLaw::Linear { start, end },
            [options.side_a, options.side_b],
            1e-6,
            0,
            0,
        )
        .unwrap();
        assert_eq!(
            unknown.decision,
            brep_core::fillet_radius::Decision::Unresolved
        );
        assert!(unknown.maximum_center_gap_mm.is_none());
        let spline = brep_core::fillet_radius::inspect(
            [&a, &b],
            skin,
            [&contacts[0], &contacts[1]],
            &RadiusLaw::Spline(vec![(0., start), (1., end)]),
            [options.side_a, options.side_b],
            1e-6,
            100000,
            1000000,
        )
        .unwrap();
        assert_eq!(
            spline.decision,
            brep_core::fillet_radius::Decision::Unresolved
        );
        assert!(spline.maximum_center_gap_mm.is_none());
        // Stored positive contact reports must not authorize a modified skin.
        assert!(contacts.iter().all(|c| c.positions.proven));
        let mut changed = skin.clone();
        changed.control_points[0][0][2] += 0.1;
        let stale = brep_core::fillet_radius::inspect(
            [&a, &b],
            &changed,
            [&contacts[0], &contacts[1]],
            &RadiusLaw::Linear { start, end },
            [options.side_a, options.side_b],
            1e-6,
            100000,
            1000000,
        )
        .unwrap();
        assert_eq!(
            stale.decision,
            brep_core::fillet_radius::Decision::Unresolved
        );
        assert!(stale.maximum_center_gap_mm.is_none());
        assert!(cells <= 100000 && exact <= 1000000);
    }
    let zero = fillet_contact::build_variable_g2(
        [&a, &b],
        RadiusLaw::Linear {
            start: 0.5,
            end: 1.5,
        },
        &options,
        1e-6,
        0,
        0,
    )
    .unwrap();
    assert!(zero.contacts.is_none());
    assert!(
        fillet_contact::build_variable_g2(
            [&a, &b],
            RadiusLaw::Linear {
                start: -0.5,
                end: 1.5
            },
            &options,
            1e-6,
            100000,
            1000000
        )
        .is_err()
    );
    assert_eq!([a, b], before);
}
#[test]
fn generated_contacts_have_one_curve_and_two_whole_interval_lifts() {
    let a = plane([1., 0., 0.], [0., 1., 0.]);
    let b = plane([0., 1., 0.], [0., 0., 1.]);
    let before = [a.clone(), b.clone()];
    for law in [
        RadiusLaw::Constant(1.5),
        RadiusLaw::Linear {
            start: 0.5,
            end: 1.5,
        },
    ] {
        let r = fillet_contact::build(
            [&a, &b],
            law,
            &FilletOptions::default(),
            1e-6,
            100000,
            1000000,
        )
        .unwrap();
        assert_eq!(
            r.status,
            Status::VerifiedContacts,
            "warnings={:?} failure={:?}",
            r.fillet.warnings,
            r.fillet.failure
        );
        assert!(r.cells <= 100000 && r.exact_work <= 1000000);
        let skin = r.fillet.surface.as_ref().unwrap();
        for (i, c) in r.contacts.as_ref().unwrap().iter().enumerate() {
            assert!(c.positions.proven && c.positions.upper.unwrap() <= 1e-6);
            let proof = nurbs_core::surface_g2_pcurves::verify_positions(
                &c.curve,
                [&c.support_pcurve, &c.fillet_pcurve],
                [if i == 0 { &a } else { &b }, skin],
                [false, false],
                1e-6,
                100000,
                1000000,
            )
            .unwrap();
            assert!(proof.proven);
        }
    }
    assert_eq!([a, b], before);
}
#[test]
fn zero_work_and_failed_construction_never_return_admitted_contacts() {
    let a = plane([1., 0., 0.], [0., 1., 0.]);
    let b = plane([0., 1., 0.], [0., 0., 1.]);
    let r = fillet_contact::build(
        [&a, &b],
        RadiusLaw::Constant(1.5),
        &FilletOptions::default(),
        1e-6,
        0,
        0,
    )
    .unwrap();
    assert_eq!(r.status, Status::UnresolvedContacts);
    assert!(r.contacts.is_none());
    assert!(r.contact_attempts.iter().all(Option::is_none));
    let r = fillet_contact::build(
        [&a, &a],
        RadiusLaw::Constant(1.5),
        &FilletOptions::default(),
        1e-6,
        100000,
        1000000,
    )
    .unwrap();
    assert_eq!(r.status, Status::BuildFailed);
    assert!(r.contacts.is_none());
    assert!(
        fillet_contact::build(
            [&a, &b],
            RadiusLaw::Constant(1.5),
            &FilletOptions::default(),
            f64::NAN,
            100000,
            1000000
        )
        .is_err()
    );
}

fn cylinder() -> Surface {
    let c =
        nurbs_core::curve::Curve::from_polyline(vec![vec![3., 0., 4.], vec![3., 8., 4.]]).unwrap();
    nurbs_core::surface::revolve(&c, [0., 0., 4.], [0., 1., 0.], 360.).unwrap()
}
#[test]
fn refused_variable_cylinder_contact_keeps_original_support_mismatch_witness() {
    let a = plane([1., 0., 0.], [0., 1., 0.]);
    let b = cylinder();
    let e = b.evaluate(0.37, 0.41).unwrap();
    let n = e.unit_normal().unwrap();
    let options = FilletOptions {
        side_b: (n[0] * e.point[0] + n[2] * (e.point[2] - 4.)).signum(),
        max_sections: 5,
        ..Default::default()
    };
    let r = fillet_contact::build(
        [&a, &b], RadiusLaw::Linear { start: 1., end: 1.5 },
        &options, 1e-6, 100000, 1000000,
    ).unwrap();
    // Regression of refusal evidence, not a qualified variable-radius fixture.
    assert_eq!(r.status, Status::UnresolvedContacts);
    assert!(r.contacts.is_none());
    assert!(r.fillet.surface.is_some());
    let first = &r.contact_attempts[0].as_ref().unwrap().positions;
    let second = &r.contact_attempts[1].as_ref().unwrap().positions;
    assert!(first.proven);
    assert!(!second.proven);
    let failed = second.sides[0].interval.as_ref().unwrap();
    assert_eq!(failed.status, nurbs_core::curve_surface_agreement::Status::Mismatch);
    assert!(failed.witness.is_some_and(|q| (0. ..=1.).contains(&q)));
    assert!(failed.witness_distance.unwrap()[0] > 1e-6);
    let attempt = r.contact_attempts[1].as_ref().unwrap();
    let q = failed.witness.unwrap();
    let map = |c: &nurbs_core::curve::Curve| {
        let lo = c.knots[c.degree];
        lo + q * (c.knots[c.control_points.len()] - lo)
    };
    let point = attempt.curve.evaluate(map(&attempt.curve)).unwrap().point;
    let uv = attempt.support_pcurve.evaluate(map(&attempt.support_pcurve)).unwrap().point;
    let support = b.evaluate(uv[0], uv[1]).unwrap().point;
    let distance = (0..3).map(|k| (point[k] - support[k]).powi(2)).sum::<f64>().sqrt();
    let enclosure = failed.witness_distance.unwrap();
    assert!(enclosure[0] - 1e-12 <= distance && distance <= enclosure[1] + 1e-12);
    assert_eq!(r.cells, first.cells + second.cells);
    assert_eq!(r.exact_work, first.exact_work + second.exact_work);
    assert!(r.cells <= 100000 && r.exact_work <= 1000000);
}
#[test]
fn variable_g2_cylinder_proposal_qualifies_original_contacts_and_prescribed_ball_centers() {
    let a = plane([1., 0., 0.], [0., 1., 0.]);
    let b = cylinder();
    let e = b.evaluate(0.37, 0.41).unwrap();
    let n = e.unit_normal().unwrap();
    let sign = (n[0] * e.point[0] + n[2] * (e.point[2] - 4.)).signum();
    let options = FilletOptions { side_b: sign, max_sections: 5, ..Default::default() };
    let law = RadiusLaw::Linear { start: 1., end: 1.5 };
    let r = fillet_contact::build_variable_g2([&a, &b], law.clone(), &options, 1e-6, 100000, 1000000).unwrap();
    assert_eq!(r.status, Status::VerifiedContacts);
    let contacts = r.contacts.as_ref().unwrap();
    assert!(r.fillet.contact_pcurves.is_some());
    assert_eq!(r.fillet.spine.len(), 5);
    assert!(r.skin_edit_error_upper.unwrap() <= 0.25e-6);
    assert!(r.fillet.skin_deviation_estimate.is_finite());
    let mut cells = r.cells;
    let mut exact = r.exact_work;
    for (side, contact) in contacts.iter().enumerate() {
        let proof = nurbs_core::surface_g2_pcurves::inspect(
            &contact.curve, [&contact.support_pcurve, &contact.fillet_pcurve],
            [if side == 0 { &a } else { &b }, r.fillet.surface.as_ref().unwrap()],
            [false, false], 1e-6, 1e-5, 1e-4, 100000-cells, 1000000-exact,
        ).unwrap();
        cells += proof.geometric.cells;
        exact += proof.positions.exact_work;
        assert!(proof.positions.proven);
        assert_eq!(proof.geometric.decision, nurbs_core::surface_join::Decision::WithinTolerance, "{:?}", proof.geometric);
    }
    let radius = brep_core::fillet_radius::inspect(
        [&a, &b], r.fillet.surface.as_ref().unwrap(), [&contacts[0], &contacts[1]],
        &law, [1., sign], 1e-6, 100000-cells, 1000000-exact,
    ).unwrap();
    assert_eq!(radius.decision, brep_core::fillet_radius::Decision::WithinTolerance, "{}", radius.reason);
    assert!(radius.maximum_center_gap_mm.unwrap()[1] <= 1e-6);
    assert!(cells + radius.cells <= 100000);
    assert!(exact + radius.exact_work <= 1000000);
}
#[test]
fn curved_rational_support_contacts_are_continuously_qualified_for_both_builders() {
    let a = plane([1., 0., 0.], [0., 1., 0.]);
    let c = cylinder();
    let e = c.evaluate(0.37, 0.41).unwrap();
    let n = e.unit_normal().unwrap();
    let dot = n[0] * e.point[0] + n[2] * (e.point[2] - 4.);
    let options = FilletOptions {
        side_b: dot.signum(),
        max_sections: 5,
        ..Default::default()
    };
    let round = fillet_contact::build(
        [&a, &c],
        RadiusLaw::Constant(1.),
        &options,
        1e-6,
        100000,
        1000000,
    )
    .unwrap();
    let smooth = fillet_contact::build_g2([&a, &c], 1., &options, 1e-6, 100000, 1000000).unwrap();
    for (is_smooth, r) in [(false, round), (true, smooth)] {
        assert_eq!(
            r.status,
            Status::VerifiedContacts,
            "failure={:?} warnings={:?} work={}",
            r.fillet.failure,
            r.fillet.warnings,
            r.cells
        );
        let contacts = r.contacts.as_ref().unwrap();
        assert!(contacts.iter().all(|c| c.positions.proven));
        if is_smooth {
            assert!(r.skin_edit_error_upper.unwrap() <= 0.25e-6);
            assert!(r.skin_edit_work > 0 && r.skin_edit_work <= 2000000);
            let skin = r.fillet.surface.as_ref().unwrap();
            for (side, contact) in contacts.iter().enumerate() {
                let proof = nurbs_core::surface_g2_pcurves::inspect(
                    &contact.curve,
                    [&contact.support_pcurve, &contact.fillet_pcurve],
                    [if side == 0 { &a } else { &c }, skin],
                    [false, false],
                    1e-6,
                    1e-5,
                    1e-4,
                    100000,
                    1000000,
                )
                .unwrap();
                assert_eq!(
                    proof.geometric.decision,
                    nurbs_core::surface_join::Decision::WithinTolerance,
                    "side={side} proof={:?}",
                    proof.geometric
                );
            }
            let radius = brep_core::fillet_radius::inspect(
                [&a, &c],
                skin,
                [&contacts[0], &contacts[1]],
                &RadiusLaw::Constant(1.),
                [options.side_a, options.side_b],
                1e-6,
                100000,
                1000000,
            )
            .unwrap();
            assert_eq!(
                radius.decision,
                brep_core::fillet_radius::Decision::WithinTolerance,
                "reason={} cells={} bound={:?}",
                radius.reason,
                radius.cells,
                radius.maximum_center_gap_mm
            );
            assert!(radius.maximum_center_gap_mm.unwrap()[1] <= 1e-6);
        }
    }
}

#[test]
fn unsuccessful_c2_storage_preparation_retains_raw_geometry_and_reports_work() {
    let a = plane([1., 0., 0.], [0., 1., 0.]);
    let b = plane([0., 1., 0.], [0., 0., 1.]);
    let options = FilletOptions {
        max_sections: 5,
        ..Default::default()
    };
    let original = nurbs_core::offset::fillet_surfaces::g2_fillet(&a, &b, 1.5, &options).unwrap();
    let report = fillet_contact::build_g2([&a, &b], 1.5, &options, 1e-30, 100000, 1000000).unwrap();
    assert_eq!(report.status, Status::SmoothSkinUnresolved);
    assert!(report.contacts.is_none());
    assert!(report.skin_edit_error_upper.is_none());
    assert!(report.skin_edit_work > 0 && report.skin_edit_work <= 2000000);
    assert_eq!(report.fillet.surface, original.surface);
}
