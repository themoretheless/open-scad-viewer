//! Bridge operations of the `interchange` family.
use crate::*;

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
    "brep_nurbs_import_step_v9",
    "brep_nurbs_import_step_v10",
    "brep_nurbs_export_step_v10",
    "brep_nurbs_compose_step_v7",
    "brep_nurbs_compose_step_v8",
    "brep_nurbs_compose_step_v9",
    "brep_nurbs_export_iges_v2",
    "brep_nurbs_import_iges_v2",
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
    if !OPS.contains(&v["op"].as_str().unwrap_or("")) {
        return Routed::Unhandled(v);
    }
    Routed::Handled(handle(v))
}

fn handle(v: Value) -> Result<Value> {
    match v["op"].as_str().unwrap_or("") {
        "brep_nurbs_export_step" => {
            let (text, cert) = brep_core::export_step(&field(&v, "model")?)?;
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
            let (model, cert) = brep_core::import_step(&field::<String>(&v, "text")?)?;
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
            let (text, cert, identity) = brep_core::export_step_v2(&field(&v, "model")?)?;
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
            let (model, cert, identity) = brep_core::import_step_v2(&field::<String>(&v, "text")?)?;
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
            let (text, cert, report) = brep_core::export_step_v3(&field(&v, "model")?)?;
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
            let (model, cert, report) = brep_core::import_step_v3(&field::<String>(&v, "text")?)?;
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
            let (text, cert, report) = brep_core::export_step_v4(&field(&v, "model")?)?;
            encode(json!({
                "text":text,"certificate":{"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"instanceCount":report.instance_count,"reachableCount":report.reachable_count,
            }))
        }
        "brep_nurbs_import_step_v4" => {
            let (model, cert, report) = brep_core::import_step_v4(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model":model,"certificate":{"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"instanceCount":report.instance_count,"reachableCount":report.reachable_count,
            }))
        }
        "brep_nurbs_export_step_v5" => {
            let (text, cert, report) = brep_core::export_step_v5(&field(&v, "model")?)?;
            encode(json!({
                "text":text,"certificate":{"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"metadataLoss":report.metadata_loss,
                "instanceCount":report.instance_count,"reachableCount":report.reachable_count,
            }))
        }
        "brep_nurbs_import_step_v5" => {
            let (model, cert, report) = brep_core::import_step_v5(&field::<String>(&v, "text")?)?;
            encode(json!({
                "model":model,"certificate":{"capability":cert.capability,"complete":cert.complete,"notes":cert.notes},
                "identity":{"preserved":report.identity.preserved,"source":report.identity.source,
                    "preservedCount":report.identity.preserved_count,"createdCount":report.identity.created_count,"lostCount":report.identity.lost_count},
                "ignoredEntities":report.ignored_entities,"metadataLoss":report.metadata_loss,
                "instanceCount":report.instance_count,"reachableCount":report.reachable_count,
            }))
        }
        "brep_nurbs_export_step_v6" => {
            let (text, cert, report) = brep_core::export_step_v6(&field(&v, "model")?)?;
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
            let (model, cert, report) = brep_core::import_step_v6(&field::<String>(&v, "text")?)?;
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
            let (text, cert, report) = brep_core::export_step_v7(&field(&v, "model")?)?;
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
            let (model, cert, report) = brep_core::import_step_v7(&field::<String>(&v, "text")?)?;
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
            let (text, cert, report) = brep_core::export_step_v8(&field(&v, "model")?)?;
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
            let (model, cert, report) = brep_core::import_step_v8(&field::<String>(&v, "text")?)?;
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
        "brep_nurbs_import_step_v10" => {
            let (model, cert, document) =
                brep_core::import_step_v10(&field::<String>(&v, "text")?)?;
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
            let document = brep_core::StepV10Document {
                source: field(&v, "source")?,
                graph_identity: field(&v, "graphIdentity")?,
                definition_identities: Vec::new(),
                occurrence_identities: Vec::new(),
                product_hierarchy: Vec::new(),
                operator_identities: Vec::new(),
                metadata_loss: Vec::new(),
            };
            encode(json!({"text":brep_core::export_step_v10(&document)?,
                "certificate":{"capability":"step-interchange/10","complete":true,
                    "notes":["retained_affine_occurrence_graph","exact_graph_isomorphism_identity"]}}))
        }
        "brep_nurbs_compose_step_v7" => {
            let models = field::<Vec<brep_core::Model>>(&v, "models")?;
            encode(brep_core::compose_step_v7_occurrences(&models)?)
        }
        "brep_nurbs_compose_step_v8" => {
            let models = field::<Vec<brep_core::Model>>(&v, "models")?;
            encode(brep_core::compose_step_v8_occurrences(&models)?)
        }
        "brep_nurbs_compose_step_v9" => {
            let models = field::<Vec<brep_core::Model>>(&v, "models")?;
            encode(brep_core::compose_step_v9_occurrences(&models)?)
        }
        "brep_nurbs_export_iges_v2" => {
            let (text, cert, report) = brep_core::export_iges_v2(&field(&v, "model")?)?;
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
        "brep_nurbs_import_iges_v2" => {
            let (model, cert, report) = brep_core::import_iges_v2(&field::<String>(&v, "text")?)?;
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
            let (text, cert) = brep_core::export_nurbs_step(&field(&v, "model")?)?;
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
            let (model, cert) = brep_core::import_nurbs_step(&field::<String>(&v, "text")?)?;
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
            let (text, cert) = brep_core::export_nurbs_step_trimmed(&field(&v, "model")?)?;
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
                brep_core::import_nurbs_step_trimmed(&field::<String>(&v, "text")?)?;
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
            let (text, cert) = brep_core::export_nurbs_step_solid(&field(&v, "model")?)?;
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
            let (model, cert) = brep_core::import_nurbs_step_solid(&field::<String>(&v, "text")?)?;
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
                brep_core::export_nurbs_step_solid_v2(&field(&v, "model")?)?;
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
                brep_core::import_nurbs_step_solid_v2(&field::<String>(&v, "text")?)?;
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
