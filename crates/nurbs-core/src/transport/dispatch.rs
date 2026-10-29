use super::*;

pub(super) fn field<T: for<'a> Deserialize<'a>>(v: &Value, k: &str) -> Result<T> {
    value_codec::from_value(v[k].clone()).map_err(|e| input(format!("Invalid {k}: {e}")))
}
pub(super) fn encode<T: Serialize>(v: T) -> Result<Value> {
    value_codec::to_value(v).map_err(|e| numeric_err(e.to_string()))
}
pub(super) fn optional_field<T: for<'a> Deserialize<'a>>(v: &Value, k: &str) -> Result<Option<T>> {
    match v.get(k) {
        Some(value) if !value.is_null() => value_codec::from_value(value.clone())
            .map(Some)
            .map_err(|e| input(format!("Invalid {k}: {e}"))),
        _ => Ok(None),
    }
}
pub(super) fn control_tangents(v: &Value) -> Result<Option<[Vec<[f64; 3]>; 2]>> {
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
    if op == "surface_offset_continuous_deviation" { return super::offset_requests::inspect(&v); }
    if op=="profile_section_bridge_chords" {return super::profile_bridge_chord_transport::inspect(&v);}
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
            "projection":report.projection,"rowWeights":report.row_weights,"profileParameterization":report.profile_parameterization.as_ref().map(|p|json!({"weights":p.weights,"derivativeLower":p.derivative_lower})),"reason":report.reason,"globalEmbeddingCertified":false}));
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
