//! Surface-intersection proposals admitted by fresh whole saved-curve proofs.
use super::{invalid, unproven};
use nurbs_core::{
    Result, curve::Curve, curve_surface_agreement as agreement, ss_intersection, surface::Surface,
};

pub struct IntersectionCurve {
    pub curve: Curve,
    pub pcurves: [Curve; 2],
    pub agreements: [agreement::Report; 2],
    pub cells: usize,
}
/// Generate a spatial cut from literal rectangular original support charts.
/// Rounded inverse parameters grant no authority: the saved curve must agree
/// continuously with BOTH original surfaces. Supplied owned endpoints are
/// applied before verification. This is a local candidate, not SS coverage;
/// branch ownership, trimmed-face membership and topology remain separate.
pub fn generate_intersection_curve(
    surfaces: [&Surface; 2],
    endpoints: Option<[[f64; 3]; 2]>,
    tolerance_mm: f64,
    max_cells: usize,
) -> Result<IntersectionCurve> {
    if !(4..=100000).contains(&max_cells) || !tolerance_mm.is_finite() || tolerance_mm <= 0. {
        return Err(invalid(
            "Need finite cut tolerance and bounded shared intersection/lift work",
        ));
    }
    surfaces[0].validate()?;
    surfaces[1].validate()?;
    let traces = ss_intersection::rectangular_trace_proposal(surfaces[0], surfaces[1])
        .ok_or_else(|| unproven("General intersection trace construction remains unsupported"))?;
    let mut cells = 1;
    let pcurves = traces
        .map(|ends| Curve::from_polyline(ends.iter().map(|p| p.to_vec()).collect()))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let pcurves: [Curve; 2] = pcurves.try_into().unwrap();
    let lifted = agreement::lift(
        &pcurves[0],
        surfaces[0],
        endpoints,
        tolerance_mm * 0.5,
        (max_cells - cells) / 2,
    )?;
    cells += lifted.cells;
    let curve = lifted
        .curve
        .ok_or_else(|| unproven("Intersection trace lift remains unproven"))?;
    let first = lifted
        .evidence
        .ok_or_else(|| unproven("Missing saved intersection lift evidence"))?;
    let second = agreement::verify(
        &curve,
        &pcurves[1],
        surfaces[1],
        false,
        tolerance_mm * 0.5,
        max_cells - cells,
    )?;
    cells += second.cells;
    if first.status != agreement::Status::WithinTolerance
        || second.status != agreement::Status::WithinTolerance
    {
        return Err(unproven(
            "Saved intersection curve does not qualify on both original supports",
        ));
    }
    Ok(IntersectionCurve {
        curve,
        pcurves,
        agreements: [first, second],
        cells,
    })
}
