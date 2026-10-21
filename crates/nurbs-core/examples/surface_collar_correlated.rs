//! Fresh original-coefficient proof on the unchanged rational collar control.
use nurbs_core::{curve::Curve, surface::Surface, surface_collar};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("Need output directory")?;
    let source = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                [0., 1.]
                    .iter()
                    .map(|&v| vec![i as f64 / 2., v, if i == 2 { 0.25 + v } else { v }])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2], vec![0.7; 2], vec![1.; 2]],
        periodic_u: false,
        periodic_v: false,
    };
    let pc = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0.2, 0.4], vec![0.4, 0.5], vec![0.7, 0.3]],
        weights: vec![1., 0.7, 1.2],
        periodic: false,
    };
    let direction = [0.05, 0.1];
    let collar = surface_collar::construct(&source, &pc, direction, 1e-7, 2000000)?;
    let proof = surface_collar::inspect_g2(
        &source,
        &pc,
        direction,
        &collar.surface,
        [1e-7, 1e-5, 1e-4],
        100000,
        2000000,
    )?;
    let report = value_codec::json!({"source":source,"pcurve":pc,"direction":direction,"collar":collar.surface,
        "constructionWork":collar.coefficient_work,"positionErrorUpperMm":collar.position_error_upper_mm,
        "decision":format!("{:?}",proof.decision),"orientation":format!("{:?}",proof.orientation),
        "upper":proof.upper,"cells":proof.cells,"coefficientWork":proof.coefficient_work,
        "witnessParameter":proof.witness_parameter,"radiusClaim":false,"solidClaim":false});
    std::fs::create_dir_all(&path)?;
    std::fs::write(
        std::path::Path::new(&path).join("rational.json"),
        value_codec::to_string_pretty(&report)?,
    )?;
    println!(
        "decision={:?} cells={} coefficient_work={} upper={:?}",
        proof.decision, proof.cells, proof.coefficient_work, proof.upper
    );
    Ok(())
}
