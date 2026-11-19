//! Bridge operations of the `nurbs` family.
use crate::*;

const OPS: &[&str] = &[
    "nurbs_curve_axis_motion",
    "nurbs_curve_directional_travel_full",
    "nurbs_curve_directional_travel_signed",
    "nurbs_curve_check_backtracking",
    "nurbs_measure_curve_axial_derivative_peak",
    "nurbs_measure_curve_directional_travel",
    "nurbs_measure_curve_directional_travel_batch",
    "brep_nurbs_exact_convex_prism_fillet",
    "brep_nurbs_exact_simple_prism_fillet",
    "brep_nurbs_exact_annular_fillet",
    "brep_nurbs_exact_layered_prism_fillet",
    "brep_nurbs_partial_annular_preview",
    "brep_nurbs_exact_variable_radius_fillet",
    "brep_nurbs_exact_valence3_corner_blend",
    "brep_nurbs_audited_parallel_frame_sweep",
    "brep_nurbs_audited_multi_section_loft_v2",
    "brep_nurbs_audited_bent_rmf_sweep_v2",
    "brep_nurbs_inspect",
    "brep_nurbs_tessellate",
    "brep_nurbs_certified_tessellate",
    "brep_nurbs_certified_freeform_tessellate",
    "brep_nurbs_display",
    "brep_nurbs_to_polygon",
    "nurbs_viewer_lod_select",
    "brep_nurbs_display_levels",
];

pub(crate) fn route(v: Value) -> Routed {
    if !OPS.contains(&v["op"].as_str().unwrap_or("")) {
        return Routed::Unhandled(v);
    }
    Routed::Handled(handle(v))
}

