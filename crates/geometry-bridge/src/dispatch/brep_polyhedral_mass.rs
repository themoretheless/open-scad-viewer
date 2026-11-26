use crate::*;
use brep_core::{analysis, closed_solid};

pub(super) fn measure(v: &Value, op: &str) -> Result<Value> {
    let source: brep_core::Model = field(v, "source")?;
    let policy = super::closed_ss::closed_solid_policy(field(v, "tolerance_uv")?);
    let solid = closed_solid::qualify(source, policy)?
        .into_solid()
        .ok_or_else(|| {
            nurbs_core::Error::new(
                "BREP_CLOSED_SOLID_UNPROVED",
                "Polyhedral measurement requires a qualified material boundary",
            )
        })?;
    let work = field(v, "max_work")?;
    let model = solid.model();
    Ok(match op {
        "measure_polyhedral_volume" => {
            let r = analysis::exact_polyhedral_volume(&solid, work)?;
            json!({"model":model,"volumeMm3":r.volume_mm3.to_string(),"workUsed":r.exact_work})
        }
        "measure_polyhedral_centroid" => {
            let r = analysis::exact_polyhedral_centroid(&solid, work)?;
            json!({"model":model,"volumeMm3":r.volume_mm3.to_string(),"centroidMm":r.centroid_mm.map(|q|q.to_string()),"workUsed":r.exact_work})
        }
        "measure_polyhedral_inertia" => {
            let r = analysis::exact_polyhedral_inertia(&solid, work)?;
            json!({"model":model,"volumeMm3":r.volume_mm3.to_string(),"centroidMm":r.centroid_mm.map(|q|q.to_string()),"inertiaMm5":r.inertia_mm5.map(|row|row.map(|q|q.to_string())),"workUsed":r.exact_work})
        }
        "measure_polyhedral_face_areas" => {
            let r = analysis::exact_polyhedral_area(&solid, work)?;
            json!({"model":model,"faceAreaSquaredMm4":r.face_area_squared_mm4.into_iter().map(|q|q.to_string()).collect::<Vec<_>>(),"workUsed":r.exact_work})
        }
        "bound_polyhedral_area" => {
            let r = analysis::bounded_polyhedral_area(&solid, field(v, "max_width_mm2")?, work)?;
            json!({"model":model,"areaBoundsMm2":r.bounds_mm2.map(|q|q.to_string()),"workUsed":r.exact_work})
        }
        "bound_polyhedral_edge_length" => {
            let r =
                analysis::bounded_polyhedral_edge_length(&solid, field(v, "max_width_mm")?, work)?;
            json!({"model":model,"edgeLengthBoundsMm":r.bounds_mm.map(|q|q.to_string()),"edgeCount":r.edge_count,"workUsed":r.exact_work})
        }
        _ => {
            return Err(nurbs_core::Error::new(
                "BREP_INVALID_INPUT",
                "Unknown polyhedral measurement",
            ));
        }
    })
}
