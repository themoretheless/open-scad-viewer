//! Distance, trimming, reparameterization, resource probes, sculpting, Coons, and continuity.
use super::*;

pub(super) fn try_dispatch(v: &Value, op: &str) -> Option<Result<Value>> {
    if let Some(res) = try_dispatch_distance_trim_and_probes(v, op) {
        return Some(res);
    }
    try_dispatch_sculpt_coons_and_continuity(v, op)
}

fn differential_side(v: &Value, key: &str) -> Result<crate::curve_differential::Side> {
    use crate::curve_differential::Side;
    match optional_field::<String>(v,key)?.as_deref().unwrap_or("automatic") {
        "automatic" => Ok(Side::Automatic),
        "left" => Ok(Side::Left),
        "right" => Ok(Side::Right),
        _ => Err(input("Unknown differential side")),
    }
}

fn try_dispatch_distance_trim_and_probes(v: &Value, op: &str) -> Option<Result<Value>> {
    let res = match op {
        "curve_join_continuity" => (|| {
            use crate::curve_join::{inspect,Endpoint,Tolerances};
            let endpoint = |key| match field::<String>(v,key)?.as_str() {
                "start" => Ok(Endpoint::Start), "end" => Ok(Endpoint::End), _ => Err(input("Invalid curve endpoint")),
            };
            let r = inspect(&field(v,"curve")?, endpoint("source_end")?, &field(v,"target")?, endpoint("target_end")?,
                Tolerances {position:field(v,"tolerance_mm")?, tangent:field(v,"tangent_tolerance")?, curvature:field(v,"curvature_tolerance_per_mm")?})?;
            Ok(json!({"g0":format!("{:?}",r.g0),"g1":format!("{:?}",r.g1),"g2":format!("{:?}",r.g2),
                "positionErrorMm":r.position_error,"tangentError":r.tangent_error,"curvatureErrorPerMm":r.curvature_error}))
        })(),
        "curve_self_contacts" => (|| {
            use crate::curve_self_intersection::{inspect, Status};
            let r = inspect(&field(v,"curve")?, field(v,"tolerance_mm")?, field(v,"max_cells")?, field(v,"max_pairs")?)?;
            let status = match r.status { Status::Absent => "absent", Status::Present => "present", Status::Unresolved => "unresolved" };
            Ok(json!({"status":status,"axis":r.axis,"cells":r.cells,"pairs":r.pairs,
                "witnessSpans":r.witness_spans,"witnessPoint":r.witness_point}))
        })(),
        "surface_span_regularity" => (|| {
            let r = crate::surface_regularity::inspect(&field(v,"surface")?, field(v,"max_cells")?)?;
            Ok(json!({"spanwiseRegular":r.spanwise_regular,"interiorBasisC1":r.interior_basis_c1,
                "cells":r.cells,"provenCells":r.proven_cells,"unresolved":r.unresolved.iter().map(|c|c.domain).collect::<Vec<_>>()}))
        })(),
        "curve_curvature_torsion" => (|| {
            let r = crate::curve_differential::at(&field(v,"curve")?, field(v,"parameter")?, differential_side(v,"query_side")?)?;
            Ok(json!({"parameter":r.parameter,"side":format!("{:?}",r.side),"continuity":r.continuity,
                "curvaturePerMm":r.curvature,"torsionPerMm":r.torsion,"curvatureStatus":format!("{:?}",r.curvature_status),
                "torsionStatus":format!("{:?}",r.torsion_status),"frameStatus":format!("{:?}",r.frame_status),
                "tangent":r.tangent,"normal":r.normal,"binormal":r.binormal}))
        })(),
        "surface_principal_curvatures" => (|| {
            let r = crate::surface_differential::at(&field(v,"surface")?, field(v,"uv")?,
                [differential_side(v,"u_side")?, differential_side(v,"v_side")?])?;
            Ok(json!({"status":format!("{:?}",r.status),"directionStatus":format!("{:?}",r.direction_status),
                "principalPerMm":r.principal,"meanPerMm":r.mean,"gaussianPerMm2":r.gaussian,"directions":r.directions}))
        })(),
        "curve_nearest_point" => (|| {
            let point: Vec<f64> = field(v, "point")?;
            let r = curve_distance::nearest_point(&field(v,"geometry")?, &point, field(v,"toleranceMm")?, field(v,"maxCells")?)?;
            Ok(json!({"parameter":r.parameter,"point":r.point,"pointEnclosure":r.point_enclosure,
                "distanceIntervalMm":r.distance_interval,"converged":r.converged,"reason":r.reason,"cells":r.cells}))
        })(),
        "surface_nearest_point" => (|| {
            let r = surface_distance::nearest_point(&field(v,"geometry")?, field(v,"point")?, field(v,"toleranceMm")?, field(v,"maxCells")?)?;
            Ok(json!({"parameters":r.parameters,"point":r.point,"pointEnclosure":r.point_enclosure,
                "distanceIntervalMm":r.distance_interval,"converged":r.converged,"reason":r.reason,"cells":r.cells}))
        })(),
        "curve_surface_distance" => (|| {
            let r = crate::curve_surface_distance::distance(&field(v,"a")?, &field(v,"b")?, field(v,"toleranceMm")?, field(v,"maxCells")?)?;
            Ok(json!({"curveParameter":r.curve_parameter,"surfaceParameters":r.surface_parameters,"points":r.points,
                "pointEnclosures":r.point_enclosures,"distanceIntervalMm":r.distance_interval,
                "converged":r.converged,"reason":r.reason,"cells":r.cells}))
        })(),
        "surface_distance" => (|| {
            Ok(surface_distance::distance(
                &field(v, "a")?,
                &field(v, "b")?,
                field(v, "toleranceMm")?,
                field(v, "maxCells")?,
            )?
            .to_value())
        })(),
        "curve_distance" => (|| {
            Ok(curve_distance::distance(
                &field(v, "a")?,
                &field(v, "b")?,
                field(v, "toleranceMm")?,
                field(v, "maxCells")?,
            )?
            .to_value())
        })(),
        "curve_trim_screen_point" => (|| {
            trim_point::trim_at_screen_point(
                &field(v, "curve")?,
                &field::<Vec<f64>>(v, "point")?,
                &field(v, "matrix")?,
                &field::<String>(v, "keep")?,
                field(v, "radius")?,
            )
        })(),
        "curve_trim_point" => (|| {
            trim_point::trim_at_point(
                &field(v, "curve")?,
                &field::<Vec<f64>>(v, "point")?,
                &field::<String>(v, "keep")?,
                field(v, "maxDistance")?,
            )
        })(),
        "curve_reparameterize_exact" => (|| {
            foundation::reparameterize_curve(
                &field(v, "curve")?,
                field(v, "domain")?,
                optional_field(v, "tolerance")?,
            )
        })(),
        "curve_reparameterized_evaluate" => (|| {
            foundation::evaluate_reparameterized_curve(
                &field(v, "curve")?,
                &v["mapping"],
                field(v, "u")?,
                optional_field(v, "tolerance")?,
            )
        })(),
        "curve_materialize_reparameterization_bounded" => (|| {
            foundation::materialize_reparameterized_curve_bounded(
                &field(v, "curve")?,
                &v["mapping"],
                field(v, "errorBudget")?,
                field(v, "maxCells")?,
                field(v, "maxMapEvaluations")?,
            )
        })(),
        "curve_materialize_reparameterization" => (|| {
            foundation::materialize_reparameterized_curve(
                &field(v, "curve")?,
                &v["mapping"],
                optional_field(v, "tolerance")?,
            )
        })(),
        "surface_surface_verify_coverage" => ss_intersection::verify_ss_coverage(&v["report"]),
        "nurbs_intersection_resource_probe" => (|| {
            intersection::resource_boundary_probe(field(v, "degree")?, field(v, "controls")?)
        })(),
        "nurbs_ss_resource_probe" => (|| {
            ss_intersection::ss_resource_probe(
                field(v, "degreeU")?,
                field(v, "degreeV")?,
                field(v, "controls")?,
            )
        })(),
        _ => return None,
    };
    Some(res)
}