fn handle(mut v: Value) -> Result<Value> {
    match v["op"].as_str().unwrap_or("") {
        "nurbs_curve_axis_motion" => curve_axis_motion(&v),
        "nurbs_curve_directional_travel_full"
        | "nurbs_curve_directional_travel_signed"
        | "nurbs_curve_check_backtracking" => full_travel_or_backtracking(&v),
        "nurbs_measure_curve_axial_derivative_peak" => axial_derivative_peak(&v),
        "nurbs_measure_curve_directional_travel" => curve_directional_travel_request(&v),
        "nurbs_measure_curve_directional_travel_batch" => {
            curve_directional_travel_batch_request(&v)
        }
        "brep_nurbs_exact_convex_prism_fillet"
        | "brep_nurbs_exact_simple_prism_fillet"
        | "brep_nurbs_exact_annular_fillet"
        | "brep_nurbs_exact_layered_prism_fillet" => {
            require_exact_fields(
                &v,
                &["op", "model", "edges", "radius"],
                "exact fillet request",
            )?;
            let author = match v["op"].as_str() {
                Some("brep_nurbs_exact_simple_prism_fillet") => {
                    brep_core::exact_simple_prism_fillet
                }
                Some("brep_nurbs_exact_annular_fillet") => brep_core::exact_annular_fillet,
                Some("brep_nurbs_exact_layered_prism_fillet") => {
                    brep_core::exact_layered_prism_fillet
                }
                _ => brep_core::exact_convex_prism_fillet,
            };
            encode(author(
                &field(&v, "model")?,
                &field::<Vec<usize>>(&v, "edges")?,
                field(&v, "radius")?,
            )?)
        }
        "brep_nurbs_partial_annular_preview" => {
            require_exact_fields(
                &v,
                &["op", "model", "edge", "radius"],
                "partial annular preview request",
            )?;
            let model = brep_core::analytic_features::build_partial_annular_preview(
                &field(&v, "model")?,
                field(&v, "edge")?,
                field(&v, "radius")?,
            )?;
            Ok(json!({
                "model":model,
                "changeSet":model.1.change_set,
                "qualification":{
                    "status":"preview-only", "commitAllowed":false,
                    "boundaryIntersectionProof":"unqualified",
                    "transitionContinuityProof":"unqualified"
                }
            }))
        }
        "brep_nurbs_exact_variable_radius_fillet" => {
            require_exact_fields(
                &v,
                &["op", "model", "edges", "radii"],
                "variable-radius fillet request",
            )?;
            encode(brep_core::exact_variable_radius_fillet(
                &field(&v, "model")?,
                &field::<Vec<usize>>(&v, "edges")?,
                &field::<Vec<[f64; 2]>>(&v, "radii")?,
            )?)
        }
        "brep_nurbs_exact_valence3_corner_blend" => {
            require_exact_fields(
                &v,
                &["op", "model", "edges", "radius"],
                "valence-3 corner blend request",
            )?;
            encode(brep_core::exact_valence3_corner_blend(
                &field(&v, "model")?,
                &field::<Vec<usize>>(&v, "edges")?,
                field::<f64>(&v, "radius")?,
            )?)
        }
        "brep_nurbs_audited_parallel_frame_sweep" => {
            let frame_law: String = field(&v, "frameLaw")?;
            encode(brep_core::audited_parallel_frame_sweep(
                &field::<Vec<[f64; 2]>>(&v, "profile")?,
                &field::<Vec<[f64; 3]>>(&v, "path")?,
                &frame_law,
            )?)
        }
        "brep_nurbs_audited_multi_section_loft_v2" => {
            require_exact_fields(&v, &["op", "sections"], "multi-section loft request")?;
            encode(brep_core::audited_multi_section_loft(&field::<
                Vec<Vec<[f64; 3]>>,
            >(
                &v, "sections"
            )?)?)
        }
        "brep_nurbs_audited_bent_rmf_sweep_v2" => {
            require_exact_fields(
                &v,
                &["op", "profile", "path", "twistRadians", "scales"],
                "bent RMF sweep request",
            )?;
            encode(brep_core::audited_bent_rmf_sweep(
                &field::<Vec<[f64; 2]>>(&v, "profile")?,
                &field::<Vec<[f64; 3]>>(&v, "path")?,
                &field::<Vec<f64>>(&v, "twistRadians")?,
                &field::<Vec<f64>>(&v, "scales")?,
            )?)
        }
        "brep_nurbs_inspect" => {
            encode(take_field::<brep_core::Model>(&mut v, "model")?.validate()?)
        }
        "brep_nurbs_tessellate" => encode(brep::nurbs(
            &take_field(&mut v, "model")?,
            field(&v, "segments")?,
        )?),
        "brep_nurbs_certified_tessellate" => encode(brep::certified_nurbs(
            &take_field(&mut v, "model")?,
            field(&v, "chordToleranceMm")?,
            v.get("maxTriangles")
                .and_then(Value::as_u64)
                .unwrap_or(20_000) as usize,
        )?),
        "brep_nurbs_certified_freeform_tessellate" => encode(brep::certified_freeform_nurbs(
            &take_field(&mut v, "model")?,
            field(&v, "chordToleranceMm")?,
            v.get("maxTriangles")
                .and_then(Value::as_u64)
                .unwrap_or(20_000) as usize,
        )?),
        "brep_nurbs_display" => brep_display::dispatch(v),
        "nurbs_viewer_lod_select" => viewer_lod::dispatch(v),
        "brep_nurbs_display_levels" => brep_display::dispatch_levels(v),
        "brep_nurbs_to_polygon" => {
            let t = brep::nurbs(&take_field(&mut v, "model")?, field(&v, "segments")?)?;
            encode(polygon_core::solid::brep::from_mesh(
                &t.built.mesh,
                Some(&t.face_ids),
            )?)
        }
        _ => unreachable!("route admits only OPS"),
    }
}

pub(crate) fn curve_directional_travel_request(request: &Value) -> Result<Value> {
    use nurbs_core::analysis::curve_measure::directional_travel_on_interval;

    let curve: nurbs_core::curve::Curve = field(request, "curve")?;
    curve.validate()?;
    let interval: [f64; 2] = if request.get("parameterInterval").is_some() {
        field(request, "parameterInterval")?
    } else {
        curve.domain()
    };
    let direction: Vec<f64> = field(request, "direction")?;
    let tolerance: f64 = field(request, "toleranceMm")?;
    let max_cells: usize = field(request, "maxCells")?;
    let reverse_limit = reverse_travel_limit(request)?;
    let report =
        directional_travel_on_interval(&curve, interval, &direction, tolerance, max_cells)?;
    directional_travel_response(interval, &report, reverse_limit)
}

