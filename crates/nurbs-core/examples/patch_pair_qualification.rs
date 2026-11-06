//! Original rational circle cylinders with independent homogeneous gauges.
//! This qualifies the local inspector fixture, not a CAD body or arbitrary NURBS.
use nurbs_core::{
    Result, surface::Surface, surface_g2_pcurves::inspect_original_patch_pair,
    surface_join::Decision,
};
fn main() -> Result<()> {
    let arc = |right: bool, gauge: f64| {
        let start = if right { 1. } else { 0. };
        let poles = if right {
            [[0., 1.], [-1., 1.], [-1., 0.]]
        } else {
            [[1., 0.], [1., 1.], [0., 1.]]
        };
        Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![start, start, start, start + 1., start + 1., start + 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: poles
                .iter()
                .map(|p| [0., 0.5, 1.].iter().map(|z| vec![p[0], p[1], *z]).collect())
                .collect(),
            weights: [1., std::f64::consts::FRAC_1_SQRT_2, 1.]
                .iter()
                .map(|w| vec![w * gauge; 3])
                .collect(),
            periodic_u: false,
            periodic_v: false,
        }
    };
    for (left_gauge, right_gauge) in [(1., 1.), (0.25, 8.), (2f64.powi(-20), 2f64.powi(20))] {
        let a = arc(false, left_gauge);
        let mut b = arc(true, right_gauge);
        let proof = inspect_original_patch_pair(&a, &b, 1e-5, 1e-4, 6000)?;
        println!(
            "circle_pair gauges={left_gauge}/{right_gauge} decision={:?} normal={:?} tensor={:?} regular={} cells={} pending={}",
            proof.decision,
            proof.normal,
            proof.tensor,
            proof.regularity_proven,
            proof.cells,
            proof.pending
        );
        assert_eq!(proof.decision, Decision::WithinTolerance);
        assert!(proof.regularity_proven && proof.pending == 0);
        // Same seam and tangent, altered second derivative on the true right cell.
        for point in &mut b.control_points[2] {
            point[1] -= 0.1;
        }
        let bad = inspect_original_patch_pair(&a, &b, 1e-5, 1e-4, 6000)?;
        println!(
            "circle_pair defective decision={:?} tensor={:?}",
            bad.decision, bad.tensor
        );
        assert_eq!(bad.decision, Decision::CorrespondenceExceedsTolerance);
        assert!(bad.tensor[0] > 1e-4);
    }
    Ok(())
}