fn try_dispatch_sculpt_coons_and_continuity(v: &Value, op: &str) -> Option<Result<Value>> {
    let res = match op {
        "nurbs_deform_curve" => (|| {
            encode(edit::deform_curve(
                &field(v, "curve")?,
                &field(v, "deformation")?,
            )?)
        })(),
        "nurbs_deform_surface" => (|| {
            encode(edit::deform_surface(
                &field(v, "surface")?,
                &field(v, "deformation")?,
            )?)
        })(),
        "nurbs_brush_curve" => {
            (|| encode(edit::brush_curve(&field(v, "curve")?, &field(v, "brush")?)?))()
        }
        "nurbs_sculpt_curve" => (|| {
            encode(edit::sculpt_curve(
                &field(v, "curve")?,
                &field(v, "brush")?,
            )?)
        })(),
        "nurbs_sculpt_surface" => (|| {
            encode(edit::sculpt_surface(
                &field(v, "surface")?,
                &field(v, "brush")?,
            )?)
        })(),
        "nurbs_brush_surface" => (|| {
            encode(edit::brush_surface(
                &field(v, "surface")?,
                &field(v, "brush")?,
            )?)
        })(),
        "basis" => (|| {
            encode(curve::basis(
                field(v, "degree")?,
                &field::<Vec<f64>>(v, "knots")?,
                field(v, "controlCount")?,
                field(v, "u")?,
                v["periodic"].as_bool().unwrap_or(false),
            )?)
        })(),
        "curve_prepare_coons_weights" => (|| {
            let result =
                coons::checked_endpoint_weights(&field(v, "curve")?, field(v, "maxError")?)?;
            Ok(json!({
                "curve": result.curve,
                "report": {
                    "accepted": result.accepted,
                    "errorUpper": result.error_upper,
                    "budget": field::<f64>(v, "maxError")?,
                    "wholeCurve": true,
                    "method": "outward-homogeneous-Bernstein-difference"
                }
            }))
        })(),
        "surface_coons_patch" => {
            (|| encode(coons::patch(&field::<Vec<curve::Curve>>(v, "boundaries")?)?))()
        }
        "curve_match_g1" => (|| {
            continuity::curve_match::checked(
                &field(v, "reference")?,
                &field(v, "edited")?,
                &field::<String>(v, "referenceEnd")?,
                &field::<String>(v, "editedEnd")?,
                optional_field::<f64>(v, "maxAngleDegrees")?.unwrap_or(1e-6),
            )
        })(),
        "surface_prepare_seams" => (|| {
            continuity::preparation::checked_with_conversion(
                &field(v, "reference")?,
                &field(v, "edited")?,
                &field::<String>(v, "referenceBoundary")?,
                &field::<String>(v, "editedBoundary")?,
                optional_field::<bool>(v, "reverse")?.unwrap_or(false),
                field(v, "maxError")?,
                optional_field::<bool>(v, "openPeriodic")?.unwrap_or(false),
            )
        })(),
        "surface_match_jets" => (|| {
            continuity::match_surface_jets_checked(
                &field(v, "reference")?,
                &field(v, "edited")?,
                &field::<String>(v, "referenceBoundary")?,
                &field::<String>(v, "editedBoundary")?,
                continuity::JetMatchOptions {
                    order: field(v, "order")?,
                    scale: field(v, "scale")?,
                    reverse: optional_field::<bool>(v, "reverse")?.unwrap_or(false),
                    max_error: optional_field::<f64>(v, "maxError")?.unwrap_or(1e-6),
                },
            )
        })(),
        _ => return None,
    };
    Some(res)
}
