use super::section_queries::{coordinate, point};
use crate::*;
use brep_core::analysis as a;
use num_rational::BigRational as Q;

fn path(v: &Value, name: &str) -> Result<Vec<[Q; 3]>> {
    v[name]
        .as_array()
        .filter(|points| (1..=4096).contains(&points.len()))
        .ok_or_else(|| {
            nurbs_core::Error::new(
                "BREP_INVALID_INPUT",
                "Expected a spatial path with 1 to 4096 vertices",
            )
        })?
        .iter()
        .map(point)
        .collect()
}

fn coordinates(points: Vec<[Q; 3]>) -> Vec<[String; 3]> {
    points
        .into_iter()
        .map(|p| p.map(|q| q.to_string()))
        .collect()
}

pub(super) fn measure(v: &Value, op: &str) -> Result<Value> {
    let points = path(v, "query_path_mm")?;
    let work = field(v, "max_work")?;
    let report = match op {
        "simplify_exact_path" => {
            let r = a::simplify_exact_spatial_polyline(&points, work)?;
            json!({"pointsMm":coordinates(r.points_mm),"sourceVertices":r.source_vertices,"workUsed":r.exact_work})
        }
        "simplify_path_with_tolerance" => {
            let r = a::simplify_bounded_spatial_polyline(
                &points,
                &coordinate(&v["deviation_mm"])?,
                work,
            )?;
            json!({"pointsMm":coordinates(r.points_mm),"sourceVertices":r.source_vertices,"deviationSquaredMm2":r.deviation_squared_mm2.to_string(),"workUsed":r.exact_work})
        }
        "project_point_to_path" => {
            let r = a::project_point_to_exact_spatial_polyline(
                &points,
                &point(&v["query_point_mm"])?,
                work,
            )?;
            let hits=r.hits.into_iter().map(|h|json!({"parameters":h.parameters.map(|q|q.to_string()),"pointMm":h.point_mm.map(|q|q.to_string())})).collect::<Vec<_>>();
            json!({"distanceSquaredMm2":r.distance_squared_mm2.to_string(),"hits":hits,"workUsed":r.exact_work})
        }
        "intersect_spatial_paths" | "find_path_self_intersections" => {
            let r = if op == "intersect_spatial_paths" {
                a::intersect_exact_spatial_polylines(&points, &path(v, "other_path_mm")?, work)?
            } else {
                a::self_intersections_exact_spatial_polyline(&points, work)?
            };
            let hits=r.intersections.into_iter().map(|h|json!({"spans":h.spans,"parameters":h.parameters.map(|v|v.map(|q|q.to_string())),"pointsMm":h.points_mm.map(|p|p.map(|q|q.to_string()))})).collect::<Vec<_>>();
            json!({"intersections":hits,"workUsed":r.exact_work})
        }
        "measure_spatial_path_distance" => {
            let r = a::distance_between_exact_spatial_polylines(
                &points,
                &path(v, "other_path_mm")?,
                work,
            )?;
            json!({"distanceSquaredMm2":r.distance_squared_mm2.to_string(),"spans":r.spans,"parameters":r.parameters.map(|q|q.to_string()),"pointsMm":r.points_mm.map(|p|p.map(|q|q.to_string())),"workUsed":r.exact_work})
        }
        _ => {
            return Err(nurbs_core::Error::new(
                "BREP_INVALID_INPUT",
                "Unknown spatial path operation",
            ));
        }
    };
    // Preserve exact rational coordinates in the report. Round only the display curve.
    let display = if op == "simplify_exact_path" || op == "simplify_path_with_tolerance" {
        path(&report, "pointsMm")?
    } else {
        points
    };
    use num_traits::ToPrimitive;
    let mut vertices = display
        .iter()
        .map(|p| {
            p.iter()
                .map(|q| {
                    q.to_f64().filter(|v| v.is_finite()).ok_or_else(|| {
                        nurbs_core::Error::new(
                            "BREP_INVALID_INPUT",
                            "Path exceeds display coordinate range",
                        )
                    })
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    if vertices.len() == 1 {
        vertices.push(vertices[0].clone());
    }
    let curve = Curve::from_polyline(vertices)?;
    Ok(json!({"curve":curve,"report":report}))
}
