//! Optional bounded JSON request boundary.
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
    if op == "sweep_repair_circle_sections" || op == "sweep_project_sections" {
        let sections: Vec<Vec<curve::Curve>> = field(&v,"sections")?;
        let max_work = field(&v,"maxWork")?;
        let r = if op == "sweep_repair_circle_sections" {
            sweep_section_correction::repair_circle(&sections,field(&v,"quantum")?,field(&v,"tolerance")?,max_work)?
        } else {
            let owned: Vec<Value> = field(&v,"corrections")?;
            let corrections: Vec<_> = owned.iter().map(|c| Ok(sweep_section_correction::Correction {
                section:field(c,"section")?,axis:field(&c["plane"],"axis")?,coefficients:field(&c["plane"],"coefficients")?,
                offset:field(&c["plane"],"offset")?,quantum:field(c,"quantum")?,tolerance:field(c,"tolerance")?,
            })).collect::<Result<_>>()?;
            sweep_section_correction::project(&sections,&corrections,max_work)?
        };
        let mut report=json!({"sections":r.sections,"wallDisplacementUpper":r.upper,"work":r.work,"reason":r.reason});
        if op=="sweep_project_sections" {report["exactPlanarSections"]=json!(r.planar);}
        return Ok(report);
    }
    if op=="curve_repair_circle_section" {
        let r=crate::section_circle_repair::repair(&field::<Vec<curve::Curve>>(&v,"curves")?,field(&v,"quantum")?,field(&v,"tolerance")?,field(&v,"maxWork")?)?;
        return Ok(json!({"curves":r.curves,"displacementUpper":r.displacement_upper,"work":r.work,"reason":r.reason}));
    }
    if op=="curve_project_section" {
        let r=crate::section_projection::project(&field::<Vec<curve::Curve>>(&v,"curves")?,field(&v,"axis")?,field(&v,"coefficients")?,field(&v,"offset")?,field(&v,"quantum")?,field(&v,"tolerance")?,field(&v,"maxWork")?)?;
        return Ok(json!({"curves":r.curves,"displacementUpper":r.displacement_upper,"exactPlanar":r.exact_planar,"work":r.work,"reason":r.reason}));
    }
    if op=="curve_miter_sections" {return encode((if optional_field::<bool>(&v,"closed")?.unwrap_or(false) {paths::closed_miter_sections} else {paths::miter_sections})(&field::<Vec<curve::Curve>>(&v,"profiles")?,&field::<Vec<[f64;3]>>(&v,"points")?,field(&v,"normal")?,field(&v,"miter_limit")?)?)}

    if op=="curve_affine" {return encode(affine::curve(&field::<curve::Curve>(&v,"curve")?,&field(&v,"matrix")?)?);}
    if op=="surface_affine" {return encode(affine::surface(&field::<surface::Surface>(&v,"surface")?,&field(&v,"matrix")?)?);}
    if op=="patches_affine" {return encode(affine::patches(&field::<Vec<surface::Surface>>(&v,"patches")?,&field(&v,"matrix")?)?);}

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

    if op=="sweep_error_upper_compose" {
        let kind=field::<String>(&v,"kind")?;
        let a=optional_field::<f64>(&v,"a")?;
        let b=optional_field::<f64>(&v,"b")?;
        let upper=match kind.as_str() {
            "add"=>a.zip(b).and_then(|(a,b)|sweep_support::error_upper::add(a,b)),
            "multiply"=>a.zip(b).and_then(|(a,b)|sweep_support::error_upper::multiply(a,b)),
            "sqrt-two"=>a.and_then(sweep_support::error_upper::sqrt_two),
            _=>return Err(input("Unknown error composition operation")),
        };
        return Ok(json!({"errorUpper":upper}));
    }
    if op=="sweep_boundary_certificate" {
        let wall=optional_field::<f64>(&v,"wall")?;
        let caps=optional_field::<[Option<f64>;2]>(&v,"caps")?.and_then(|c|c[0].zip(c[1]).map(|(a,b)|[a,b]));
        let closed=field::<bool>(&v,"closed")?;
        let budget=optional_field::<f64>(&v,"budget")?;
        let valid=|x:f64|x.is_finite()&&x>=0.;
        let budget_valid=budget.is_some_and(|x|x.is_finite()&&x>0.);
        let upper=if budget_valid {sweeps::filled_cap_error::boundary(wall,caps,closed)} else {None};
        let within=upper.zip(budget).map(|(u,b)|u<=b);
        let reason=if !budget_valid {Some("invalid-budget")} else if !wall.is_some_and(valid) {Some("wall-bound-unproved")}
            else if upper.is_none() {Some("filled-cap-bound-unproved")} else if within==Some(false) {Some("boundary-budget-exceeded")} else {None};
        return Ok(json!({"method":"retained-sweep-boundary-union","scope":"boundary-set-hausdorff","continuousBound":upper.is_some(),"withinBudget":within,"errorUpper":upper,"budget":budget,"closed":closed,"wallErrorUpper":wall.filter(|x|valid(*x)),"filledCapErrorUpper":if !closed {caps.filter(|c|c.iter().all(|x|valid(*x)))} else {None},"reason":reason}));
    }
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
    if op == "sweep_retained_cap_contour_audit" {
        let (exact,work)=retained_wall_coefficients::contour_matches(
            &field::<Vec<curve::Curve>>(&v,"expected")?,&field::<Vec<curve::Curve>>(&v,"actual")?,
            &field::<Vec<bool>>(&v,"reversed")?,field(&v,"maxWork")?);
        return Ok(json!({"contourIdentity":exact,"work":work,"filledRegionCertified":false}));
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
    if op == "surface_progressive_sweep" || op == "surface_progressive_sweep_profiles" || op=="surface_progressive_sweep_level" {
        use progressive_sweep::{Options, Orientation, Spacing};
        let orientation = match field::<String>(&v,"orientation")?.as_str() {
            "rmf" => Orientation::RotationMinimizing,
            "fixed" | "authored" => Orientation::Fixed,
            "fixed_normal" => Orientation::FixedNormal,
            "frenet" => Orientation::Frenet,"corrected_frenet"=>Orientation::CorrectedFrenet,
            _ => return Err(input("Unknown sweep orientation")),
        };
        let spacing = match field::<String>(&v,"spacing")?.as_str() {
            "parameter" => Spacing::Parameter,
            "arc_length" => Spacing::ArcLength {tolerance:field(&v,"length_tolerance")?,max_cells:field(&v,"length_max_cells")?},
            _ => return Err(input("Unknown sweep station spacing")),
        };
        let options = Options {normal:field(&v,"normal")?,orientation,spacing,initial_sections:field(&v,"initial_sections")?,max_sections:field(&v,"max_sections")?,max_deviation:field(&v,"max_deviation")?};
        let axes=optional_field::<curve::Curve>(&v,"axis_scale")?;
        let center=optional_field::<curve::Curve>(&v,"center_law")?;
        let contact=optional_field::<f64>(&v,"contact_parameter")?;
        let contact_profile=optional_field::<usize>(&v,"contact_profile")?;
        check(contact.is_some() || contact_profile.is_none(), "Contact profile requires contact parameter")?;
        check(contact.is_none() || optional_field::<curve::Curve>(&v,"orientation_guide")?.is_some(), "Contact anchor requires orientation guide")?;
        if op=="surface_progressive_sweep_level" {
            let profiles=field::<Vec<curve::Curve>>(&v,"profiles")?;
            let path=field::<curve::Curve>(&v,"path")?;
            let scale=field::<curve::Curve>(&v,"scale")?;
            let twist=field::<curve::Curve>(&v,"twist")?;
            let guide=optional_field::<curve::Curve>(&v,"orientation_guide")?;
            let authored=field::<String>(&v,"orientation")?=="authored";
            check(!authored || guide.is_none(), "Orientation guide cannot be combined with authored frames")?;
            let frame_axis=if authored {Some(field::<curve::Curve>(&v,"frame_axis")?)} else {None};
            let frame_normal=if authored {Some(field::<curve::Curve>(&v,"frame_normal")?)} else {None};
            let use_affine=axes.is_some() || center.is_some() || authored || guide.is_some();
            let axes=axes.unwrap_or(progressive_sweep::constant_vector_law([1.;3])?);
            let center=center.unwrap_or(progressive_sweep::constant_vector_law([0.;3])?);
            let mut sweep=progressive_sweep::MultiSweep::new(&profiles,&path,&scale,&twist,options)?;
            if let Some(guide)=guide.as_ref() {
                sweep=if let Some(parameter)=contact {sweep.with_contact_guide(guide,contact_profile.unwrap_or(0),parameter)?} else {sweep.with_orientation_guide(guide)?};
            }
            if authored {sweep=sweep.with_frame_laws(frame_axis.as_ref().unwrap(),frame_normal.as_ref().unwrap())?;}
            if use_affine {sweep=sweep.with_affine_laws(&axes,&center)?;}
            return encode(sweep.preview_at(field::<usize>(&v,"preview_sections")?)?);
        }
        if let Some(guide)=optional_field::<curve::Curve>(&v,"orientation_guide")? {
            check(field::<String>(&v,"orientation")?!="authored", "Orientation guide cannot be combined with authored frames")?;
            let axes=axes.unwrap_or(progressive_sweep::constant_vector_law([1.;3])?);
            let center=center.unwrap_or(progressive_sweep::constant_vector_law([0.;3])?);
            let profiles=if op=="surface_progressive_sweep_profiles" {field::<Vec<curve::Curve>>(&v,"profiles")?} else {vec![field::<curve::Curve>(&v,"profile")?]};
            if let Some(parameter)=contact {
                return encode(progressive_sweep::approximate_contact_profiles(&profiles,&field::<curve::Curve>(&v,"path")?,&field::<curve::Curve>(&v,"scale")?,&field::<curve::Curve>(&v,"twist")?,&guide,contact_profile.unwrap_or(0),parameter,&axes,&center,options)?);
            }
            return encode(progressive_sweep::approximate_guided_profiles(&profiles,&field::<curve::Curve>(&v,"path")?,&field::<curve::Curve>(&v,"scale")?,&field::<curve::Curve>(&v,"twist")?,&guide,&axes,&center,options)?);
        }
        if field::<String>(&v,"orientation")? == "authored" {
            let axes=axes.unwrap_or(progressive_sweep::constant_vector_law([1.;3])?);
            let center=center.unwrap_or(progressive_sweep::constant_vector_law([0.;3])?);
            let profiles=if op=="surface_progressive_sweep_profiles" {field::<Vec<curve::Curve>>(&v,"profiles")?} else {vec![field::<curve::Curve>(&v,"profile")?]};
            return encode(progressive_sweep::approximate_authored_profiles(&profiles,&field::<curve::Curve>(&v,"path")?,&field::<curve::Curve>(&v,"scale")?,&field::<curve::Curve>(&v,"twist")?,&field::<curve::Curve>(&v,"frame_axis")?,&field::<curve::Curve>(&v,"frame_normal")?,&axes,&center,options)?);
        }
        if axes.is_some() || center.is_some() {
            let axes=axes.unwrap_or(progressive_sweep::constant_vector_law([1.;3])?);
            let center=center.unwrap_or(progressive_sweep::constant_vector_law([0.;3])?);
            let profiles=if op=="surface_progressive_sweep_profiles" {field::<Vec<curve::Curve>>(&v,"profiles")?} else {vec![field::<curve::Curve>(&v,"profile")?]};
            return encode(progressive_sweep::approximate_affine_profiles(&profiles,&field::<curve::Curve>(&v,"path")?,&field::<curve::Curve>(&v,"scale")?,&field::<curve::Curve>(&v,"twist")?,&axes,&center,options)?);
        }
        if op == "surface_progressive_sweep_profiles" {
            return encode(progressive_sweep::approximate_profiles(&field::<Vec<curve::Curve>>(&v,"profiles")?,&field::<curve::Curve>(&v,"path")?,&field::<curve::Curve>(&v,"scale")?,&field::<curve::Curve>(&v,"twist")?,options)?);
        }
        return encode(progressive_sweep::approximate(&field::<curve::Curve>(&v,"profile")?,&field::<curve::Curve>(&v,"path")?,&field::<curve::Curve>(&v,"scale")?,&field::<curve::Curve>(&v,"twist")?,options)?);
    }
    if op == "curve_decomposition_batch_audit" {
        let values = field::<Vec<Value>>(&v, "pairs")?;
        let owned = values.iter().map(|pair| Ok((field::<curve::Curve>(pair,"curve")?,field::<usize>(pair,"span")?,field::<curve::Curve>(pair,"retained")?))).collect::<Result<Vec<_>>>()?;
        let pairs = owned.iter().map(|(curve,span,retained)| (curve,*span,retained)).collect::<Vec<_>>();
        let r = crate::curve_decomposition_certificate::inspect_batch(&pairs,field(&v,"maxProducts")?)?;
        return Ok(json!({"errorUpper":r.error_upper,"products":r.products,"pairsInspected":r.pairs_inspected,"reason":r.reason,"method":"original-span-bernstein-decomposition","continuousBound":false}));
    }
    if op == "curve_decomposition_audit" {
        let r=crate::curve_decomposition_certificate::inspect(&field(&v,"curve")?,field(&v,"span")?,&field(&v,"retained")?,field(&v,"maxProducts")?)?;
        return Ok(json!({"errorUpper":r.error_upper,"products":r.products,"reason":r.reason,"method":"original-span-bernstein-decomposition","continuousBound":false}));
    }

    if op=="curve_progressive_miter" || op=="curve_progressive_miter_level" || op=="curve_progressive_miter_wall_audit" || op=="curve_progressive_miter_cap_projection" || op=="curve_progressive_miter_cap_domains" || op=="curve_progressive_miter_cap_parallelism" {
        let profiles=field::<Vec<curve::Curve>>(&v,"profiles")?;
        let points=field::<Vec<[f64;3]>>(&v,"points")?;
        let scale=field::<curve::Curve>(&v,"scale")?;let twist=field::<curve::Curve>(&v,"twist")?;
        let options=progressive_miter::Options {normal:field(&v,"normal")?,closed:field(&v,"closed")?,miter_limit:field(&v,"miter_limit")?,initial_steps:field(&v,"initial_steps")?,max_steps:field(&v,"max_steps")?,max_deviation:field(&v,"max_deviation")?};
        let report=|r:progressive_miter::Report|json!({"accepted":r.accepted,"phaseResolved":r.phase_resolved,"frameTransportCertified":r.frame_transport_certified,"frameTransportReason":r.frame_transport_reason,"steps":r.steps,"sections":r.sections,"stations":r.stations,"sampledControlDeviation":r.sampled_control_deviation,"continuousErrorUpper":r.continuous_error_upper,"certifiedErrorUpper":r.certified_error_upper,"endpointContourErrorUpper":r.endpoint_contour_error_upper,"errorCertificateCells":r.error_certificate_cells,"errorCertificateReason":r.error_certificate_reason,"profileRegularityCertified":r.profile_regularity_certified,"wallRegularityCertified":r.wall_regularity_certified,"regularityCells":r.regularity_cells,"unresolvedWallPatches":r.unresolved_wall_patches,"affineLawsApplied":r.affine_laws_applied,"authoredFramesApplied":r.authored_frames_applied,"orientationGuideApplied":r.orientation_guide_applied,"continuousErrorMethod":if r.orientation_guide_applied && r.authored_frames_applied {"interval-authored-axis-guide-frame-interpolation"}else if r.orientation_guide_applied {"interval-guide-frame-interpolation"}else if r.authored_frames_applied {"interval-authored-frame-interpolation"}else if r.affine_laws_applied {"interval-affine-law-interpolation"}else{"rational-law-derivative-interpolation-real-arithmetic"},"budget":r.budget,"closedPath":r.closed_path,"holonomyCorrectionRadians":r.holonomy_correction,"continuousBound":false,"roundingCertified":false,"seamContinuity":if r.closed_path {"C0"} else {"open"},"method":"progressive-miter-fourfold-section-refinement"});
        let axes=optional_field::<curve::Curve>(&v,"axis_scale")?;
        let center=optional_field::<curve::Curve>(&v,"center_law")?;
        let use_affine=axes.is_some()||center.is_some();
        let axes=axes.unwrap_or(miter_constant_vector_law([1.;3])?);
        let center=center.unwrap_or(miter_constant_vector_law([0.;3])?);
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
        return Ok(json!({"sections":sections,"report":levels.last(),"levels":levels}));
    }

    if op == "sweep_projective_seam_set_audit" || op == "sweep_exact_seam_set_audit" {
        let owned: Vec<Value> = field(&v, "seams")?;
        let declarations: Vec<_> = owned.iter().map(|s| Ok(sweep_seam_set::Seam {
            patches: field(s, "patches")?, boundaries: field(s, "boundaries")?,
            order: field(s, "order")?, normal_scale: field(s, "normalScale")?,
        })).collect::<Result<_>>()?;
        let projective = op == "sweep_projective_seam_set_audit";
        let report = sweep_seam_set::inspect(&field::<Vec<surface::Surface>>(&v, "patches")?, &declarations, field(&v, "maxWork")?, projective)?;
        let results: Vec<_> = report.seams.iter().zip(&declarations).map(|(r, s)| {
            let mut value = json!({"certified":r.certified,"exactIdentity":r.exact_identity,
                "regularityCertified":r.regularity_certified,"work":r.work,"reason":r.reason});
            if projective {value["method"] = json!("constant-projective-strip-jets"); value["certifiedOrder"] = json!(if r.certified {Some(s.order)} else {None});}
            value
        }).collect();
        let mut value = json!({"exactG1G2Certified":report.certified,"certifiedOrder":report.order,
            "exactWork":report.work,"unresolvedSeams":report.unresolved,"seams":results});
        if projective {value["method"] = json!("constant-projective-strip-jets");}
        return Ok(value);
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
    if op == "curve_segmented_bezier_controls" {
        return encode(retained_wall_coefficients::segmented_bezier_controls(
            &field::<curve::Curve>(&v,"curve")?,field(&v,"maxControlRows")?));
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
            &field::<surface::Surface>(&v, "reference")?,
            &field::<surface::Surface>(&v, "edited")?,
            &field::<String>(&v, "referenceBoundary")?,
            &field::<String>(&v, "editedBoundary")?,
        )?);
    }
    if op=="curve_transition_polyline" {return encode((if optional_field::<bool>(&v,"closed")?.unwrap_or(false) {paths::closed_transition_polyline} else {paths::transition_polyline})(&field::<Vec<[f64;3]>>(&v,"points")?,field(&v,"setback")?)?)}
    if op=="curve_round_polyline" {return encode((if optional_field::<bool>(&v,"closed")?.unwrap_or(false) {paths::closed_round_polyline} else {paths::round_polyline})(&field::<Vec<[f64;3]>>(&v,"points")?,field(&v,"radius")?)?)}
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
        let mappings = optional_field::<Vec<Option<Value>>>(&v,"section_mappings")?;
        let mapped = mappings.as_ref().map(|m| loft_reparameterization::prepare_with_budget(&curves,m,
            field(&v,"budget")?,optional_field(&v,"maxCells")?.unwrap_or(50_000),
            optional_field(&v,"maxMapEvaluations")?.unwrap_or(200_000))).transpose()?;
        let sections = mapped.as_ref().map_or(curves.as_slice(), |m| m.curves.as_slice());
        let parameters=field::<Vec<f64>>(&v,"parameters")?;
        let guides=field::<Vec<curve::Curve>>(&v,"guides")?;
        let budget=field::<f64>(&v,"budget")?;
        let parameter_tolerance=optional_field::<f64>(&v,"parameter_tolerance")?.unwrap_or(1e-8);
        let mut result=if op=="surface_auto_guided_loft_cartesian" {
            if let Some(controls)=control_tangents(&v)? {
                loft_alignment::interpolate_cartesian_with_control_tangents(sections,&parameters,&guides,&controls,
                    budget,parameter_tolerance,field(&v,"maxCells")?,field(&v,"maxMapEvaluations")?)?
            } else if let Some(targets)=optional_field::<[curve::Curve;2]>(&v,"boundary_tangents")? {
                loft_alignment::interpolate_cartesian_with_tangents(sections,&parameters,&guides,&targets,
                    budget,parameter_tolerance,field(&v,"maxCells")?,field(&v,"maxMapEvaluations")?)?
            } else {
                loft_alignment::interpolate_cartesian_with_parameter_tolerance(sections,&parameters,&guides,
                    budget,parameter_tolerance,field(&v,"maxCells")?,field(&v,"maxMapEvaluations")?)?
            }
        } else {
            loft_alignment::interpolate_with_parameter_tolerance(sections,&parameters,&guides,budget,parameter_tolerance)?
        };
        let original_section_certificates = if let Some(mappings) = mappings.as_ref() {
            let certificates = loft_reparameterization::certify_final_sections(&result.surface,
                &curves,&parameters,mappings,budget,
                optional_field(&v,"maxCells")?.unwrap_or(50_000),
                optional_field(&v,"maxMapEvaluations")?.unwrap_or(200_000))?;
            result.section_error_upper = certificates.iter().map(|c| c["errorUpper"].as_f64().unwrap()).collect();
            certificates
        } else {Vec::new()};
        let mut output = json!({"surface":result.surface,"guides":result.guides,"guide_parameters":result.guide_parameters,"guide_order":result.guide_order,"reversed":result.reversed,"section_error_upper":result.section_error_upper,"guide_error_upper":result.guide_error_upper});
        if op=="surface_auto_guided_loft_cartesian" {
            output["certificate"]=json!({"operation":"cartesian-auto-guided-loft","exact":false,
                "fittedToExactPromotion":false,"curves":result.curve_certificates,"tangents":result.tangent_certificates});
        }
        if !original_section_certificates.is_empty() {
            output["original_section_certificates"] = Value::Array(original_section_certificates);
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
        let originals=field::<Vec<curve::Curve>>(&v,"curves")?;
        let mappings=optional_field::<Vec<Option<Value>>>(&v,"section_mappings")?;
        let mapped=mappings.as_ref().map(|m|loft_reparameterization::prepare_with_budget(&originals,m,
            field(&v,"errorBudget")?,field(&v,"maxCells")?,field(&v,"maxMapEvaluations")?)).transpose()?;
        let sections=mapped.as_ref().map_or(originals.as_slice(),|m|m.curves.as_slice());
        let parameters=field::<Vec<f64>>(&v,"parameters")?;
        let guides=field::<Vec<curve::Curve>>(&v,"guides")?;
        let guide_parameters=field::<Vec<f64>>(&v,"guide_parameters")?;
        let tolerance=field(&v,"errorBudget")?;
        let max_cells=field(&v,"maxCells")?;
        let max_map_evaluations=field(&v,"maxMapEvaluations")?;
        let (surface,curves,tangents)=if let Some(controls)=control_tangents(&v)? {
            guided_loft::interpolate_cartesian_with_control_tangents(sections,&parameters,&guides,&guide_parameters,
                &controls,tolerance,max_cells,max_map_evaluations)?
        } else if let Some(targets)=optional_field::<[curve::Curve;2]>(&v,"boundary_tangents")? {
            guided_loft::interpolate_cartesian_with_tangents(sections,&parameters,&guides,&guide_parameters,
                &targets,tolerance,max_cells,max_map_evaluations)?
        } else {
            let (surface,curves)=guided_loft::interpolate_cartesian(sections,&parameters,&guides,&guide_parameters,
                tolerance,max_cells,max_map_evaluations)?;
            (surface,curves,Vec::new())
        };
        let original_sections=if let Some(mappings)=mappings.as_ref() {
            loft_reparameterization::certify_final_sections(&surface,&originals,&parameters,mappings,
                tolerance,max_cells,max_map_evaluations)?
        } else {Vec::new()};
        let mut output=json!({"surface":surface,"certificate":{"operation":"cartesian-guided-loft",
            "exact":false,"fittedToExactPromotion":false,"curves":curves,"tangents":tangents}});
        if let Some(mapped)=mapped {
            output["sections"]=encode(mapped.curves)?;
            output["section_mapping_certificates"]=Value::Array(mapped.certificates);
            output["original_section_certificates"]=Value::Array(original_sections);
        }
        return Ok(output);
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
    if op == "surface_natural_loft_checked" {
        let curves=field::<Vec<curve::Curve>>(&v,"curves")?;
        let parameters=field::<Vec<f64>>(&v,"parameters")?;
        let mappings=field::<Vec<Option<Value>>>(&v,"section_mappings")?;
        let prepared=loft_reparameterization::prepare(&curves,&mappings)?;
        let surface=natural_loft::interpolate(&prepared.curves,&parameters)?;
        let certificates=loft_reparameterization::certify_final_sections(&surface,&curves,&parameters,&mappings,1e-6,50_000,200_000)?;
        return Ok(json!({"surface":surface,"sections":prepared.curves,
            "section_mapping_certificates":prepared.certificates,"original_section_certificates":certificates,
            "certificate":{"operation":"mapped-natural-loft","exact":false,"fittedToExactPromotion":false}}));
    }
    if op == "surface_natural_loft" {
        let curves = field::<Vec<curve::Curve>>(&v,"curves")?;
        let parameters = field::<Vec<f64>>(&v,"parameters")?;
        return encode(match optional_field::<Vec<Option<Value>>>(&v,"section_mappings")? {
            Some(m) => loft_reparameterization::interpolate(&curves,&parameters,&m)?,
            None => natural_loft::interpolate(&curves,&parameters)?,
        });
    }
    if op == "surface_boundary_v_tangent_certify" {
        return gordon::certify_boundary_v_tangent(&field(&v,"surface")?,
            &field::<Vec<curve::Curve>>(&v,"targets")?,field(&v,"end")?,
            field(&v,"tolerance")?,field(&v,"maxCells")?);
    }
    if op == "surface_gordon_cartesian" {
        let u=field::<Vec<curve::Curve>>(&v,"u_curves")?;
        let guides=field::<Vec<curve::Curve>>(&v,"v_curves")?;
        let pu=field::<Vec<f64>>(&v,"parameters_u")?;
        let pv=field::<Vec<f64>>(&v,"parameters_v")?;
        let tolerance=field::<f64>(&v,"errorBudget")?;
        let max_cells=field::<usize>(&v,"maxCells")?;
        let max_map_evaluations=field::<usize>(&v,"maxMapEvaluations")?;
        let (surface,curves,tangents)=if let Some(targets)=optional_field::<[curve::Curve;2]>(&v,"boundary_tangents")? {
            gordon::patch_cartesian_with_tangents(&u,&guides,&pu,&pv,&targets,tolerance,max_cells,max_map_evaluations)?
        } else {
            let (surface,curves)=gordon::patch_cartesian(&u,&guides,&pu,&pv,tolerance,max_cells,max_map_evaluations)?;
            (surface,curves,Vec::new())
        };
        return Ok(json!({"surface":surface,"certificate":{"operation":"cartesian-gordon",
            "exact":false,"fittedToExactPromotion":false,"curves":curves,"tangents":tangents}}));
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
#[path="surface_offset_transport_tests.rs"]
mod surface_offset_transport_tests;
