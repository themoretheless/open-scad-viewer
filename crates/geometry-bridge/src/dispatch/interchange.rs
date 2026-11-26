//! Bridge operations of the `interchange` family.
use crate::*;
#[path="trimmed_shell_requests.rs"]
mod trimmed_shell_requests;

#[path="shared_assembly_rays.rs"]
mod shared_assembly_rays;
#[path="compact_product_step.rs"]
mod compact_product_step;
#[path="named_step_parts.rs"]
mod named_step_parts;
#[path="step_occurrence_styles.rs"]
mod step_occurrence_styles;
#[path="shared_assembly_material.rs"]
mod shared_assembly_material;
#[path="shared_assembly_properties.rs"]
mod shared_assembly_properties;
#[path="shared_assembly_edits.rs"]
mod shared_assembly_edits;

const OPS: &[&str] = &[
    "brep_nurbs_export_step",
    "brep_nurbs_import_step",
    "brep_nurbs_export_step_v2",
    "brep_nurbs_import_step_v2",
    "brep_nurbs_export_step_v3",
    "brep_nurbs_import_step_v3",
    "brep_nurbs_export_step_v4",
    "brep_nurbs_import_step_v4",
    "brep_nurbs_export_step_v5",
    "brep_nurbs_import_step_v5",
    "brep_nurbs_export_step_v6",
    "brep_nurbs_import_step_v6",
    "brep_nurbs_export_step_v7",
    "brep_nurbs_import_step_v7",
    "brep_nurbs_export_step_v8",
    "brep_nurbs_import_step_v8",
    "brep_nurbs_export_step_v9",
    "brep_nurbs_export_shared_step",
    "brep_nurbs_import_step_v9",
    "brep_nurbs_import_step_scene",
    "brep_nurbs_import_step_lod0",
    "brep_nurbs_step_lod0_hausdorff_estimate",
    "brep_nurbs_import_step_v10",
    "brep_nurbs_export_step_v10",
    "brep_nurbs_compose_step_v7",
    "brep_nurbs_compose_step_v8",
    "brep_nurbs_compose_step_v9",
    "brep_nurbs_export_iges_v2",
    "brep_nurbs_import_iges_v2",
    "import_iges_curve",
    "import_iges_surface",
    "brep_nurbs_export_step_freeform",
    "brep_nurbs_import_step_freeform",
    "brep_nurbs_export_step_trimmed",
    "brep_nurbs_import_step_trimmed",
    "brep_nurbs_export_step_solid",
    "brep_nurbs_import_step_solid",
    "brep_nurbs_export_step_solid_v2",
    "brep_nurbs_import_step_solid_v2",
];

