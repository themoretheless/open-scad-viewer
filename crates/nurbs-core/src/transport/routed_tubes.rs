use super::*;
use crate::analysis::curve_measure::{tube_bundle_clearance, tube_pair_clearance};
use crate::curve::Curve;

pub(super) fn apply(v:&Value, op:&str) -> Result<Value> {
    let curves: Vec<Curve> = field(v,"curves")?;
    let radii: Vec<f64> = field(v,"radii_mm")?;
    check((2..=256).contains(&curves.len()) && curves.len()==radii.len(),
        "Expected one radius per tube and two to 256 centerlines")?;
    let cells = field(v,"max_cells")?;
    check(cells <= 1_000_000,"Tube clearance cell budget exceeded")?;
    let tolerance = field(v,"tolerance_mm")?;
    let tubes: Vec<_> = curves.iter().zip(radii).collect();
    if op=="routed_tube_pair" {
        check(tubes.len()==2,"Expected exactly two tube centerlines")?;
        let pair = tube_pair_clearance(tubes[0],tubes[1],tolerance,cells)?;
        let r = pair.centerline_distance;
        return Ok(json!({"signed_margin_mm":pair.signed_margin_mm,"distance_mm":r.distance_interval_mm,
            "parameters":r.parameters,"points":r.points,"cells":r.cells,"converged":r.converged}));
    }
    let r = tube_bundle_clearance(&tubes,tolerance,cells)?;
    Ok(json!({"signed_margin_mm":r.signed_margin_mm,"witness_pair":r.witness_pair,
        "parameters":r.witness_parameters,"cells":r.work_cells}))
}
