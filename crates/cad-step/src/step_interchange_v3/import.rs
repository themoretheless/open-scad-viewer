use super::*;
fn import_step_direct(
    text: &str,
    analytic_surfaces: bool,
    ap242_composition: bool,
    allow_degenerate: bool,
    capability: &'static str,
) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    if !text.to_ascii_uppercase().contains("ISO-10303-21") {
        return Err(refuse("Not an ISO-10303-21 exchange"));
    }
    if ap242_composition {
        validate_exchange_structure(text)?;
    }
    let entities = parse(if ap242_composition {
        data_payload(text)?
    } else {
        text
    })?;
    let representations: Vec<_> = if ap242_composition {
        let mut selected = Vec::<usize>::new();
        for entity in entities.values() {
            if let Value::Call(name, args) = &entity.value
                && name == "SHAPE_DEFINITION_REPRESENTATION"
                && args.len() == 2
            {
                selected.push(one_ref(&args[1], "selected shape representation")?);
            }
        }
        selected.sort_unstable();
        selected.dedup();
        if selected.is_empty() {
            return Err(refuse(
                "AP242 import requires SHAPE_DEFINITION_REPRESENTATION selection",
            ));
        }
        let allow_open_shells = capability == STEP_INTERCHANGE_V9_CAPABILITY;
        for id in &selected {
            let (ty, _) = call(&entities, *id)?;
            if ty != "ADVANCED_BREP_SHAPE_REPRESENTATION"
                && !(allow_open_shells && ty == "SHAPE_REPRESENTATION")
                && !(allow_open_shells && ty == "MANIFOLD_SURFACE_SHAPE_REPRESENTATION")
            {
                return Err(refuse(
                    "Selected representation is outside the admitted direct B-rep products",
                ));
            }
        }
        selected
    } else {
        entities
            .keys()
            .filter_map(|id| {
                call(&entities, *id)
                    .ok()
                    .and_then(|(ty, _)| (ty == "ADVANCED_BREP_SHAPE_REPRESENTATION").then_some(*id))
            })
            .collect()
    };
    let allow_open_shells = capability == STEP_INTERCHANGE_V9_CAPABILITY;
    let all_bodies: Vec<_> = entities
        .keys()
        .filter_map(|id| {
            call(&entities, *id).ok().and_then(|(ty, _)| {
                (matches!(ty, "MANIFOLD_SOLID_BREP" | "BREP_WITH_VOIDS")
                    || allow_open_shells && ty == "SHELL_BASED_SURFACE_MODEL")
                    .then_some(*id)
            })
        })
        .collect();
    let graph_roots = if representations.is_empty() {
        all_bodies.clone()
    } else {
        representations.clone()
    };
    if graph_roots.is_empty() {
        return Err(refuse("STEP /3 requires a manifold advanced B-rep root"));
    }
    let occurrences = if ap242_composition {
        representation_occurrences(
            &entities,
            &representations,
            allow_degenerate && capability != STEP_INTERCHANGE_V8_CAPABILITY,
            capability == STEP_INTERCHANGE_V9_CAPABILITY,
            allow_open_shells,
        )?
    } else {
        Vec::new()
    };
    let occurrence_count = occurrences.len();
    let mut definition_identities = Vec::new();
    let mut occurrence_identities = Vec::new();
    if allow_degenerate {
        for occurrence in &occurrences {
            let definition = format!(
                "representation:{}",
                entity_digest(&entities, occurrence.representation)?
            );
            if !definition_identities.contains(&definition) {
                definition_identities.push(definition)
            }
            let path = occurrence
                .path
                .iter()
                .map(|id| entity_digest(&entities, *id))
                .collect::<Result<Vec<_>>>()?
                .join("/");
            let matrix = occurrence
                .transform
                .iter()
                .flatten()
                .map(|value| format!("{:016x}", value.to_bits()))
                .collect::<String>();
            occurrence_identities.push(format!("occurrence:{}", hash(&(path + &matrix))));
        }
    }
    let product_hierarchy = if allow_degenerate {
        entities
            .keys()
            .filter_map(|id| {
                call(&entities, *id).ok().and_then(|(ty, args)| {
                    if ty == "PRODUCT" && args.len() >= 3 {
                        Some(format!(
                            "product:#{}:{}:{}",
                            id,
                            string_value(&args[0], "product id").ok()?,
                            string_value(&args[1], "product name").ok()?
                        ))
                    } else if matches!(
                        ty,
                        "PRODUCT_DEFINITION"
                            | "PRODUCT_DEFINITION_FORMATION"
                            | "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE"
                    ) && args.len() >= 2
                    {
                        Some(format!(
                            "{}:#{}:{}:{}",
                            ty.to_ascii_lowercase(),
                            id,
                            string_value(&args[0], "product metadata id").ok()?,
                            string_value(&args[1], "product metadata description").ok()?
                        ))
                    } else if ty == "PRODUCT_RELATED_PRODUCT_CATEGORY" && args.len() >= 3 {
                        Some(format!(
                            "product-category:#{}:{}",
                            id,
                            string_value(&args[0], "product category").ok()?
                        ))
                    } else if ty == "PROPERTY_DEFINITION" && args.len() >= 2 {
                        Some(format!(
                            "property-definition:#{}:{}",
                            id,
                            string_value(&args[0], "property name").ok()?
                        ))
                    } else if ty == "NEXT_ASSEMBLY_USAGE_OCCURRENCE" && args.len() >= 5 {
                        Some(format!(
                            "assembly-use:#{}:#{}:#{}",
                            id,
                            one_ref(&args[3], "relating product").ok()?,
                            one_ref(&args[4], "related product").ok()?
                        ))
                    } else {
                        None
                    }
                })
            })
            .collect()
    } else {
        Vec::new()
    };
    let external_references = if allow_degenerate {
        entities
            .keys()
            .filter_map(|id| {
                let (ty, args) = call(&entities, *id).ok()?;
                if ty == "DOCUMENT_FILE" && !args.is_empty() {
                    Some(format!(
                        "#{}:{}",
                        id,
                        string_value(&args[0], "document URI").ok()?
                    ))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut linked = reachable(&entities, &graph_roots)?;
    if ap242_composition {
        if capability != STEP_INTERCHANGE_V9_CAPABILITY {
            linked.clear();
        }
        for occurrence in &occurrences {
            linked.extend(occurrence.linked.iter().copied())
        }
    }
    let roots: Vec<_> = all_bodies
        .into_iter()
        .filter(|id| linked.contains(id))
        .collect();
    if roots.is_empty() {
        return Err(refuse(
            "Selected representation contains no manifold B-rep body",
        ));
    }
    const REACHABLE_TYPES: &[&str] = &[
        "SHAPE_REPRESENTATION",
        "ADVANCED_BREP_SHAPE_REPRESENTATION",
        "MANIFOLD_SURFACE_SHAPE_REPRESENTATION",
        "MANIFOLD_SOLID_BREP",
        "BREP_WITH_VOIDS",
        "SHELL_BASED_SURFACE_MODEL",
        "CLOSED_SHELL",
        "OPEN_SHELL",
        "ORIENTED_CLOSED_SHELL",
        "ADVANCED_FACE",
        "FACE_OUTER_BOUND",
        "FACE_BOUND",
        "EDGE_LOOP",
        "ORIENTED_EDGE",
        "EDGE_CURVE",
        "VERTEX_POINT",
        "CARTESIAN_POINT",
        "DIRECTION",
        "VECTOR",
        "AXIS2_PLACEMENT_2D",
        "AXIS2_PLACEMENT_3D",
        "LINE",
        "CIRCLE",
        "ELLIPSE",
        "TRIMMED_CURVE",
        "B_SPLINE_CURVE_WITH_KNOTS",
        "PCURVE",
        "SURFACE_CURVE",
        "SEAM_CURVE",
        "DEFINITIONAL_REPRESENTATION",
        "PLANE",
        "CYLINDRICAL_SURFACE",
        "CONICAL_SURFACE",
        "SPHERICAL_SURFACE",
        "TOROIDAL_SURFACE",
        "B_SPLINE_SURFACE_WITH_KNOTS",
        "GEOMETRIC_REPRESENTATION_CONTEXT",
        "GLOBAL_UNIT_ASSIGNED_CONTEXT",
        "SI_UNIT",
        "CONVERSION_BASED_UNIT",
        "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT",
        "UNCERTAINTY_MEASURE_WITH_UNIT",
        "LENGTH_MEASURE_WITH_UNIT",
        "MEASURE_WITH_UNIT",
        "ITEM_DEFINED_TRANSFORMATION",
        "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION",
        "CARTESIAN_TRANSFORMATION_OPERATOR_3D",
    ];
    for id in &linked {
        if allow_degenerate
            && capability != STEP_INTERCHANGE_V8_CAPABILITY
            && component(
                &components(&entities, *id)?,
                "CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM",
            )
            .is_some()
        {
            continue;
        }
        let (ty, _) = call(&entities, *id)?;
        if !REACHABLE_TYPES.contains(&ty) && !matches!(ty, "COMPLEX") {
            return Err(refuse(format!(
                "Reachable entity {ty} is outside the direct finite graph subset"
            )));
        }
    }
    let mut ignored = BTreeSet::new();
    for id in entities.keys() {
        if linked.contains(id) {
            continue;
        }
        if ap242_composition && relationship_components(&entities, *id)?.is_some() {
            continue;
        }
        if allow_degenerate
            && capability != STEP_INTERCHANGE_V8_CAPABILITY
            && component(
                &components(&entities, *id)?,
                "CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM",
            )
            .is_some()
        {
            continue;
        }
        let (ty, _) = call(&entities, *id)?;
        if allow_degenerate
            && matches!(
                ty,
                "NEXT_ASSEMBLY_USAGE_OCCURRENCE" | "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION"
            )
        {
            continue;
        }
        if allow_degenerate
            && matches!(
                ty,
                "DOCUMENT_FILE"
                    | "DOCUMENT"
                    | "DOCUMENT_TYPE"
                    | "DOCUMENT_REPRESENTATION_TYPE"
                    | "EXTERNALLY_DEFINED_ITEM"
                    | "EXTERNAL_SOURCE"
                    | "APPLIED_EXTERNAL_IDENTIFICATION_ASSIGNMENT"
            )
        {
            continue;
        }
        if UNREACHABLE_ALLOWLIST.contains(&ty) {
            ignored.insert(ty.to_string());
        } else {
            return Err(refuse(format!(
                "Unreachable entity {ty} is not presentation metadata allowlisted by /3"
            )));
        }
    }
    let (mut model, identity_map) = if ap242_composition {
        if occurrences.is_empty() {
            return Err(refuse(
                "Selected placement graph contains no manifold B-rep occurrence",
            ));
        }
        let mut aggregate = Model::empty(1e-7)?;
        let mut aggregate_map = Vec::new();
        for occurrence in occurrences {
            let scale = length_scale(&entities, &occurrence.linked, false)?;
            let (component, map) = build_direct_roots(
                &entities,
                &occurrence.bodies,
                scale,
                DirectBuildOptions {
                    analytic_surfaces,
                    ap242_composition: true,
                    allow_degenerate,
                    whole_domain_proofs: matches!(
                        capability,
                        STEP_INTERCHANGE_V7_CAPABILITY
                            | STEP_INTERCHANGE_V8_CAPABILITY
                            | STEP_INTERCHANGE_V9_CAPABILITY
                    ),
                    interior_point_selectors: capability == STEP_INTERCHANGE_V9_CAPABILITY,
                    allow_open_shells,
                },
            )?;
            let component = brep_core::transform::affine(&component, occurrence.transform)?;
            append_direct_model(&mut aggregate, component, &map, &mut aggregate_map);
        }
        aggregate.rebuild_topology_ids();
        (aggregate, aggregate_map)
    } else {
        let scale = length_scale(&entities, &linked, true)?;
        let placement = reachable_transform(&entities, &linked, scale, false)?;
        let (mut model, map) = build_direct_roots(
            &entities,
            &roots,
            scale,
            DirectBuildOptions {
                analytic_surfaces,
                ap242_composition: false,
                allow_degenerate,
                whole_domain_proofs: matches!(
                    capability,
                    STEP_INTERCHANGE_V7_CAPABILITY
                        | STEP_INTERCHANGE_V8_CAPABILITY
                        | STEP_INTERCHANGE_V9_CAPABILITY
                ),
                interior_point_selectors: capability == STEP_INTERCHANGE_V9_CAPABILITY,
                allow_open_shells,
            },
        )?;
        if let Some(matrix) = placement {
            model = brep_core::transform::affine(&model, matrix)?
        }
        (model, map)
    };
    let identity = if capability == STEP_INTERCHANGE_V9_CAPABILITY
        || allow_degenerate && occurrence_count > 1
    {
        crate::step_interchange::StepIdentityReport {
            preserved: false,
            source: "assembly-definition-plus-occurrence",
            preserved_count: 0,
            created_count: identity_map.len(),
            lost_count: 0,
        }
    } else {
        restore_identity(&entities, &identity_map, &mut model)?
    };
    model.validate()?;
    let metadata_loss = if identity.preserved {
        vec![]
    } else if allow_degenerate {
        vec![
            "topology.occurrence_instance_ids_created".into(),
            "presentation_style".into(),
        ]
    } else {
        vec![
            "representation_item.name".into(),
            "product.id".into(),
            "product.name".into(),
            "product.description".into(),
            "presentation_style".into(),
        ]
    };
    let mut certificate_notes = vec![
        "direct_part21_topology",
        "line_circle_ellipse_trimmed_curve",
        "plane_and_rational_multispan_bspline_surface",
        "periodic_analytic_surfaces_typed_refused_parameterization",
        "shared_topology_senses_holes",
        "multi_body_multi_cavity",
        "si_and_positive_conversion_units",
        "reachable_rigid_placements_only",
        "independent_curve_pcurve_check",
        "bounded_reachable_graph",
    ];
    if matches!(
        capability,
        STEP_INTERCHANGE_V7_CAPABILITY
            | STEP_INTERCHANGE_V8_CAPABILITY
            | STEP_INTERCHANGE_V9_CAPABILITY
    ) {
        certificate_notes.push("whole_domain_rational_boundary_and_constant_pole_identity");
    }
    if capability == STEP_INTERCHANGE_V9_CAPABILITY {
        certificate_notes.extend([
            "certified_trimmed_curve_point_inverse",
            "bounded_freeform_collapsed_boundary_ownership",
            "invertible_general_affine_occurrences",
            "open_shell_surface_products",
        ]);
    }
    Ok((
        model,
        FeatureCertificate {
            capability,
            complete: true,
            notes: certificate_notes,
        },
        StepV3Report {
            identity,
            ignored_entities: ignored.into_iter().collect(),
            metadata_loss,
            instance_count: entities.len(),
            reachable_count: linked.len(),
            definition_identities,
            occurrence_identities,
            product_hierarchy,
            external_references,
        },
    ))
}

pub fn import_step_v3(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    import_step_direct(text, false, false, false, STEP_INTERCHANGE_V3_CAPABILITY)
}

pub fn import_step_v4(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    import_step_direct(text, true, false, false, STEP_INTERCHANGE_V4_CAPABILITY)
}

pub fn import_step_v5(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    import_step_direct(text, true, true, false, STEP_INTERCHANGE_V5_CAPABILITY)
}

pub fn import_step_v6(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    import_step_direct(text, true, true, true, STEP_INTERCHANGE_V6_CAPABILITY)
}

/// Append-only `/7` product route. The native direct graph subset is identical
/// to `/6`; `/7` adds bundle composition, durable lifecycle, and independent
/// semantic-conformance status in the product adapters. Keeping this entry
/// point distinct prevents a future `/7` claim from rewriting `/6` evidence.
pub fn import_step_v7(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    import_step_direct(text, true, true, true, STEP_INTERCHANGE_V7_CAPABILITY)
}

/// Qualified successor: `/7` remains rejected and immutable.
pub fn import_step_v8(text: &str) -> Result<(Model, StepV8Certificate, StepV3Report)> {
    let (model, _, report) =
        import_step_direct(text, true, true, true, STEP_INTERCHANGE_V8_CAPABILITY)?;
    let certificate = certify_step_v8_topology(&model)?;
    Ok((model, certificate, report))
}

pub fn import_step_v9(text: &str) -> Result<(Model, StepV8Certificate, StepV3Report)> {
    let (model, _, report) =
        import_step_direct(text, true, true, true, STEP_INTERCHANGE_V9_CAPABILITY)?;
    let mut certificate = certify_step_v8_topology(&model)?;
    certificate.capability = STEP_INTERCHANGE_V9_CAPABILITY;
    certificate.notes.extend([
        "certified_interior_point_selectors",
        "bounded_freeform_poles",
        "general_affine_occurrences",
        "retained_open_shell_products",
    ]);
    Ok((model, certificate, report))
}

fn step_v10_graph_identity(entities: &BTreeMap<usize, Entity>) -> Result<String> {
    let mut nodes = entities
        .keys()
        .map(|id| entity_digest(entities, *id))
        .collect::<Result<Vec<_>>>()?;
    nodes.sort();
    Ok(hash(&nodes.join("\n")))
}

/// Imports the editable B-rep projection while retaining the complete Part 21
/// graph. Entity-number changes do not alter `graph_identity`.
pub fn import_step_v10(text: &str) -> Result<(Model, StepV8Certificate, StepV10Document)> {
    let (model, mut certificate, report) = import_step_v9(text)?;
    let entities = parse(data_payload(text)?)?;
    let mut operator_identities = Vec::new();
    for id in entities.keys() {
        if let Some((_, _, operator)) = relationship_components(&entities, *id)? {
            operator_identities.push(format!("operator:{}", entity_digest(&entities, operator)?));
        }
    }
    operator_identities.sort();
    operator_identities.dedup();
    certificate.capability = STEP_INTERCHANGE_V10_CAPABILITY;
    certificate.notes.extend([
        "retained_affine_occurrence_graph",
        "exact_graph_isomorphism_identity",
    ]);
    let document = StepV10Document {
        source: text.to_string(),
        graph_identity: step_v10_graph_identity(&entities)?,
        definition_identities: report.definition_identities,
        occurrence_identities: report.occurrence_identities,
        product_hierarchy: report.product_hierarchy,
        operator_identities,
        metadata_loss: Vec::new(),
    };
    Ok((model, certificate, document))
}

/// Exports the authoritative graph byte-for-byte after revalidating its
/// identifier-independent canonical identity.
pub fn export_step_v10(document: &StepV10Document) -> Result<String> {
    if document.source.len() > MAX_OUTPUT_BYTES {
        return Err(refuse("STEP /10 output exceeds 16 MiB"));
    }
    let entities = parse(data_payload(&document.source)?)?;
    if step_v10_graph_identity(&entities)? != document.graph_identity {
        return Err(refuse("STEP /10 retained graph identity mismatch"));
    }
    Ok(document.source.clone())
}