fn curve_directional_travel_batch_request(request: &Value) -> Result<Value> {
    use nurbs_core::analysis::curve_measure::directional_travel_on_intervals;

    let curve = field(request, "curve")?;
    let intervals: Vec<[f64; 2]> = field(request, "parameterIntervals")?;
    let direction: Vec<f64> = field(request, "direction")?;
    let tolerance = field(request, "toleranceMm")?;
    let max_cells = field(request, "maxCells")?;
    let reverse_limit = reverse_travel_limit(request)?;
    let reports =
        directional_travel_on_intervals(&curve, &intervals, &direction, tolerance, max_cells)?;
    let cells = reports
        .iter()
        .map(|report| report.total.cells)
        .sum::<usize>();
    let intervals = intervals
        .into_iter()
        .zip(&reports)
        .map(|(interval, report)| directional_travel_response(interval, report, reverse_limit))
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({"cells": cells, "intervals": intervals}))
}

fn reverse_travel_limit(request: &Value) -> Result<Option<f64>> {
    request
        .get("reverseLimitMm")
        .map(|_| {
            let limit: f64 = field(request, "reverseLimitMm")?;
            if !limit.is_finite() || limit < 0. {
                return Err(input("Reverse travel limit must be finite and nonnegative"));
            }
            Ok(limit)
        })
        .transpose()
}

fn directional_travel_response(
    interval: [f64; 2],
    report: &nurbs_core::analysis::curve_measure::DirectionalTravelReport,
    reverse_limit: Option<f64>,
) -> Result<Value> {
    use nurbs_core::analysis::curve_measure::ReverseTravelStatus;
    let reverse_status = reverse_limit
        .map(|limit| report.reverse_status(limit))
        .transpose()?
        .map(|status| match status {
            ReverseTravelStatus::Within => "within",
            ReverseTravelStatus::Exceeds => "exceeds",
            ReverseTravelStatus::Undetermined => "undetermined",
        });
    let stop_reason = stop_reason_label(report.total.stop_reason);
    Ok(json!({
        "parameterInterval": interval,
        "totalBoundsMm": report.total.bounds,
        "forwardBoundsMm": report.forward_bounds,
        "reverseBoundsMm": report.reverse_bounds,
        "signedTravelBoundsMm": report.signed_travel_bounds,
        "withinTolerance": report.total.within_tolerance,
        "cells": report.total.cells,
        "stopReason": stop_reason,
        "reverseStatus": reverse_status,
    }))
}

fn stop_reason_label(reason: nurbs_core::analysis::curve_measure::StopReason) -> &'static str {
    use nurbs_core::analysis::curve_measure::StopReason;
    match reason {
        StopReason::Tolerance => "tolerance",
        StopReason::WorkLimit => "workLimit",
        StopReason::PrecisionLimit => "precisionLimit",
        StopReason::IterationLimit => "iterationLimit",
        StopReason::TargetUncertainty => "targetUncertainty",
    }
}

