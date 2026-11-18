//! Bridge operations of the `nurbs` family.
use crate::*;

const OPS: &[&str] = &[
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
        "nurbs_measure_curve_directional_travel" => curve_directional_travel_request(&v),
        "nurbs_measure_curve_directional_travel_batch" => curve_directional_travel_batch_request(&v),
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
    let report = directional_travel_on_interval(&curve, interval, &direction, tolerance, max_cells)?;
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
    let reports = directional_travel_on_intervals(&curve, &intervals, &direction, tolerance, max_cells)?;
    let cells = reports.iter().map(|report| report.total.cells).sum::<usize>();
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
    use nurbs_core::analysis::curve_measure::{ReverseTravelStatus, StopReason};
    let reverse_status = reverse_limit
        .map(|limit| report.reverse_status(limit))
        .transpose()?
        .map(|status| match status {
            ReverseTravelStatus::Within => "within",
            ReverseTravelStatus::Exceeds => "exceeds",
            ReverseTravelStatus::Undetermined => "undetermined",
        });
    let stop_reason = match report.total.stop_reason {
        StopReason::Tolerance => "tolerance",
        StopReason::WorkLimit => "workLimit",
        StopReason::PrecisionLimit => "precisionLimit",
        StopReason::IterationLimit => "iterationLimit",
        StopReason::TargetUncertainty => "targetUncertainty",
    };
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
