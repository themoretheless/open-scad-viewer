use super::section_queries::{coordinate, point};
use crate::*;
use brep_core::analysis as a;
fn witness(r: a::ExactPolyhedralSweptBallClearance) -> Value {
    json!({"comparison":format!("{:?}",r.clearance),"distanceSquaredMm2":r.distance_squared_mm2.to_string(),"centerMm":r.center_mm.map(|q|q.to_string()),"boundaryMm":r.boundary_mm.map(|q|q.to_string()),"face":r.face})
}
pub(super) fn measure(v: &Value, op: &str) -> Result<Value> {
    let solid = super::polyhedral_mass::qualify(
        field(v, "source")?,
        super::closed_ss::closed_solid_policy(field(v, "tolerance_uv")?),
    )?;
    let model = solid.model();
    let work = field(v, "max_work")?;
    let radius = coordinate(&v["query_radius_mm"])?;
    let raw = v["query_path_mm"]
        .as_array()
        .filter(|p| (2..=4096).contains(&p.len()))
        .ok_or_else(|| {
            nurbs_core::Error::new(
                "BREP_INVALID_INPUT",
                "Expected a bounded path of at least two centers",
            )
        })?;
    let path = raw.iter().map(point).collect::<Result<Vec<_>>>()?;
    Ok(match op {
        "measure_swept_ball_gap" => {
            let ends = path.try_into().map_err(|_| {
                nurbs_core::Error::new("BREP_INVALID_INPUT", "Expected two sphere centers")
            })?;
            let r = a::exact_polyhedral_swept_ball_clearance(&solid, &ends, &radius, work)?;
            let used = r.exact_work;
            json!({"model":model,"nearest":witness(r),"workUsed":used})
        }
        "measure_swept_path_gap" => {
            let r = a::exact_polyhedral_swept_path_clearance(&solid, &path, &radius, work)?;
            json!({"model":model,"span":r.span,"nearest":witness(r.nearest),"workUsed":r.exact_work})
        }
        "classify_swept_path_material" => {
            let r = a::exact_polyhedral_swept_path_material(&solid, &path, &radius, work)?;
            json!({"model":model,"relation":format!("{:?}",r.relation),"span":r.clearance.span,"nearest":witness(r.clearance.nearest),"workUsed":r.clearance.exact_work})
        }
        "bound_swept_first_contact" => {
            let r = a::bounded_polyhedral_swept_path_first_contact(
                &solid,
                &path,
                &radius,
                field(v, "max_parameter_width")?,
                work,
            )?;
            let contact=r.contact.map(|c|json!({"span":c.span,"parameterBounds":c.parameter_bounds.map(|q|q.to_string())}));
            json!({"model":model,"contact":contact,"workUsed":r.exact_work})
        }
        _ => {
            let r = a::bounded_polyhedral_swept_face_contact_ranges(
                &solid,
                &path,
                &radius,
                field(v, "max_parameter_width")?,
                work,
            )?;
            let contacts=r.contacts.into_iter().map(|c|json!({"span":c.span,"face":c.face,"firstBounds":c.first_parameter_bounds.map(|q|q.to_string()),"lastBounds":c.last_parameter_bounds.map(|q|q.to_string())})).collect::<Vec<_>>();
            json!({"model":model,"contacts":contacts,"workUsed":r.exact_work})
        }
    })
}
