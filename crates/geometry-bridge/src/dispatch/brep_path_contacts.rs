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
        "recover_material_path" => {
            let result = a::exact_polyhedral_material_path_components(&solid, &path, work)?;
            let components=result.components.into_iter().map(|component|{
                let points=component.points_mm.into_iter().map(|p|p.map(|q|q.to_string())).collect::<Vec<_>>();
                json!({"parameters":component.parameters.map(|q|q.to_string()),"pointsMm":points})
            }).collect::<Vec<_>>();
            json!({"model":model,"components":components,"workUsed":result.exact_work})
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
        "clip_exterior_path" => {
            let result = a::exact_polyhedral_path_exterior_clip(&solid, &path, work)?;
            let spans=result.span_intervals.into_iter().map(|parts|parts.into_iter().map(|p|json!({"parameters":p.parameters.map(|q|q.to_string()),"included":p.endpoints_included})).collect::<Vec<_>>()).collect::<Vec<_>>();
            json!({"model":model,"spans":spans,"workUsed":result.exact_work})
        }
        "clip_interior_path" => {
            let result = a::exact_polyhedral_path_interior_clip(&solid, &path, work)?;
            let spans=result.span_intervals.into_iter().map(|parts|parts.into_iter().map(|p|json!({"parameters":p.parameters.map(|q|q.to_string()),"included":p.endpoints_included})).collect::<Vec<_>>()).collect::<Vec<_>>();
            json!({"model":model,"spans":spans,"workUsed":result.exact_work})
        }
        "partition_solid_path" => {
            let result = a::exact_polyhedral_path_partition(&solid, &path, work)?;
            let spans=result.spans.into_iter().map(|parts|parts.into_iter().map(|p|json!({"parameters":p.parameters.map(|q|q.to_string()),"included":p.endpoints_included,"location":format!("{:?}",p.location)})).collect::<Vec<_>>()).collect::<Vec<_>>();
            json!({"model":model,"spans":spans,"workUsed":result.exact_work})
        }
        "bound_path_material_length" => {
            let result = a::bounded_polyhedral_path_material_length(
                &solid,
                &path,
                field(v, "tolerance_mm")?,
                work,
            )?;
            json!({"model":model,"lengthBoundsMm":result.length_bounds_mm.map(|q|q.to_string()),"workUsed":result.exact_work})
        }
        "bound_path_length_breakdown" => {
            let result = a::bounded_polyhedral_path_length_breakdown(
                &solid,
                &path,
                field(v, "tolerance_mm")?,
                work,
            )?;
            json!({"model":model,"interiorMm":result.interior_bounds_mm.map(|q|q.to_string()),"boundaryMm":result.boundary_bounds_mm.map(|q|q.to_string()),"exteriorMm":result.exterior_bounds_mm.map(|q|q.to_string()),"workUsed":result.exact_work})
        }
        _ => {
            let result = a::exact_polyhedral_path_boundary_events(&solid, &path, work)?;
            let events=result.events.into_iter().map(|e|json!({"parameters":e.parameters.map(|q|q.to_string()),"before":e.before.map(|l|format!("{l:?}")),"after":e.after.map(|l|format!("{l:?}"))})).collect::<Vec<_>>();
            json!({"model":model,"events":events,"workUsed":result.exact_work})
        }
    })
}
