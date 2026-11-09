use super::*;

pub(super) const UNREACHABLE_ALLOWLIST: &[&str] = &[
    "SURFACE_STYLE_PARAMETER_LINE", "SURFACE_STYLE_TRANSPARENT", "TESSELLATED_SHAPE_REPRESENTATION",
    "CONCENTRICITY_TOLERANCE", "PRODUCT_DEFINITION_CONTEXT_ROLE", "SYMMETRY_TOLERANCE",
    "ROUNDNESS_TOLERANCE", "CYLINDRICITY_TOLERANCE", "APPLIED_DOCUMENT_REFERENCE",
    "TOTAL_RUNOUT_TOLERANCE", "OBJECT_ROLE", "LIMITS_AND_FITS", "DOCUMENT_PRODUCT_EQUIVALENCE",
    "ROLE_ASSOCIATION", "APPLICATION_CONTEXT", "APPLICATION_PROTOCOL_DEFINITION",
    "DEFAULT_MODEL_GEOMETRIC_VIEW", "MODEL_GEOMETRIC_VIEW", "CAMERA_MODEL_D3",
    "REPRESENTATION_MAP", "PRODUCT_CATEGORY_RELATIONSHIP", "PRODUCT", "PRODUCT_DEFINITION",
    "PRODUCT_DEFINITION_FORMATION", "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE",
    "PRODUCT_RELATED_PRODUCT_CATEGORY", "PRODUCT_CATEGORY",
    "MECHANICAL_DESIGN_AND_DRAUGHTING_RELATIONSHIP", "MAPPED_ITEM", "REAL_REPRESENTATION_ITEM",
    "PLANE", "DRAUGHTING_MODEL", "VIEW_VOLUME", "DRAUGHTING_PRE_DEFINED_CURVE_FONT",
    "DIMENSIONAL_EXPONENTS", "PRODUCT_CONTEXT", "PRODUCT_DEFINITION_CONTEXT",
    "PRODUCT_DEFINITION_SHAPE", "SHAPE_DEFINITION_REPRESENTATION",
    "ADVANCED_BREP_SHAPE_REPRESENTATION", "MANIFOLD_SURFACE_SHAPE_REPRESENTATION",
    "SHELL_BASED_SURFACE_MODEL", "COORDINATES_LIST", "TRIANGULATED_FACE", "TESSELLATED_FACE",
    "TESSELLATED_SHELL", "GEOMETRIC_MODEL_RELATIONSHIP", "GEOMETRIC_REPRESENTATION_CONTEXT",
    "GLOBAL_UNIT_ASSIGNED_CONTEXT", "SI_UNIT", "PRESENTATION_LAYER_ASSIGNMENT", "STYLED_ITEM",
    "COLOUR_RGB", "SURFACE_STYLE_USAGE", "SURFACE_SIDE_STYLE", "SURFACE_STYLE_FILL_AREA",
    "FILL_AREA_STYLE", "FILL_AREA_STYLE_COLOUR", "DRAUGHTING_PRE_DEFINED_COLOUR",
    "CAMERA_MODEL_D3_MULTI_CLIPPING", "PROPERTY_DEFINITION", "PROPERTY_DEFINITION_REPRESENTATION",
    "DESCRIPTIVE_REPRESENTATION_ITEM", "MEASURE_REPRESENTATION_ITEM", "PLANAR_BOX", "INVISIBILITY",
    "VALUE_REPRESENTATION_ITEM", "CURVE_STYLE", "DRAUGHTING_CALLOUT_RELATIONSHIP",
    "GENERAL_PROPERTY_ASSOCIATION", "PRESENTATION_STYLE_ASSIGNMENT", "DIMENSIONAL_LOCATION",
    "SURFACE_PROFILE_TOLERANCE", "LINE", "GENERAL_PROPERTY",
    "FEATURE_FOR_DATUM_TARGET_RELATIONSHIP", "SHAPE_ASPECT", "PRE_DEFINED_MARKER",
    "GEOMETRIC_TOLERANCE", "FLATNESS_TOLERANCE", "ANGULARITY_TOLERANCE", "TRIMMED_CURVE",
    "INTEGER_REPRESENTATION_ITEM", "PLACED_DATUM_TARGET_FEATURE", "ID_ATTRIBUTE", "POINT_STYLE",
    "ALL_AROUND_SHAPE_ASPECT", "TOLERANCE_ZONE_FORM", "COMPOSITE_CURVE_SEGMENT",
    "DRAUGHTING_MODEL_ITEM_ASSOCIATION", "GEOMETRIC_CURVE_SET", "GEOMETRIC_TOLERANCE_RELATIONSHIP",
    "COMPLEX_TRIANGULATED_SURFACE_SET", "DATUM_FEATURE", "ITEM_IDENTIFIED_REPRESENTATION_USAGE",
    "TOLERANCE_ZONE", "COMPOSITE_CURVE", "ANNOTATION_PLANE", "MEASURE_WITH_UNIT", "CIRCLE",
    "DATUM_REFERENCE_ELEMENT", "TESSELLATED_CURVE_SET", "VERTEX_POINT", "SHAPE_ASPECT_RELATIONSHIP",
    "ANGULAR_LOCATION", "PERPENDICULARITY_TOLERANCE", "DATUM_REFERENCE_COMPARTMENT",
    "PLANE_ANGLE_MEASURE_WITH_UNIT", "OVER_RIDING_STYLED_ITEM", "EDGE_CURVE",
    "PARALLELISM_TOLERANCE", "TESSELLATED_GEOMETRIC_SET", "COMPOSITE_GROUP_SHAPE_ASPECT", "DATUM",
    "DATUM_SYSTEM", "CONVERSION_BASED_UNIT", "CHARACTERIZED_ITEM_WITHIN_REPRESENTATION",
    "COMPOUND_REPRESENTATION_ITEM", "ORIENTED_EDGE", "TESSELLATED_ANNOTATION_OCCURRENCE",
    "TOLERANCE_VALUE", "DRAUGHTING_MODEL_ITEM_ASSOCIATION_WITH_PLACEHOLDER", "EDGE_LOOP",
    "DRAUGHTING_CALLOUT", "PLUS_MINUS_TOLERANCE", "SHAPE_DIMENSION_REPRESENTATION",
    "ANNOTATION_PLACEHOLDER_OCCURRENCE", "FACE_OUTER_BOUND", "ANNOTATION_CURVE_OCCURRENCE",
    "MEASURE_QUALIFICATION", "GEOMETRIC_ITEM_SPECIFIC_USAGE",
    "DIMENSIONAL_CHARACTERISTIC_REPRESENTATION", "GEOMETRIC_SET", "ADVANCED_FACE", "POLYLINE",
    "COMPOSITE_SHAPE_ASPECT", "CYLINDRICAL_SURFACE", "DIMENSIONAL_SIZE",
    "VALUE_FORMAT_TYPE_QUALIFIER", "OPEN_SHELL",
    "GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION", "TYPE_QUALIFIER", "COLOUR",
    "LENGTH_MEASURE_WITH_UNIT", "B_SPLINE_CURVE_WITH_KNOTS", "PRE_DEFINED_POINT_MARKER_SYMBOL",
    "CONSTRUCTIVE_GEOMETRY_REPRESENTATION", "CIRCULAR_RUNOUT_TOLERANCE", "MASS_MEASURE_WITH_UNIT",
    "CONSTRUCTIVE_GEOMETRY_REPRESENTATION_RELATIONSHIP", "PROJECTED_ZONE_DEFINITION", "FACE_BOUND",
    "SURFACE_STYLE_BOUNDARY", "SURFACE_STYLE_RENDERING_WITH_PROPERTIES", "ANGULAR_SIZE",
    "STRAIGHTNESS_TOLERANCE", "PRODUCT_DEFINITION_CONTEXT_ASSOCIATION",
    "MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION", "DATUM_TARGET", "VECTOR",
    "REPRESENTATION", "DERIVED_UNIT", "DERIVED_UNIT_ELEMENT", "DENSITY_UNIT", "MASS_UNIT",
    "LENGTH_UNIT", "AREA_UNIT", "VOLUME_UNIT", "NAMED_UNIT", "NAME_ATTRIBUTE",
    "MATERIAL_DESIGNATION", "APPLIED_NAME_ASSIGNMENT", "APPLIED_DESCRIPTION_TEXT_ASSIGNMENT",
    "PERSON", "ORGANIZATION", "PERSON_AND_ORGANIZATION", "PERSON_AND_ORGANIZATION_ROLE",
    "APPLIED_PERSON_AND_ORGANIZATION_ASSIGNMENT", "DATE_AND_TIME", "CALENDAR_DATE", "LOCAL_TIME",
    "COORDINATED_UNIVERSAL_TIME_OFFSET", "DATE_TIME_ROLE", "APPLIED_DATE_AND_TIME_ASSIGNMENT",
    "APPROVAL", "APPROVAL_STATUS", "APPROVAL_ROLE", "APPLIED_APPROVAL_ASSIGNMENT",
    "SECURITY_CLASSIFICATION", "SECURITY_CLASSIFICATION_LEVEL",
    "APPLIED_SECURITY_CLASSIFICATION_ASSIGNMENT", "EXTERNAL_CLASS_LIBRARY",
    "ITEM_DEFINED_TRANSFORMATION", "AXIS2_PLACEMENT_3D", "CARTESIAN_POINT", "DIRECTION",
    "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION", "CARTESIAN_TRANSFORMATION_OPERATOR_3D",
];

