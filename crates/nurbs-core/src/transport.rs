//! Optional bounded JSON request boundary.
use super::*;
use value_codec::{Deserialize, Serialize, Value, json};

fn field<T: for<'a> Deserialize<'a>>(v: &Value, k: &str) -> Result<T> {
    value_codec::from_value(v[k].clone()).map_err(|e| input(format!("Invalid {k}: {e}")))
}
fn encode<T: Serialize>(v: T) -> Result<Value> {
    value_codec::to_value(v).map_err(|e| numeric_err(e.to_string()))
}
fn optional_field<T: for<'a> Deserialize<'a>>(v: &Value, k: &str) -> Result<Option<T>> {
    match v.get(k) {
        Some(value) if !value.is_null() => value_codec::from_value(value.clone())
            .map(Some)
            .map_err(|e| input(format!("Invalid {k}: {e}"))),
        _ => Ok(None),
    }
}
pub fn dispatch(v: Value) -> Result<Value> {
    let op: String = field(&v, "op")?;
    if op=="curve_compose" {return encode(paths::compose(&field::<Vec<curve::Curve>>(&v,"curves")?)?)}
    if op=="curve_polyline" {return encode(primitives::polyline(&field::<Vec<[f64;3]>>(&v,"points")?,optional_field(&v,"closed")?.unwrap_or(false))?)}
    if op=="curve_bezier" {return encode(paths::bezier(field(&v,"points")?,optional_field(&v,"weights")?)?)}
    if op=="curve_circle_arc" {return encode(primitives::circle_arc(field(&v,"center")?,field(&v,"normal")?,field(&v,"radius")?,field(&v,"startDegrees")?,field(&v,"sweepDegrees")?)?)}
    if op=="curve_circle" {return encode(primitives::circle(field(&v,"center")?,field(&v,"normal")?,field(&v,"radius")?)?)}
    if op=="curve_line" {return encode(primitives::line(field(&v,"start")?,field(&v,"end")?)?)}
    if op == "curve_hermite" { return encode(hermite::interpolate(&field::<Vec<[f64;3]>>(&v,"points")?,&field::<Vec<[f64;3]>>(&v,"tangents")?,&field::<Vec<f64>>(&v,"parameters")?)?); }
    if op == "curve_natural_spline" { return encode(natural_spline::interpolate(&field::<Vec<[f64;3]>>(&v,"points")?,&field::<Vec<f64>>(&v,"parameters")?)?); }
    if op == "curve_clamped_spline" { return encode(natural_spline::clamped(&field::<Vec<[f64;3]>>(&v,"points")?,&field::<Vec<f64>>(&v,"parameters")?,field(&v,"start_tangent")?,field(&v,"end_tangent")?)?); }
    if op == "curve_closed_spline" { return encode(closed_spline::interpolate(&field::<Vec<[f64;3]>>(&v,"points")?,&field::<Vec<f64>>(&v,"parameters")?)?); }
    if op == "surface_closed_loft" { return encode(natural_loft::closed(&field::<Vec<curve::Curve>>(&v,"curves")?,&field::<Vec<f64>>(&v,"parameters")?)?); }
    if op == "surface_auto_guided_loft" || op == "surface_auto_guided_loft_cartesian" {
        let curves = field::<Vec<curve::Curve>>(&v,"curves")?;
        let mapped = optional_field::<Vec<Option<Value>>>(&v,"section_mappings")?
            .map(|m| loft_reparameterization::prepare(&curves,&m)).transpose()?;
        let sections = mapped.as_ref().map_or(curves.as_slice(), |m| m.curves.as_slice());
        let parameters=field::<Vec<f64>>(&v,"parameters")?;
        let guides=field::<Vec<curve::Curve>>(&v,"guides")?;
        let budget=field::<f64>(&v,"budget")?;
        let parameter_tolerance=optional_field::<f64>(&v,"parameter_tolerance")?.unwrap_or(1e-8);
        let result=if op=="surface_auto_guided_loft_cartesian" {
            loft_alignment::interpolate_cartesian_with_parameter_tolerance(sections,&parameters,&guides,
                budget,parameter_tolerance,field(&v,"maxCells")?,field(&v,"maxMapEvaluations")?)?
        } else {
            loft_alignment::interpolate_with_parameter_tolerance(sections,&parameters,&guides,budget,parameter_tolerance)?
        };
        let mut output = json!({"surface":result.surface,"guides":result.guides,"guide_parameters":result.guide_parameters,"guide_order":result.guide_order,"reversed":result.reversed,"section_error_upper":result.section_error_upper,"guide_error_upper":result.guide_error_upper});
        if op=="surface_auto_guided_loft_cartesian" {
            output["certificate"]=json!({"operation":"cartesian-auto-guided-loft","exact":false,
                "fittedToExactPromotion":false,"curves":result.curve_certificates});
        }
        if let Some(mapped) = mapped {
            output["sections"] = value_codec::to_value(mapped.curves).map_err(|e| input(e.to_string()))?;
            output["section_mapping_certificates"] = Value::Array(mapped.certificates);
        }
        return Ok(output);
    }
    if op == "surface_loft_match_ends" {
        let mut references:[Option<surface::Surface>;2]=[None,None];
        let mut boundaries=[String::new(),String::new()];
        let mut orders=[1usize;2];let mut scales=[1.;2];let mut reversed=[false;2];
        for (i,key) in ["start","end"].iter().enumerate(){
            if let Some(value)=v.get(*key).filter(|x|!x.is_null()) {
                references[i]=Some(field(value,"reference")?);boundaries[i]=field(value,"boundary")?;
                orders[i]=field(value,"order")?;scales[i]=field(value,"scale")?;
                reversed[i]=optional_field(value,"reverse")?.unwrap_or(false);
            }
        }
        let ends=std::array::from_fn(|i|references[i].as_ref().map(|reference|loft_continuity::EndConstraint{reference,boundary:&boundaries[i],order:orders[i],scale:scales[i],reverse:reversed[i]}));
        let result=loft_continuity::match_ends(&field(&v,"surface")?,&field::<Vec<curve::Curve>>(&v,"curves")?,&field::<Vec<f64>>(&v,"parameters")?,&optional_field::<Vec<curve::Curve>>(&v,"guides")?.unwrap_or_default(),&optional_field::<Vec<f64>>(&v,"guide_parameters")?.unwrap_or_default(),ends,field(&v,"budget")?)?;
        return Ok(json!({"surface":result.surface,"seams":result.seams,"section_error_upper":result.section_error_upper,"guide_error_upper":result.guide_error_upper}));
    }
    if op == "surface_guided_loft_cartesian" {
        check(v.get("start_tangents").is_none_or(Value::is_null)
            && v.get("end_tangents").is_none_or(Value::is_null),
            "Cartesian guided loft endpoint tangent constraints are not yet qualified")?;
        let (surface, curves)=guided_loft::interpolate_cartesian(
            &field::<Vec<curve::Curve>>(&v,"curves")?, &field::<Vec<f64>>(&v,"parameters")?,
            &field::<Vec<curve::Curve>>(&v,"guides")?, &field::<Vec<f64>>(&v,"guide_parameters")?,
            field::<f64>(&v,"errorBudget")?, field::<usize>(&v,"maxCells")?, field::<usize>(&v,"maxMapEvaluations")?)?;
        return Ok(json!({"surface":surface,"certificate":{"operation":"cartesian-guided-loft",
            "exact":false,"fittedToExactPromotion":false,"curves":curves}}));
    }
    if op == "surface_guided_loft" {
        let start = optional_field::<Vec<[f64; 3]>>(&v, "start_tangents")?;
        let end = optional_field::<Vec<[f64; 3]>>(&v, "end_tangents")?;
        let tangents = match (start, end) {
            (None, None) => None,
            (Some(a), Some(b)) => Some([a,b]),
            _ => return Err(input("Guided loft requires both endpoint tangent fields")),
        };
        return encode(guided_loft::interpolate(
            &field::<Vec<curve::Curve>>(&v,"curves")?,
            &field::<Vec<f64>>(&v,"parameters")?,
            &field::<Vec<curve::Curve>>(&v,"guides")?,
            &field::<Vec<f64>>(&v,"guide_parameters")?, tangents,
        )?);
    }
    if op == "surface_control_tangent_loft" {
        return encode(natural_loft::clamped_control_tangents(
            &field::<Vec<curve::Curve>>(&v, "curves")?,
            &field::<Vec<f64>>(&v, "parameters")?,
            &field::<Vec<[f64; 3]>>(&v, "start_tangents")?,
            &field::<Vec<[f64; 3]>>(&v, "end_tangents")?,
        )?);
    }
    if op == "surface_clamped_loft" { return encode(natural_loft::clamped(&field::<Vec<curve::Curve>>(&v,"curves")?,&field::<Vec<f64>>(&v,"parameters")?,field(&v,"start_tangent")?,field(&v,"end_tangent")?)?); }
    if op == "surface_natural_loft" {
        let curves = field::<Vec<curve::Curve>>(&v,"curves")?;
        let parameters = field::<Vec<f64>>(&v,"parameters")?;
        return encode(match optional_field::<Vec<Option<Value>>>(&v,"section_mappings")? {
            Some(m) => loft_reparameterization::interpolate(&curves,&parameters,&m)?,
            None => natural_loft::interpolate(&curves,&parameters)?,
        });
    }
    if op == "surface_gordon_cartesian" {
        let (surface, curves) = gordon::patch_cartesian(
            &field::<Vec<curve::Curve>>(&v,"u_curves")?, &field::<Vec<curve::Curve>>(&v,"v_curves")?,
            &field::<Vec<f64>>(&v,"parameters_u")?, &field::<Vec<f64>>(&v,"parameters_v")?,
            field::<f64>(&v,"errorBudget")?, field::<usize>(&v,"maxCells")?, field::<usize>(&v,"maxMapEvaluations")?)?;
        return Ok(json!({"surface":surface,"certificate":{"operation":"cartesian-gordon",
            "exact":false,"fittedToExactPromotion":false,"curves":curves}}));
    }
    if op == "surface_gordon" {return encode(gordon::patch(&field::<Vec<curve::Curve>>(&v,"u_curves")?,&field::<Vec<curve::Curve>>(&v,"v_curves")?,&field::<Vec<f64>>(&v,"parameters_u")?,&field::<Vec<f64>>(&v,"parameters_v")?)?);}
    if op == "surface_grid_spline" { return encode(grid_spline::interpolate(&field::<Vec<Vec<[f64;3]>>>(&v,"points")?,&field::<Vec<f64>>(&v,"parameters_u")?,&field::<Vec<f64>>(&v,"parameters_v")?)?); }

    if op == "curve_chain_diagnostics" {
        return Ok(crate::curve_offset_diagnostics::inspect_curves(&field::<Vec<curve::Curve>>(&v,"curves")?, optional_field::<usize>(&v,"maxPairs")?.unwrap_or(1_000_000))?.to_value());
    }
    if op == "curve_offset_trimmed_bevel" {
        let wire=curve_offset_wire::bevel_wire(&field(&v,"curve")?,field(&v,"distance")?,field(&v,"toleranceMm")?,field(&v,"maxCells")?)?;
        crate::check(wire.closed, "Trimmed offset requires a closed source.")?;
        let rule=match field::<String>(&v,"fillRule")?.as_str() {
            "nonzero" => crate::chord_winding::FillRule::NonZero,
            "evenodd" => crate::chord_winding::FillRule::EvenOdd,
            _ => { crate::check(false,"Choose nonzero or evenodd fill.")?; unreachable!() }
        };
        let graph=crate::chord_arrangement::split(&wire.edges,true,field(&v,"intersectionToleranceMm")?,optional_field::<usize>(&v,"maxPairs")?.unwrap_or(1_000_000))?;
        let loops=crate::chord_fill_selection::boundary_curves(&graph,rule,optional_field::<usize>(&v,"maxWitnessChecks")?.unwrap_or(1_000_000))?;
        let intersection_error=graph.vertices.iter().map(|p|p.error_upper_mm).fold(0.,f64::max);
        return Ok(json!({"loops":encode(loops)?,"report":{
            "accepted":true,"method":"represented-bevel-offset-fill/1",
            "fillRule":v["fillRule"].clone(),"regionTrimmed":true,
            "topologyScope":"represented-reconstructed-chord-graph",
            "regionTopologyCertified":false,"originalOffsetTopologyCertified":false,
            "sourceWireErrorUpperMm":wire.error_upper_mm,
            "intersectionConstructionErrorUpperMm":intersection_error
        }}));
    }
    if op == "curve_offset_bevel_wire" {
        let result=curve_offset_wire::bevel_wire(&field(&v,"curve")?,field(&v,"distance")?,field(&v,"toleranceMm")?,field(&v,"maxCells")?)?;
        let diagnostics=crate::curve_offset_diagnostics::inspect_chain(&result.edges,result.closed,optional_field::<usize>(&v,"maxPairs")?.unwrap_or(1_000_000))?.to_value();
        let cells:Vec<_>=result.edges.iter().zip(&result.roles).map(|(edge,role)|{
            let source=match role {curve_offset_wire::Role::Source(domain)=>json!({"kind":"source-offset","domain":domain}),curve_offset_wire::Role::Bevel(knot)=>json!({"kind":"bevel","sourceKnot":knot})};
            json!({"domain":edge.domain,"stationDomain":edge.domain,"source":source,"errorUpperMm":edge.error_upper_mm})
        }).collect();
        return Ok(json!({"curves":encode(result.curves)?,"report":{"accepted":true,"closed":result.closed,"wholeWire":true,"wholeCurve":false,"method":"outward-source-offset-bevel-wire/1","errorUpperMm":result.error_upper_mm,"toleranceMm":v["toleranceMm"].clone(),"cells":cells,"chainDiagnostics":diagnostics,"offsetRegularityCertified":false,"regionTopologyCertified":false,"regionTrimmed":false}}));
    }
    if op == "curve_offset_bounded" {
        let result = curve_offset::approximate_curve(&field(&v,"curve")?,field(&v,"distance")?,
            field(&v,"toleranceMm")?,field(&v,"maxCells")?)?;
        let cells: Vec<_> = result.segments.iter().map(|s|json!({
            "domain":s.domain,"errorUpperMm":s.error_upper_mm
        })).collect();
        let diagnostics = if result.segments.is_empty() {Value::Null} else {
            crate::curve_offset_diagnostics::inspect_chain(&result.segments,result.closed,
                optional_field::<usize>(&v,"maxPairs")?.unwrap_or(1_000_000))?.to_value()
        };
        return Ok(json!({"curves":encode(result.curves)?,"report":{
            "closed":result.closed,
            "accepted":true,"errorUpperMm":result.error_upper_mm,
            "toleranceMm":v["toleranceMm"].clone(),"wholeCurve":true,
            "method":"outward-rational-jets-chord-bound/1","cells":cells,
            "chainDiagnostics":diagnostics,
            "offsetRegularityCertified":false,
            "regionTopologyCertified":false
        }}));
    }
    if op == "curve_certify_foundation" {
        return foundation::certify_curve(&field(&v, "curve")?, optional_field(&v, "tolerance")?);
    }
    if op == "surface_certify_foundation" {
        return foundation::certify_surface(
            &field(&v, "surface")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "surface_distance" {
        return Ok(surface_distance::distance(
            &field(&v, "a")?,
            &field(&v, "b")?,
            field(&v, "toleranceMm")?,
            field(&v, "maxCells")?,
        )?
        .to_value());
    }
    if op == "curve_distance" {
        return Ok(curve_distance::distance(
            &field(&v, "a")?,
            &field(&v, "b")?,
            field(&v, "toleranceMm")?,
            field(&v, "maxCells")?,
        )?
        .to_value());
    }
    if op == "curve_trim_screen_point" {
        return trim_point::trim_at_screen_point(
            &field(&v, "curve")?,
            &field::<Vec<f64>>(&v, "point")?,
            &field(&v, "matrix")?,
            &field::<String>(&v, "keep")?,
            field(&v, "radius")?,
        );
    }
    if op == "curve_trim_point" {
        return trim_point::trim_at_point(
            &field(&v, "curve")?,
            &field::<Vec<f64>>(&v, "point")?,
            &field::<String>(&v, "keep")?,
            field(&v, "maxDistance")?,
        );
    }
    if op == "curve_project_certified" {
        return foundation::project_curve(
            &field(&v, "curve")?,
            &field::<Vec<f64>>(&v, "point")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "surface_project_certified" {
        return foundation::project_surface(
            &field(&v, "surface")?,
            field(&v, "point")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_interpolate_certified" {
        return foundation::interpolate_polyline(
            field(&v, "points")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_approximate_certified" {
        return foundation::approximate_curve(
            &field(&v, "curve")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_reparameterize_exact" {
        return foundation::reparameterize_curve(
            &field(&v, "curve")?,
            field(&v, "domain")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_insert_certified" {
        return foundation::certify_exact_edit(
            &field(&v, "curve")?,
            "insert",
            field(&v, "u")?,
            field(&v, "count")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_elevate_certified" {
        return foundation::certify_exact_edit(
            &field(&v, "curve")?,
            "elevate",
            0.,
            field(&v, "degree")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_remove_certified" {
        return foundation::remove_curve_knot(
            &field(&v, "curve")?,
            field(&v, "u")?,
            field(&v, "maxError")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_rebuild_certified" {
        return foundation::rebuild_curve(
            &field(&v, "curve")?,
            field(&v, "degree")?,
            field(&v, "controlCount")?,
            field(&v, "maxError")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_reduce_certified" {
        return foundation::reduce_curve_degree(
            &field(&v, "curve")?,
            field(&v, "degree")?,
            field(&v, "maxError")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "surface_rebuild_certified" {
        return foundation::rebuild_surface(
            &field(&v, "surface")?,
            field(&v, "axis")?,
            field(&v, "degree")?,
            field(&v, "controlCount")?,
            field(&v, "maxError")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "surface_remove_certified" || op == "surface_reduce_certified" {
        return foundation::reduce_surface_axis(
            &field(&v, "surface")?,
            field(&v, "axis")?,
            if op == "surface_remove_certified" {
                "remove"
            } else {
                "reduce"
            },
            v.get("u").and_then(Value::as_f64).unwrap_or(0.),
            v.get("degree").and_then(Value::as_u64).unwrap_or(1) as usize,
            field(&v, "maxError")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_periodic_edit_certified" {
        let operation: String = field(&v, "operation")?;
        return foundation::edit_periodic_curve(
            &field(&v, "curve")?,
            &operation,
            v.get("u").and_then(Value::as_f64).unwrap_or(0.),
            v.get("degree").and_then(Value::as_u64).unwrap_or(1) as usize,
            field(&v, "maxError")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_periodic_split_certified" {
        return foundation::split_periodic_curve(
            &field(&v, "curve")?,
            field(&v, "u")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "surface_periodic_edit_certified" {
        let operation: String = field(&v, "operation")?;
        return foundation::edit_periodic_surface(
            &field(&v, "surface")?,
            field(&v, "axis")?,
            &operation,
            v.get("u").and_then(Value::as_f64).unwrap_or(0.),
            v.get("degree").and_then(Value::as_u64).unwrap_or(1) as usize,
            field(&v, "maxError")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_reparameterization_certify" {
        return foundation::certify_reparameterization(
            &v["mapping"],
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_reparameterized_evaluate" {
        return foundation::evaluate_reparameterized_curve(
            &field(&v, "curve")?,
            &v["mapping"],
            field(&v, "u")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_fit_certified" {
        return foundation::fit_curve_points(
            field(&v, "points")?,
            field(&v, "controlCount")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "surface_interpolate_certified" || op == "surface_fit_certified" {
        return foundation::interpolate_surface_grid(
            field(&v, "points")?,
            op == "surface_fit_certified",
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_materialize_reparameterization_bounded" {
        return foundation::materialize_reparameterized_curve_bounded(
            &field(&v, "curve")?, &v["mapping"], field(&v, "errorBudget")?,
            field(&v, "maxCells")?, field(&v, "maxMapEvaluations")?,
        );
    }
    if op == "curve_materialize_reparameterization" {
        return foundation::materialize_reparameterized_curve(
            &field(&v, "curve")?,
            &v["mapping"],
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_fit_cloud_certified" {
        return foundation::fit_curve_cloud_certified(
            field(&v, "points")?,
            field(&v, "controlCount")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "surface_fit_cloud_certified" {
        return foundation::fit_surface_cloud_certified(
            field(&v, "points")?,
            field(&v, "controlsU")?,
            field(&v, "controlsV")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_curve_intersect_certified" {
        return intersection::intersect_curve_curve(
            &field(&v, "first")?,
            &field(&v, "second")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_surface_intersect_certified" {
        return intersection::intersect_curve_surface(
            &field(&v, "curve")?,
            &field(&v, "surface")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "surface_surface_intersect_certified" {
        return ss_intersection::intersect_surface_surface(
            &field(&v, "first")?,
            &field(&v, "second")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "surface_surface_verify_coverage" {
        return ss_intersection::verify_ss_coverage(&v["report"]);
    }
    if op == "nurbs_intersection_resource_probe" {
        return intersection::resource_boundary_probe(field(&v, "degree")?, field(&v, "controls")?);
    }
    if op == "nurbs_ss_resource_probe" {
        return ss_intersection::ss_resource_probe(
            field(&v, "degreeU")?,
            field(&v, "degreeV")?,
            field(&v, "controls")?,
        );
    }
    if op == "nurbs_deform_curve" {
        return encode(edit::deform_curve(
            &field(&v, "curve")?,
            &field(&v, "deformation")?,
        )?);
    }
    if op == "nurbs_deform_surface" {
        return encode(edit::deform_surface(
            &field(&v, "surface")?,
            &field(&v, "deformation")?,
        )?);
    }
    if op == "nurbs_brush_curve" {
        return encode(edit::brush_curve(
            &field(&v, "curve")?,
            &field(&v, "brush")?,
        )?);
    }
    if op == "nurbs_sculpt_curve" {
        return encode(edit::sculpt_curve(
            &field(&v, "curve")?,
            &field(&v, "brush")?,
        )?);
    }
    if op == "nurbs_sculpt_surface" {
        return encode(edit::sculpt_surface(
            &field(&v, "surface")?,
            &field(&v, "brush")?,
        )?);
    }
    if op == "nurbs_brush_surface" {
        return encode(edit::brush_surface(
            &field(&v, "surface")?,
            &field(&v, "brush")?,
        )?);
    }
    if op == "basis" {
        return encode(curve::basis(
            field(&v, "degree")?,
            &field::<Vec<f64>>(&v, "knots")?,
            field(&v, "controlCount")?,
            field(&v, "u")?,
            v["periodic"].as_bool().unwrap_or(false),
        )?);
    }
    if op == "curve_prepare_coons_weights" {
        let result=coons::checked_endpoint_weights(&field(&v,"curve")?,field(&v,"maxError")?)?;
        return Ok(json!({"curve":result.curve,"report":{"accepted":result.accepted,
            "errorUpper":result.error_upper,"budget":field::<f64>(&v,"maxError")?,
            "wholeCurve":true,"method":"outward-homogeneous-Bernstein-difference"}}));
    }
    if op == "surface_coons_patch" {
        return encode(coons::patch(&field::<Vec<curve::Curve>>(
            &v,
            "boundaries",
        )?)?);
    }
    if op == "curve_match_g1" {
        return continuity::curve_match::checked(
            &field(&v, "reference")?,
            &field(&v, "edited")?,
            &field::<String>(&v, "referenceEnd")?,
            &field::<String>(&v, "editedEnd")?,
            optional_field::<f64>(&v, "maxAngleDegrees")?.unwrap_or(1e-6),
        );
    }
    if op == "surface_prepare_seams" {
        return continuity::preparation::checked_with_conversion(
            &field(&v, "reference")?,
            &field(&v, "edited")?,
            &field::<String>(&v, "referenceBoundary")?,
            &field::<String>(&v, "editedBoundary")?,
            optional_field::<bool>(&v, "reverse")?.unwrap_or(false),
            field(&v, "maxError")?,
            optional_field::<bool>(&v, "openPeriodic")?.unwrap_or(false),
        );
    }
    if op == "surface_match_jets" {
        return continuity::match_surface_jets_checked(
            &field(&v, "reference")?,
            &field(&v, "edited")?,
            &field::<String>(&v, "referenceBoundary")?,
            &field::<String>(&v, "editedBoundary")?,
            field(&v, "order")?,
            field(&v, "scale")?,
            optional_field::<bool>(&v, "reverse")?.unwrap_or(false),
            optional_field::<f64>(&v, "maxError")?.unwrap_or(1e-6),
        );
    }
    if op == "surface_framed_sweep" {
        return framed_sweep::checked_sweep(
            &field(&v, "profile")?,
            &field(&v, "path")?,
            field(&v, "normal")?,
            field(&v, "sections")?,
            field(&v, "maxDeviation")?,
        );
    }
    if op == "surface_sweep" {
        return encode(surface::sweep(&field(&v, "profile")?, &field(&v, "path")?)?);
    }
    if op == "loft_aligned" {
        return encode(surface::loft_aligned(&field::<Vec<curve::Curve>>(
            &v, "curves",
        )?)?);
    }
    if op == "loft" {
        return encode(surface::loft(&field::<Vec<curve::Curve>>(&v, "curves")?)?);
    }
    if op.starts_with("curve_") || op == "extrude" || op == "revolve" {
        let c: curve::Curve = field(&v, "curve")?;
        return match op.as_str() {
            "curve_validate" => {
                c.validate()?;
                Ok(Value::Null)
            }
            "curve_evaluate" => encode(c.evaluate(field(&v, "u")?)?),
            "curve_insert" => encode(c.insert(field(&v, "u")?, field(&v, "count")?)?),
            "curve_trim" => encode(c.trim(field(&v, "a")?, field(&v, "b")?)?),
            "curve_split" => encode(c.split(field(&v, "u")?)?),
            "curve_reverse" => encode(c.reverse()?),
            "curve_elevate" => encode(c.elevate(field(&v, "degree")?)?),
            "curve_decompose" => encode(c.decompose()?),
            "curve_bounds" => c.bounds(),
            "extrude" => encode(surface::extrude(&c, field(&v, "vector")?)?),
            "revolve" => encode(surface::revolve(
                &c,
                field(&v, "origin")?,
                field(&v, "axis")?,
                field(&v, "angle")?,
            )?),
            _ => Err(input("Unknown curve operation")),
        };
    }
    let s: surface::Surface = field(&v, "surface")?;
    match op.as_str() {
        "surface_validate" => {
            s.validate()?;
            Ok(Value::Null)
        }
        "surface_evaluate" => encode(s.evaluate(field(&v, "u")?, field(&v, "v")?)?),
        "surface_insert" => encode(s.edit_axis(field(&v, "axis")?, |c| {
            c.insert(field(&v, "u")?, field(&v, "count")?)
        })?),
        "surface_elevate" => {
            encode(s.edit_axis(field(&v, "axis")?, |c| c.elevate(field(&v, "degree")?))?)
        }
        "surface_reverse" => encode(s.edit_axis(field(&v, "axis")?, |c| c.reverse())?),
        "surface_trim" => encode(s.trim(field(&v, "bounds")?)?),
        "surface_iso" => encode(s.iso(field(&v, "axis")?, field(&v, "u")?)?),
        "surface_bounds" => s.bounds(),
        _ => Err(input("Unknown NURBS operation")),
    }
}
fn error_value(error: &Error) -> Value {
    json!({"code": error.code, "message": error.message})
}
fn response(result: Result<Value>) -> String {
    match result {
        Ok(value) => json!({"ok":true,"value":value}),
        Err(error) => json!({"ok":false,"error": error_value(&error)}),
    }
    .to_string()
}
/// Versioned, bounded JSON boundary; no host geometry fallback.
pub fn execute(request: &str) -> String {
    if request.len() > 2 * 1024 * 1024 {
        return response(Err(resource("NURBS request exceeds 2 MiB")));
    }
    response(
        value_codec::from_str(request)
            .map_err(|e| input(e.to_string()))
            .and_then(dispatch),
    )
}
