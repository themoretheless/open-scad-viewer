use super::*;
use crate::{curve::Curve, editing::exact_derivative as moment};
pub(super) fn apply(v: &Value, op: &str) -> Result<Value> {
    let outer: Vec<Curve> = field(v, "outer")?;

    if op == "exact_wire_area" {
        let r = moment::exact_planar_boundary_area(&outer, field(v, "max_work")?)?;
        let moment::ExactPlanarAreaOutcome::Area(area) = r.outcome else {
            return Err(input(format!("Exact wire area refused: {:?}", r.outcome)));
        };
        return Ok(json!({"signedArea":area.to_string(),"workUsed":r.work_used}));
    }
    let mut rings = vec![outer];
    rings.extend(field::<Vec<Vec<Curve>>>(v, "holes")?);
    if op == "bound_wire_moments" {
        let report = crate::trim::planar_mass_bounds::bound_planar_boundary_moments(
            &rings,
            field(v, "chart_origin")?,
            field(v, "moment_tolerances")?,
            field(v, "max_cells")?,
        )?;
        return Ok(
            json!({"moments":report.moments,"cells":report.cells,"converged":report.converged}),
        );
    }
    let work = field(v, "max_work")?;
    let (mass, second, used) = if op == "exact_wire_inertia" {
        let r = moment::exact_planar_boundary_inertia(&rings, work)?;
        (r.mass, r.second_moments, r.work_used)
    } else {
        let r = moment::exact_planar_boundary_mass(&rings, work)?;
        (r.outcome, None, r.work_used)
    };
    let moment::ExactPlanarMassOutcome::Mass(m) = mass else {
        return Err(input(format!("Exact wire moments refused: {mass:?}")));
    };
    let mut report = json!({"signedArea":m.signed_area.to_string(),"firstMoments":m.first_moments.map(|x|x.to_string()),
        "centroid":m.centroid.map(|p|p.map(|x|x.to_string())),"workUsed":used});
    if let Some(s) = second {
        report["aboutOrigin"] = json!(s.about_origin.map(|x| x.to_string()));
        report["aboutCentroid"] = json!(s.about_centroid.map(|p| p.map(|x| x.to_string())));
    }
    Ok(report)
}
