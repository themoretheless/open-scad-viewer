//! Whole-interval G2 receipts on the actual plane/cylinder contact definitions.
use brep_core::fillet_contact;
use nurbs_core::{curve::Curve, offset::fillet_surfaces::FilletOptions, surface::Surface};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from(std::env::args().nth(1).expect("Output directory"));
    std::fs::create_dir_all(&path)?;
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
    let profile = Curve::from_polyline(vec![vec![3., 0., 4.], vec![3., 8., 4.]])?;
    let c = nurbs_core::surface::revolve(&profile, [0., 0., 4.], [0., 1., 0.], 360.)?;
    let e = c.evaluate(0.37, 0.41)?;
    let n = e.unit_normal().unwrap();
    let side = (n[0] * e.point[0] + n[2] * (e.point[2] - 4.)).signum();
    let options = FilletOptions {
        side_b: side,
        max_sections: 5,
        ..Default::default()
    };
    let built = fillet_contact::build_g2([&a, &c], 1., &options, 1e-6, 100000, 1000000)?;
    assert_eq!(built.status, fillet_contact::Status::VerifiedContacts);
    let skin = built.fillet.surface.as_ref().unwrap();
    let mut seams = Vec::new();
    let mut cells = built.cells;
    let mut exact_work = built.exact_work;
    for (i, contact) in built.contacts.as_ref().unwrap().iter().enumerate() {
        let proof = nurbs_core::surface_g2_pcurves::inspect(
            &contact.curve,
            [&contact.support_pcurve, &contact.fillet_pcurve],
            [if i == 0 { &a } else { &c }, skin],
            [false, false],
            1e-6,
            1e-5,
            1e-4,
            100000 - cells,
            1000000 - exact_work,
        )?;
        cells += proof.geometric.cells;
        exact_work += proof.positions.exact_work;
        assert_eq!(
            proof.geometric.decision,
            nurbs_core::surface_join::Decision::WithinTolerance
        );
        let branch = match proof.geometric.orientation.unwrap() {
            nurbs_core::surface_g2::Orientation::Same => 0,
            nurbs_core::surface_g2::Orientation::Opposite => 1,
        };
        let bounds = &proof.geometric.branches[branch];
        println!(
            "side={i} whole_g2=true cells={} exact_work={} normal_upper={:?} tensor_upper={:?}",
            proof.geometric.cells,
            proof.positions.exact_work,
            bounds.normal_upper,
            bounds.tensor_upper
        );
        seams.push(value_codec::json!({"side":i,"wholeG2":true,"cells":proof.geometric.cells,
            "exactWork":proof.positions.exact_work,"normalUpper":bounds.normal_upper,"tensorUpper":bounds.tensor_upper,
            "curve":&contact.curve,"supportPcurve":&contact.support_pcurve,"filletPcurve":&contact.fillet_pcurve}));
    }
    let artifact = value_codec::json!({"schema":"fillet-contact-g2/1","positionTolerance":1e-6,"normalTolerance":1e-5,"curvatureTolerance":1e-4,
        "supports":[&a,&c],"skin":skin,"skinEditErrorUpper":built.skin_edit_error_upper,"skinEditWork":built.skin_edit_work,
        "seams":seams,"totalCells":cells,"totalExactWork":exact_work,"globalSolidClaim":false,"wholeRadiusClaim":false,"branchCoverageClaim":false});
    std::fs::write(
        path.join("receipt.json"),
        value_codec::to_string_pretty(&artifact)?,
    )?;
    Ok(())
}
