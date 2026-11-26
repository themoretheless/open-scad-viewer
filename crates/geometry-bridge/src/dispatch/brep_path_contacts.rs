use super::section_queries::point;
use crate::*;
use brep_core::analysis as a;
pub(super) fn measure(v: &Value, op: &str) -> Result<Value> {
    let solid = super::polyhedral_mass::qualify(
        field(v, "source")?,
        super::closed_ss::closed_solid_policy(field(v, "tolerance_uv")?),
    )?;
    let model = solid.model();
    let work = field(v, "max_work")?;
    let raw = v["query_path_mm"]
        .as_array()
        .filter(|p| (2..=4096).contains(&p.len()))
        .ok_or_else(|| {
            nurbs_core::Error::new(
                "BREP_INVALID_INPUT",
                "Expected a bounded path of at least two points",
            )
        })?;
    let path = raw.iter().map(point).collect::<Result<Vec<_>>>()?;
    Ok(match op {
        "find_path_face_contacts" => {
            let result = a::exact_polyhedral_path_boundary_contacts(&solid, &path, work)?;
            let contacts = result
                .contacts
                .into_iter()
                .map(|c| {
                    let intervals = c
                        .intervals
                        .into_iter()
                        .map(|p| p.map(|q| q.to_string()))
                        .collect::<Vec<_>>();
                    json!({"span":c.span,"face":c.face,"intervals":intervals})
                })
                .collect::<Vec<_>>();
            json!({"model":model,"contacts":contacts,"workUsed":result.exact_work})
        }
        "clip_path_to_material" => {
            let result = a::exact_polyhedral_path_material_clip(&solid, &path, work)?;
            let spans = result
                .span_intervals
                .into_iter()
                .map(|intervals| {
                    intervals
                        .into_iter()
                        .map(|p| p.map(|q| q.to_string()))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            json!({"model":model,"spans":spans,"workUsed":result.exact_work})
        }
        _ => {
            let result = a::exact_polyhedral_path_boundary_events(&solid, &path, work)?;
            let events=result.events.into_iter().map(|e|json!({"parameters":e.parameters.map(|q|q.to_string()),"before":e.before.map(|l|format!("{l:?}")),"after":e.after.map(|l|format!("{l:?}"))})).collect::<Vec<_>>();
            json!({"model":model,"events":events,"workUsed":result.exact_work})
        }
    })
}