pub(crate) fn route(v: Value) -> Routed {
    if v["op"].as_str() == Some("assemble_owned_trimmed_shell") {
        return Routed::Handled(trimmed_shell_requests::assemble(&v));
    }
    if v["op"].as_str() == Some("brep_section_blade") {
        return Routed::Handled(build_section_blade(&v));
    }
    if v["op"].as_str() == Some("inspect_patch_closure") {
        return Routed::Handled(crate::cad_face_contacts::inspect_linear_patch_closure(&v));
    }
    if v["op"].as_str() == Some("thicken_nurbs_patch") {
        return Routed::Handled(crate::cad_face_contacts::thicken_authored_patch(&v));
    }
    if v["op"].as_str() == Some("inspect_shell_orientation") {
        return Routed::Handled(crate::cad_face_contacts::inspect_authored_shell_orientation(&v));
    }
    if matches!(v["op"].as_str(), Some("compress_brep_faces" | "compress_brep_edge")) {
        return Routed::Handled(compact_product_step::compress_brep(&v));
    }
    if matches!(v["op"].as_str(),Some("compact_body_step"|"compact_product_step")) { return Routed::Handled(compact_product_step::export(&v)); }
    if step_occurrence_styles::BALANCE_OPS.contains(&v["op"].as_str().unwrap_or("")) {
        return Routed::Handled(step_occurrence_styles::balance(&v));
    }
    if step_occurrence_styles::BRANCH_OPS.contains(&v["op"].as_str().unwrap_or("")) {
        return Routed::Handled(step_occurrence_styles::branch(&v));
    }
    if matches!(v["op"].as_str(),Some("group_step_parts"|"style_step_occurrences"|"read_occurrence_styles")) {return Routed::Handled(step_occurrence_styles::apply(&v));}
    if named_step_parts::OPS.contains(&v["op"].as_str().unwrap_or("")) {return Routed::Handled(named_step_parts::apply(&v));}
    if shared_assembly_rays::OPS.contains(&v["op"].as_str().unwrap_or("")) { return Routed::Handled(shared_assembly_rays::apply(&v)); }
    if shared_assembly_material::OPS.contains(&v["op"].as_str().unwrap_or("")) { return Routed::Handled(shared_assembly_material::apply(&v)); }
    if shared_assembly_properties::OPS.contains(&v["op"].as_str().unwrap_or("")) {
        return Routed::Handled(shared_assembly_properties::apply(&v));
    }
    if shared_assembly_edits::OPS.contains(&v["op"].as_str().unwrap_or("")) {
        return Routed::Handled(shared_assembly_edits::apply(&v));
    }
    if !OPS.contains(&v["op"].as_str().unwrap_or("")) {
        return Routed::Unhandled(v);
    }
    Routed::Handled(handle(v))
}

fn build_section_blade(v: &Value) -> Result<Value> {
    let stations: Vec<[f64; 6]> = field(v, "blade_stations")?;
    let sections: Vec<_> = stations.into_iter().map(|[x,y,z,chord,thickness,twist]| brep_core::blade::Section {
        leading_edge: [x,y,z], chord, thickness_control:thickness, twist_radians:twist.to_radians()
    }).collect();
    let model = brep_core::blade::build(&sections)?;
    Ok(json!({
        "model":encode(model)?,
        "stations":sections.len(),
        "closedOwnedIncidence":true,
        "globalEmbeddingCertified":false
    }))
}

