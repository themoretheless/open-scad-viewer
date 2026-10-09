use super::*;

pub(super) fn dispatch(v: Value, op: String) -> Result<Value> {
if matches!(op.as_str(), "sweep_error_upper_compose") {
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
    }
if matches!(op.as_str(), "sweep_exact_seam_set_audit" | "sweep_projective_seam_set_audit") {
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
    }
if op == "surface_progressive_sweep_spatial_rmf_error" || op == "surface_progressive_sweep" || op == "surface_progressive_sweep_profiles" || op=="surface_progressive_sweep_level" || op=="surface_progressive_sweep_cap_domains" || op=="surface_progressive_sweep_cap_projection" || op=="surface_progressive_sweep_station_seams" || op=="surface_progressive_sweep_profile_join" || op=="surface_progressive_sweep_decomposition_joins" || op=="surface_progressive_sweep_decomposition_smoothness" || op=="surface_progressive_sweep_frame_smoothness" || op=="surface_progressive_sweep_closed_frame_smoothness" || op=="surface_progressive_sweep_closed_path_frame_smoothness" || op=="surface_progressive_sweep_closed_guided_frame_smoothness" {
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
        let rmf_steps=optional_field::<usize>(&v,"rmf_transport_steps")?;
        let rmf_cells=optional_field::<usize>(&v,"error_max_cells")?;
        let rmf_products=optional_field::<usize>(&v,"error_max_products")?;
        check(rmf_steps.is_some() || (rmf_cells.is_none() && rmf_products.is_none()),
            "RMF error budgets require rmf_transport_steps")?;
        let rmf_limits=if let Some(steps)=rmf_steps {
            check(["surface_progressive_sweep","surface_progressive_sweep_profiles","surface_progressive_sweep_level"].contains(&op.as_str()),
                "RMF proof options require a sweep construction operation")?;
            Some([steps,field::<usize>(&v,"error_max_cells")?,field::<usize>(&v,"error_max_products")?])
        }else{None};
        let axes=optional_field::<curve::Curve>(&v,"axis_scale")?;
        let center=optional_field::<curve::Curve>(&v,"center_law")?;
        let contact=optional_field::<f64>(&v,"contact_parameter")?;
        let contact_profile=optional_field::<usize>(&v,"contact_profile")?;
        check(contact.is_some() || contact_profile.is_none(), "Contact profile requires contact parameter")?;
        check(contact.is_none() || optional_field::<curve::Curve>(&v,"orientation_guide")?.is_some(), "Contact anchor requires orientation guide")?;
        if op=="surface_progressive_sweep_spatial_rmf_error" {
            check(optional_field::<curve::Curve>(&v,"orientation_guide")?.is_none()
                && contact.is_none() && field::<String>(&v,"orientation")?=="rmf",
                "Spatial RMF error requires unguided RMF")?;
            let profile=field::<curve::Curve>(&v,"profile")?;
            let path=field::<curve::Curve>(&v,"path")?;
            let scale=field::<curve::Curve>(&v,"scale")?;
            let twist=field::<curve::Curve>(&v,"twist")?;
            let default_axes=progressive_sweep::constant_vector_law([1.;3])?;
            let default_center=progressive_sweep::constant_vector_law([0.;3])?;
            let mut sweep=progressive_sweep::Sweep::new(&profile,&path,&scale,&twist,options)?;
            if axes.is_some() || center.is_some(){sweep=sweep.with_affine_laws(axes.as_ref().unwrap_or(&default_axes),center.as_ref().unwrap_or(&default_center))?;}
            let r=sweep.rmf_spatial_patch_error_bound(field(&v,"preview_sections")?,field(&v,"transport_steps")?,field(&v,"maxCells")?,field(&v,"maxProducts")?)?;
            return Ok(json!({"continuousBound":r.status==progressive_miter::scalar_certificate::Status::Certified,
                "withinBudget":r.within_budget,"errorUpper":r.error_upper,"cells":r.cells,"products":r.products,
                "originalSectionEndpointErrorUpper":r.original_section_endpoint_error_upper,"patches":r.patches,"reason":r.reason,
                "scope":if options.spacing==progressive_sweep::Spacing::Parameter {"retained-patches-relative-to-original-spatial-rmf-parameter-transport"} else {"retained-patches-relative-to-original-spatial-rmf-arc-transport"},
                "method":"original-bishop-skew-flow-and-retained-section-decomposition",
                "capJoinsCertified":false,"globalEmbeddingCertified":false,"solidCertified":false}));
        }
        if op=="surface_progressive_sweep_level" || op=="surface_progressive_sweep_cap_domains" || op=="surface_progressive_sweep_cap_projection" || op=="surface_progressive_sweep_station_seams" || op=="surface_progressive_sweep_profile_join" || op=="surface_progressive_sweep_decomposition_joins" || op=="surface_progressive_sweep_decomposition_smoothness" || op=="surface_progressive_sweep_frame_smoothness" || op=="surface_progressive_sweep_closed_frame_smoothness" || op=="surface_progressive_sweep_closed_path_frame_smoothness" || op=="surface_progressive_sweep_closed_guided_frame_smoothness" {
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
            if let Some([steps,cells,products])=rmf_limits {sweep=sweep.with_spatial_rmf_error_limits(steps,cells,products)?;}
            if op=="surface_progressive_sweep_cap_projection" {
                let loops=field::<Vec<usize>>(&v,"loopSizes")?;
                let caps=field::<[surface::Surface;2]>(&v,"caps")?;
                let r=sweep.certify_endpoint_cap_projection(&loops,&caps,field(&v,"tolerance")?,field(&v,"maxPairs")?,field(&v,"maxCells")?,field(&v,"maxExactWork")?)?;
                return Ok(json!({"capProjectionCertified":r.normal_dots.is_some(),
                    "idealCapDomainsCertified":r.original.domains.ideal_endpoint_domains_certified,
                    "endpointNormals":r.original.endpoint_normals,"normalDots":r.normal_dots,
                    "reversesOrientation":r.reverses_orientation,"cells":r.cells,"exactWork":r.exact_work,"reason":r.reason,
                    "method":"original-progressive-endpoint-plane-projection",
                    "continuousBound":false,"retainedCapRegionsCertified":false,"globalEmbeddingCertified":false,"solidCertified":false}));
            }
            if op=="surface_progressive_sweep_cap_domains" {
                let loops=field::<Vec<usize>>(&v,"loopSizes")?;
                let r=sweep.certify_ideal_endpoint_domains(&loops,field(&v,"tolerance")?,field(&v,"maxPairs")?,field(&v,"maxCells")?,field(&v,"maxExactWork")?)?;
                return Ok(json!({"idealCapDomainsCertified":r.ideal_endpoint_domains_certified,
                    "localDomainCertified":r.source.local_domain_certified,
                    "sourcePlaneAxis":r.source.source_plane_axis,"endpointFrameAxes":r.endpoint_frame_axes,
                    "cells":r.cells,"pairs":r.source.pairs,"exactWork":r.source.exact_work,"reason":r.reason,
                    "method":"original-progressive-endpoint-material-domains",
                    "continuousBound":false,"retainedCapRegionsCertified":false,"globalEmbeddingCertified":false,"solidCertified":false}));
            }
            if op=="surface_progressive_sweep_closed_guided_frame_smoothness" {
                let r=sweep.certify_closed_guided_frame_smoothness(field(&v,"order")?,field(&v,"maxCells")?,field(&v,"maxExactWork")?)?;
                return Ok(json!({"requestedOrder":r.order,
                    "closedSourceFrameSmoothnessCertified":r.closed_source_frame_smoothness_certified,
                    "cells":r.cells,"exactWork":r.exact_work,"reason":r.reason,
                    "method":"exact-original-guided-endpoint-jets-and-joint-frame-cover",
                    "scope":"closed-original-guided-frame-only",
                    "pathSeamCertified":false,"retainedSeamsCertified":false,
                    "profileJoinsCertified":false,"capJoinsCertified":false,
                    "continuousBound":false,"solidCertified":false}));
            }
            if op=="surface_progressive_sweep_closed_path_frame_smoothness" {
                let r=sweep.certify_closed_path_frame_smoothness(field(&v,"order")?,field(&v,"maxCells")?,field(&v,"maxExactWork")?)?;
                return Ok(json!({"requestedOrder":r.order,
                    "closedSourceFrameSmoothnessCertified":r.closed_source_frame_smoothness_certified,
                    "cells":r.cells,"exactWork":r.exact_work,"reason":r.reason,
                    "method":"exact-original-path-twist-endpoint-jets-and-frame-cover",
                    "scope":"closed-original-path-frame-only",
                    "pathSeamCertified":false,"retainedSeamsCertified":false,
                    "profileJoinsCertified":false,"capJoinsCertified":false,
                    "continuousBound":false,"solidCertified":false}));
            }
            if op=="surface_progressive_sweep_closed_frame_smoothness" {
                let r=sweep.certify_closed_authored_frame_smoothness(field(&v,"order")?,field(&v,"maxCells")?,field(&v,"maxExactWork")?)?;
                return Ok(json!({"requestedOrder":r.order,
                    "closedSourceFrameSmoothnessCertified":r.closed_source_frame_smoothness_certified,
                    "cells":r.cells,"exactWork":r.exact_work,"reason":r.reason,
                    "method":"exact-original-authored-endpoint-jets-and-frame-cover",
                    "scope":"closed-original-authored-frame-only",
                    "pathSeamCertified":false,"retainedSeamsCertified":false,
                    "profileJoinsCertified":false,"capJoinsCertified":false,
                    "continuousBound":false,"solidCertified":false}));
            }
            if op=="surface_progressive_sweep_frame_smoothness" {
                let r=sweep.certify_original_frame_smoothness(field(&v,"order")?,field(&v,"maxCells")?,field(&v,"maxExactWork")?)?;
                return Ok(json!({"requestedOrder":r.order,
                    "sourceFrameSmoothnessCertified":r.source_frame_smoothness_certified,
                    "cells":r.cells,"exactWork":r.exact_work,"reason":r.reason,
                    "method":"original-frame-continuity-and-nondegeneracy-cover",
                    "scope":"open-original-frame-only",
                    "retainedSeamsCertified":false,"profileJoinsCertified":false,
                    "capJoinsCertified":false,"continuousBound":false,"solidCertified":false}));
            }
            if op=="surface_progressive_sweep_decomposition_smoothness" {
                let level=sweep.preview_at(field::<usize>(&v,"preview_sections")?)?;
                let r=level.certify_retained_decomposition_smoothness(field(&v,"transverseScale")?,field(&v,"maxExactWork")?)?;
                let mut result=encode(r)?;
                result["profilePatchRanges"]=encode(level.profile_patch_ranges)?;
                return Ok(result);
            }
            if op=="surface_progressive_sweep_decomposition_joins" {
                let level=sweep.preview_at(field::<usize>(&v,"preview_sections")?)?;
                let r=level.certify_retained_decomposition_joins(field(&v,"order")?,field(&v,"transverseScale")?,field(&v,"maxExactWork")?)?;
                let joins=r.joins.iter().map(|(pair,p)|json!({"leftPatch":pair[0],"rightPatch":pair[1],
                    "certified":p.certified,"exactIdentity":p.exact_identity,
                    "regularityCertified":p.regularity_certified,"exactWork":p.work,"reason":p.reason})).collect::<Vec<_>>();
                return Ok(json!({"requestedOrder":r.order,"expectedJoins":r.expected_joins,
                    "checkedJoins":joins.len(),"coverageComplete":r.coverage_complete,
                    "decompositionJoinsCertified":r.all_joins_certified,"exactWork":r.exact_work,
                    "joins":joins,"reason":r.reason,"profilePatchRanges":level.profile_patch_ranges,
                    "method":"exact-retained-decomposition-strip-jets",
                    "scope":"within-source-profile-decomposition-only",
                    "allProfileJoinsCertified":false,"closedProfileSeamsCertified":false,
                    "sourceFrameSmoothnessCertified":false,"capJoinsCertified":false,
                    "continuousBound":false,"solidCertified":false}));
            }
            if op=="surface_progressive_sweep_profile_join" {
                let level=sweep.preview_at(field::<usize>(&v,"preview_sections")?)?;
                let left:usize=field(&v,"leftPatch")?;
                let right:usize=field(&v,"rightPatch")?;
                let order:usize=field(&v,"order")?;
                let ranges=level.profile_patch_ranges;
                let r=progressive_sweep::Level {patches:level.patches,report:level.report}
                    .certify_retained_profile_join(left,right,order,field(&v,"transverseScale")?,field(&v,"maxExactWork")?)?;
                return Ok(json!({"requestedOrder":order,"leftPatch":left,"rightPatch":right,
                    "profilePatchRanges":ranges,"certified":r.certified,
                    "exactIdentity":r.exact_identity,"regularityCertified":r.regularity_certified,
                    "exactWork":r.work,"reason":r.reason,
                    "method":"exact-retained-profile-strip-jets","scope":"explicit-retained-profile-join-only",
                    "allProfileJoinsCertified":false,"sourceFrameSmoothnessCertified":false,
                    "capJoinsCertified":false,"continuousBound":false,"solidCertified":false}));
            }
            if op=="surface_progressive_sweep_station_seams" {
                let level=sweep.preview_at(field::<usize>(&v,"preview_sections")?)?;
                return encode(progressive_sweep::Level {patches:level.patches,report:level.report}
                    .certify_retained_station_seams(field(&v,"order")?,field(&v,"maxExactWork")?)?);
            }
            let mut result=encode(sweep.preview_at(field::<usize>(&v,"preview_sections")?)?)?;
            if let Some([_,cells,products])=rmf_limits {
                result["report"]["errorCertificateMaxCells"]=json!(cells);
                result["report"]["decompositionMaxProducts"]=json!(products);
            }
            return Ok(result);
        }
        if let Some([steps,cells,products])=rmf_limits {
            check(optional_field::<curve::Curve>(&v,"orientation_guide")?.is_none()
                && contact.is_none() && field::<String>(&v,"orientation")?=="rmf",
                "Spatial RMF proof options require unguided RMF")?;
            let profiles=if op=="surface_progressive_sweep_profiles" {field::<Vec<curve::Curve>>(&v,"profiles")?}else{vec![field::<curve::Curve>(&v,"profile")?]};
            let path=field::<curve::Curve>(&v,"path")?;let scale=field::<curve::Curve>(&v,"scale")?;let twist=field::<curve::Curve>(&v,"twist")?;
            let default_axes=progressive_sweep::constant_vector_law([1.;3])?;let default_center=progressive_sweep::constant_vector_law([0.;3])?;
            let mut sweep=progressive_sweep::MultiSweep::new(&profiles,&path,&scale,&twist,options)?;
            if axes.is_some() || center.is_some(){sweep=sweep.with_affine_laws(axes.as_ref().unwrap_or(&default_axes),center.as_ref().unwrap_or(&default_center))?;}
            let mut result=encode(progressive_sweep::collect_profiles(sweep.with_spatial_rmf_error_limits(steps,cells,products)?)?)?;
            result["report"]["errorCertificateMaxCells"]=json!(cells);
            result["report"]["decompositionMaxProducts"]=json!(products);
            if let Some(levels)=result["levels"].as_array_mut(){for level in levels {
                level["errorCertificateMaxCells"]=json!(cells);level["decompositionMaxProducts"]=json!(products);
            }}
            return Ok(result);
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
if op == "surface_twist_sweep" {return encode(twist_sweep::sweep(&field(&v,"profile")?,&field(&v,"path")?,field(&v,"origin")?,field(&v,"axis")?,field(&v,"start_degrees")?,field(&v,"sweep_degrees")?)?);}
if op == "surface_two_guide_sweep" {return encode(two_guide_sweep::sweep(&field(&v,"profile")?,&field(&v,"guide_a")?,&field(&v,"guide_b")?,field(&v,"width")?,field(&v,"axis_y")?,field(&v,"axis_z")?)?);}
if op=="sweep_repair_circle_sections" {
        let r=crate::circle_sweep_repair::repair(&field::<Vec<Vec<curve::Curve>>>(&v,"sections")?,field(&v,"quantum")?,
            field(&v,"tolerance")?,v.get("maxWork").and_then(Value::as_f64))?;
        return Ok(json!({"sections":r.sections,"wallDisplacementUpper":r.wall_displacement_upper,"work":r.work,"reason":r.reason}));
    }
if op == "sweep_retained_patch_regularity" {
        let r=progressive_sweep::inspect_retained_regularity(
            &field::<Vec<surface::Surface>>(&v,"patches")?,field(&v,"maxCells")?,
        )?;
        return encode(json!({"surfaceRegularityCertified":r.spanwise_regular,
            "cells":r.cells,"unresolvedPatches":r.unresolved_patches,
            "continuousBound":false,"globalEmbeddingCertified":false,"solidCertified":false,
            "method":"actual-retained-patch-shared-jacobian-regularity"}));
    }
if op == "sweep_frenet_frame_regularity" {
        let r=progressive_miter::authored_frame_certificate::certify_frenet_cover(
            &field::<curve::Curve>(&v,"path")?,&field::<curve::Curve>(&v,"twist")?,field(&v,"maxCells")?,
        )?;
        return encode(json!({"regularityCertified":r.status==progressive_miter::scalar_certificate::Status::Certified,
            "cells":r.cells,"certifiedIntervals":r.certified_intervals,"reason":r.reason,
            "continuousBound":false,"surfaceRegularityCertified":false,"globalEmbeddingCertified":false,
            "method":"original-path-adaptive-frenet-frame-regularity"}));
    }
if op == "sweep_fixed_normal_frame_regularity" {
        let r=progressive_miter::authored_frame_certificate::certify_fixed_normal_cover(
            &field::<curve::Curve>(&v,"path")?,field(&v,"normal")?,
            &field::<curve::Curve>(&v,"twist")?,field(&v,"maxCells")?,
        )?;
        return encode(json!({
            "regularityCertified":r.status==progressive_miter::scalar_certificate::Status::Certified,
            "cells":r.cells,"certifiedIntervals":r.certified_intervals,"reason":r.reason,
            "continuousBound":false,"surfaceRegularityCertified":false,"globalEmbeddingCertified":false,
            "method":"original-path-adaptive-fixed-normal-frame-regularity"
        }));
    }
if op == "sweep_authored_frame_regularity" {
        let r = progressive_miter::authored_frame_certificate::certify_regularity(
            &field::<curve::Curve>(&v, "longitudinal")?,
            &field::<curve::Curve>(&v, "transverse")?,
            field(&v, "maxCells")?,
        )?;
        return encode(json!({
            "regularityCertified": r.status == progressive_miter::scalar_certificate::Status::Certified,
            "cells": r.cells,
            "certifiedIntervals": r.certified_intervals,
            "reason": r.reason,
            "continuousBound": false,
            "method": "original-law-adaptive-authored-frame-regularity"
        }));
    }
if op=="sweep_correct_miter_sections" {
        use crate::miter_section_correction::{self,Budget,CapCorrection};
        let budget=|b:&Value|->Result<Budget> {Ok(Budget {quantum:field(b,"quantum")?,tolerance:field(b,"tolerance")?,max_work:b.get("maxWork").and_then(Value::as_f64)})};
        let circle=optional_field::<Value>(&v,"circleCorrection")?.as_ref().map(budget).transpose()?;
        let cap=optional_field::<Value>(&v,"capCorrection")?.as_ref().map(|b|->Result<_> {Ok(CapCorrection {budget:budget(b)?,authored_frame:optional_field::<bool>(b,"authoredFrame")?.unwrap_or(false)})}).transpose()?;
        let axis=optional_field::<curve::Curve>(&v,"frameAxis")?;
        let r=miter_section_correction::correct(&field::<Vec<Vec<curve::Curve>>>(&v,"sections")?,&field::<Vec<[f64;3]>>(&v,"points")?,field(&v,"closed")?,circle,cap,axis.as_ref())?;
        return Ok(r.map_or(Value::Null,|r|json!({"sections":r.sections,"wallDisplacementUpper":r.wall_displacement_upper,
            "exactPlanarSections":r.exact_planar_sections,"work":r.work,"reason":r.reason})));
    }
if op=="sweep_project_miter_caps" {
        let axis=optional_field::<curve::Curve>(&v,"frameAxis")?;
        let r=crate::sweep_section_projection::project_miter_caps(&field::<Vec<Vec<curve::Curve>>>(&v,"sections")?,
            &field::<Vec<[f64;3]>>(&v,"points")?,field(&v,"closed")?,field(&v,"authoredFrame")?,axis.as_ref(),
            field(&v,"quantum")?,field(&v,"tolerance")?,v.get("maxWork").and_then(Value::as_f64))?;
        return Ok(json!({"sections":r.sections,"wallDisplacementUpper":r.wall_displacement_upper,
            "exactPlanarSections":r.exact_planar_sections,"work":r.work,"reason":r.reason}));
    }
if op=="sweep_project_sections" {
        use crate::sweep_section_projection::{self,Correction,Target};
        let sections=field::<Vec<Vec<curve::Curve>>>(&v,"sections")?;
        let corrections=field::<Vec<Value>>(&v,"corrections")?.iter().map(|c| {
            let target=if let Some(p)=c.get("plane").filter(|v|!v.is_null()) {
                Target::Plane {axis:field(p,"axis")?,coefficients:field(p,"coefficients")?,offset:field(p,"offset")?}
            } else {
                Target::AuthoredAxis {axis:field(c,"frameAxis")?,traversal:field(c,"traversal")?}
            };
            Ok(Correction {section:field(c,"section")?,quantum:field(c,"quantum")?,tolerance:field(c,"tolerance")?,target})
        }).collect::<Result<Vec<_>>>()?;
        let r=sweep_section_projection::project(&sections,&corrections,v.get("maxWork").and_then(Value::as_f64))?;
        return Ok(json!({"sections":r.sections,"wallDisplacementUpper":r.wall_displacement_upper,
            "exactPlanarSections":r.exact_planar_sections,"work":r.work,"reason":r.reason}));
    }
if op=="curve_miter_sections" {return encode((if optional_field::<bool>(&v,"closed")?.unwrap_or(false) {paths::closed_miter_sections} else {paths::miter_sections})(&field::<Vec<curve::Curve>>(&v,"profiles")?,&field::<Vec<[f64;3]>>(&v,"points")?,field(&v,"normal")?,field(&v,"miter_limit")?)?)}
if op == "surface_scaled_sweep" {
        return encode(sweeps::scaled_sweep(&field(&v, "profile")?, &field(&v, "path")?, &field(&v, "scale")?, field(&v, "origin")?)?);
    }
if op == "surface_profile_sweep" {
        return sweeps::checked_profile_sweep_with_cells(&field(&v, "profile")?, &field(&v, "path")?, &field(&v, "scale")?, field(&v, "normal")?, field(&v, "sections")?, match optional_field::<f64>(&v, "maxDeviation")? {Some(x)=>x,None=>field(&v,"max_deviation")?},optional_field::<usize>(&v,"maxCells")?.unwrap_or(16384));
    }
if op == "surface_framed_sweep" {
        return encode(framed_sweep::checked_sweep(
            &field(&v, "profile")?,
            &field(&v, "path")?,
            field(&v, "normal")?,
            field(&v, "sections")?,
            field(&v, "maxDeviation")?,
        )?);
    }
if op == "surface_sweep" {
        return encode(surface::sweep(&field(&v, "profile")?, &field(&v, "path")?)?);
    }
Err(input("Unknown NURBS operation"))
}
