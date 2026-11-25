use super::*;
use crate::editing::exact_surface_derivative as gap;

fn graph(value: Option<gap::CurvatureGapGraph>, reason: Option<gap::Refusal>) -> Result<Value> {
    use gap::CurvatureGapGraph::*;
    Ok(
        match value.ok_or_else(|| input(format!("Contour proof refused: {reason:?}")))? {
            Unique { increasing } => json!({"status":"unique","increasing":increasing}),
            NoContour => json!({"status":"empty"}),
            EverywhereAtLevel => json!({"status":"coincident"}),
            Unresolved => json!({"status":"unresolved"}),
        },
    )
}
pub(super) fn inspect(v: &Value, op: &str) -> Result<Value> {
    let s = field(v, "surface")?;
    let span = field(v, "knot_span")?;
    let level = field(v, "gap_level_per_mm2")?;
    let budget = field(v, "max_work")?;
    if op == "surface_gap_edge_roots" {
        use gap::{CurvatureGapEdge::*, EdgeThresholdRoots::*};
        let side = match field::<String>(v, "source_edge")?.as_str() {
            "u_min" => UMin,
            "u_max" => UMax,
            "v_min" => VMin,
            "v_max" => VMax,
            _ => return Err(input("Unknown edge")),
        };
        let r =
            gap::curvature_gap_edge_roots(&s, span, side, level, field(v, "max_depth")?, budget)?;
        let roots = match r
            .roots
            .ok_or_else(|| input(format!("Edge proof refused: {:?}", r.refusal)))?
        {
            Coincident => json!({"status":"coincident"}),
            UnresolvedDegree => json!({"status":"unresolvedDegree"}),
            Isolated {
                exact_points,
                brackets,
            } => {
                json!({"status":"isolated","exactUnitPoints":exact_points,"brackets":brackets.iter().map(|b|json!({"unitInterval":b.unit_interval,"variation":b.variation})).collect::<Vec<_>>()})
            }
        };
        return Ok(json!({"varyingDomain":r.varying_domain,"roots":roots,"workUsed":r.work_used}));
    }
    let axis = match field::<String>(v, "direction")?.as_str() {
        "u" => gap::Axis::U,
        "v" => gap::Axis::V,
        _ => return Err(input("Unknown contour axis")),
    };
    match op {
        "surface_gap_pair_distance" => {
            let target = field(v, "target")?;
            let stations: Vec<f64> = field(v, "transverse")?;
            let target_stations: Vec<f64> = field(v, "target_transverse")?;
            let a = gap::CurvatureGapContourSpec {
                surface: &s,
                span,
                axis,
                level,
                transverse: &stations,
                root_depth: field(v, "max_depth")?,
            };
            let b = gap::CurvatureGapContourSpec {
                surface: &target,
                span: field(v, "target_knot_span")?,
                axis: match field::<String>(v, "target_direction")?.as_str() {
                    "u" => gap::Axis::U,
                    "v" => gap::Axis::V,
                    _ => return Err(input("Unknown target contour axis")),
                },
                level: field(v, "target_gap_level_per_mm2")?,
                transverse: &target_stations,
                root_depth: field(v, "target_max_depth")?,
            };
            let r = gap::distance_between_curvature_gap_contours(a, b, budget)?;
            if let Some(reason) = r.refusal {
                return Err(input(format!("Contour pair proof refused: {reason:?}")));
            }
            Ok(
                json!({"distanceBoundsMm":r.distance_bounds,"upperSegments":r.upper_segments,"comparedPairs":r.compared_pairs,"prunedPairs":r.pruned_pairs,"workUsed":r.work_used}),
            )
        }
        "surface_gap_contour_graph" => {
            let r = gap::certify_curvature_gap_graph(&s, span, axis, level, budget)?;
            Ok(json!({"domain":r.domain,"graph":graph(r.graph,r.refusal)?,"workUsed":r.work_used}))
        }
        "surface_gap_contour_samples" => {
            let stations: Vec<f64> = field(v, "transverse")?;
            let r = gap::sample_curvature_gap_graph(
                &s,
                span,
                axis,
                level,
                &stations,
                field(v, "max_depth")?,
                budget,
            )?;
            let samples = r.samples.map(|s| {
                s.into_iter()
                    .map(|p| json!({"transverse":p.transverse,"unitRootInterval":p.root_interval}))
                    .collect::<Vec<_>>()
            });
            Ok(
                json!({"domain":r.domain,"graph":graph(r.graph,r.refusal)?,"samples":samples,"workUsed":r.work_used}),
            )
        }
        "surface_gap_contour_slope" => {
            let r = gap::curvature_gap_graph_slope(&s, span, axis, level, budget)?;
            Ok(
                json!({"domain":r.domain,"graph":graph(r.graph,r.refusal)?,"slope":r.slope,"workUsed":r.work_used}),
            )
        }
        "surface_gap_uv_polyline" => {
            let stations: Vec<f64> = field(v, "transverse")?;
            let r = gap::certify_curvature_gap_polyline(
                &s,
                span,
                axis,
                level,
                &stations,
                field(v, "max_depth")?,
                budget,
            )?;
            polyline(r, None)
        }
        "surface_gap_adaptive_polyline" | "surface_gap_second_order_polyline" => {
            let options = gap::AdaptiveCurvaturePolylineOptions {
                max_error: field(v, "uv_error")?,
                max_vertices: field(v, "max_vertices")?,
                root_depth: field(v, "max_depth")?,
                max_work: budget,
            };
            let r = if op == "surface_gap_adaptive_polyline" {
                gap::adapt_curvature_gap_polyline(&s, span, axis, level, options)?
            } else {
                gap::adapt_curvature_gap_polyline_second_order(&s, span, axis, level, options)?
            };
            polyline(r.polyline, r.stop)
        }
        "surface_gap_second_derivative" => {
            let r = gap::curvature_gap_graph_second_derivative(&s, span, axis, level, budget)?;
            Ok(
                json!({"domain":r.domain,"graph":graph(r.graph,r.refusal)?,"secondDerivative":r.second_derivative,"workUsed":r.work_used}),
            )
        }
        "surface_gap_world_polyline"
        | "surface_gap_world_length"
        | "surface_gap_plane_section"
        | "surface_gap_world_distance"
        | "surface_gap_world_bounds" => {
            let stations: Vec<f64> = field(v, "transverse")?;
            let depth = field(v, "max_depth")?;
            match op {
                "surface_gap_world_polyline" => {
                    let r = gap::certify_curvature_gap_world_polyline(
                        &s, span, axis, level, &stations, depth, budget,
                    )?;
                    let status = graph(r.uv.graph, r.uv.refusal)?;
                    let curve = if field::<bool>(v, "return_field")? {
                        r.points
                            .as_ref()
                            .map(|rows| {
                                crate::curve::Curve::from_polyline(
                                    rows.iter().map(|p| p.to_vec()).collect(),
                                )
                            })
                            .transpose()?
                    } else {
                        None
                    };
                    Ok(
                        json!({"curve":curve,"domain":r.uv.domain,"graph":status,"pointsMm":r.points,"worldErrorUpperMm":r.world_error_upper,"workUsed":r.uv.work_used}),
                    )
                }
                "surface_gap_world_length" => {
                    let r = gap::measure_curvature_gap_contour(
                        &s, span, axis, level, &stations, depth, budget,
                    )?;
                    Ok(
                        json!({"graph":graph(r.graph,r.refusal)?,"lengthBoundsMm":r.length_bounds,"segmentBoundsMm":r.segment_bounds,"workUsed":r.work_used}),
                    )
                }
                "surface_gap_plane_section" => {
                    let spec = gap::CurvatureGapContourSpec {
                        surface: &s,
                        span,
                        axis,
                        level,
                        transverse: &stations,
                        root_depth: depth,
                    };
                    let report = gap::section_curvature_gap_contour(
                        spec,
                        field(v, "plane_normal")?,
                        field(v, "plane_offset_mm")?,
                        budget,
                    )?;
                    let events = report.events.map(|events| events.into_iter().map(|event| {
                        let status = match event.status {
                            gap::CurvatureGapPlaneStatus::Crossing => "crossing",
                            gap::CurvatureGapPlaneStatus::Possible => "possible",
                        };
                        json!({"segment":event.segment,"transverse":event.transverse,"status":status})
                    }).collect::<Vec<_>>());
                    Ok(
                        json!({"graph":graph(report.graph,report.refusal)?,"events":events,"workUsed":report.work_used}),
                    )
                }
                "surface_gap_world_distance" => {
                    let r = gap::distance_to_curvature_gap_contour(
                        &s,
                        span,
                        axis,
                        level,
                        field(v, "probe_point")?,
                        &stations,
                        depth,
                        budget,
                    )?;
                    Ok(
                        json!({"graph":graph(r.graph,r.refusal)?,"distanceBoundsMm":r.distance_bounds,"upperSegment":r.upper_segment,"workUsed":r.work_used}),
                    )
                }
                _ => {
                    let r = gap::bound_curvature_gap_contour(
                        &s, span, axis, level, &stations, depth, budget,
                    )?;
                    Ok(
                        json!({"domain":r.uv.domain,"graph":graph(r.uv.graph,r.uv.refusal)?,"worldBoundsMm":r.bounds,"segmentBoxesMm":r.segment_bounds,"workUsed":r.uv.work_used}),
                    )
                }
            }
        }
        _ => Err(input("Unknown curvature contour query")),
    }
}

fn polyline(
    r: gap::CurvatureGapPolyline,
    stop: Option<gap::CurvaturePolylineStop>,
) -> Result<Value> {
    let samples = r.samples.map(|rows| {
        rows.into_iter()
            .map(|p| json!({"transverse":p.transverse,"unitRootInterval":p.root_interval}))
            .collect::<Vec<_>>()
    });
    Ok(
        json!({"domain":r.domain,"graph":graph(r.graph,r.refusal)?,"samples":samples,"segmentErrorUpperUv":r.segment_error_upper,"stop":stop.map(|v|format!("{v:?}")),"workUsed":r.work_used}),
    )
}