fn import_step_direct_impl(
    text: &str,
    analytic_surfaces: bool,
    ap242_composition: bool,
    allow_degenerate: bool,
    capability: &'static str,
    include_semantic: bool,
) -> Result<(Model, FeatureCertificate, StepV3Report, Option<StepSemanticMetadata>)> {
    if !text.to_ascii_uppercase().contains("ISO-10303-21") {
        return Err(refuse("Not an ISO-10303-21 exchange"));
    }
    if ap242_composition {
        validate_exchange_structure(text)?;
    }
    let entities = parse(if ap242_composition { data_payload(text)? } else { text })?;
    let mut auxiliary_selections = BTreeSet::new();
    let representations: Vec<_> = if ap242_composition && capability != STEP_INTERCHANGE_V10_CAPABILITY {
        let mut selected = Vec::<usize>::new();
        for entity in entities.values() {
            if let Value::Call(name, args) = &entity.value && name == "SHAPE_DEFINITION_REPRESENTATION" && args.len() == 2 {
                selected.push(one_ref(&args[1], "selected shape representation")?);
            }
        }
        selected.sort_unstable();
        selected.dedup();
        if selected.is_empty() {
            return Err(refuse("AP242 import requires SHAPE_DEFINITION_REPRESENTATION selection"));
        }
        let allow_open_shells = matches!(capability, STEP_INTERCHANGE_V9_CAPABILITY | STEP_INTERCHANGE_V10_CAPABILITY);
        for id in &selected {
            let (ty, _) = call(&entities, *id)?;
            if ty != "ADVANCED_BREP_SHAPE_REPRESENTATION"
                && !(allow_open_shells && ty == "SHAPE_REPRESENTATION")
                && !(allow_open_shells && ty == "MANIFOLD_SURFACE_SHAPE_REPRESENTATION")
            {
                return Err(refuse("Selected representation is outside the admitted direct B-rep products"));
            }
        }
        selected
    } else if ap242_composition {
        let mut selected = Vec::<usize>::new();
        for entity in entities.values() {
            if let Value::Call(name, args) = &entity.value && name == "SHAPE_DEFINITION_REPRESENTATION" && args.len() == 2 {
                selected.push(one_ref(&args[1], "selected shape representation")?);
            }
        }
        selected.sort_unstable();
        selected.dedup();
        if selected.is_empty() {
            return Err(refuse("AP242 import requires SHAPE_DEFINITION_REPRESENTATION selection"));
        }
        let exact = selected.iter().copied().filter(|id| call(&entities, *id).is_ok_and(|(ty, _)| ty == "ADVANCED_BREP_SHAPE_REPRESENTATION")).collect::<Vec<_>>();
        let direct_exact = selected.iter().copied().filter(|id| {
            call(&entities, *id).is_ok_and(|(ty, args)| {
                ty == "SHAPE_REPRESENTATION" && args.get(1).is_some_and(|items| {
                    list(items, "exact representation items").is_ok_and(|items| {
                        items.iter().any(|item| {
                            one_ref(item, "exact representation item").is_ok_and(|item| {
                                call(&entities, item).is_ok_and(|(ty, _)| matches!(ty, "MANIFOLD_SOLID_BREP" | "BREP_WITH_VOIDS"))
                            })
                        })
                    })
                })
            })
        }).collect::<Vec<_>>();
        let containers = selected.iter().copied()
            .filter(|id| !exact.contains(id) && !direct_exact.contains(id))
            .filter(|id| call(&entities, *id).is_ok_and(|(ty, _)| ty == "SHAPE_REPRESENTATION"))
            .filter(|id| {
                entities.keys().any(|relationship| {
                    relationship_components(&entities, *relationship).is_ok_and(|edge| {
                        edge.is_some_and(|(from, to, operator)| {
                            operator == 0 && (from == *id || to == *id)
                                && call(&entities, if from == *id { to } else { from }).is_ok_and(|(ty, _)| ty == "ADVANCED_BREP_SHAPE_REPRESENTATION")
                        })
                    })
                })
            })
            .collect::<Vec<_>>();
        if !exact.is_empty() || !containers.is_empty() {
            if containers.len() > 1 || containers.is_empty() && exact.len() != 1 || containers.len() == 1 && exact.len() > 1 {
                return Err(refuse("Ambiguous multiple exact B-rep representation roots"));
            }
            let mut root = containers.first().or_else(|| exact.first()).copied().unwrap();
            if !containers.is_empty() {
                if exact.iter().any(|candidate| {
                    !entities.keys().any(|relationship| {
                        relationship_components(&entities, *relationship).is_ok_and(|edge| {
                            edge.is_some_and(|(from, to, operator)| operator == 0 && (from == root && to == *candidate || from == *candidate && to == root))
                        })
                    })
                }) {
                    return Err(refuse("Ambiguous multiple exact B-rep representation roots"));
                }
                let incoming = entities.keys().filter_map(|relationship| {
                    relationship_components(&entities, *relationship).ok().flatten().and_then(|(from, to, operator)| {
                        (operator == 0 && to == root && call(&entities, from).is_ok_and(|(ty, _)| ty == "ADVANCED_BREP_SHAPE_REPRESENTATION")).then_some(from)
                    })
                }).collect::<Vec<_>>();
                if incoming.len() > 1 {
                    return Err(refuse("Ambiguous multiple exact B-rep representation roots"));
                }
                if let Some(exact) = incoming.first() {
                    auxiliary_selections.insert(root);
                    root = *exact;
                }
            }
            auxiliary_selections.extend(containers.iter().copied());
            auxiliary_selections.extend(selected.iter().copied().filter(|id| *id != root));
            selected = vec![root];
        } else if !direct_exact.is_empty() {
            if direct_exact.len() != 1 {
                return Err(refuse("Ambiguous multiple exact B-rep representation roots"));
            }
            auxiliary_selections.extend(selected.iter().copied().filter(|id| *id != direct_exact[0]));
            selected = direct_exact;
        } else {
            let allow_open_shells = matches!(capability, STEP_INTERCHANGE_V9_CAPABILITY | STEP_INTERCHANGE_V10_CAPABILITY);
            for id in &selected {
                let (ty, _) = call(&entities, *id)?;
                if ty != "ADVANCED_BREP_SHAPE_REPRESENTATION"
                    && !(allow_open_shells && ty == "SHAPE_REPRESENTATION")
                    && !(allow_open_shells && ty == "MANIFOLD_SURFACE_SHAPE_REPRESENTATION")
                {
                    return Err(refuse("Selected representation is outside the admitted direct B-rep products"));
                }
            }
        }
        selected
    } else {
        entities.keys().filter_map(|id| call(&entities, *id).ok().and_then(|(ty, _)| (ty == "ADVANCED_BREP_SHAPE_REPRESENTATION").then_some(*id))).collect()
    };
    let allow_open_shells = matches!(capability, STEP_INTERCHANGE_V9_CAPABILITY | STEP_INTERCHANGE_V10_CAPABILITY);
    let all_bodies: Vec<_> = entities.keys().filter_map(|id| {
        call(&entities, *id).ok().and_then(|(ty, _)| {
            (matches!(ty, "MANIFOLD_SOLID_BREP" | "BREP_WITH_VOIDS") || allow_open_shells && ty == "SHELL_BASED_SURFACE_MODEL").then_some(*id)
        })
    }).collect();
    let graph_roots = if representations.is_empty() { all_bodies.clone() } else { representations.clone() };
    if graph_roots.is_empty() {
        return Err(refuse("STEP /3 requires a manifold advanced B-rep root"));
    }
    let occurrences = if ap242_composition {
        representation_occurrences(
            &entities, &representations,
            allow_degenerate && capability != STEP_INTERCHANGE_V8_CAPABILITY,
            matches!(capability, STEP_INTERCHANGE_V9_CAPABILITY | STEP_INTERCHANGE_V10_CAPABILITY),
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
            let definition = format!("representation:{}", entity_digest(&entities, occurrence.representation)?);
            if !definition_identities.contains(&definition) {
                definition_identities.push(definition)
            }
            let path = occurrence.path.iter().map(|id| entity_digest(&entities, *id)).collect::<Result<Vec<_>>>()?.join("/");
            let matrix = occurrence.transform.iter().flatten().map(|value| format!("{:016x}", value.to_bits())).collect::<String>();
            occurrence_identities.push(format!("occurrence:{}", hash(&(path + &matrix))));
        }
    }
    let product_hierarchy = if allow_degenerate {
        entities.keys().filter_map(|id| {
            call(&entities, *id).ok().and_then(|(ty, args)| {
                if ty == "PRODUCT" && args.len() >= 3 {
                    Some(format!("product:#{}:{}:{}", id, string_value(&args[0], "product id").ok()?, string_value(&args[1], "product name").ok()?))
                } else if matches!(ty, "PRODUCT_DEFINITION" | "PRODUCT_DEFINITION_FORMATION" | "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE") && args.len() >= 2 {
                    Some(format!("{}:#{}:{}:{}", ty.to_ascii_lowercase(), id, string_value(&args[0], "product metadata id").ok()?, string_value(&args[1], "product metadata description").ok()?))
                } else if ty == "PRODUCT_RELATED_PRODUCT_CATEGORY" && args.len() >= 3 {
                    Some(format!("product-category:#{}:{}", id, string_value(&args[0], "product category").ok()?))
                } else if ty == "PROPERTY_DEFINITION" && args.len() >= 2 {
                    Some(format!("property-definition:#{}:{}", id, string_value(&args[0], "property name").ok()?))
                } else if ty == "NEXT_ASSEMBLY_USAGE_OCCURRENCE" && args.len() >= 5 {
                    Some(format!("assembly-use:#{}:#{}:#{}", id, one_ref(&args[3], "relating product").ok()?, one_ref(&args[4], "related product").ok()?))
                } else {
                    None
                }
            })
        }).collect()
    } else {
        Vec::new()
    };
    let external_references = if allow_degenerate {
        entities.keys().filter_map(|id| {
            let (ty, args) = call(&entities, *id).ok()?;
            if ty == "DOCUMENT_FILE" && !args.is_empty() {
                Some(format!("#{}:{}", id, string_value(&args[0], "document URI").ok()?))
            } else {
                None
            }
        }).collect::<Vec<_>>()
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
    let roots: Vec<_> = all_bodies.into_iter().filter(|id| linked.contains(id)).collect();
    if roots.is_empty() {
        return Err(refuse("Selected representation contains no manifold B-rep body"));
    }
    const REACHABLE_TYPES: &[&str] = &[
        "SHAPE_REPRESENTATION", "ADVANCED_BREP_SHAPE_REPRESENTATION", "MANIFOLD_SURFACE_SHAPE_REPRESENTATION",
        "MANIFOLD_SOLID_BREP", "BREP_WITH_VOIDS", "SHELL_BASED_SURFACE_MODEL", "CLOSED_SHELL", "OPEN_SHELL",
        "ORIENTED_CLOSED_SHELL", "ADVANCED_FACE", "FACE_OUTER_BOUND", "FACE_BOUND", "EDGE_LOOP", "VERTEX_LOOP",
        "ORIENTED_EDGE", "EDGE_CURVE", "VERTEX_POINT", "CARTESIAN_POINT", "DIRECTION", "VECTOR",
        "AXIS2_PLACEMENT_2D", "AXIS2_PLACEMENT_3D", "LINE", "CIRCLE", "ELLIPSE", "TRIMMED_CURVE",
        "B_SPLINE_CURVE_WITH_KNOTS", "PCURVE", "SURFACE_CURVE", "SEAM_CURVE", "DEFINITIONAL_REPRESENTATION",
        "PLANE", "CYLINDRICAL_SURFACE", "CONICAL_SURFACE", "SPHERICAL_SURFACE", "TOROIDAL_SURFACE",
        "B_SPLINE_SURFACE_WITH_KNOTS", "CURVE_BOUNDED_SURFACE", "GEOMETRIC_REPRESENTATION_CONTEXT",
        "GLOBAL_UNIT_ASSIGNED_CONTEXT", "SI_UNIT", "CONVERSION_BASED_UNIT",
        "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT", "UNCERTAINTY_MEASURE_WITH_UNIT", "LENGTH_MEASURE_WITH_UNIT",
        "PLANE_ANGLE_MEASURE_WITH_UNIT", "MEASURE_WITH_UNIT", "DIMENSIONAL_EXPONENTS",
        "ITEM_DEFINED_TRANSFORMATION", "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION",
        "CARTESIAN_TRANSFORMATION_OPERATOR_3D", "GEOMETRIC_TOLERANCE",
        "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE", "POSITION_TOLERANCE", "TESSELLATED_GEOMETRIC_SET",
    ];
    loop {
        let previous = auxiliary_selections.len();
        for id in entities.keys() {
            if let Some((from, to, 0)) = relationship_components(&entities, *id)?
                && (representations.contains(&from) || auxiliary_selections.contains(&from))
                && call(&entities, to)?.0 != "ADVANCED_BREP_SHAPE_REPRESENTATION"
            {
                auxiliary_selections.insert(to);
            }
        }
        if auxiliary_selections.len() == previous { break; }
    }
    for rep in &representations {
        let (_, args) = call(&entities, *rep)?;
        if let Some(items) = args.get(1) {
            for item in list(items, "selected representation items")? {
                let item = one_ref(item, "selected representation item")?;
                if matches!(call(&entities, item)?.0, "MAPPED_ITEM" | "GEOMETRIC_CURVE_SET") {
                    auxiliary_selections.insert(item);
                }
            }
        }
    }
    let auxiliary_linked = if auxiliary_selections.is_empty() {
        BTreeSet::new()
    } else {
        reachable(&entities, &auxiliary_selections.into_iter().collect::<Vec<_>>())?
    };
    linked.retain(|id| !auxiliary_linked.contains(id));
    for id in &linked {
        if allow_degenerate && capability != STEP_INTERCHANGE_V8_CAPABILITY
            && component(&components(&entities, *id)?, "CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM").is_some()
        {
            continue;
        }
        let (ty, _) = call(&entities, *id)?;
        if !REACHABLE_TYPES.contains(&ty) && !matches!(ty, "COMPLEX") {
            return Err(refuse(format!("Reachable entity {ty} is outside the direct finite graph subset")));
        }
    }
    let mut ignored = BTreeSet::new();
    for id in entities.keys() {
        if linked.contains(id) { continue; }
        if auxiliary_linked.contains(id) {
            ignored.insert(call(&entities, *id)?.0.to_string());
            continue;
        }
        if ap242_composition && relationship_components(&entities, *id)?.is_some() { continue; }
        if allow_degenerate && capability != STEP_INTERCHANGE_V8_CAPABILITY
            && component(&components(&entities, *id)?, "CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM").is_some()
        {
            continue;
        }
        if matches!(entities[id].value, Value::List(_)) {
            let parts = components(&entities, *id)?;
            if parts.iter().all(|(ty, _)| UNREACHABLE_ALLOWLIST.contains(ty)) {
                ignored.extend(parts.into_iter().map(|(ty, _)| ty.to_string()));
                continue;
            }
        }
        let (ty, _) = call(&entities, *id)?;
        if allow_degenerate && matches!(ty, "NEXT_ASSEMBLY_USAGE_OCCURRENCE" | "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION") {
            continue;
        }
        if allow_degenerate && matches!(ty, "DOCUMENT_FILE" | "DOCUMENT" | "DOCUMENT_TYPE" | "DOCUMENT_REPRESENTATION_TYPE" | "EXTERNALLY_DEFINED_ITEM" | "EXTERNAL_SOURCE" | "APPLIED_EXTERNAL_IDENTIFICATION_ASSIGNMENT") {
            continue;
        }
        if UNREACHABLE_ALLOWLIST.contains(&ty) {
            ignored.insert(ty.to_string());
        } else {
            return Err(refuse(format!("Unreachable entity {ty} is not presentation metadata allowlisted by /3")));
        }
    }
    let (mut model, identity_map) = if ap242_composition {
        if occurrences.is_empty() {
            return Err(refuse("Selected placement graph contains no manifold B-rep occurrence"));
        }
        let mut aggregate = Model::empty(DEFAULT_STEP_TOLERANCE_MM)?;
        let mut aggregate_tolerance = None::<f64>;
        let mut aggregate_map = Vec::new();
        for occurrence in occurrences {
            let scale = length_scale(&entities, &occurrence.linked, false)?;
            let tolerance_mm = uncertainty_mm(&entities, &occurrence.linked)?.unwrap_or(DEFAULT_STEP_TOLERANCE_MM);
            aggregate_tolerance = Some(aggregate_tolerance.map_or(tolerance_mm, |current| current.min(tolerance_mm)));
            let (mut component, map) = build_direct_roots(
                &entities, &occurrence.bodies, scale,
                DirectBuildOptions {
                    analytic_surfaces, ap242_composition: true, allow_degenerate,
                    whole_domain_proofs: matches!(capability, STEP_INTERCHANGE_V7_CAPABILITY | STEP_INTERCHANGE_V8_CAPABILITY | STEP_INTERCHANGE_V9_CAPABILITY | STEP_INTERCHANGE_V10_CAPABILITY),
                    interior_point_selectors: matches!(capability, STEP_INTERCHANGE_V9_CAPABILITY | STEP_INTERCHANGE_V10_CAPABILITY),
                    allow_open_shells,
                    preserve_authored_charts: capability == STEP_INTERCHANGE_V6_CAPABILITY,
                },
            )?;
            component.tolerance_mm = tolerance_mm;
            let component = crate::transform::affine(&component, occurrence.transform)?;
            append_direct_model(&mut aggregate, component, &map, &mut aggregate_map);
        }
        aggregate.tolerance_mm = aggregate_tolerance.unwrap_or(DEFAULT_STEP_TOLERANCE_MM);
        aggregate.rebuild_topology_ids();
        (aggregate, aggregate_map)
    } else {
        let scale = length_scale(&entities, &linked, true)?;
        let tolerance_mm = uncertainty_mm(&entities, &linked)?.unwrap_or(DEFAULT_STEP_TOLERANCE_MM);
        let placement = reachable_transform(&entities, &linked, scale, false)?;
        let (mut model, map) = build_direct_roots(
            &entities, &roots, scale,
            DirectBuildOptions {
                analytic_surfaces, ap242_composition: false, allow_degenerate,
                whole_domain_proofs: matches!(capability, STEP_INTERCHANGE_V7_CAPABILITY | STEP_INTERCHANGE_V8_CAPABILITY | STEP_INTERCHANGE_V9_CAPABILITY | STEP_INTERCHANGE_V10_CAPABILITY),
                interior_point_selectors: matches!(capability, STEP_INTERCHANGE_V9_CAPABILITY | STEP_INTERCHANGE_V10_CAPABILITY),
                allow_open_shells,
                preserve_authored_charts: capability == STEP_INTERCHANGE_V6_CAPABILITY,
            },
        )?;
        model.tolerance_mm = tolerance_mm;
        if let Some(matrix) = placement {
            model = crate::transform::affine(&model, matrix)?
        }
        (model, map)
    };
    let identity = if occurrence_count > 1 && (matches!(capability, STEP_INTERCHANGE_V9_CAPABILITY | STEP_INTERCHANGE_V10_CAPABILITY) || allow_degenerate) {
        crate::step_interchange::StepIdentityReport {
            preserved: false, source: "assembly-definition-plus-occurrence",
            preserved_count: 0, created_count: identity_map.len(), lost_count: 0,
        }
    } else {
        restore_identity(&entities, &identity_map, &mut model)?
    };
    model.validate()?;
    let mut metadata_loss = if identity.preserved {
        vec![]
    } else if allow_degenerate {
        vec!["topology.occurrence_instance_ids_created".into(), "presentation_style".into()]
    } else {
        vec!["representation_item.name".into(), "product.id".into(), "product.name".into(), "product.description".into(), "presentation_style".into()]
    };
    if !auxiliary_linked.is_empty() {
        metadata_loss.push("projection.non_exact_representations_retained_as_metadata".into());
    }
    let mut certificate_notes = vec![
        "direct_part21_topology", "line_circle_ellipse_trimmed_curve",
        "plane_and_rational_multispan_bspline_surface", "periodic_analytic_surfaces_typed_refused_parameterization",
        "shared_topology_senses_holes", "multi_body_multi_cavity", "si_and_positive_conversion_units",
        "reachable_rigid_placements_only", "independent_curve_pcurve_check", "bounded_reachable_graph",
    ];
    if matches!(capability, STEP_INTERCHANGE_V7_CAPABILITY | STEP_INTERCHANGE_V8_CAPABILITY | STEP_INTERCHANGE_V9_CAPABILITY | STEP_INTERCHANGE_V10_CAPABILITY) {
        certificate_notes.push("whole_domain_rational_boundary_and_constant_pole_identity");
    }
    if matches!(capability, STEP_INTERCHANGE_V9_CAPABILITY | STEP_INTERCHANGE_V10_CAPABILITY) {
        certificate_notes.extend([
            "certified_trimmed_curve_point_inverse", "bounded_freeform_collapsed_boundary_ownership",
            "invertible_general_affine_occurrences", "open_shell_surface_products",
        ]);
    }
    let semantic = include_semantic.then(|| {
        let xde = step_xde::parse(&entities, &identity_map)?;
        let presentation = step_presentation::parse(&entities, &identity_map, &xde.body_products, xde.product_parents.clone())?;
        Ok::<StepSemanticMetadata, Error>(StepSemanticMetadata { presentation, xde })
    }).transpose()?;
    if semantic.is_some() {
        certificate_notes.extend(["ap214_face_body_product_presentation", "typed_xde_product_attributes"]);
    }
    Ok((
        model,
        FeatureCertificate { capability, complete: true, notes: certificate_notes },
        StepV3Report {
            identity, ignored_entities: ignored.into_iter().collect(), metadata_loss,
            instance_count: entities.len(), reachable_count: linked.len(),
            definition_identities, occurrence_identities, product_hierarchy, external_references,
        },
        semantic,
    ))
}

pub(super) fn import_step_direct(
    text: &str, analytic_surfaces: bool, ap242_composition: bool, allow_degenerate: bool, capability: &'static str,
) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    let (model, certificate, report, _) = import_step_direct_impl(text, analytic_surfaces, ap242_composition, allow_degenerate, capability, false)?;
    Ok((model, certificate, report))
}

