//! Exact source profile fallback for compact higher-degree cap boundaries.
use super::*;
pub(super) fn build(
    loops: &[Vec<Curve>],
    projected: &[Vec<Curve>],
    z: f64,
    tolerance: f64,
    max_cells: usize,
) -> Result<Vec<Cap>> {
    let fail = |message| Error::new("BREP_CAP_UNRESOLVED", message);
    if max_cells == 0 {
        return Err(fail("No cap boundary agreement budget"));
    }
    let witness = crate::profile_certificate::certify(projected, tolerance)?;
    let verified = crate::profile_certificate::verify(projected, tolerance, &witness)?;
    if verified.nesting_depth().first() != Some(&0)
        || verified.nesting_depth()[1..].iter().any(|&d| d != 1)
    {
        return Err(fail(
            "Exact curved cap requires one outer contour followed by holes",
        ));
    }
    let extent = projected
        .iter()
        .flatten()
        .flat_map(|c| c.control_points.iter().flatten())
        .map(|x| x.abs())
        .fold(1_f64, f64::max)
        * 2.;
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-extent, -extent, extent, extent],
        knots_v: vec![-extent, -extent, extent, extent],
        control_points: vec![
            vec![vec![-extent, -extent, z], vec![-extent, extent, z]],
            vec![vec![extent, -extent, z], vec![extent, extent, z]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate()?;
    let mut used = 0;
    let mut agreements = Vec::new();
    for (wire, uv) in loops.iter().zip(projected) {
        let mut reports = Vec::new();
        for (curve, pcurve) in wire.iter().zip(uv) {
            if used >= max_cells {
                return Err(fail("Cap agreement budget exhausted"));
            }
            let proof = curve_surface_agreement::verify(
                curve,
                pcurve,
                &surface,
                false,
                tolerance,
                max_cells - used,
            )?;
            used += proof.cells;
            if proof.status != Status::WithinTolerance {
                return Err(fail("Exact curved cap world agreement remains unproved"));
            }
            reports.push(proof);
        }
        agreements.push(reports);
    }
    Ok(vec![Cap {
        surface,
        boundaries: loops.to_vec(),
        pcurves: projected.to_vec(),
        agreements,
    }])
}