fn axial_derivative_peak(request: &Value) -> Result<Value> {
    use nurbs_core::analysis::curve_measure::*;
    let curve = field(request, "curve")?;
    let interval = field(request, "parameterInterval")?;
    let direction: Vec<f64> = field(request, "direction")?;
    let tolerance = field(request, "tolerance")?;
    let cells = field(request, "maxCells")?;
    let order: usize = field(request, "order")?;
    let location: Option<f64> = request
        .get("parameterTolerance")
        .map(|_| field(request, "parameterTolerance"))
        .transpose()?;
    let (bounds, cells, stop, locations) = match (order, location) {
        (1, None) => {
            let p = directional_peak_speed(&curve, interval, &direction, tolerance, cells)?;
            (p.bounds, p.cells, p.stop_reason, None)
        }
        (1, Some(width)) => {
            let p = locate_directional_peak_speed(
                &curve, interval, &direction, tolerance, width, cells,
            )?;
            (
                p.peak.bounds,
                p.peak.cells,
                p.peak.stop_reason,
                Some(p.parameter_intervals),
            )
        }
        (2, None) => {
            let p = directional_peak_acceleration(&curve, interval, &direction, tolerance, cells)?;
            (p.bounds, p.cells, p.stop_reason, None)
        }
        (2, Some(width)) => {
            let p = locate_directional_peak_acceleration(
                &curve, interval, &direction, tolerance, width, cells,
            )?;
            (
                p.peak.bounds,
                p.peak.cells,
                p.peak.stop_reason,
                Some(p.parameter_intervals),
            )
        }
        (3, None) => {
            let p = directional_peak_jerk(&curve, interval, &direction, tolerance, cells)?;
            (p.bounds, p.cells, p.stop_reason, None)
        }
        (3, Some(width)) => {
            let p = locate_directional_peak_jerk(
                &curve, interval, &direction, tolerance, width, cells,
            )?;
            (
                p.peak.bounds,
                p.peak.cells,
                p.peak.stop_reason,
                Some(p.parameter_intervals),
            )
        }
        _ => return Err(input("Derivative order must be 1, 2 or 3")),
    };
    Ok(
        json!({"order":order,"parameterInterval":interval,"peakBounds":bounds,"cells":cells,
        "stopReason":stop_reason_label(stop),"parameterIntervals":locations}),
    )
}

fn curve_axis_motion(request: &Value) -> Result<Value> {
    use nurbs_core::analysis::curve_measure::*;
    let curve = field(request, "curve")?;
    let window = field(request, "parameterInterval")?;
    let tolerances = || field(request, "tolerances");
    let cells = field(request, "maxCells")?;
    let directions: Vec<Vec<f64>> = field(request, "directions")?;
    let limits: Vec<[f64; 3]> = field(request, "limits")?;
    if directions.is_empty() || directions.len() != limits.len() || directions.len() > 64 {
        return Err(input(
            "Each direction needs a matching limit triple (1 to 64 axes)",
        ));
    }
    let axes: Vec<_> = directions
        .iter()
        .zip(limits)
        .map(|(direction, [speed, acceleration, jerk])| MotionAxis {
            direction,
            limits: DirectionalMotionLimits {
                speed,
                acceleration,
                jerk,
            },
        })
        .collect();
    let status = |value| match value {
        DirectionalMotionStatus::Within => "within",
        DirectionalMotionStatus::Exceeds => "exceeds",
        DirectionalMotionStatus::Undetermined => "undetermined",
    };
    // JSON numbers cannot represent the absence of a certified finite dilation.
    let bound = |value: f64| {
        if value == f64::INFINITY {
            json!("unbounded")
        } else {
            json!(value)
        }
    };
    match request.get("mode").and_then(Value::as_str) {
        Some("partition_speed") if axes.len() == 1 => {
            let p = partition_directional_speed_limit(
                &curve,
                window,
                axes[0].direction,
                axes[0].limits.speed,
                field(request, "parameterTolerance")?,
                cells,
            )?;
            let classified: Vec<_> = p.cells.into_iter().map(|c| json!({"parameterInterval":c.parameter_interval,"span":c.span,"status":status(c.status)})).collect();
            Ok(json!({"cells":classified,"workCells":p.work_cells}))
        }
        Some("partition_motion") if axes.len() == 1 => {
            let p = partition_directional_motion_limits(
                &curve,
                window,
                axes[0].direction,
                axes[0].limits,
                field(request, "parameterTolerance")?,
                cells,
            )?;
            let classified: Vec<_> = p.cells.into_iter().map(|c| json!({"parameterInterval":c.parameter_interval,"span":c.span,"statuses":c.statuses.map(status)})).collect();
            Ok(json!({"cells":classified,"workCells":p.work_cells}))
        }
        Some("partition_axes") => {
            let p = partition_multi_axis_motion_limits(
                &curve,
                window,
                &axes,
                field(request, "parameterTolerance")?,
                cells,
            )?;
            let classified: Vec<_> = p.cells.into_iter().map(|c| json!({"parameterInterval":c.parameter_interval,"span":c.span,
                "axisStatuses":c.axis_statuses.into_iter().map(|a| a.map(status)).collect::<Vec<_>>()})).collect();
            Ok(json!({"cells":classified,"workAxisCells":p.work_axis_cells}))
        }
        Some("limits") if axes.len() == 1 => {
            let p = check_directional_motion_limits(
                &curve,
                window,
                axes[0].direction,
                axes[0].limits,
                tolerances()?,
                cells,
            )?;
            Ok(
                json!({"status": status(p.status), "speedBounds":p.speed.bounds, "accelerationBounds":p.acceleration.bounds,
                      "jerkBounds":p.jerk.bounds,"cells":[p.speed.cells,p.acceleration.cells,p.jerk.cells],
                      "stopReasons":[stop_reason_label(p.speed.stop_reason),stop_reason_label(p.acceleration.stop_reason),stop_reason_label(p.jerk.stop_reason)]}),
            )
        }
        Some("slowdown") if axes.len() == 1 => {
            let bounds = directional_motion_slowdown_bounds(
                &curve,
                window,
                axes[0].direction,
                axes[0].limits,
                tolerances()?,
                cells,
            )?;
            Ok(json!({"slowdownBounds":bounds.map(bound)}))
        }
        Some("multiSlowdown") => {
            let p = multi_axis_motion_slowdown_bounds(&curve, window, &axes, tolerances()?, cells)?;
            Ok(
                json!({"slowdownBounds":p.bounds.map(bound),"axisBounds":p.axis_bounds.into_iter().map(|b| b.map(bound)).collect::<Vec<_>>(),"cells":p.cells}),
            )
        }
        Some("duration") => {
            let p = multi_axis_motion_duration_bounds(&curve, window, &axes, tolerances()?, cells)?;
            Ok(
                json!({"durationBounds":p.duration_bounds.map(bound),"slowdownBounds":p.slowdown.bounds.map(bound),
                      "axisBounds":p.slowdown.axis_bounds.into_iter().map(|b| b.map(bound)).collect::<Vec<_>>(),"cells":p.slowdown.cells}),
            )
        }
        Some("deadline") => {
            let p = check_multi_axis_motion_deadline(
                &curve,
                window,
                &axes,
                field(request, "deadline")?,
                tolerances()?,
                cells,
            )?;
            Ok(
                json!({"status":status(p.status),"axisStatuses":p.axis_statuses.into_iter().map(status).collect::<Vec<_>>(),"cells":p.cells}),
            )
        }
        _ => Err(input("Invalid motion measurement mode or axis count")),
    }
}

