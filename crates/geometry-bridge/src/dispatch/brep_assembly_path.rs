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
    let others: Vec<brep_core::Model> = field(v, "others")?;
    if others.len() > 31 {
        return Err(nurbs_core::Error::new(
            "BREP_INVALID_INPUT",
            "Expected at most 31 additional solids",
        ));
    }
    let tolerance = field(v, "tolerance_uv")?;
    let additional = others
        .into_iter()
        .map(|m| {
            super::polyhedral_mass::qualify(m, super::closed_ss::closed_solid_policy(tolerance))
        })
        .collect::<Result<Vec<_>>>()?;
    let bodies = std::iter::once(&solid)
        .chain(additional.iter())
        .collect::<Vec<_>>();
    Ok(match op {
        "clip_common_path" | "clip_union_path" => {
            let result = if op == "clip_common_path" {
                a::exact_polyhedral_path_common_material_clip(&bodies, &path, work)?
            } else {
                a::exact_polyhedral_path_union_material_clip(&bodies, &path, work)?
            };
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
        "subtract_path_cutters" => {
            let result =
                a::exact_polyhedral_path_material_difference(&solid, &bodies[1..], &path, work)?;
            let spans=result.span_intervals.into_iter().map(|intervals|intervals.into_iter().map(|p|json!({"parameters":p.parameters.map(|q|q.to_string()),"included":p.endpoints_included})).collect::<Vec<_>>()).collect::<Vec<_>>();
            json!({"model":model,"spans":spans,"workUsed":result.exact_work})
        }
        "classify_path_membership" => {
            let result = a::exact_polyhedral_path_coverage(&bodies, &path, work)?;
            let spans=result.spans.into_iter().map(|parts|parts.into_iter().map(|p|json!({"parameters":p.parameters.map(|q|q.to_string()),"included":p.endpoints_included,"solids":p.solids})).collect::<Vec<_>>()).collect::<Vec<_>>();
            json!({"model":model,"spans":spans,"workUsed":result.exact_work})
        }
        "bound_overlap_lengths" => {
            let result = a::bounded_polyhedral_path_coverage_lengths(
                &bodies,
                &path,
                field(v, "tolerance_mm")?,
                work,
            )?;
            let lengths=result.lengths.into_iter().map(|p|json!({"count":p.solid_count,"boundsMm":p.length_bounds_mm.map(|q|q.to_string())})).collect::<Vec<_>>();
            json!({"model":model,"lengths":lengths,"workUsed":result.exact_work})
        }
        _ => {
            let result = a::exact_polyhedral_path_coverage_components(
                &bodies,
                &path,
                field(v, "min_count")?..=field(v, "max_count")?,
                work,
            )?;
            let components=result.components.into_iter().map(|p|{
                let points=p.points_mm.into_iter().map(|p|p.map(|q|q.to_string())).collect::<Vec<_>>();
                json!({"parameters":p.parameters.map(|q|q.to_string()),"included":p.endpoints_included,"pointsMm":points})
            }).collect::<Vec<_>>();
            json!({"model":model,"components":components,"workUsed":result.exact_work})
        }
    })
}
