use super::*;

/// Identity subset of validateOccurrences: positional IDs, canonical-row
/// parent/staticParent references, re-derived occv1:/entity:v2: digests,
/// per-ID output-slot density, dynamic-slot duplicate-ordinal discipline, the
/// staticParent runtime ancestor-chain walk, and $expansion continuation /
/// module-activation anchoring (ports of isChildrenExpansionContinuation and
/// moduleActivationAnchor).
pub fn validate_occurrence_identity(
    occurrences: &[Value],
    operations: &[Value],
    node_count: usize,
) -> Result<()> {
    if occurrences.len() > 100_000 {
        return Err(invalid("Semantic occurrence limit exceeded"));
    }
    if operations.is_empty() && !occurrences.is_empty() {
        return Err(invalid("A semantic occurrence requires an operation"));
    }
    let operation_parents: Vec<Option<usize>> = operations
        .iter()
        .map(|operation| {
            operation["parent"]
                .as_u64()
                .and_then(|value| usize::try_from(value).ok())
        })
        .collect();
    let operation_ids: Vec<&str> = operations
        .iter()
        .map(|operation| {
            operation["operationId"]
                .as_str()
                .ok_or_else(|| invalid("Expected a semantic operation ID"))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut scene_ids: HashSet<String> = HashSet::new();
    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    let mut group_order: Vec<String> = Vec::new();
    #[allow(clippy::type_complexity)]
    let mut duplicate_slots: HashMap<
        (Option<usize>, usize, usize, String, String),
        Vec<(u64, String)>,
    > = HashMap::new();
    let mut occurrence_ids: Vec<String> = Vec::with_capacity(occurrences.len());
    let mut rows: Vec<OccurrenceRow> = Vec::with_capacity(occurrences.len());
    let mut reparented_static_roots: Vec<usize> = Vec::new();
    #[allow(clippy::type_complexity)]
    let mut pending_continuations: Vec<(usize, usize, Option<usize>, usize, Vec<Value>)> =
        Vec::new();
    let mut proof_budget: usize = 0;
    for (index, occurrence) in occurrences.iter().enumerate() {
        exact(
            occurrence,
            &[
                "id",
                "occurrenceId",
                "operation",
                "parent",
                "staticParent",
                "dynamicSlots",
                "node",
                "outputOrdinal",
                "sceneEntityId",
            ],
        )?;
        if occurrence["id"].as_u64() != Some(index as u64) {
            return Err(invalid(
                "Semantic occurrence IDs must equal array positions",
            ));
        }
        let operation = occurrence["operation"]
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .filter(|value| *value < operations.len())
            .ok_or_else(|| invalid("Semantic occurrence operation reference is invalid"))?;
        let parent = optional_row_reference(&occurrence["parent"], index)?;
        let static_parent = optional_row_reference(&occurrence["staticParent"], index)?;
        let slots = validate_dynamic_slots(&occurrence["dynamicSlots"])?;
        match operation_parents[operation] {
            None => {
                if static_parent.is_some() {
                    return Err(invalid(
                        "A static-root operation requires a null staticParent",
                    ));
                }
                if parent.is_some() {
                    let root_operation = &operations[operation];
                    let root_path = operation_path(root_operation)?;
                    let last = root_path
                        .last()
                        .ok_or_else(|| invalid("Expected a semantic structural path"))?;
                    if operation_category(root_operation)? != "module"
                        || segment_parts(last).0 != Some("module")
                    {
                        return Err(invalid(
                            "A static-root operation cannot have a runtime parent outside a module-definition activation",
                        ));
                    }
                    reparented_static_roots.push(index);
                }
            }
            Some(static_parent_operation) => {
                let referenced = static_parent.ok_or_else(|| {
                    invalid("staticParent must instantiate the static parent operation")
                })?;
                if rows[referenced].operation != static_parent_operation {
                    return Err(invalid(
                        "staticParent must instantiate the static parent operation",
                    ));
                }
                // Walk the runtime ancestor chain from parent up to the
                // staticParent row: only control/module frames may separate
                // them, and the static parent must be the nearest match.
                let mut cursor = parent;
                let mut crossed_non_expansion_frame = false;
                while cursor != Some(referenced) {
                    consume_proof_budget(&mut proof_budget, 1)?;
                    let Some(skipped_index) = cursor else {
                        return Err(invalid("staticParent is not on the runtime ancestor chain"));
                    };
                    let skipped = &rows[skipped_index];
                    if skipped.operation == static_parent_operation {
                        return Err(invalid(
                            "staticParent must be the nearest matching ancestor",
                        ));
                    }
                    let category = operation_category(&operations[skipped.operation])?;
                    if category != "control" && category != "module" {
                        crossed_non_expansion_frame = true;
                    }
                    cursor = skipped.parent;
                }
                if operation_name(&operations[operation])? == "$expansion" {
                    pending_continuations.push((
                        index,
                        operation,
                        parent,
                        referenced,
                        slots.clone(),
                    ));
                } else if crossed_non_expansion_frame {
                    return Err(invalid(
                        "Only control/module frames may separate parent from staticParent",
                    ));
                }
            }
        }
        let parent_occurrence_id = parent.map(|row| occurrence_ids[row].as_str());
        let static_parent_occurrence_id = static_parent.map(|row| occurrence_ids[row].as_str());
        let expected_id = derive_occurrence_id(
            parent_occurrence_id,
            static_parent_occurrence_id,
            operation_ids[operation],
            &slots,
        );
        if occurrence["occurrenceId"].as_str() != Some(expected_id.as_str()) {
            return Err(invalid(
                "Semantic occurrence ID does not match operation, parent, and dynamic slots",
            ));
        }
        if let Some(group) = groups.get(&expected_id) {
            let first = group[0];
            let first_row = &rows[first];
            if first_row.operation != operation
                || first_row.parent != parent
                || first_row.static_parent != static_parent
                || !same_dynamic_slots(&first_row.slots, &slots)?
            {
                return Err(invalid(
                    "Rows sharing an occurrence ID must describe the same logical evaluation",
                ));
            }
        } else {
            group_order.push(expected_id.clone());
        }
        groups.entry(expected_id.clone()).or_default().push(index);
        for (slot_index, slot) in slots.iter().enumerate() {
            let key = (
                parent,
                operation,
                slot_index,
                slot["name"].as_str().unwrap_or_default().to_owned(),
                identity_value_key(&slot["value"])?,
            );
            let ordinal = slot["duplicateOrdinal"]
                .as_u64()
                .ok_or_else(|| invalid("Expected a semantic duplicate ordinal"))?;
            let ordinals = duplicate_slots.entry(key).or_default();
            if let Some((_, existing)) = ordinals.iter().find(|(known, _)| *known == ordinal) {
                if *existing != expected_id {
                    return Err(invalid(
                        "Duplicate ordinals must distinguish equal dynamic values",
                    ));
                }
            } else {
                if ordinal != ordinals.len() as u64 {
                    return Err(invalid(
                        "Duplicate ordinals must be dense in language evaluation order",
                    ));
                }
                ordinals.push((ordinal, expected_id.clone()));
            }
        }
        let node = if occurrence["node"].is_null() {
            None
        } else {
            let node = occurrence["node"]
                .as_u64()
                .and_then(|value| usize::try_from(value).ok())
                .filter(|value| *value < node_count)
                .ok_or_else(|| invalid("Semantic occurrence node reference is invalid"))?;
            Some(node)
        };
        match node {
            None => {
                if !occurrence["outputOrdinal"].is_null() || !occurrence["sceneEntityId"].is_null()
                {
                    return Err(invalid(
                        "A non-producing occurrence cannot own an output identity",
                    ));
                }
            }
            Some(_) => {
                if occurrence["outputOrdinal"].is_null() {
                    if !occurrence["sceneEntityId"].is_null() {
                        return Err(invalid(
                            "A producer-only occurrence cannot claim scene identity",
                        ));
                    }
                } else {
                    let ordinal = bounded_integer(&occurrence["outputOrdinal"], 1_000_000)?;
                    let expected_scene = derive_scene_entity_id(&expected_id, ordinal);
                    if occurrence["sceneEntityId"].as_str() != Some(expected_scene.as_str()) {
                        return Err(invalid(
                            "Scene entity ID does not match occurrence and output ordinal",
                        ));
                    }
                    if !scene_ids.insert(expected_scene) {
                        return Err(invalid("Scene entity IDs must be unique"));
                    }
                }
            }
        }
        occurrence_ids.push(expected_id);
        rows.push(OccurrenceRow {
            operation,
            parent,
            static_parent,
            slots,
        });
    }
    // $expansion continuation and module-activation anchoring (host
    // moduleActivationAnchor / isChildrenExpansionContinuation ports).
    let mut operation_children: Vec<Vec<usize>> = vec![Vec::new(); operations.len()];
    for operation in operations.iter() {
        if let Some(parent) = operation["parent"]
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
        {
            operation_children[parent].push(
                operation["id"]
                    .as_u64()
                    .and_then(|value| usize::try_from(value).ok())
                    .ok_or_else(|| invalid("Expected a semantic operation ID"))?,
            );
        }
    }
    let mut runtime_children: Vec<Vec<usize>> = vec![Vec::new(); occurrences.len()];
    for (index, row) in rows.iter().enumerate() {
        if let Some(parent) = row.parent {
            runtime_children[parent].push(index);
        }
    }
    let mut occurrence_index = OccurrenceIndex {
        operations,
        rows: &rows,
        operation_children,
        runtime_children,
        activation_by_definition: HashMap::new(),
        proof_budget,
    };
    for definition_occurrence in reparented_static_roots {
        if module_activation_anchor(definition_occurrence, &mut occurrence_index)?.is_none() {
            return Err(invalid(
                "Module-definition activation is not anchored to its exact matching call body",
            ));
        }
    }
    for (occurrence, operation, parent, static_parent, slots) in pending_continuations {
        if !is_children_expansion_continuation(
            occurrence,
            operation,
            parent,
            static_parent,
            &slots,
            &mut occurrence_index,
        )? {
            return Err(invalid(
                "$expansion must be the exact children() continuation of its matching caller",
            ));
        }
    }
    // parent/staticParent must reference the canonical first row of its ID.
    for (index, occurrence) in occurrences.iter().enumerate() {
        for field_name in ["parent", "staticParent"] {
            let Some(reference) = optional_row_reference(&occurrence[field_name], index)? else {
                continue;
            };
            let referenced_id = &occurrence_ids[reference];
            if groups
                .get(referenced_id)
                .and_then(|group| group.first())
                .copied()
                != Some(reference)
            {
                return Err(invalid(
                    "Occurrence references must target the canonical first row of a logical occurrence",
                ));
            }
        }
    }
    // Rows sharing an ID: one zero/frame row, or dense output rows 0..k.
    for id in &group_order {
        let group = &groups[id];
        let output_rows: Vec<usize> = group
            .iter()
            .copied()
            .filter(|index| !occurrences[*index]["outputOrdinal"].is_null())
            .collect();
        if group.len() > 1 && output_rows.len() != group.len() {
            return Err(invalid(
                "An occurrence ID may repeat only for its output slots",
            ));
        }
        for (ordinal, index) in output_rows.iter().enumerate() {
            if occurrences[*index]["outputOrdinal"].as_u64() != Some(ordinal as u64) {
                return Err(invalid(
                    "Output ordinals must be unique, ordered, and dense from zero per occurrence ID",
                ));
            }
        }
    }
    for ordinals in duplicate_slots.values() {
        let mut ordered: Vec<u64> = ordinals.iter().map(|(ordinal, _)| *ordinal).collect();
        ordered.sort_unstable();
        if ordered
            .iter()
            .enumerate()
            .any(|(index, ordinal)| *ordinal != index as u64)
        {
            return Err(invalid(
                "Duplicate ordinals for equal dynamic slot values must be dense from zero",
            ));
        }
    }
    Ok(())
}
