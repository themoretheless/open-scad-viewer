//! Optional bounded JSON request boundary.
mod sweep_requests;
mod loft_requests;
mod certificate_requests;
use super::*;
use crate::surface::Surface;
#[path="retained_body_coverage.rs"]
mod retained_body_coverage;
use crate::retained_wall_domain;
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
fn control_tangents(v: &Value) -> Result<Option<[Vec<[f64; 3]>; 2]>> {
    let start = optional_field(v, "start_tangents")?;
    let end = optional_field(v, "end_tangents")?;
    let controls = match (start, end) {
        (None, None) => None,
        (Some(a), Some(b)) => Some([a, b]),
        _ => return Err(input("Cartesian loft requires both endpoint tangent fields")),
    };
    check(controls.is_none() || v.get("boundary_tangents").is_none_or(Value::is_null),
        "Specify authored control tangents or normalized boundary fields, not both")?;
    Ok(controls)
}
pub fn dispatch(v: Value) -> Result<Value> {
    let op: String = field(&v, "op")?;
    if op == "sweep_filled_cap_error_upper" {
        let p=sweeps::filled_cap_error::Premises {
            ideal_domains_certified:field(&v,"idealCapDomainsCertified")?,
            retained_regions_exact:field(&v,"retainedCapRegionsExact")?,
            projection_normal_dots:optional_field(&v,"projectionNormalDots")?,
            endpoint_error:optional_field(&v,"endpointContourErrorUpper")?,
            correction:optional_field(&v,"correctionDisplacementUpper")?,
            decomposition:optional_field(&v,"decompositionErrorUpper")?,
            parallel_planes:optional_field(&v,"parallelPlanesCertified")?.unwrap_or([false;2]),
        };
        return Ok(json!({"capErrorUpper":sweeps::filled_cap_error::filled_caps(&p),"continuousBound":false}));
    }
    if op == "sweep_boundary_certificate" {
        let closed=field::<bool>(&v,"closed")?;
        let wall=v.get("wall").and_then(Value::as_f64);
        let budget=v.get("budget").and_then(Value::as_f64);
        let caps=v.get("caps").and_then(Value::as_array).filter(|a|a.len()==2)
            .and_then(|a|Some([a[0].as_f64()?,a[1].as_f64()?]));
        let r=sweeps::filled_cap_error::compose_boundary(wall,caps,closed,budget);
        return Ok(json!({"method":"retained-sweep-boundary-union","scope":"boundary-set-hausdorff",
            "continuousBound":r.continuous_bound,"withinBudget":r.within_budget,"errorUpper":r.error_upper,
            "budget":budget,"closed":closed,"wallErrorUpper":r.wall_error_upper,
            "filledCapErrorUpper":r.filled_cap_error_upper,"reason":r.reason}));
    }
    if op == "sweep_boundary_error_upper" {
        let bound=sweeps::filled_cap_error::boundary(optional_field(&v,"wall")?,optional_field(&v,"caps")?,field(&v,"closed")?);
        return Ok(json!({"boundaryErrorUpper":bound,"continuousBound":false}));
    }

    if op == "sweep_retained_wall_domain_audit" { return retained_wall_domain::inspect(&v); }
    if op == "sweep_retained_body_coverage_audit" {
        let model=field::<retained_body_coverage::Model>(&v,"model")?;
        let covered=retained_body_coverage::covers(&model,field(&v,"maxFaces")?);
        return Ok(json!({"faceCoverageCertified":covered,"globalEmbeddingCertified":false}));
    }
    if op == "curve_decomposition_batch_audit" {
        let budget=field::<f64>(&v,"maxProducts")?;
        if !budget.is_finite() || budget<0. || budget>1000000. || budget.fract()!=0. {
            return Ok(json!({"errorUpper":null,"products":0,"pairsInspected":0,"reason":"work-limit",
                "method":"original-span-bernstein-decomposition","continuousBound":false}));
        }
        let declarations=field::<Vec<Value>>(&v,"pairs")?;
        let pairs=declarations.iter().map(|p|Ok((field::<curve::Curve>(p,"curve")?,field::<usize>(p,"span")?,field::<curve::Curve>(p,"retained")?)))
            .collect::<Result<Vec<_>>>()?;
        let refs=pairs.iter().map(|(a,s,b)|(a,*s,b)).collect::<Vec<_>>();
        let r=crate::curve_decomposition_certificate::inspect_batch(&refs,budget as usize)?;
        return Ok(json!({"errorUpper":r.error_upper,"products":r.products,"pairsInspected":r.pairs_inspected,
            "reason":r.reason,"method":"original-span-bernstein-decomposition","continuousBound":false}));
    }
    if op == "curve_segmented_bezier_controls" {
        return encode(retained_wall_coefficients::segmented_bezier_controls(
            &field::<curve::Curve>(&v,"curve")?,field(&v,"maxControlRows")?));
    }
    if op == "sweep_retained_cap_contour_audit" {
        let (exact,work)=retained_wall_coefficients::contour_matches(
            &field::<Vec<curve::Curve>>(&v,"expected")?,&field::<Vec<curve::Curve>>(&v,"actual")?,
            &field::<Vec<bool>>(&v,"reversed")?,field(&v,"maxWork")?);
        return Ok(json!({"contourIdentity":exact,"work":work,"filledRegionCertified":false}));
    }
    if op == "sweep_retained_wall_family_audit" {
        let exact=retained_wall_coefficients::family_matches(
            &field::<Vec<surface::Surface>>(&v,"surfaces")?,
            &field::<Vec<Vec<Vec<curve::Curve>>>>(&v,"sections")?,field(&v,"closed")?,field(&v,"maxFaces")?);
        return Ok(json!({"coefficientFamilyIdentity":exact,"globalEmbeddingCertified":false}));
    }
    if op == "sweep_retained_wall_coefficients_audit" {
        let exact=retained_wall_coefficients::matches(&field::<surface::Surface>(&v,"surface")?,
            &field::<curve::Curve>(&v,"start")?,&field::<curve::Curve>(&v,"end")?,field(&v,"maxControls")?);
        return Ok(json!({"coefficientIdentity":exact,"globalEmbeddingCertified":false}));
    }
    if op == "surface_station_normal_scale" {
        return encode(continuity::propose_station_normal_scale(
            &field::<surface::Surface>(&v,"reference")?, &field::<surface::Surface>(&v,"edited")?,
            &field::<String>(&v,"referenceBoundary")?, &field::<String>(&v,"editedBoundary")?)?);
    }
    if op == "surface_projective_strip_jets_audit" {
        let order:usize=field(&v,"order")?;
        let report=continuity::inspect_surface_projective_strip_jets(
            &field::<surface::Surface>(&v,"reference")?,&field::<surface::Surface>(&v,"edited")?,
            &field::<String>(&v,"referenceBoundary")?,&field::<String>(&v,"editedBoundary")?,
            order,field(&v,"normalScale")?,field(&v,"maxWork")?)?;
        return Ok(json!({"certified":report.certified,"exactIdentity":report.exact_identity,
            "regularityCertified":report.regularity_certified,"work":report.work,"reason":report.reason,
            "method":"constant-projective-strip-jets","certifiedOrder":if report.certified {Some(order)}else{None}}));
    }
    if op == "sweep_projective_seams_audit" || op == "sweep_exact_seams_audit" {
        let patches=field::<Vec<surface::Surface>>(&v,"patches")?;
        let declarations=field::<Vec<Value>>(&v,"seams")?;
        let seams=declarations.iter().map(|s|Ok(continuity::ProjectiveSeam {
            patches:field(s,"patches")?,boundaries:field(s,"boundaries")?,
            order:field(s,"order")?,normal_scale:field(s,"normalScale")?,
        })).collect::<Result<Vec<_>>>()?;
        let projective=op=="sweep_projective_seams_audit";
        let r=if projective {continuity::inspect_projective_seam_collection(&patches,&seams,field(&v,"maxWork")?)?}
            else {continuity::inspect_exact_seam_collection(&patches,&seams,field(&v,"maxWork")?)?};
        let reports=r.seams.iter().zip(&seams).map(|(p,s)|json!({
            "certified":p.certified,"exactIdentity":p.exact_identity,"regularityCertified":p.regularity_certified,
            "work":p.work,"reason":p.reason,
            "certifiedOrder":if p.certified {Some(s.order)} else {None},
        })).collect::<Vec<_>>();
        let mut result=json!({"exactG1G2Certified":r.certified,
            "certifiedOrder":r.certified_order,"exactWork":r.exact_work,
            "unresolvedSeams":r.unresolved_seams,"seams":reports});
        if projective {
            result["method"]=json!("constant-projective-strip-jets");
            for report in result["seams"].as_array_mut().unwrap() {
                report["method"]=json!("constant-projective-strip-jets");
            }
        }
        return Ok(result);
    }
    if op == "surface_exact_strip_jets_audit" {
        let a=field::<surface::Surface>(&v,"reference")?;
        let b=field::<surface::Surface>(&v,"edited")?;
        let ab=field::<String>(&v,"referenceBoundary")?;
        let bb=field::<String>(&v,"editedBoundary")?;
        let report=continuity::inspect_surface_exact_strip_jets(&a,&b,&ab,&bb,
            field(&v,"order")?,field(&v,"normalScale")?,field(&v,"maxWork")?)?;
        return Ok(json!({"certified":report.certified,"exactIdentity":report.exact_identity,
            "regularityCertified":report.regularity_certified,"work":report.work,"reason":report.reason}));
    }
    if op == "sweep_profile_regularity_audit" {
        let profiles=field::<Vec<curve::Curve>>(&v,"profiles")?;
        let max_cells=field::<usize>(&v,"maxCells")?;
        check(!profiles.is_empty()&&profiles.len()<=64&&max_cells<=100000,"Invalid profile regularity budget")?;
        let mut cells=0;
        let mut unresolved_profiles=Vec::new();
        for (index,profile) in profiles.iter().enumerate() {
            let report=curve_regularity::inspect(profile,max_cells-cells)?;
            cells+=report.cells;
            if !report.spanwise_regular {unresolved_profiles.push(index);}
        }
        return Ok(json!({"spanwiseRegular":unresolved_profiles.is_empty(),"cells":cells,
            "unresolvedProfiles":unresolved_profiles,"continuityCertified":false}));
    }
    if op == "surface_linear_injectivity_audit" {
        let surface=field::<surface::Surface>(&v,"surface")?;
        let max_cells=field::<usize>(&v,"maxCells")?;
        let report=if let Some(projection)=optional_field::<[[i8;3];2]>(&v,"projection")? {
            surface_linear_monotonicity::inspect(&surface,projection,max_cells)?
        }else{surface_linear_monotonicity::inspect_candidate(&surface,max_cells)?};
        return Ok(json!({"certified":report.certified,"cells":report.cells,
            "projection":report.projection,"reason":report.reason,"globalEmbeddingCertified":false}));
    }
    if op == "sweep_boundary_coverage_audit" {
        let label=field::<String>(&v,"boundary")?;
        let boundary=match label.as_str(){
            "uMin"=>sweep_cap_wall::Boundary::UMin,"uMax"=>sweep_cap_wall::Boundary::UMax,
            "vMin"=>sweep_cap_wall::Boundary::VMin,"vMax"=>sweep_cap_wall::Boundary::VMax,
            _=>return Err(input("Invalid coverage boundary")),
        };
        let covered=sweep_cap_wall::covers_boundary(&field::<surface::Surface>(&v,"surface")?,
            &field::<curve::Curve>(&v,"uv")?,boundary)?;
        return Ok(json!({"wholeBoundaryCovered":covered,"injectivityCertified":false,
            "boundaryOwnershipCertified":false,"globalEmbeddingCertified":false}));
    }
    if op == "sweep_cap_wall_audit" {
        let declarations=field::<Vec<Option<String>>>(&v,"boundaries")?;
        check(declarations.len()<=1024,"Too many cap/wall declarations")?;
        let boundaries=declarations.iter().map(|value| match value.as_deref(){
            None=>Ok(None),Some("uMin")=>Ok(Some(sweep_cap_wall::Boundary::UMin)),
            Some("uMax")=>Ok(Some(sweep_cap_wall::Boundary::UMax)),
            Some("vMin")=>Ok(Some(sweep_cap_wall::Boundary::VMin)),
            Some("vMax")=>Ok(Some(sweep_cap_wall::Boundary::VMax)),
            _=>Err(input("Invalid cap/wall boundary")),
        }).collect::<Result<Vec<_>>>()?;
        let report=sweep_cap_wall::inspect(&field::<surface::Surface>(&v,"cap")?,
            &field::<Vec<surface::Surface>>(&v,"walls")?,&boundaries,field(&v,"maxWalls")?)?;
        return Ok(json!({"allWallInteriorsExcluded":report.all_wall_interiors_excluded,
            "planeAxis":report.plane_axis,"inspectedWalls":report.inspected_walls,
            "separatedWalls":report.separated_walls,"boundaryRestrictedWalls":report.boundary_restricted_walls,
            "unresolvedWalls":report.unresolved_walls,"reason":report.reason,
            "boundaryOwnershipCertified":false,"globalEmbeddingCertified":false}));
    }
    if op == "sweep_coedge_exact_audit" {
        let max_work=field::<u64>(&v,"maxWork")?;
        check(max_work<=10000000,"Exact coedge budget exceeds 10000000")?;
        let decision=curve_surface_agreement::verify_exact(
            &field::<curve::Curve>(&v,"world")?,&field::<curve::Curve>(&v,"uv")?,
            &field::<surface::Surface>(&v,"surface")?,field(&v,"reversed")?,max_work)?;
        let (status,work)=match decision {
            None=>("unsupported",0),
            Some(decision)=>{let status=match decision.outcome {
                cad_predicates::BezierIdentity::Equal=>"equal",
                cad_predicates::BezierIdentity::Different=>"different",
                cad_predicates::BezierIdentity::Indeterminate(_)=>"unresolved",
            };(status,decision.work_used)},
        };
        return Ok(json!({"status":status,"work":work,"exactIdentityCertified":status=="equal",
            "globalEmbeddingCertified":false}));
    }
    if op == "sweep_coedge_agreement_audit" {
        let world=field::<curve::Curve>(&v,"world")?;
        let uv=field::<curve::Curve>(&v,"uv")?;
        let surface=field::<surface::Surface>(&v,"surface")?;
        let reversed=field::<bool>(&v,"reversed")?;
        let tolerance=field::<f64>(&v,"tolerance")?;
        let max_cells=field::<usize>(&v,"maxCells")?;
        world.validate()?;uv.validate()?;surface.validate()?;
        check(world.control_points[0].len()==3 && uv.control_points[0].len()==2,"Agreement needs a 3D curve and 2D pcurve")?;
        check(tolerance.is_finite() && tolerance>0.,"Agreement tolerance must be positive")?;
        check(max_cells<=100000,"Agreement budget exceeds 100000")?;
        if max_cells==0 {
            return Ok(json!({"withinTolerance":false,"status":"unresolved","cells":0,
                "witness":null,"witnessDistance":null,"exactIdentityCertified":false,
                "globalEmbeddingCertified":false}));
        }
        let report=curve_surface_agreement::verify(&world,&uv,&surface,reversed,tolerance,max_cells)?;
        let status=match report.status {
            curve_surface_agreement::Status::WithinTolerance=>"within-tolerance",
            curve_surface_agreement::Status::Mismatch=>"mismatch",
            curve_surface_agreement::Status::Unresolved=>"unresolved",
        };
        return Ok(json!({"withinTolerance":report.status==curve_surface_agreement::Status::WithinTolerance,
            "status":status,"cells":report.cells,"witness":report.witness,
            "witnessDistance":report.witness_distance,"exactIdentityCertified":false,
            "globalEmbeddingCertified":false}));
    }
    if op == "sweep_cap_boundary_audit" {
        let report = sweep_cap_boundary::inspect(
            &field::<surface::Surface>(&v,"surface")?,
            &field::<curve::Curve>(&v,"world")?, &field::<curve::Curve>(&v,"uv")?,
            field(&v,"tolerance")?,field(&v,"maxProducts")?,
        )?;
        return Ok(json!({"withinBudget":report.within_budget,
            "errorUpper":report.error_upper,"products":report.products,"reason":report.reason,
            "capGeometryCertified":false,"globalEmbeddingCertified":false}));
    }
    if op == "sweep_contour_audit" {
        let report = sweep_contour_audit::inspect(
            &field::<Vec<Vec<curve::Curve>>>(&v, "loops")?,
            field(&v, "tolerance")?, field(&v, "maxPairs")?, field(&v, "maxCells")?,
        )?;
        return Ok(json!({
            "capDomainCertified": report.cap_domain_certified,
            "globalEmbeddingCertified": false,
            "capGeometryCertified": false,
            "planeAxis": report.plane_axis,
            "pairs": report.pairs, "cells": report.cells, "reason": report.reason,
        }));
    }
    if op == "sweep_wall_audit" {
        let walls = field::<Vec<surface::Surface>>(&v, "walls")?;
        let shared = field::<Vec<[usize; 2]>>(&v, "sharedBoundaries")?;
        let report = sweep_wall_audit::inspect(
            &walls, &shared, field(&v, "clearance")?, field(&v, "distanceTolerance")?,
            field(&v, "maxInjectivityCells")?, field(&v, "maxPairs")?,
            field(&v, "maxPairCells")?,
        )?;
        return encode_wall_audit(report);
    }
    if op == "sweep_seam_audit" {
        let patches = field::<Vec<surface::Surface>>(&v, "patches")?;
        let declarations = field::<Vec<Value>>(&v, "seams")?;
        check(declarations.len() <= 4096, "Too many seam declarations")?;
        let mut owned = Vec::with_capacity(declarations.len());
        for declaration in declarations {
            owned.push((
                field::<[usize; 2]>(&declaration, "patches")?,
                field::<[String; 2]>(&declaration, "boundaries")?,
                field::<usize>(&declaration, "order")?,
                field::<f64>(&declaration, "normalScale")?,
                field::<f64>(&declaration, "jetTolerance")?,
            ));
        }
        let seams: Vec<_> = owned.iter().map(|item| sweep_seam_audit::Seam {
            patches: item.0, boundaries: [&item.1[0], &item.1[1]],
            order: item.2, normal_scale: item.3, jet_tolerance: item.4,
        }).collect();
        let report = sweep_seam_audit::inspect(&patches, &seams, field(&v, "maxSeams")?)?;
        return Ok(json!({
            "allWithinJetBudget": report.all_within_jet_budget,
            "inspectedSeams": report.inspected_seams,
            "exactG1G2Certified": false,
            "seams": report.seams.iter().map(|item| json!({
                "withinJetBudget": item.within_jet_budget,
                "regularityCertified": item.regularity_certified,
                "tangentialSmoothnessCertified": item.tangential_smoothness_certified,
                "errorUpper": item.error_upper, "reason": item.reason,
            })).collect::<Vec<_>>(),
        }));
    }
    if op=="curve_progressive_miter" || op=="curve_progressive_miter_level" || op=="curve_progressive_miter_wall_audit" || op=="curve_progressive_miter_cap_projection" || op=="curve_progressive_miter_cap_domains" || op=="curve_progressive_miter_cap_parallelism" {
        let profiles=field::<Vec<curve::Curve>>(&v,"profiles")?;
        let points=field::<Vec<[f64;3]>>(&v,"points")?;
        let scale=field::<curve::Curve>(&v,"scale")?;let twist=field::<curve::Curve>(&v,"twist")?;
        let options=progressive_miter::Options {normal:field(&v,"normal")?,closed:field(&v,"closed")?,miter_limit:field(&v,"miter_limit")?,initial_steps:field(&v,"initial_steps")?,max_steps:field(&v,"max_steps")?,max_deviation:field(&v,"max_deviation")?};
        let report=|r:progressive_miter::Report|r.to_value();
        let axes=optional_field::<curve::Curve>(&v,"axis_scale")?;
        let center=optional_field::<curve::Curve>(&v,"center_law")?;
        let use_affine=axes.is_some()||center.is_some();
        let axes=axes.unwrap_or(progressive_sweep::constant_vector_law([1.;3])?);
        let center=center.unwrap_or(progressive_sweep::constant_vector_law([0.;3])?);
        let mut sweep=progressive_miter::Sweep::new(&profiles,&points,&scale,&twist,options)?;
        if use_affine {sweep=sweep.with_affine_laws(&axes,&center)?;}
        let frame_axis=optional_field::<curve::Curve>(&v,"frame_axis")?;
        let frame_normal=optional_field::<curve::Curve>(&v,"frame_normal")?;
        crate::check(frame_axis.is_some()==frame_normal.is_some(),"Authored miter requires both frame_axis and frame_normal")?;
        if let (Some(axis),Some(normal))=(frame_axis.as_ref(),frame_normal.as_ref()) {sweep=sweep.with_frame_laws(axis,normal)?;}
        let orientation_guide=optional_field::<curve::Curve>(&v,"orientation_guide")?;
        if let Some(guide)=orientation_guide.as_ref() {sweep=sweep.with_orientation_guide(guide)?;}
        if op=="curve_progressive_miter_cap_domains" {
            let loops=field::<Vec<usize>>(&v,"loopSizes")?;
            let r=sweep.certify_ideal_cap_domains(&loops,field(&v,"tolerance")?,field(&v,"maxPairs")?,field(&v,"maxCells")?,field(&v,"maxExactWork")?)?;
            return Ok(json!({"idealCapDomainsCertified":r.ideal_cap_domains_certified,"localDomainCertified":r.source.local_domain_certified,"sourcePlaneAxis":r.source.source_plane_axis,"endpointNormals":r.endpoint_normals,"cells":r.cells,"pairs":r.source.pairs,"exactWork":r.source.exact_work,"reason":r.reason,"method":"original-profile-endpoint-material-domains","continuousBound":false}));
        }
        if op=="curve_progressive_miter_cap_parallelism" {
            let caps=field::<Vec<surface::Surface>>(&v,"caps")?;
            crate::check(caps.len()==2,"Endpoint parallelism requires exactly two caps")?;
            let r=sweep.certify_endpoint_cap_parallelism([&caps[0],&caps[1]],field(&v,"maxCells")?,field(&v,"maxExactWork")?)?;
            return Ok(json!({"parallel":r.parallel,"cells":r.cells,"exactWork":r.exact_work,"reason":r.reason,"method":"original-endpoint-plane-parallelism","continuousBound":false}));
        }
        if op=="curve_progressive_miter_cap_projection" {
            let caps=field::<Vec<surface::Surface>>(&v,"caps")?;
            crate::check(caps.len()==2,"Endpoint projection requires exactly two caps")?;
            let r=sweep.certify_endpoint_cap_projection([&caps[0],&caps[1]],field(&v,"maxCells")?,field(&v,"maxExactWork")?)?;
            return Ok(json!({"normalDots":r.normal_dots,"reversesOrientation":r.reverses_orientation,"cells":r.cells,"exactWork":r.exact_work,"reason":r.reason,"method":"original-endpoint-plane-projection","continuousBound":false}));
        }
        if op == "curve_progressive_miter_wall_audit" {
            let sections = field::<Vec<Vec<curve::Curve>>>(&v, "sections")?;
            let loops = optional_field::<Vec<usize>>(&v, "loopSizes")?;
            let args = (field(&v, "clearance")?, field(&v, "distanceTolerance")?,
                field(&v, "maxInjectivityCells")?, field(&v, "maxPairs")?,
                field(&v, "maxPairCells")?);
            return encode_wall_audit(if let Some(loops) = loops {
                sweep.inspect_wall_geometry_with_loops(&sections, &loops,
                    args.0, args.1, args.2, args.3, args.4)?
            } else {
                sweep.inspect_wall_geometry(&sections,
                    args.0, args.1, args.2, args.3, args.4)?
            });
        }
        if op=="curve_progressive_miter_level" {
            let level=sweep.preview_at(field(&v,"preview_steps")?)?;
            return Ok(json!({"preview":true,"sections":level.sections,"report":report(level.report)}));
        }
        let mut levels=Vec::new();let mut sections=None;
        for level in sweep {
            let level=level?;
            if level.report.accepted {sections=Some(level.sections);}
            levels.push(report(level.report));
        }
        Ok(json!({"sections":sections,"report":levels.last(),"levels":levels}))
    } else {
        dispatch_other(v,op)
    }
}
fn dispatch_other(v:Value,op:String)->Result<Value> {
    if matches!(op.as_str(), "sweep_error_upper_compose") { return sweep_requests::dispatch(v, op); }
    if matches!(op.as_str(), "curve_decomposition_audit") { return certificate_requests::dispatch(v, op); }
    if matches!(op.as_str(), "sweep_exact_seam_set_audit" | "sweep_projective_seam_set_audit") { return sweep_requests::dispatch(v, op); }
    if matches!(op.as_str(), "surface_auto_guided_loft" | "surface_auto_guided_loft_cartesian") { return loft_requests::dispatch(v, op); }
    if matches!(op.as_str(), "surface_guided_loft_cartesian") { return loft_requests::dispatch(v, op); }
    if matches!(op.as_str(), "surface_natural_loft_checked") { return loft_requests::dispatch(v, op); }
    if matches!(op.as_str(), "surface_boundary_v_tangent_certify") { return certificate_requests::dispatch(v, op); }
    if matches!(op.as_str(), "surface_gordon_cartesian") { return loft_requests::dispatch(v, op); }

    if op == "surface_progressive_sweep_spatial_rmf_error" || op == "surface_progressive_sweep" || op == "surface_progressive_sweep_profiles" || op=="surface_progressive_sweep_level" || op=="surface_progressive_sweep_cap_domains" || op=="surface_progressive_sweep_cap_projection" || op=="surface_progressive_sweep_station_seams" || op=="surface_progressive_sweep_profile_join" || op=="surface_progressive_sweep_decomposition_joins" || op=="surface_progressive_sweep_decomposition_smoothness" || op=="surface_progressive_sweep_frame_smoothness" || op=="surface_progressive_sweep_closed_frame_smoothness" || op=="surface_progressive_sweep_closed_path_frame_smoothness" || op=="surface_progressive_sweep_closed_guided_frame_smoothness" { return sweep_requests::dispatch(v, op); }
    if op=="curve_affine" {return encode(affine::curve(&field::<curve::Curve>(&v,"curve")?,&field(&v,"matrix")?)?);}
    if op=="surface_affine" {return encode(affine::surface(&field::<surface::Surface>(&v,"surface")?,&field(&v,"matrix")?)?);}
    if op=="patches_affine" {return encode(affine::patches(&field::<Vec<surface::Surface>>(&v,"patches")?,&field(&v,"matrix")?)?);}
    if op == "surface_circle_rectangle_transition" {
        let circle=circle_transition::CircleSection {center:field(&v,"circle_center")?,normal:field(&v,"circle_normal")?,seam:field(&v,"circle_seam")?,radius:field(&v,"circle_radius")?};
        let rectangle=circle_rectangle_transition::RectangleSection {center:field(&v,"rectangle_center")?,axis_u:field(&v,"rectangle_axis_u")?,axis_v:field(&v,"rectangle_axis_v")?};
        return encode(circle_rectangle_transition::ruled(&circle,&rectangle)?);
    }
    if op == "surface_ellipse_transition" {
        let start=ellipse_transition::EllipseSection {center:field(&v,"start_center")?,axis_u:field(&v,"start_axis_u")?,axis_v:field(&v,"start_axis_v")?};
        let end=ellipse_transition::EllipseSection {center:field(&v,"end_center")?,axis_u:field(&v,"end_axis_u")?,axis_v:field(&v,"end_axis_v")?};
        return encode(ellipse_transition::ruled(&start,&end)?);
    }
    if op == "surface_circle_transition" {
        let start=circle_transition::CircleSection {center:field(&v,"start_center")?,normal:field(&v,"start_normal")?,seam:field(&v,"start_seam")?,radius:field(&v,"start_radius")?};
        let end=circle_transition::CircleSection {center:field(&v,"end_center")?,normal:field(&v,"end_normal")?,seam:field(&v,"end_seam")?,radius:field(&v,"end_radius")?};
        return encode(circle_transition::ruled(&start,&end)?);
    }
    if op == "surface_ribbon" {return encode(ribbon::checked(&field::<curve::Curve>(&v,"path")?,&field::<curve::Curve>(&v,"width")?,field(&v,"normal")?,field(&v,"sections")?,field(&v,"max_deviation")?)?);}
    if op == "surface_variable_pipe" {return encode(pipe::checked_variable(&field::<curve::Curve>(&v,"path")?,&field::<curve::Curve>(&v,"radius")?,field(&v,"normal")?,field(&v,"sections")?,field(&v,"max_deviation")?)?);}
    if op == "surface_pipe" {return encode(pipe::checked(&field::<curve::Curve>(&v,"path")?,field(&v,"radius")?,field(&v,"normal")?,field(&v,"sections")?,field(&v,"max_deviation")?)?);}
    if op == "surface_catenoid_patches" {return encode(catenoid::approximate_patches(field(&v,"center")?,field(&v,"scale")?,field(&v,"start_z")?,field(&v,"end_z")?,field(&v,"max_deviation")?)?);}
    if op == "surface_screw" {return encode(screw_surface::approximate(&field::<curve::Curve>(&v,"profile")?,field(&v,"origin")?,field(&v,"axis")?,field(&v,"height")?,field(&v,"turns")?,field(&v,"phase_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "curve_clothoid" {return encode(clothoid::approximate(field(&v,"center")?,field(&v,"length")?,field(&v,"start_curvature")?,field(&v,"end_curvature")?,field(&v,"phase_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "curve_spherical_spiral" {return encode(spherical_spiral::approximate(field(&v,"center")?,field(&v,"radius")?,field(&v,"longitude_turns")?,field(&v,"latitude_turns")?,field(&v,"longitude_phase_degrees")?,field(&v,"latitude_phase_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "surface_extrude_patches" {return encode(extrusion_patches::extrude(&field::<curve::Curve>(&v,"curve")?,field(&v,"vector")?)?);}
    if op == "curve_toroidal_spiral" {return encode(toroidal_spiral::approximate(field(&v,"center")?,field(&v,"major_radius")?,field(&v,"minor_radius")?,field(&v,"major_turns")?,field(&v,"minor_turns")?,field(&v,"major_phase_degrees")?,field(&v,"minor_phase_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "curve_torus_knot" {return encode(toroidal_spiral::approximate_knot(field(&v,"center")?,field(&v,"major_radius")?,field(&v,"minor_radius")?,field(&v,"p")?,field(&v,"q")?,field(&v,"major_phase_degrees")?,field(&v,"minor_phase_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "surface_helicoid_patches" {return encode(helicoid::approximate_patches(field(&v,"center")?,field(&v,"inner_radius")?,field(&v,"outer_radius")?,field(&v,"height")?,field(&v,"turns")?,field(&v,"phase_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "surface_helicoid" {return encode(helicoid::approximate(field(&v,"center")?,field(&v,"inner_radius")?,field(&v,"outer_radius")?,field(&v,"height")?,field(&v,"turns")?,field(&v,"phase_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "surface_catenoid" {return encode(catenoid::approximate(field(&v,"center")?,field(&v,"scale")?,field(&v,"start_z")?,field(&v,"end_z")?,field(&v,"max_deviation")?)?);}
    if op == "curve_catenary" {return encode(catenary::approximate(field(&v,"center")?,field(&v,"scale")?,field(&v,"start_x")?,field(&v,"end_x")?,field(&v,"max_deviation")?)?);}
    if op == "curve_archimedean_spiral" {return encode(archimedean_spiral::approximate(field(&v,"center")?,field(&v,"start_radius")?,field(&v,"end_radius")?,field(&v,"start_degrees")?,field(&v,"end_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "curve_epicycloid" {return encode(circular_rolling::approximate_epicycloid(field(&v,"center")?,field(&v,"fixed_radius")?,field(&v,"rolling_radius")?,field(&v,"start_radians")?,field(&v,"end_radians")?,field(&v,"max_deviation")?)?);}
    if op == "curve_hypocycloid" {return encode(circular_rolling::approximate_hypocycloid(field(&v,"center")?,field(&v,"fixed_radius")?,field(&v,"rolling_radius")?,field(&v,"start_radians")?,field(&v,"end_radians")?,field(&v,"max_deviation")?)?);}
    if op == "curve_trochoid" {return encode(trochoid::approximate(field(&v,"center")?,field(&v,"rolling_radius")?,field(&v,"tracing_radius")?,field(&v,"start_radians")?,field(&v,"end_radians")?,field(&v,"max_deviation")?)?);}
    if op == "curve_cycloid" {return encode(trochoid::approximate_cycloid(field(&v,"center")?,field(&v,"radius")?,field(&v,"start_radians")?,field(&v,"end_radians")?,field(&v,"max_deviation")?)?);}
    if op == "curve_lissajous" {return encode(lissajous::approximate(field(&v,"center")?,field(&v,"amplitudes")?,field(&v,"frequencies")?,field(&v,"phases_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "curve_logarithmic_spiral" {return encode(logarithmic_spiral::approximate(field(&v,"center")?,field(&v,"radius")?,field(&v,"growth")?,field(&v,"start_radians")?,field(&v,"end_radians")?,field(&v,"max_deviation")?)?);}
    if op == "curve_involute" {return encode(involute::approximate(field(&v,"center")?,field(&v,"radius")?,field(&v,"start_radians")?,field(&v,"end_radians")?,field(&v,"max_deviation")?)?);}
    if op == "curve_variable_pitch_helix" {return encode(helix::approximate_variable_pitch(field(&v,"center")?,field(&v,"radius")?,field(&v,"height")?,field(&v,"turns")?,field(&v,"start_pitch")?,field(&v,"end_pitch")?,field(&v,"phase_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "curve_elliptic_helix" {return encode(helix::approximate_elliptic(field(&v,"center")?,field(&v,"radius_x")?,field(&v,"radius_y")?,field(&v,"height")?,field(&v,"turns")?,field(&v,"phase_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "curve_conical_helix" {return encode(helix::approximate_conical(field(&v,"center")?,field(&v,"start_radius")?,field(&v,"end_radius")?,field(&v,"height")?,field(&v,"turns")?,field(&v,"phase_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "curve_helix" {return encode(helix::approximate(field(&v,"center")?,field(&v,"radius")?,field(&v,"height")?,field(&v,"turns")?,field(&v,"phase_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "surface_twist_sweep" { return sweep_requests::dispatch(v, op); }
    if op == "surface_two_guide_sweep" { return sweep_requests::dispatch(v, op); }
    if op == "surface_boundary_fill" {return encode(boundary_fill::fan(&field::<Vec<curve::Curve>>(&v,"boundaries")?,field(&v,"center")?)?);}
    if op == "surface_triangular_patch" {return encode(triangular_patch::patch(&field(&v,"base")?,&field(&v,"side_a")?,&field(&v,"side_b")?)?);}
    if op == "surface_gordon" { return loft_requests::dispatch(v, op); }
    if op == "surface_closed_loft" { return loft_requests::dispatch(v, op); }
    if op == "surface_loft_match_ends" { return loft_requests::dispatch(v, op); }
    if op == "surface_guided_loft" { return loft_requests::dispatch(v, op); }
    if op == "surface_control_tangent_loft" { return loft_requests::dispatch(v, op); }
    if op == "surface_clamped_loft" { return loft_requests::dispatch(v, op); }
    if op == "surface_natural_loft" { return loft_requests::dispatch(v, op); }
    if op == "surface_grid_spline" { return encode(grid_spline::interpolate(&field::<Vec<Vec<[f64;3]>>>(&v,"points")?,&field::<Vec<f64>>(&v,"parameters_u")?,&field::<Vec<f64>>(&v,"parameters_v")?)?); }
    if op == "surface_hermite_patch" { return encode(hermite_patch::patch(field(&v,"corners")?,field(&v,"tangent_u")?,field(&v,"tangent_v")?,field(&v,"twist")?)?); }
    if op == "curve_closed_spline" { return encode(closed_spline::interpolate(&field::<Vec<[f64;3]>>(&v,"points")?,&field::<Vec<f64>>(&v,"parameters")?)?); }
    if op == "curve_clamped_spline" { return encode(natural_spline::clamped(&field::<Vec<[f64;3]>>(&v,"points")?,&field::<Vec<f64>>(&v,"parameters")?,field(&v,"start_tangent")?,field(&v,"end_tangent")?)?); }
    if op == "curve_natural_spline" { return encode(natural_spline::interpolate(&field::<Vec<[f64;3]>>(&v,"points")?,&field::<Vec<f64>>(&v,"parameters")?)?); }
    if op == "curve_hermite" { return encode(hermite::interpolate(&field::<Vec<[f64;3]>>(&v,"points")?,&field::<Vec<[f64;3]>>(&v,"tangents")?,&field::<Vec<f64>>(&v,"parameters")?)?); }
    if op == "surface_formula" { return encode(formula::surface(field(&v,"domain")?,&formula::tokens(&field::<Vec<Value>>(&v,"expressions")?,true)?)?); }
    if op == "curve_formula" { return encode(formula::curve(field(&v,"domain")?,&formula::tokens(&field::<Vec<Value>>(&v,"expressions")?,false)?)?); }
    if op == "surface_sphere" { return encode(primitives::sphere(field(&v,"center")?,field(&v,"radius")?)?); }
    if op == "surface_cylinder" { return encode(primitives::cylinder(field(&v,"center")?,field(&v,"radius")?,field(&v,"height")?)?); }
    if op == "surface_cone" { return encode(primitives::cone(field(&v,"center")?,field(&v,"radius")?,field(&v,"height")?)?); }
    if op=="surface_bezier" {return encode(patches::bezier(field(&v,"points")?,optional_field(&v,"weights")?)?)}
    if op=="surface_bilinear" {return encode(patches::bilinear(field(&v,"corners")?)?)}
    if op=="surface_plane" {return encode(patches::plane(field(&v,"origin")?,field(&v,"axisU")?,field(&v,"axisV")?)?)}
    if op=="curve_bezier" {return encode(paths::bezier(field(&v,"points")?,optional_field(&v,"weights")?)?)}
    if op=="sweep_repair_circle_sections" { return sweep_requests::dispatch(v, op); }
    if op=="curve_repair_circle_section" {
        let r=crate::section_circle_repair::repair(&field::<Vec<curve::Curve>>(&v,"curves")?,field(&v,"quantum")?,field(&v,"tolerance")?,field(&v,"maxWork")?)?;
        return Ok(json!({"curves":r.curves,"displacementUpper":r.displacement_upper,"work":r.work,"reason":r.reason}));
    }
    if op == "sweep_retained_patch_regularity" { return sweep_requests::dispatch(v, op); }
    if op == "sweep_frenet_frame_regularity" { return sweep_requests::dispatch(v, op); }
    if op == "sweep_fixed_normal_frame_regularity" { return sweep_requests::dispatch(v, op); }
    if op == "sweep_authored_frame_regularity" { return sweep_requests::dispatch(v, op); }
    if op=="sweep_correct_miter_sections" { return sweep_requests::dispatch(v, op); }
    if op=="sweep_project_miter_caps" { return sweep_requests::dispatch(v, op); }
    if op=="sweep_project_sections" { return sweep_requests::dispatch(v, op); }
    if op=="curve_project_section_authored_axis" {
        let r=crate::section_projection::project_authored_axis(&field::<Vec<curve::Curve>>(&v,"curves")?,&field::<curve::Curve>(&v,"frameAxis")?,field(&v,"traversal")?,field(&v,"quantum")?,field(&v,"tolerance")?,field(&v,"maxWork")?)?;
        return Ok(json!({"curves":r.curves,"displacementUpper":r.displacement_upper,"exactPlanar":r.exact_planar,"work":r.work,"reason":r.reason}));
    }
    if op=="curve_project_section" {
        let r=crate::section_projection::project(&field::<Vec<curve::Curve>>(&v,"curves")?,field(&v,"axis")?,field(&v,"coefficients")?,field(&v,"offset")?,field(&v,"quantum")?,field(&v,"tolerance")?,field(&v,"maxWork")?)?;
        return Ok(json!({"curves":r.curves,"displacementUpper":r.displacement_upper,"exactPlanar":r.exact_planar,"work":r.work,"reason":r.reason}));
    }
    if op=="curve_miter_sections" { return sweep_requests::dispatch(v, op); }
    if op=="curve_transition_polyline" {return encode((if optional_field::<bool>(&v,"closed")?.unwrap_or(false) {paths::closed_transition_polyline} else {paths::transition_polyline})(&field::<Vec<[f64;3]>>(&v,"points")?,field(&v,"setback")?)?)}
    if op=="curve_round_polyline" {return encode((if optional_field::<bool>(&v,"closed")?.unwrap_or(false) {paths::closed_round_polyline} else {paths::round_polyline})(&field::<Vec<[f64;3]>>(&v,"points")?,field(&v,"radius")?)?)}
    if op=="curve_compose" {return encode(paths::compose(&field::<Vec<curve::Curve>>(&v,"curves")?)?)}
    if op=="curve_line" {return encode(primitives::line(field(&v,"start")?,field(&v,"end")?)?)}
    if op=="curve_polyline" {return encode(primitives::polyline(&field::<Vec<[f64;3]>>(&v,"points")?,optional_field(&v,"closed")?.unwrap_or(false))?)}
    if op=="curve_circle" {return encode(primitives::circle(field(&v,"center")?,field(&v,"normal")?,field(&v,"radius")?)?)}
    if op=="curve_circle_arc" {return encode(primitives::circle_arc(field(&v,"center")?,field(&v,"normal")?,field(&v,"radius")?,field(&v,"startDegrees")?,field(&v,"sweepDegrees")?)?)}
    if op == "curve_rational_polynomial" {
        return encode(polynomial::rational_curve(field(&v,"domain")?,&field::<Vec<[f64;4]>>(&v,"coefficients")?)?);
    }
    if op == "surface_rational_polynomial" {
        return encode(polynomial::rational_surface(field(&v,"domain")?,&field::<Vec<Vec<[f64;4]>>>(&v,"coefficients")?)?);
    }
    if op == "curve_polynomial_parametric" {
        return encode(polynomial::parametric_curve(field(&v,"domain")?,&field::<Vec<[f64;3]>>(&v,"coefficients")?)?);
    }
    if op == "surface_polynomial_parametric" {
        return encode(polynomial::parametric_surface(field(&v,"domain")?,&field::<Vec<Vec<[f64;3]>>>(&v,"coefficients")?)?);
    }
    if op == "surface_polynomial_graph" {
        return encode(polynomial::graph(field(&v,"bounds")?,&field::<Vec<Vec<f64>>>(&v,"coefficients")?)?);
    }
    if op == "surface_hyperboloid_one_sheet" {
        return encode(primitives::hyperboloid_one_sheet(field(&v,"center")?,field(&v,"radii")?,field(&v,"start")?,field(&v,"end")?)?);
    }
    if op == "surface_hyperboloid_two_sheet" {
        return encode(primitives::hyperboloid_two_sheet(field(&v,"center")?,field(&v,"radii")?,field(&v,"start")?,field(&v,"end")?,optional_field(&v,"lower")?.unwrap_or(false))?);
    }
    if op == "surface_quadratic_patch" {
        return encode(primitives::quadratic_patch(field(&v,"bounds")?,field(&v,"coefficients")?)?);
    }
    if op == "curve_parabola" || op == "curve_hyperbola" {
        let constructor=if op=="curve_parabola"{primitives::parabola}else{primitives::hyperbola};
        return encode(constructor(field(&v,"center")?,field(&v,"axisU")?,field(&v,"axisV")?,field(&v,"start")?,field(&v,"end")?)?);
    }
    if op == "surface_elliptic_cylinder" {
        return encode(primitives::elliptic_cylinder(field(&v,"center")?,field(&v,"radiusX")?,field(&v,"radiusY")?,field(&v,"height")?)?);
    }
    if op == "surface_cone_frustum" {
        return encode(primitives::cone_frustum(field(&v,"center")?,field(&v,"bottomRadius")?,field(&v,"topRadius")?,field(&v,"height")?)?);
    }
    if op == "curve_ellipse_arc" {
        return encode(primitives::ellipse_arc(field(&v,"center")?,field(&v,"axisU")?,field(&v,"axisV")?,field(&v,"startDegrees")?,field(&v,"sweepDegrees")?)?);
    }
    if op == "surface_ellipsoid" {
        return encode(primitives::ellipsoid(field(&v,"center")?,field(&v,"radii")?)?);
    }
    if op == "surface_torus" {
        return encode(primitives::torus(field(&v,"center")?,field(&v,"majorRadius")?,field(&v,"radialRadius")?,field(&v,"axialRadius")?)?);
    }
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
    if op == "curve_certify_foundation" { return certificate_requests::dispatch(v, op); }
    if op == "surface_certify_foundation" { return certificate_requests::dispatch(v, op); }
    if op == "surface_offset_evaluate" {
        let e = surface_offset::evaluate(
            &field(&v, "surface")?,
            field(&v, "parameters")?,
            field(&v, "distance")?,
        )?;
        return Ok(
            json!({"method":"normal-offset-numerical-jets","point":e.point,"du":e.du,"dv":e.dv,
            "sourceUnitNormal":e.source_unit_normal,"certified":false,"topologyAuthority":false}),
        );
    }
    if op == "surface_offset_bounds" {
        let r = surface_offset::bounds(
            &field(&v, "surface")?,
            field(&v, "domain")?,
            field(&v, "distance")?,
            field(&v, "maxSpans")?,
        )?;
        return Ok(
            json!({"method":"interval-source-normal-offset","scope":"incident-span-offset-images",
            "image":r.image,"unitNormals":r.unit_normals,"normalSpanVisits":r.spans,"reason":r.reason,
            "offsetRegularityCertified":false,"continuityCertified":false,"topologyAuthority":false}),
        );
    }
    if op == "surface_offset_jacobian_bounds" {
        let r = surface_offset::jacobian_bounds(
            &field(&v, "surface")?,
            field(&v, "domain")?,
            field(&v, "distance")?,
            field(&v, "maxSpans")?,
        )?;
        return Ok(
            json!({"method":"interval-source-normal-offset-jets","scope":"incident-span-offset-jets",
            "image":r.image,"derivatives":r.derivatives,"normalSpanVisits":r.spans,"reason":r.reason,
            "offsetRegularityCertified":false,"continuityCertified":false,"topologyAuthority":false}),
        );
    }
    if op == "surface_offset_candidates" {
        let a: Surface = field(&v, "a")?;
        let b: Surface = field(&v, "b")?;
        let r = surface_offset::intersection_candidates(
            [&a, &b],
            field(&v, "domains")?,
            field(&v, "distances")?,
            field(&v, "parameterTolerance")?,
            field(&v, "maxBoxes")?,
            field(&v, "maxSpans")?,
        )?;
        return Ok(
            json!({"method":"interval-offset-pair-exclusion","scope":"untrimmed-offset-carriers",
            "candidateBoxes":r.boxes,"pendingBoxes":r.pending,"visitedBoxes":r.visited_boxes,
            "excludedBoxes":r.excluded_boxes,"normalSpanVisits":r.normal_span_visits,"reason":r.reason,
            "rootExistenceProven":false,"wholeCurveComplete":false,"trimMembershipProven":false,"topologyAuthority":false}),
        );
    }
    if op == "surface_offset_contact_qualification" {
        let a: Surface = field(&v,"a")?;let b: Surface = field(&v,"b")?;
        let candidate: Surface = field(&v,"candidate")?;
        let pcurves: [crate::curve::Curve;2] = field(&v,"sourcePcurves")?;
        let first_loops: Vec<Vec<crate::curve::Curve>> = field(&v,"firstLoops")?;
        let second_loops: Vec<Vec<crate::curve::Curve>> = field(&v,"secondLoops")?;
        let report=crate::offset_contact_trims::certify_with_tangency(&candidate,[&pcurves[0],&pcurves[1]],[&a,&b],[&first_loops,&second_loops],
            field(&v,"distances")?,field(&v,"fixedAxis")?,field(&v,"fixedInterval")?,field(&v,"firstOther")?,field(&v,"secondDomain")?,field(&v,"maxSpans")?,
            field(&v,"toleranceMm")?,field(&v,"toleranceUv")?,crate::offset_contact_trims::Limits {
                fit_cells:field(&v,"maxFitCells")?,uv_cells:field(&v,"maxUvCells")?,agreement_cells:field(&v,"maxAgreementCells")?,root_refinements:field(&v,"rootRefinements")?,
                trim:crate::trimmed_offset_contact::Limits {max_pairs:field(&v,"maxPairs")?,max_cells:field(&v,"maxTrimCells")?,max_domain_cells:field(&v,"maxDomainCells")?},
            },field(&v,"maxSineSquared")?,crate::offset_contact_trims::TangentLimits {
                position_cells:field(&v,"maxTangentPositionCells")?,normal_cells:field(&v,"maxNormalCells")?,normal_spans:field(&v,"maxNormalSpans")?,
            })?;
        return Ok(json!({"method":"offset-contact-qualification-delivery","request":v,"candidateSurface":candidate,"qualification":report.to_value()}));
    }
    if op == "surface_offset_envelope_fit" {
        let a: Surface = field(&v,"a")?;let b: Surface = field(&v,"b")?;
        let candidate: Option<Surface> = v.get("candidate").filter(|x|!x.is_null()).map(|_|field(&v,"candidate")).transpose()?;
        return crate::offset_envelope_fit::deliver(candidate.as_ref(),[&a,&b],field(&v,"distances")?,
            field(&v,"fixedAxis")?,field(&v,"fixedInterval")?,field(&v,"firstOther")?,
            field(&v,"secondDomain")?,field(&v,"maxSpans")?,field(&v,"toleranceMm")?,field(&v,"maxCells")?);
    }
    if op == "surface_offset_envelope" {
        let a: Surface = field(&v,"a")?;let b: Surface = field(&v,"b")?;
        return Ok(crate::offset_envelope::certify([&a,&b],field(&v,"distances")?,
            field(&v,"fixedAxis")?,field(&v,"fixedInterval")?,field(&v,"firstOther")?,
            field(&v,"secondDomain")?,field(&v,"maxSpans")?,field(&v,"maxCells")?)?.to_value());
    }
    if op == "surface_offset_contact_tangent" {
        let a: Surface = field(&v,"a")?;let b: Surface = field(&v,"b")?;
        return Ok(crate::offset_contact_tangent::certify([&a,&b],field(&v,"distances")?,
            field(&v,"fixedAxis")?,field(&v,"fixedInterval")?,field(&v,"firstOther")?,
            field(&v,"secondDomain")?,field(&v,"maxSpans")?)?.to_value());
    }
    if op == "surface_offset_source_boundary" {
        let a: Surface = field(&v,"a")?;let b: Surface = field(&v,"b")?;
        let first_loops: Vec<Vec<crate::curve::Curve>> = field(&v,"firstLoops")?;
        let second_loops: Vec<Vec<crate::curve::Curve>> = field(&v,"secondLoops")?;
        let first_world: Vec<Vec<crate::offset_source_boundary::Coedge>> = field(&v,"firstCoedges")?;
        let second_world: Vec<Vec<crate::offset_source_boundary::Coedge>> = field(&v,"secondCoedges")?;
        return Ok(crate::offset_source_boundary::certify([&a,&b],[&first_loops,&second_loops],[&first_world,&second_world],
            field(&v,"distances")?,field(&v,"fixedAxis")?,field(&v,"fixedInterval")?,field(&v,"firstOther")?,
            field(&v,"secondDomain")?,field(&v,"maxSpans")?,field(&v,"toleranceUv")?,
            crate::trimmed_offset_contact::Limits{max_pairs:field(&v,"maxPairs")?,max_cells:field(&v,"maxCells")?,max_domain_cells:field(&v,"maxDomainCells")?},
            field(&v,"toleranceMm")?,field(&v,"maxExactWork")?,field(&v,"maxAgreementCells")?)?.to_value());
    }
    if op == "surface_offset_trimmed_contact_band" {
        let a: Surface = field(&v,"a")?;
        let b: Surface = field(&v,"b")?;
        let first_loops: Vec<Vec<crate::curve::Curve>> = field(&v,"firstLoops")?;
        let second_loops: Vec<Vec<crate::curve::Curve>> = field(&v,"secondLoops")?;
        return Ok(crate::trimmed_offset_contact::certify([&a,&b],[&first_loops,&second_loops],
            field(&v,"distances")?,field(&v,"fixedAxis")?,field(&v,"fixedInterval")?,
            field(&v,"firstOther")?,field(&v,"secondDomain")?,field(&v,"maxSpans")?,field(&v,"toleranceUv")?,
            crate::trimmed_offset_contact::Limits{max_pairs:field(&v,"maxPairs")?,max_cells:field(&v,"maxCells")?,max_domain_cells:field(&v,"maxDomainCells")?})?.to_value());
    }
    if op == "surface_offset_contact_band" {
        let a: Surface = field(&v, "a")?;
        let b: Surface = field(&v, "b")?;
        let r = surface_offset::certify_contact_band(
            [&a, &b], field(&v, "distances")?, field(&v, "fixedAxis")?,
            field(&v, "fixedInterval")?, field(&v, "firstOther")?,
            field(&v, "secondDomain")?, field(&v, "maxSpans")?,
        )?;
        let (status, witness) = match r {
            surface_offset::ContactBand::Excluded => ("excluded", Value::Null),
            surface_offset::ContactBand::Unresolved => ("unresolved", Value::Null),
            surface_offset::ContactBand::ContinuousBranch(w) => (
                "continuous-branch", json!({"firstUV":w.first_uv,"secondUV":w.second_uv,
                "centerIntervalMm":w.point,"contractionUpper":w.contraction_upper}),
            ),
        };
        return Ok(json!({"method":"interval-offset-band-krawczyk","scope":"parameter-band-within-tube",
            "status":status,"witness":witness,"rootForEveryParameterProven":status=="continuous-branch",
            "uniqueWithinTube":status=="continuous-branch","continuousBranchProven":status=="continuous-branch",
            "wholeCurveComplete":false,"trimMembershipProven":false,"topologyAuthority":false}));
    }
    if op == "surface_offset_contact_section" {
        let a: Surface = field(&v, "a")?;
        let b: Surface = field(&v, "b")?;
        let r = surface_offset::certify_contact_section(
            [&a, &b],
            field(&v, "distances")?,
            field(&v, "fixedAxis")?,
            field(&v, "fixed")?,
            field(&v, "firstOther")?,
            field(&v, "secondDomain")?,
            field(&v, "maxSpans")?,
        )?;
        let (status, witness) = match r {
            surface_contact::Verdict::Excluded => ("excluded", Value::Null),
            surface_contact::Verdict::Unresolved => ("unresolved", Value::Null),
            surface_contact::Verdict::Witness(w) => (
                "unique-contact",
                json!({"firstUV":w.first_uv,"secondUV":w.second_uv,
                "centerIntervalMm":w.point,"contractionUpper":w.contraction_upper}),
            ),
        };
        return Ok(
            json!({"method":"interval-offset-section-krawczyk","scope":"fixed-parameter-offset-section",
            "status":status,"witness":witness,"rootExistenceProven":status=="unique-contact",
            "uniqueInSection":status=="unique-contact","wholeCurveComplete":false,"trimMembershipProven":false,"topologyAuthority":false}),
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
    if op == "curve_project_certified" { return certificate_requests::dispatch(v, op); }
    if op == "surface_project_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_interpolate_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_approximate_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_reparameterize_exact" {
        return foundation::reparameterize_curve(
            &field(&v, "curve")?,
            field(&v, "domain")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_insert_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_elevate_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_remove_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_rebuild_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_reduce_certified" { return certificate_requests::dispatch(v, op); }
    if op == "surface_rebuild_certified" { return certificate_requests::dispatch(v, op); }
    if op == "surface_remove_certified" || op == "surface_reduce_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_periodic_edit_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_periodic_split_certified" { return certificate_requests::dispatch(v, op); }
    if op == "surface_periodic_edit_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_reparameterization_certify" { return certificate_requests::dispatch(v, op); }
    if op == "curve_reparameterized_evaluate" {
        return foundation::evaluate_reparameterized_curve(
            &field(&v, "curve")?,
            &v["mapping"],
            field(&v, "u")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_fit_certified" { return certificate_requests::dispatch(v, op); }
    if op == "surface_interpolate_certified" || op == "surface_fit_certified" { return certificate_requests::dispatch(v, op); }
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
    if op == "curve_fit_cloud_certified" { return certificate_requests::dispatch(v, op); }
    if op == "surface_fit_cloud_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_curve_intersect_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_surface_intersect_certified" { return certificate_requests::dispatch(v, op); }
    if op == "surface_surface_intersect_certified" { return certificate_requests::dispatch(v, op); }
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
    if op == "surface_scaled_sweep" { return sweep_requests::dispatch(v, op); }
    if op == "surface_profile_sweep" { return sweep_requests::dispatch(v, op); }
    if op == "surface_framed_sweep" { return sweep_requests::dispatch(v, op); }
    if op == "surface_sweep" { return sweep_requests::dispatch(v, op); }
    if op == "loft_aligned" { return loft_requests::dispatch(v, op); }
    if op == "loft" { return loft_requests::dispatch(v, op); }
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
            "curve_decomposition_audit" => {
                let r=crate::curve_decomposition_certificate::inspect(&c,field(&v,"span")?,&field(&v,"retained")?,field(&v,"maxProducts")?)?;
                Ok(json!({"errorUpper":r.error_upper,"products":r.products,"reason":r.reason,"method":"original-span-bernstein-decomposition","continuousBound":false}))
            },
            "curve_bounds" => encode(c.bounds()?),
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
        "surface_bounds" => encode(s.bounds()?),
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

fn encode_wall_audit(report: sweep_wall_audit::Report) -> Result<Value> {
    Ok(json!({
            "chartsAndPairsCertified": report.charts_and_pairs_certified,
            "globalEmbeddingCertified": false,
            "injectivityCells": report.injectivity_cells,
            "unresolvedCharts": report.unresolved_charts,
            "charts": report.charts.iter().map(|chart| json!({
                "certified": chart.certified, "cells": chart.cells,
                "projection": chart.projection, "reason": chart.reason,
            })).collect::<Vec<_>>(),
            "declaredBoundariesC0": report.declared_boundaries_c0,
            "c0Boundaries": report.c0_boundaries,
            "unresolvedBoundaries": report.unresolved_boundaries,
            "pairs": {
                "allPairsSeparated": report.pairs.all_pairs_separated,
                "allPairsCompatible": report.pairs.all_pairs_compatible,
                "boundaryOnlyPairs": report.pairs.boundary_only_pairs,
                "separatedPairs": report.pairs.separated_pairs,
                "pairs": report.pairs.pairs, "cells": report.pairs.cells,
                "unresolved": report.pairs.unresolved.iter().map(|pair| json!({
                    "patches": pair.patches, "reason": pair.reason,
                })).collect::<Vec<_>>(),
            },
    }))
}

fn miter_constant_vector_law(value:[f64;3])->Result<curve::Curve> {
    let curve=curve::Curve{degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![value.to_vec();2],weights:vec![1.,1.],periodic:false};
    curve.validate()?;Ok(curve)
}

#[cfg(test)]
mod scalar_dispatch_regression {
    use super::*;
    #[test]
    fn profile_dispatch_preserves_continuous_certificate_and_zero_cell_refusal() {
        let profile = json!({"degree":1,"knots":[0.,0.,1.,1.],"controlPoints":[[1.,0.,0.],[2.,0.,0.]],"weights":[1.,1.],"periodic":false});
        let path = json!({"degree":1,"knots":[0.,0.,1.,1.],"controlPoints":[[0.,0.,0.],[0.,0.,4.]],"weights":[1.,1.],"periodic":false});
        let scale = json!({"degree":1,"knots":[0.,0.,1.,1.],"controlPoints":[[1.,0.,0.],[1.,0.,0.]],"weights":[1.,1.],"periodic":false});
        let mut request = json!({"op":"surface_profile_sweep","profile":profile,"path":path,"scale":scale,"normal":[1.,0.,0.],"sections":5,"max_deviation":0.01});
        let result = dispatch(request.clone()).unwrap();
        assert_eq!(result["report"]["continuousCertificate"]["withinBudget"],true);
        request["maxCells"] = json!(0);
        let result = dispatch(request).unwrap();
        assert_eq!(result["report"]["accepted"],false);
        assert_eq!(result["report"]["continuousCertificate"]["maxCells"],0);
        assert!(result["surface"].is_null());
    }
}
