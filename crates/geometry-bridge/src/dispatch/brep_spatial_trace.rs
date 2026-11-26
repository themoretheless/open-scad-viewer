use super::section_queries::{coordinate, point};
use crate::*;
use brep_core::analysis as a;
use num_rational::BigRational as Q;

fn path(v: &Value, name: &str) -> Result<Vec<[Q; 3]>> {
    point_list(&v[name])
}

fn point_list(v: &Value) -> Result<Vec<[Q; 3]>> {
    v.as_array()
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

fn hole_paths(v: &Value) -> Result<Vec<Vec<[Q; 3]>>> {
    v.as_array()
        .filter(|h| h.len() <= 64)
        .ok_or_else(|| {
            nurbs_core::Error::new("BREP_INVALID_INPUT", "Expected up to 64 hole contours")
        })?
        .iter()
        .map(point_list)
        .collect::<Result<Vec<_>>>()
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
        "intersect_transverse_profiles"
        | "measure_profile_overlap_area"
        | "measure_profile_overlap_centroid"
        | "classify_profile_overlap"
        | "decompose_profile_overlap" => {
            let second = path(v, "other_path_mm")?;
            let h1 = hole_paths(&v["holes_mm"])?;
            let h2 = hole_paths(&v["other_holes_mm"])?;
            let first_holes = h1.iter().map(Vec::as_slice).collect::<Vec<_>>();
            let second_holes = h2.iter().map(Vec::as_slice).collect::<Vec<_>>();
            match op {
                "intersect_transverse_profiles" => {
                    let r = a::intersect_transverse_exact_planar_polyline_regions(
                        &points,
                        &first_holes,
                        &second,
                        &second_holes,
                        work,
                    )?;
                    let segments = r
                        .segments_mm
                        .into_iter()
                        .map(|s| s.map(|p| p.map(|q| q.to_string())))
                        .collect::<Vec<_>>();
                    json!({"segmentsMm":segments,"workUsed":r.exact_work})
                }
                "measure_profile_overlap_area" => {
                    let r = a::exact_coplanar_planar_polyline_region_overlap_area(
                        &points,
                        &first_holes,
                        &second,
                        &second_holes,
                        work,
                    )?;
                    json!({"areaVectorMm2":r.area_vector_mm2.map(|q|q.to_string()),"areaSquaredMm4":r.area_squared_mm4.to_string(),"workUsed":r.exact_work})
                }
                "measure_profile_overlap_centroid" => {
                    let r = a::exact_coplanar_planar_polyline_region_overlap_mass(
                        &points,
                        &first_holes,
                        &second,
                        &second_holes,
                        work,
                    )?;
                    json!({"centroidMm":r.centroid_mm.map(|p|p.map(|q|q.to_string())),"areaSquaredMm4":r.area_squared_mm4.to_string(),"workUsed":r.exact_work})
                }
                "classify_profile_overlap" => {
                    let r = a::classify_exact_coplanar_planar_polyline_regions(
                        &points,
                        &first_holes,
                        &second,
                        &second_holes,
                        work,
                    )?;
                    json!({"relation":format!("{:?}",r.relation),"areaSquaredMm4":r.overlap_area_squared_mm4.to_string(),"workUsed":r.exact_work})
                }
                _ => {
                    let r = a::decompose_exact_coplanar_planar_polyline_region_overlap(
                        &points,
                        &first_holes,
                        &second,
                        &second_holes,
                        work,
                    )?;
                    let cells = r.cells_mm.into_iter().map(coordinates).collect::<Vec<_>>();
                    json!({"cellsMm":cells,"workUsed":r.exact_work})
                }
            }
        }
        "node_spatial_path" => {
            let r = a::node_exact_spatial_polyline(&points, work)?;
            let edges=r.edges.into_iter().map(|e|json!({"sourceSpan":e.source_span,"parameters":e.parameters.map(|q|q.to_string()),"nodes":e.nodes})).collect::<Vec<_>>();
            json!({"nodesMm":coordinates(r.nodes_mm),"edges":edges,"workUsed":r.exact_work})
        }
        "triangulate_planar_path" => {
            let r = a::triangulate_exact_planar_spatial_polyline(&points, work)?;
            json!({"triangles":r.triangles,"workUsed":r.exact_work})
        }
        "measure_path_second_moments" => {
            let r = a::exact_planar_spatial_polyline_second_moments(&points, work)?;
            json!({"centroidMm":r.centroid_mm.map(|q|q.to_string()),"normalizedMomentsMm2":r.normalized_central_moments_mm2.map(|row|row.map(|q|q.to_string())),"areaSquaredMm4":r.area_squared_mm4.to_string(),"workUsed":r.exact_work})
        }
        "bound_path_principal_moments" => {
            let r = a::bounded_planar_spatial_polyline_principal_moments(
                &points,
                field(v, "tolerance_mm2")?,
                work,
            )?;
            json!({"boundsMm2":r.principal_bounds_mm2.map(|row|row.map(|q|q.to_string())),"workUsed":r.exact_work})
        }
        "measure_region_mass"
        | "clip_path_to_perforated_region"
        | "measure_region_second_moments" => {
            let holes = hole_paths(&v["holes_mm"])?;
            let rings = holes.iter().map(Vec::as_slice).collect::<Vec<_>>();
            match op {
                "measure_region_mass" => {
                    let r = a::exact_planar_spatial_polyline_region_mass(&points, &rings, work)?;
                    json!({"centroidMm":r.centroid_mm.map(|q|q.to_string()),"areaVectorMm2":r.area_vector_mm2.map(|q|q.to_string()),"areaSquaredMm4":r.area_squared_mm4.to_string(),"workUsed":r.exact_work})
                }
                "clip_path_to_perforated_region" => {
                    let r = a::clip_spatial_path_to_exact_planar_polyline_region(
                        &points,
                        &rings,
                        &path(v, "other_path_mm")?,
                        work,
                    )?;
                    let spans = r
                        .span_intervals
                        .into_iter()
                        .map(|intervals| {
                            intervals
                                .into_iter()
                                .map(|p| p.map(|q| q.to_string()))
                                .collect::<Vec<_>>()
                        })
                        .collect::<Vec<_>>();
                    json!({"spans":spans,"workUsed":r.exact_work})
                }
                _ => {
                    let r = a::exact_planar_spatial_polyline_region_second_moments(
                        &points, &rings, work,
                    )?;
                    json!({"centroidMm":r.centroid_mm.map(|q|q.to_string()),"normalizedMomentsMm2":r.normalized_central_moments_mm2.map(|row|row.map(|q|q.to_string())),"areaSquaredMm4":r.area_squared_mm4.to_string(),"workUsed":r.exact_work})
                }
            }
        }
        "measure_planar_path_area" => {
            let r = a::exact_planar_spatial_polyline_area(&points, work)?;
            json!({"areaVectorMm2":r.area_vector_mm2.map(|q|q.to_string()),"areaSquaredMm4":r.area_squared_mm4.to_string(),"workUsed":r.exact_work})
        }
        "measure_planar_path_centroid" => {
            let r = a::exact_planar_spatial_polyline_centroid(&points, work)?;
            json!({"centroidMm":r.centroid_mm.map(|q|q.to_string()),"workUsed":r.exact_work})
        }
        "locate_planar_path_points" => {
            let r = a::locate_points_in_exact_planar_spatial_polyline(
                &points,
                &path(v, "other_path_mm")?,
                work,
            )?;
            let locations = r
                .locations
                .into_iter()
                .map(|l| format!("{l:?}"))
                .collect::<Vec<_>>();
            json!({"locations":locations,"workUsed":r.exact_work})
        }
        "clip_path_to_planar_region" => {
            let r = a::clip_spatial_path_to_exact_planar_polyline(
                &points,
                &path(v, "other_path_mm")?,
                work,
            )?;
            let spans = r
                .span_intervals
                .into_iter()
                .map(|intervals| {
                    intervals
                        .into_iter()
                        .map(|p| p.map(|q| q.to_string()))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            json!({"spans":spans,"workUsed":r.exact_work})
        }
        "project_to_planar_region" => {
            let r = a::project_point_to_exact_planar_polyline_region(
                &points,
                &point(&v["query_point_mm"])?,
                work,
            )?;
            json!({"closestMm":r.closest_mm.map(|q|q.to_string()),"location":format!("{:?}",r.location),"distanceSquaredMm2":r.distance_squared_mm2.to_string(),"workUsed":r.exact_work})
        }
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