fn full_travel_or_backtracking(request: &Value) -> Result<Value> {
    use nurbs_core::analysis::curve_measure::*;
    let curve: nurbs_core::curve::Curve = field(request, "curve")?;
    curve.validate()?;
    let direction: Vec<f64> = field(request, "direction")?;
    let tolerance = field(request, "toleranceMm")?;
    let cells = field(request, "maxCells")?;
    match request["op"].as_str() {
        Some("nurbs_curve_directional_travel_full") => {
            let p = directional_travel(&curve, &direction, tolerance, cells)?;
            Ok(
                json!({"parameterInterval":curve.domain(),"totalBoundsMm":p.bounds,"cells":p.cells,
                "withinTolerance":p.within_tolerance,"stopReason":stop_reason_label(p.stop_reason)}),
            )
        }
        Some("nurbs_curve_directional_travel_signed") => {
            let p = directional_travel_by_sign(&curve, &direction, tolerance, cells)?;
            directional_travel_response(curve.domain(), &p, None)
        }
        _ => {
            let interval = field(request, "parameterInterval")?;
            let limit = field(request, "reverseLimitMm")?;
            let (p, verdict) = check_directional_backtracking(
                &curve, interval, &direction, limit, tolerance, cells,
            )?;
            let mut response = directional_travel_response(interval, &p, None)?;
            response["reverseStatus"] = json!(match verdict {
                ReverseTravelStatus::Within => "within",
                ReverseTravelStatus::Exceeds => "exceeds",
                ReverseTravelStatus::Undetermined => "undetermined",
            });
            Ok(response)
        }
    }
}
