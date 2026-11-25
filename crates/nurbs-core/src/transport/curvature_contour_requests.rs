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