fn handle(v: Value) -> Result<Value> {
    match v["op"].as_str().unwrap_or("") {
        "brep_nurbs_export_step" => {
            let (text, cert) = cad_step::export_step(&field(&v, "model")?)?;
            encode(json!({
                "text": text,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                }
            }))
        }
        "brep_nurbs_import_step" => {
            let (model, cert) = cad_step::import_step(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model": model,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                }
            }))
        }
        "brep_nurbs_export_step_v2" => {
            let (text, cert, identity) = cad_step::export_step_v2(&field(&v, "model")?)?;
            encode(json!({
                "text": text,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                },
                "identity": {
                    "preserved": identity.preserved,
                    "source": identity.source,
                    "preservedCount": identity.preserved_count,
                    "createdCount": identity.created_count,
                    "lostCount": identity.lost_count,
                }
            }))
        }
        "brep_nurbs_import_step_v2" => {
            let (model, cert, identity) = cad_step::import_step_v2(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model": model,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                },
                "identity": {
                    "preserved": identity.preserved,
                    "source": identity.source,
                    "preservedCount": identity.preserved_count,
                    "createdCount": identity.created_count,
                    "lostCount": identity.lost_count,
                }
            }))
        }
        "brep_nurbs_export_step_v3" => {
            let (text, cert, report) = cad_step::export_step_v3(&field(&v, "model")?)?;
            encode(json!({
                "text": text,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                },
                "identity": {
                    "preserved": report.identity.preserved,
                    "source": report.identity.source,
                    "preservedCount": report.identity.preserved_count,
                    "createdCount": report.identity.created_count,
                    "lostCount": report.identity.lost_count,
                },
                "ignoredEntities": report.ignored_entities,
                "instanceCount": report.instance_count,
                "reachableCount": report.reachable_count,
            }))
        }
        "brep_nurbs_import_step_v3" => {
            let (model, cert, report) = cad_step::import_step_v3(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model": model,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                },
                "identity": {
                    "preserved": report.identity.preserved,
                    "source": report.identity.source,
                    "preservedCount": report.identity.preserved_count,
                    "createdCount": report.identity.created_count,
                    "lostCount": report.identity.lost_count,
                },
                "ignoredEntities": report.ignored_entities,
                "instanceCount": report.instance_count,
                "reachableCount": report.reachable_count,
            }))
        }
        "brep_nurbs_export_step_v4" => {
            let (text, cert, report) = cad_step::export_step_v4(&field(&v, "model")?)?;
            encode(json!({
                "text":text,"certificate":{"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"instanceCount":report.instance_count,"reachableCount":report.reachable_count,
            }))
        }
        "brep_nurbs_import_step_v4" => {
            let (model, cert, report) = cad_step::import_step_v4(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model":model,"certificate":{"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"instanceCount":report.instance_count,"reachableCount":report.reachable_count,
            }))
        }
        "brep_nurbs_export_step_v5" => {
            let (text, cert, report) = cad_step::export_step_v5(&field(&v, "model")?)?;
            encode(json!({
                "text":text,"certificate":{"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"metadataLoss":report.metadata_loss,
                "instanceCount":report.instance_count,"reachableCount":report.reachable_count,
            }))
        }
        "brep_nurbs_import_step_v5" => {
            let (model, cert, report) = cad_step::import_step_v5(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model":model,"certificate":{"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"metadataLoss":report.metadata_loss,
                "instanceCount":report.instance_count,"reachableCount":report.reachable_count,
            }))
        }
        "brep_nurbs_export_step_v6" => {
            let (text, cert, report) = cad_step::export_step_v6(&field(&v, "model")?)?;
            encode(json!({
                "text":text,"certificate":{"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"metadataLoss":report.metadata_loss,
                "instanceCount":report.instance_count,"reachableCount":report.reachable_count,
                "definitionIdentities":report.definition_identities,"occurrenceIdentities":report.occurrence_identities,
                "productHierarchy":report.product_hierarchy,"externalReferences":report.external_references,
            }))
        }
        "brep_nurbs_import_step_v6" => {
            let (model, cert, report) = cad_step::import_step_v6(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model":model,"certificate":{"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"metadataLoss":report.metadata_loss,
                "instanceCount":report.instance_count,"reachableCount":report.reachable_count,
                "definitionIdentities":report.definition_identities,"occurrenceIdentities":report.occurrence_identities,
                "productHierarchy":report.product_hierarchy,"externalReferences":report.external_references,
            }))
        }
        "brep_nurbs_export_step_v7" => {
            let (text, cert, report) = cad_step::export_step_v7(&field(&v, "model")?)?;
            encode(json!({
                "text":text,"certificate":{"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"metadataLoss":report.metadata_loss,
                "instanceCount":report.instance_count,"reachableCount":report.reachable_count,
                "definitionIdentities":report.definition_identities,"occurrenceIdentities":report.occurrence_identities,
                "productHierarchy":report.product_hierarchy,"externalReferences":report.external_references,
            }))
        }
        "brep_nurbs_import_step_v7" => {
            let (model, cert, report) = cad_step::import_step_v7(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model":model,"certificate":{"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"metadataLoss":report.metadata_loss,
                "instanceCount":report.instance_count,"reachableCount":report.reachable_count,
                "definitionIdentities":report.definition_identities,"occurrenceIdentities":report.occurrence_identities,
                "productHierarchy":report.product_hierarchy,"externalReferences":report.external_references,
            }))
        }
        "brep_nurbs_export_step_v8" => {
            let (text, cert, report) = cad_step::export_step_v8(&field(&v, "model")?)?;
            let regularity=cert.regularity.iter().map(|row|json!({
                "carrier":row.carrier,"parameterU":row.parameter_u,"parameterV":row.parameter_v,
                "liftedPeriods":row.lifted_periods,"denominatorLowerBound":row.denominator_lower_bound,
                "jacobianLowerBound":row.jacobian_lower_bound,"collapsedBoundaries":row.collapsed_boundaries,
                "regularOpenDomain":row.regular_open_domain,"identity":row.identity,
            })).collect::<Vec<_>>();
            encode(json!({
                "text":text,"certificate":{"capability":cert.capability,"complete":cert.complete,
                    "regularity":regularity,"senseLayers":cert.sense_layers,
                    "coupledSenseCases":cert.coupled_sense_cases,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"metadataLoss":report.metadata_loss,
                "instanceCount":report.instance_count,"reachableCount":report.reachable_count,
                "definitionIdentities":report.definition_identities,"occurrenceIdentities":report.occurrence_identities,
                "productHierarchy":report.product_hierarchy,"externalReferences":report.external_references,
            }))
        }
        "brep_nurbs_import_step_v8" => {
            let (model, cert, report) = cad_step::import_step_v8(&field::<String>(&v, "text")?)?;
            let regularity=cert.regularity.iter().map(|row|json!({
                "carrier":row.carrier,"parameterU":row.parameter_u,"parameterV":row.parameter_v,
                "liftedPeriods":row.lifted_periods,"denominatorLowerBound":row.denominator_lower_bound,
                "jacobianLowerBound":row.jacobian_lower_bound,"collapsedBoundaries":row.collapsed_boundaries,
                "regularOpenDomain":row.regular_open_domain,"identity":row.identity,
            })).collect::<Vec<_>>();
            encode(json!({
                "model":model,"certificate":{"capability":cert.capability,"complete":cert.complete,
                    "regularity":regularity,"senseLayers":cert.sense_layers,
                    "coupledSenseCases":cert.coupled_sense_cases,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"metadataLoss":report.metadata_loss,
                "instanceCount":report.instance_count,"reachableCount":report.reachable_count,
                "definitionIdentities":report.definition_identities,"occurrenceIdentities":report.occurrence_identities,
                "productHierarchy":report.product_hierarchy,"externalReferences":report.external_references,
            }))
        }
        "brep_nurbs_export_shared_step" => {
            let rows = v["occurrences"].as_array()
                .ok_or_else(|| input("STEP assembly requires an occurrence array"))?;
            let models = v["definitions"].as_array()
                .ok_or_else(|| input("STEP assembly requires a definition array"))?;
            if rows.is_empty() || rows.len() > 256 || models.is_empty() || models.len() > rows.len() {
                return Err(input("STEP assembly requires 1 to 256 occurrences referencing local solids"));
            }
            let occurrences = rows.iter().map(|row| Ok(cad_step::StepAssemblyOccurrence {
                definition: field(row, "definition")?,
                transform: field(row, "transform")?,
            })).collect::<Result<Vec<_>>>()?;
            let assembly = cad_step::StepSharedAssembly::from_placed_solids(field(&v, "definitions")?, occurrences)?;
            let text = if v.get("groups").is_some() || v.get("parents").is_some() || v.get("colours").is_some() {
                let groups: Vec<String> = field(&v, "groups")?;
                let parents: Vec<Option<usize>> = field(&v, "parents")?;
                let names: Vec<_> = groups.iter().map(String::as_str).collect();
                if let Some(styles) = v.get("colours") {
                    let rows = styles.as_array().ok_or_else(|| input("Expected an occurrence colour array"))?;
                    let colours = rows.iter().map(|row| {
                        if row.is_null() { return Ok(None); }
                        let name = row["name"].as_str().ok_or_else(|| input("Expected a colour name"))?;
                        Ok(Some(cad_step::StepOccurrenceColour { name, rgb: field(row, "rgb")? }))
                    }).collect::<Result<Vec<_>>>()?;
                    cad_step::export_styled_step_assembly(&assembly, &names, &parents, &colours)?
                } else {
                    cad_step::export_grouped_step_assembly(&assembly, &names, &parents)?
                }
            } else {
                cad_step::export_step_shared_assembly(&assembly)?
            };
            Ok(json!({"text":text}))
        }
        "brep_nurbs_export_step_v9" => {
            let (text, cert, report) = cad_step::export_step_v9(&field(&v, "model")?)?;
            let regularity=cert.regularity.iter().map(|row|json!({
                "carrier":row.carrier,"parameterU":row.parameter_u,"parameterV":row.parameter_v,
                "liftedPeriods":row.lifted_periods,"denominatorLowerBound":row.denominator_lower_bound,
                "jacobianLowerBound":row.jacobian_lower_bound,"collapsedBoundaries":row.collapsed_boundaries,
                "regularOpenDomain":row.regular_open_domain,"identity":row.identity,
            })).collect::<Vec<_>>();
            encode(json!({
                "text":text,"certificate":{"capability":cert.capability,"complete":cert.complete,
                    "regularity":regularity,"senseLayers":cert.sense_layers,
                    "coupledSenseCases":cert.coupled_sense_cases,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"metadataLoss":report.metadata_loss,
                "instanceCount":report.instance_count,"reachableCount":report.reachable_count,
                "definitionIdentities":report.definition_identities,"occurrenceIdentities":report.occurrence_identities,
                "productHierarchy":report.product_hierarchy,"externalReferences":report.external_references,
            }))
        }
        "brep_nurbs_import_step_v9" => {
            let (model, cert, report) = cad_step::import_step_v9(&field::<String>(&v, "text")?)?;
            let regularity=cert.regularity.iter().map(|row|json!({
                "carrier":row.carrier,"parameterU":row.parameter_u,"parameterV":row.parameter_v,
                "liftedPeriods":row.lifted_periods,"denominatorLowerBound":row.denominator_lower_bound,
                "jacobianLowerBound":row.jacobian_lower_bound,"collapsedBoundaries":row.collapsed_boundaries,
                "regularOpenDomain":row.regular_open_domain,"identity":row.identity,
            })).collect::<Vec<_>>();
            encode(json!({
                "model":model,"certificate":{"capability":cert.capability,"complete":cert.complete,
                    "regularity":regularity,"senseLayers":cert.sense_layers,
                    "coupledSenseCases":cert.coupled_sense_cases,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"metadataLoss":report.metadata_loss,
                "instanceCount":report.instance_count,"reachableCount":report.reachable_count,
                "definitionIdentities":report.definition_identities,"occurrenceIdentities":report.occurrence_identities,
                "productHierarchy":report.product_hierarchy,"externalReferences":report.external_references,
            }))
        }
        "brep_nurbs_import_step_scene" => crate::step_scene::import(v),
        "brep_nurbs_import_step_lod0" => crate::step_lod0::import(v),
        "brep_nurbs_step_lod0_hausdorff_estimate" => crate::step_lod0::hausdorff_estimate(v),
        "brep_nurbs_import_step_v10" => {
            let (model, cert, document) =
                cad_step::import_step_v10(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model":model,
                "certificate":{"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "document":{
                    "source":document.source,"graphIdentity":document.graph_identity,
                    "definitionIdentities":document.definition_identities,
                    "occurrenceIdentities":document.occurrence_identities,
                    "productHierarchy":document.product_hierarchy,
                    "operatorIdentities":document.operator_identities,
                    "metadataLoss":document.metadata_loss,
                }
            }))
        }
        "brep_nurbs_export_step_v10" => {
            let document = cad_step::StepV10Document {
                source: field(&v, "source")?,
                graph_identity: field(&v, "graphIdentity")?,
                definition_identities: Vec::new(),
                occurrence_identities: Vec::new(),
                product_hierarchy: Vec::new(),
                operator_identities: Vec::new(),
                metadata_loss: Vec::new(),
            };
            encode(json!({"text":cad_step::export_step_v10(&document)?,
                "certificate":{"capability":"step-interchange/10","complete":true,
                    "notes":["retained_affine_occurrence_graph","exact_graph_isomorphism_identity"]}}))
        }
        "brep_nurbs_compose_step_v7" => {
            let models = field::<Vec<brep_core::Model>>(&v, "models")?;
            encode(cad_step::compose_step_v7_occurrences(&models)?)
        }
        "brep_nurbs_compose_step_v8" => {
            let models = field::<Vec<brep_core::Model>>(&v, "models")?;
            encode(cad_step::compose_step_v8_occurrences(&models)?)
        }
        "brep_nurbs_compose_step_v9" => {
            let models = field::<Vec<brep_core::Model>>(&v, "models")?;
            encode(cad_step::compose_step_v9_occurrences(&models)?)
        }
        "brep_nurbs_export_iges_v2" => {
            let (text, cert, report) = cad_step::export_iges_v2(&field(&v, "model")?)?;
            encode(json!({
                "text": text,
                "certificate": {"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "identity": {
                    "preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,
                    "createdCount":report.identity.created_count,"lostCount":report.identity.lost_count,
                },
                "ignoredMetadata":report.ignored_metadata,
                "entityCount":report.entity_count,
                "topologyCount":report.topology_count,
            }))
        }
        "import_iges_curve" => {
            let text: String = field(&v, "text")?;
            encode(cad_step::iges_interchange_v2::import_iges_curve(&text, field(&v, "directory_entry")?)?)
        }
        "import_iges_surface" => {
            let text: String = field(&v, "text")?;
            encode(cad_step::iges_interchange_v2::import_iges_surface(&text, field(&v, "directory_entry")?)?)
        }
        "brep_nurbs_import_iges_v2" => {
            let (model, cert, report) = cad_step::import_iges_v2(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model":model,
                "certificate": {"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "identity": {
                    "preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,
                    "createdCount":report.identity.created_count,"lostCount":report.identity.lost_count,
                },
                "ignoredMetadata":report.ignored_metadata,
                "entityCount":report.entity_count,
                "topologyCount":report.topology_count,
            }))
        }
        "brep_nurbs_export_step_freeform" => {
            let (text, cert) = cad_step::export_nurbs_step(&field(&v, "model")?)?;
            encode(json!({
                "text": text,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                }
            }))
        }
        "brep_nurbs_import_step_freeform" => {
            let (model, cert) = cad_step::import_nurbs_step(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model": model,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                }
            }))
        }
        "brep_nurbs_export_step_trimmed" => {
            let (text, cert) = cad_step::export_nurbs_step_trimmed(&field(&v, "model")?)?;
            encode(json!({
                "text": text,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                }
            }))
        }
        "brep_nurbs_import_step_trimmed" => {
            let (model, cert) =
                cad_step::import_nurbs_step_trimmed(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model": model,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                }
            }))
        }
        "brep_nurbs_export_step_solid" => {
            let (text, cert) = cad_step::export_nurbs_step_solid(&field(&v, "model")?)?;
            encode(json!({
                "text": text,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                }
            }))
        }
        "brep_nurbs_import_step_solid" => {
            let (model, cert) = cad_step::import_nurbs_step_solid(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model": model,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                }
            }))
        }
        "brep_nurbs_export_step_solid_v2" => {
            let (text, cert, identity) =
                cad_step::export_nurbs_step_solid_v2(&field(&v, "model")?)?;
            encode(json!({
                "text": text,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                },
                "identity": {
                    "preserved": identity.preserved,
                    "source": identity.source,
                    "preservedCount": identity.preserved_count,
                    "createdCount": identity.created_count,
                    "lostCount": identity.lost_count,
                }
            }))
        }
        "brep_nurbs_import_step_solid_v2" => {
            let (model, cert, identity) =
                cad_step::import_nurbs_step_solid_v2(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model": model,
                "certificate": {
                    "capability": cert.capability,
                    "complete": cert.complete,
                    "notes": cert.notes,
                },
                "identity": {
                    "preserved": identity.preserved,
                    "source": identity.source,
                    "preservedCount": identity.preserved_count,
                    "createdCount": identity.created_count,
                    "lostCount": identity.lost_count,
                }
            }))
        }
        _ => unreachable!("route admits only OPS"),
    }
}
