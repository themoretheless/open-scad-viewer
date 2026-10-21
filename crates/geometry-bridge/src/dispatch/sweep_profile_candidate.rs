//! Original-source candidate constructions, without solid admission claims.
use crate::*;
pub(super) fn build(mut v:Value)->Result<Value> {
            use nurbs_core::progressive_sweep::{Options, Orientation, Spacing};
            let automatic_mode=v["op"].as_str().is_some_and(|op|op.contains("profile_auto_body"));
            let annular_mode=v["op"].as_str().is_some_and(|op|op.contains("profile_annular_body")||automatic_mode);
            if annular_mode {
                for key in ["model","sections","boundaryCertificate","filledCapErrorUpper","retainedAnnularFilling",
                    "capFaceGroups","capFaces","planarCapFaces","continuousBound","boundaryErrorUpper",
                    "boundaryContinuousBound","boundaryErrorWithinBudget","globalEmbeddingCertified","retainedCaps",
                    "retainedWalls","capProjection","retainedSmoothness","stationReconstruction","approximation","capSelectionExactWork","capRepresentation"] {
                    if v.get(key).is_some() {return Err(input("Annular progressive body requires original source, not supplied geometry/proofs"));}
                }
            }
            let orientation = match field::<String>(&v, "orientation")?.as_str() {
                "rmf" => Orientation::RotationMinimizing,
                "fixed" | "authored" => Orientation::Fixed,
                "fixed_normal" => Orientation::FixedNormal,
                "frenet" => Orientation::Frenet,
                "corrected_frenet" => Orientation::CorrectedFrenet,
                _ => {
                    return Err(Error::new(
                        "BREP_RATIONAL_SWEEP_REFUSED",
                        "Unknown orientation",
                    ));
                }
            };
            let spacing = match field::<String>(&v, "spacing")?.as_str() {
                "parameter" => Spacing::Parameter,
                "arc_length" => Spacing::ArcLength {
                    tolerance: field(&v, "length_tolerance")?,
                    max_cells: field(&v, "length_max_cells")?,
                },
                _ => return Err(Error::new("BREP_RATIONAL_SWEEP_REFUSED", "Unknown spacing")),
            };
            let options = Options {
                normal: field(&v, "normal")?,
                orientation,
                spacing,
                initial_sections: field(&v, "initial_sections")?,
                max_sections: field(&v, "max_sections")?,
                max_deviation: field(&v, "max_deviation")?,
            };
            let loops = field::<Vec<Vec<Curve>>>(&v, "loops")?;
            let path = field::<Curve>(&v, "path")?;
            let scale = field::<Curve>(&v, "scale")?;
            let twist = field::<Curve>(&v, "twist")?;
            let axes = optional_field::<Curve>(&v, "axis_scale")?;
            let center = optional_field::<Curve>(&v, "center_law")?;
            let guide = optional_field::<Curve>(&v, "orientation_guide")?;
            let contact = optional_field::<f64>(&v, "contact_parameter")?;
            let contact_profile = optional_field::<usize>(&v, "contact_profile")?;
            if contact.is_some() && guide.is_none()
                || contact_profile.is_some() && contact.is_none()
            {
                return Err(Error::new(
                    "BREP_RATIONAL_SWEEP_REFUSED",
                    "Contact anchor requires orientation guide and parameter",
                ));
            }
            let authored = field::<String>(&v, "orientation")? == "authored";
            if authored && guide.is_some() {
                return Err(Error::new(
                    "BREP_RATIONAL_SWEEP_REFUSED",
                    "Orientation guide cannot be combined with authored frames",
                ));
            }
            let frame_axis = if authored {
                Some(field::<Curve>(&v, "frame_axis")?)
            } else {
                None
            };
            let frame_normal = if authored {
                Some(field::<Curve>(&v, "frame_normal")?)
            } else {
                None
            };
            let default_axes = nurbs_core::progressive_sweep::constant_vector_law([1.; 3])?;
            let default_center = nurbs_core::progressive_sweep::constant_vector_law([0.; 3])?;
            let affine = if authored || guide.is_some() || axes.is_some() || center.is_some() {
                Some((
                    axes.as_ref().unwrap_or(&default_axes),
                    center.as_ref().unwrap_or(&default_center),
                ))
            } else {
                None
            };
            let options = if authored {
                Options {
                    orientation: Orientation::Fixed,
                    ..options
                }
            } else {
                options
            };
            let tolerance = optional_field::<f64>(&v, "cap_correction_tolerance")?;
            let quantum = optional_field::<f64>(&v, "cap_correction_quantum")?;
            let max_work = optional_field::<u64>(&v, "cap_correction_max_work")?;
            if tolerance.is_none() && (quantum.is_some() || max_work.is_some()) {
                return Err(Error::new(
                    "BREP_RATIONAL_SWEEP_REFUSED",
                    "Cap correction requires a displacement tolerance",
                ));
            }
            let correction =
                tolerance.map(|tolerance| brep_core::analytic::EndpointCapCorrection {
                    tolerance,
                    quantum: quantum.unwrap_or(2_f64.powi(-40)),
                    max_work: max_work.unwrap_or(1000000),
                });
            let circle_tolerance=optional_field::<f64>(&v,"circle_correction_tolerance")?;
            let circle_quantum=optional_field::<f64>(&v,"circle_correction_quantum")?;
            let circle_work=optional_field::<u64>(&v,"circle_correction_max_work")?;
            if circle_tolerance.is_none() && (circle_quantum.is_some() || circle_work.is_some()) {
                return Err(Error::new("BREP_RATIONAL_SWEEP_REFUSED","Profile circle correction requires a displacement tolerance"));
            }
            let profile_correction=circle_tolerance.map(|tolerance|brep_core::analytic::EndpointCapCorrection {
                tolerance,quantum:circle_quantum.unwrap_or(2_f64.powi(-40)),max_work:circle_work.unwrap_or(1000000),
            });
            let rmf_steps = optional_field::<usize>(&v, "rmf_transport_steps")?;
            let rmf_cells = optional_field::<usize>(&v, "error_max_cells")?;
            let rmf_products = optional_field::<usize>(&v, "error_max_products")?;
            let rmf_policy = match (rmf_steps, rmf_cells, rmf_products) {
                (None, None, None) => None,
                (Some(steps), Some(cells), Some(products))
                    if field::<String>(&v, "orientation")? == "rmf" =>
                {
                    Some((steps, cells, products))
                }
                _ => {
                    return Err(Error::new(
                        "BREP_RATIONAL_SWEEP_REFUSED",
                        "Spatial RMF policy requires unguided RMF and three proof budgets",
                    ));
                }
            };
            let guidance=guide.as_ref().map(|g|(g,contact.map(|p|(contact_profile.unwrap_or(0),p))));
            let owned=if annular_mode {
                let construct=if automatic_mode {brep_core::analytic::progressive_profile_body_with_automatic_caps}else{brep_core::analytic::progressive_profile_body_with_annular_caps};
                Some(construct(
                    &loops,&path,&scale,&twist,affine,frame_axis.as_ref().zip(frame_normal.as_ref()),
                    guidance,options,correction,profile_correction,rmf_policy,
                    brep_core::retained_annular_cap::reference_filling::Limits {
                        exact_work:optional_field(&v,"annular_cap_max_work")?.unwrap_or(1000000),
                        products:optional_field(&v,"annular_cap_max_products")?.unwrap_or(100000),
                        trim_pairs:optional_field(&v,"annular_cap_max_pairs")?.unwrap_or(10000),
                        trim_cells:optional_field(&v,"annular_cap_max_cells")?.unwrap_or(100000),
                        trim_domain_cells:1000000,spans:1024,linear_cells:100000,
                        contacts:brep_core::face_contacts::Limits {pairs:10000,cells:100000,domain_cells:1000000,
                            cells_per_pair:1000,domain_cells_per_pair:10000},
                    },
                )?)
            }else{None};
            let legacy=if annular_mode {None}else{
                Some(brep_core::analytic::progressive_profile_body_with_rmf_policy_and_corrections(
                    &loops,&path,&scale,&twist,affine,frame_axis.as_ref().zip(frame_normal.as_ref()),
                    guidance,options,correction,profile_correction,rmf_policy,
                )?)
            };
            let result=owned.as_ref().map(|body|body.evidence()).or(legacy.as_ref()).unwrap();
            // Replay the complete original constructor, never caller-authored sections,
            // retained geometry or a scalar certificate. The base receipt stays separate.
            let station_reconstruction=if v["op"].as_str().is_some_and(|op|op.ends_with("_reconstruct_stations")) {
                if let Some(expected)=optional_field::<brep_core::Model>(&v,"expected_source_model")? {
                    if expected!=result.model {
                        return Err(input("Station reconstruction source model differs from native original constructor"));
                    }
                }
                let report=result.reconstruct_stations(
                    field(&v,"station_quantum")?,field(&v,"station_tolerance")?,field(&v,"station_max_work")?,
                )?;
                let closed=result.approximation.levels.last().unwrap().closed_path;
                let actual_smoothness=if let Some(model)=report.reconstruction.as_ref().and_then(|r|r.model.as_ref()) {
                    let caps=owned.as_ref().map(|body|body.cap_faces()).unwrap_or_else(||
                        if closed {Vec::new()}else{vec![model.faces.len()-2,model.faces.len()-1]});
                    Some(cad_sweep_smoothness::diagnose(json!({"model":model,"capFaces":caps,
                        "maxWork":field::<u64>(&v,"smoothness_max_work")?}),true)?)
                }else{None};
                Some(json!({"reason":report.reason,"source":"original-progressive-profile-request",
                    "globalEmbeddingCertified":false,"profileSmoothness":actual_smoothness,
                    "reconstruction":report.reconstruction.map(|r|json!({"model":r.model,
                        "wallDisplacementUpper":r.candidate.wall_displacement_upper,
                        "work":r.candidate.work,"reason":r.candidate.reason})),
                    "boundaryCertificate":report.boundary.map(|b|json!({
                        "method":"retained-sweep-boundary-union","scope":"boundary-set-hausdorff",
                        "continuousBound":b.continuous_bound,"withinBudget":b.within_budget,
                        "errorUpper":b.error_upper,"wallErrorUpper":b.wall_error_upper,
                        "filledCapErrorUpper":b.filled_cap_error_upper,"reason":b.reason,
                        "closed":closed,"budget":options.max_deviation}))}))
            }else{None};
            let retained_caps=result.retained_caps.as_ref().map(|r|json!({"exact":r.exact,
                "capErrorUpper":if r.exact {Some(0.)}else{None},"exactWork":r.exact_work,"inspectedEdges":r.inspected_edges,"reason":r.reason,
                "scope":"constructor-owned-retained-endpoint-regions","continuousBound":false,"globalEmbeddingCertified":false}));
            let cap_projection = result.cap_projection.as_ref().map(|r| {
                json!({
                "idealCapDomainsCertified":r.original.domains.ideal_endpoint_domains_certified,
                "normalDots":r.normal_dots,"reversesOrientation":r.reverses_orientation,
                "cells":r.cells,"exactWork":r.exact_work,"reason":r.reason,
                "scope":"constructor-owned-endpoint-plane-projection"})
            });
            let mut output = encode(
                json!({"model":result.model,"approximation":result.approximation,"retainedCaps":retained_caps,
                "capProjection":cap_projection,"filledCapErrorUpper":result.filled_cap_error_upper,
                "capCorrectionErrorUpper":result.cap_correction_error_upper,
                "profileCorrectionErrorUpper":result.profile_correction_error_upper,
                "bodyDecompositionErrorUpper":result.body_decomposition_error_upper,
                "bodyDecompositionProducts":result.body_decomposition_products,
                "retainedWalls":{"certified":result.retained_walls.certified,"faceCoverageCertified":result.retained_walls.face_coverage_certified,
                    "coefficientFamilyCertified":result.retained_walls.coefficient_family_certified,"inspectedFaces":result.retained_walls.inspected_faces,
                    "exactWork":result.retained_walls.exact_work,"reason":result.retained_walls.reason},
                "boundaryErrorUpper":result.boundary_error_upper,"boundaryContinuousBound":result.boundary_error_upper.is_some(),
                "boundaryErrorWithinBudget":result.boundary_error_within_budget,
                "boundaryErrorScope":"constructor-owned-retained-wall-and-cap-union","globalEmbeddingCertified":false}),
            )?;
            if let Some((_, cells, products)) = rmf_policy {
                output["approximation"]["report"]["errorCertificateMaxCells"] = json!(cells);
                output["approximation"]["report"]["decompositionMaxProducts"] = json!(products);
                if let Some(levels) = output["approximation"]["levels"].as_array_mut() {
                    for level in levels {
                        level["errorCertificateMaxCells"] = json!(cells);
                        level["decompositionMaxProducts"] = json!(products);
                    }
                }
            }
            if let Some(body)=&owned {
                output["capFaces"]=json!(body.cap_faces());
                output["planarCapFaces"]=json!(body.planar_cap_faces());
                output["capSelectionExactWork"]=json!(body.cap_selection_work());
                output["capFaceGroups"]=json!(body.cap_face_groups());
                output["capRepresentation"]=json!(if body.caps().is_some(){"radial-annular"}else if result.approximation.levels.last().unwrap().closed_path {"closed"}else{"legacy"});
                output["retainedAnnularFilling"]=body.caps().map_or(Value::Null,|caps| {
                    let proof=caps.filling();
                    json!({"method":"authored-smooth-endpoint-annular-filling",
                        "sourceFillingCovered":proof.source_filling_covered,"filledCapErrorUpper":proof.filled_cap_error_upper,
                        "withinBudget":proof.within_budget,"exactWork":caps.exact_work(),"constructionExactWork":caps.construction_work(),
                        "fillingExactWork":proof.exact_work,"selectionExactWork":body.cap_selection_work(),"products":proof.products,"endpointCells":proof.endpoint_cells,
                        "sourceUvFacesInjective":proof.source_uv_faces.as_ref().is_some_and(|r|r.all_faces_injective),
                        "sourceUvPairsClassified":proof.source_uv_pairs.as_ref().is_some_and(|r|r.all_pairs_classified),
                        "reason":proof.reason,"solidGeometryCertified":false})
                });
                output["retainedSmoothness"]=if let Some(max_work)=optional_field::<u64>(&v,"smoothness_max_work")? {
                    let actual=body.inspect_smoothness(max_work)?;
                    let geometric_max_cells=optional_field::<usize>(&v,"geometric_g2_max_cells")?.unwrap_or(0);
                    if geometric_max_cells>100000 {return Err(input("Geometric G2 shared cell budget must be within 0..100000"));}
                    let mut remaining=geometric_max_cells;
                    let caps=actual.cap_seams.iter().map(|r|cad_geometric_g2::cap_value(body.model(),r,&mut remaining)).collect::<Result<Vec<_>>>()?;
                    json!({"walls":cad_sweep_smoothness::profile_value(&actual.walls),"maxWork":max_work,"totalExactWork":actual.exact_work,"auditOrder":"caps-first",
                        "geometricMaxCells":geometric_max_cells,"geometricCells":geometric_max_cells-remaining,
                        "capInternalSeams":caps,
                        "capWallContinuity":if result.approximation.levels.last().unwrap().closed_path {"absent"}else{"C0"},"fullBoundarySmoothnessCertified":false})
                }else{Value::Null};
            }
            if let Some(report)=station_reconstruction {output["stationReconstruction"]=report;}
            Ok(output)
}
