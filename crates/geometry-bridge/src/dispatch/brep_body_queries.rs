use super::section_queries::{coordinate, point};
use crate::*;
use brep_core::analysis as a;

fn sample(r: a::ExactPolyhedralSignedDistance) -> Value {
    json!({"signedDistanceSquaredMm2":r.signed_distance_squared_mm2.to_string(),"nearestMm":r.nearest_mm.map(|x|x.to_string()),"face":r.face,"workUsed":r.exact_work})
}
pub(super) fn measure(v: &Value, op: &str) -> Result<Value> {
    let solid = super::polyhedral_mass::qualify(
        field(v, "source")?,
        super::closed_ss::closed_solid_policy(field(v, "tolerance_uv")?),
    )?;
    let model = solid.model();
    let work = field(v, "max_work")?;
    Ok(match op {
        "clip_polyhedral_segment" => {
            let raw = v["query_path_mm"]
                .as_array()
                .filter(|p| p.len() == 2)
                .ok_or_else(|| {
                    nurbs_core::Error::new("BREP_INVALID_INPUT", "A segment requires two endpoints")
                })?;
            let r = a::exact_polyhedral_segment_clip(
                &solid,
                &[point(&raw[0])?, point(&raw[1])?],
                work,
            )?;
            json!({"model":model,"intervals":r.intervals.into_iter().map(|i|i.map(|x|x.to_string())).collect::<Vec<_>>(),"workUsed":r.exact_work})
        }
        "sample_polyhedral_signed_distances" => {
            let raw = v["query_path_mm"]
                .as_array()
                .filter(|p| p.len() <= 4096)
                .ok_or_else(|| {
                    nurbs_core::Error::new("BREP_INVALID_INPUT", "Expected a bounded point batch")
                })?;
            let points = raw.iter().map(point).collect::<Result<Vec<_>>>()?;
            let r = a::exact_polyhedral_signed_distance_batch(&solid, &points, work)?;
            json!({"model":model,"samples":r.samples.into_iter().map(sample).collect::<Vec<_>>(),"workUsed":r.exact_work})
        }
        "locate_polyhedral_point" => {
            let r =
                a::exact_polyhedral_point_location(&solid, &point(&v["query_point_mm"])?, work)?;
            let location = match r.location {
                a::PolyhedralPointLocation::Material => json!({"kind":"material"}),
                a::PolyhedralPointLocation::Exterior => json!({"kind":"exterior"}),
                a::PolyhedralPointLocation::Boundary { face } => {
                    json!({"kind":"boundary","face":face})
                }
            };
            json!({"model":model,"location":location,"workUsed":r.exact_work})
        }
        "measure_polyhedral_boundary_distance" => {
            let r =
                a::exact_polyhedral_boundary_distance(&solid, &point(&v["query_point_mm"])?, work)?;
            json!({"model":model,"distanceSquaredMm2":r.distance_squared_mm2.to_string(),"nearestMm":r.point_mm.map(|x|x.to_string()),"face":r.face,"workUsed":r.exact_work})
        }
        "measure_polyhedral_signed_distance" => {
            let r =
                a::exact_polyhedral_signed_distance(&solid, &point(&v["query_point_mm"])?, work)?;
            json!({"model":model,"sample":sample(r)})
        }
        _ => {
            let r = a::exact_polyhedral_ball_clearance(
                &solid,
                &point(&v["query_point_mm"])?,
                &coordinate(&v["query_radius_mm"])?,
                work,
            )?;
            json!({"model":model,"relation":format!("{:?}",r.relation),"signedDistanceSquaredMm2":r.signed_distance_squared_mm2.to_string(),"nearestMm":r.nearest_mm.map(|x|x.to_string()),"face":r.face,"workUsed":r.exact_work})
        }
    })
}
