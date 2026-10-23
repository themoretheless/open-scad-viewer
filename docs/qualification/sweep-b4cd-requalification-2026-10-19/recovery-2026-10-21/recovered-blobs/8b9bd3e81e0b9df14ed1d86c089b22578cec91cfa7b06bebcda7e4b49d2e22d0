//! Whole-position certificates and independent original-source seam decisions.
use nurbs_core::{
    curve::Curve,
    surface::{Axis, Surface},
    surface_collar, surface_g2_pcurves,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("Need output directory")?;
    std::fs::create_dir_all(&path)?;
    for rational in [false, true] {
        let mut source = Surface {
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
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let mut pc = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.5], vec![0.5, 0.5], vec![1., 0.25]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let direction = if rational {
            source.weights[1] = vec![0.7; 2];
            pc.control_points = vec![vec![0.2, 0.4], vec![0.4, 0.5], vec![0.7, 0.3]];
            pc.weights = vec![1., 0.7, 1.2];
            [0.05, 0.1]
        } else {
            [0., 0.1]
        };
        let collar = surface_collar::construct(&source, &pc, direction, 1e-7, 2000000)?;
        let edge = collar.surface.iso(Axis::V, 0.)?;
        let own = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]])?;
        let proof = surface_g2_pcurves::inspect(
            &edge,
            [&pc, &own],
            [&source, &collar.surface],
            [false; 2],
            1e-7,
            1e-5,
            1e-4,
            100000,
            1000000,
        )?;
        let report = value_codec::json!({"rational":rational,"source":source,"pcurve":pc,"direction":direction,"collar":collar.surface,
            "wholePositionErrorUpperMm":collar.position_error_upper_mm,"coefficientWork":collar.coefficient_work,
            "positionsProven":proof.positions.proven,"positionCells":proof.positions.cells,"exactWork":proof.positions.exact_work,
            "g2Decision":format!("{:?}",proof.geometric.decision),"g2Cells":proof.geometric.cells,
            "normalUpper":proof.geometric.branches[0].normal_upper,"tensorUpper":proof.geometric.branches[0].tensor_upper,
            "radiusClaim":false,"solidClaim":false});
        let name = if rational {
            "rational.json"
        } else {
            "polynomial.json"
        };
        std::fs::write(
            std::path::Path::new(&path).join(name),
            value_codec::to_string_pretty(&report)?,
        )?;
        println!(
            "{}: position_upper={} coefficient_work={} G2={:?} cells={}",
            name,
            collar.position_error_upper_mm,
            collar.coefficient_work,
            proof.geometric.decision,
            proof.geometric.cells
        );
    }
    Ok(())
}