pub fn import_step_v3(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    import_step_direct(text, false, false, false, STEP_INTERCHANGE_V3_CAPABILITY)
}

pub fn import_step_v3_with_metadata(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report, StepSemanticMetadata)> {
    let (model, certificate, report, semantic) = import_step_direct_impl(text, false, false, false, STEP_INTERCHANGE_V3_CAPABILITY, true)?;
    Ok((model, certificate, report, semantic.unwrap()))
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

pub fn import_step_v7(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    import_step_direct(text, true, true, true, STEP_INTERCHANGE_V7_CAPABILITY)
}

pub fn import_step_v8(text: &str) -> Result<(Model, StepV8Certificate, StepV3Report)> {
    let (model, _, report) = import_step_direct(text, true, true, true, STEP_INTERCHANGE_V8_CAPABILITY)?;
    let certificate = certify_step_v8_topology(&model)?;
    Ok((model, certificate, report))
}

pub fn import_step_v9(text: &str) -> Result<(Model, StepV8Certificate, StepV3Report)> {
    let (model, _, report) = import_step_direct(text, true, true, true, STEP_INTERCHANGE_V9_CAPABILITY)?;
    let mut certificate = certify_step_v8_topology(&model)?;
    certificate.capability = STEP_INTERCHANGE_V9_CAPABILITY;
    certificate.notes.extend([
        "certified_interior_point_selectors", "bounded_freeform_poles",
        "general_affine_occurrences", "retained_open_shell_products",
    ]);
    Ok((model, certificate, report))
}

pub fn import_step_v9_with_metadata(text: &str) -> Result<(Model, StepV8Certificate, StepV3Report, StepSemanticMetadata)> {
    let (model, _, report, semantic) = import_step_direct_impl(text, true, true, true, STEP_INTERCHANGE_V9_CAPABILITY, true)?;
    let mut certificate = certify_step_v8_topology(&model)?;
    certificate.capability = STEP_INTERCHANGE_V9_CAPABILITY;
    certificate.notes.extend([
        "certified_interior_point_selectors", "bounded_freeform_poles",
        "general_affine_occurrences", "retained_open_shell_products",
        "ap214_face_body_product_presentation", "typed_xde_product_attributes",
    ]);
    Ok((model, certificate, report, semantic.unwrap()))
}

fn step_v10_graph_identity(entities: &BTreeMap<usize, Entity>) -> Result<String> {
    let mut nodes = entities.keys().map(|id| entity_digest(entities, *id)).collect::<Result<Vec<_>>>()?;
    nodes.sort();
    Ok(hash(&nodes.join("\n")))
}

pub fn import_step_v10(text: &str) -> Result<(Model, StepV8Certificate, StepV10Document)> {
    let (model, _, report) = import_step_direct(text, true, true, true, STEP_INTERCHANGE_V10_CAPABILITY)?;
    let mut certificate = certify_step_v8_topology(&model)?;
    certificate.notes.extend([
        "certified_interior_point_selectors", "bounded_freeform_poles",
        "general_affine_occurrences", "retained_open_shell_products",
    ]);
    let entities = parse(data_payload(text)?)?;
    let mut operator_identities = Vec::new();
    for id in entities.keys() {
        if let Some((_, _, operator)) = relationship_components(&entities, *id)? && operator != 0 {
            operator_identities.push(format!("operator:{}", entity_digest(&entities, operator)?));
        }
    }
    operator_identities.sort();
    operator_identities.dedup();
    certificate.capability = STEP_INTERCHANGE_V10_CAPABILITY;
    certificate.notes.extend(["retained_affine_occurrence_graph", "exact_graph_isomorphism_identity"]);
    let document = StepV10Document {
        source: text.to_string(),
        graph_identity: step_v10_graph_identity(&entities)?,
        definition_identities: report.definition_identities,
        occurrence_identities: report.occurrence_identities,
        product_hierarchy: report.product_hierarchy,
        operator_identities,
        metadata_loss: report.metadata_loss,
    };
    Ok((model, certificate, document))
}

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
