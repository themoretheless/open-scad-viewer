use nurbs_core::{
    curve::Curve,
    offset::fillet_surfaces::{FilletOptions, RadiusLaw},
    surface::Surface,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 10., 0.]],
            vec![vec![10., 0., 0.], vec![10., 10., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let line = Curve::from_polyline(vec![vec![3., 0., 4.], vec![3., 8., 4.]])?;
    let b = nurbs_core::surface::revolve(&line, [0., 0., 4.], [0., 1., 0.], 360.)?;
    let e = b.evaluate(0.37, 0.41)?;
    let n = e.unit_normal().unwrap();
    let sign = (n[0] * e.point[0] + n[2] * (e.point[2] - 4.)).signum();
    let options = FilletOptions {
        side_b: sign,
        max_sections: 5,
        ..Default::default()
    };
    let report = brep_core::fillet_contact::build_variable_g2(
        [&a, &b],
        RadiusLaw::Linear {
            start: 1.,
            end: 1.5,
        },
        &options,
        1e-6,
        100000,
        1000000,
    )?;
    println!(
        "status={:?} failure={:?} cells={} exact={} skin_error={:?}",
        report.status,
        report.fillet.failure,
        report.cells,
        report.exact_work,
        report.skin_edit_error_upper
    );
    for (side, attempt) in report.contact_attempts.iter().enumerate() {
        if let Some(p) = attempt {
            for (chart, evidence) in p.positions.sides.iter().enumerate() {
                println!(
                    "contact={side} chart={chart} interval={:?}",
                    evidence.interval
                );
            }
        }
    }
    if let Some(contacts) = &report.contacts {
        let skin = report.fillet.surface.as_ref().unwrap();
        for (side, contact) in contacts.iter().enumerate() {
            let proof = nurbs_core::surface_g2_pcurves::inspect(
                &contact.curve,
                [&contact.support_pcurve, &contact.fillet_pcurve],
                [if side == 0 { &a } else { &b }, skin],
                [false, false],
                1e-6,
                1e-5,
                1e-4,
                100000,
                1000000,
            )?;
            println!(
                "G2 side={side} decision={:?} cells={} bounds={:?}",
                proof.geometric.decision, proof.geometric.cells, proof.geometric
            );
        }
        let radius = brep_core::fillet_radius::inspect(
            [&a, &b],
            report.fillet.surface.as_ref().unwrap(),
            [&contacts[0], &contacts[1]],
            &RadiusLaw::Linear {
                start: 1.,
                end: 1.5,
            },
            [1., sign],
            1e-6,
            100000,
            1000000,
        )?;
        println!(
            "radius={:?} reason={} upper={:?}",
            radius.decision, radius.reason, radius.maximum_center_gap_mm
        );
    }
    Ok(())
}
