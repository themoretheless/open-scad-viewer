use super::*;

/// Shared indexes built after the identity loop (SemanticOccurrenceValidationIndex).
pub(super) struct OccurrenceIndex<'a> {
    pub(super) operations: &'a [Value],
    pub(super) rows: &'a [OccurrenceRow],
    pub(super) operation_children: Vec<Vec<usize>>,
    pub(super) runtime_children: Vec<Vec<usize>>,
    pub(super) activation_by_definition: HashMap<usize, Option<ModuleAnchor>>,
    pub(super) proof_budget: usize,
}

/// moduleActivationAnchor: proves a re-parented module-definition occurrence
/// is anchored to its exact matching call body (cached per definition row).
pub(super) fn module_activation_anchor(
    definition_occurrence_index: usize,
    index: &mut OccurrenceIndex,
) -> Result<Option<ModuleAnchor>> {
    if let Some(cached) = index
        .activation_by_definition
        .get(&definition_occurrence_index)
    {
        return Ok(*cached);
    }
    let operations = index.operations;
    let rows = index.rows;
    let reject = |index: &mut OccurrenceIndex| {
        index
            .activation_by_definition
            .insert(definition_occurrence_index, None);
        None
    };
    let definition_occurrence = &rows[definition_occurrence_index];
    let definition_operation = operations
        .get(definition_occurrence.operation)
        .ok_or_else(|| invalid("Semantic occurrence operation reference is invalid"))?;
    let definition_path = operation_path(definition_operation)?;
    let definition_last = definition_path
        .last()
        .ok_or_else(|| invalid("Expected a semantic structural path"))?;
    if definition_operation["parent"].as_u64().is_some()
        || operation_category(definition_operation)? != "module"
        || operation_name(definition_operation)?.starts_with('$')
        || segment_parts(definition_last).0 != Some("module")
    {
        return Ok(reject(index));
    }
    let Some(body_occurrence_index) = definition_occurrence.parent else {
        return Ok(reject(index));
    };
    let body_occurrence = &rows[body_occurrence_index];
    let body_operation = operations
        .get(body_occurrence.operation)
        .ok_or_else(|| invalid("Semantic occurrence operation reference is invalid"))?;
    let body_path = operation_path(body_operation)?;
    let body_segment = body_path
        .last()
        .ok_or_else(|| invalid("Expected a semantic structural path"))?;
    let body_segment_parts = segment_parts(body_segment);
    if operation_category(body_operation)? != "control"
        || operation_name(body_operation)? != "$body"
        || body_segment_parts.0 != Some("body")
        || body_segment_parts.1 != Some("$body")
        || body_segment_parts.2 != Some(0)
        || body_operation["parent"].as_u64().is_none()
    {
        return Ok(reject(index));
    }
    let Some(call_occurrence_index) = body_occurrence.parent else {
        return Ok(reject(index));
    };
    let call_occurrence = &rows[call_occurrence_index];
    let call_operation = operations
        .get(call_occurrence.operation)
        .ok_or_else(|| invalid("Semantic occurrence operation reference is invalid"))?;
    let call_path = operation_path(call_operation)?;
    let call_last = call_path
        .last()
        .ok_or_else(|| invalid("Expected a semantic structural path"))?;
    if body_operation["parent"]
        .as_u64()
        .and_then(|v| usize::try_from(v).ok())
        != Some(call_occurrence.operation)
        || operation_category(call_operation)? != "module"
        || operation_name(call_operation)?.starts_with('$')
        || segment_parts(call_last).0 != Some("call")
        || operation_name(call_operation)? != operation_name(definition_operation)?
        || !same_dynamic_bindings(&call_occurrence.slots, &definition_occurrence.slots)?
    {
        return Ok(reject(index));
    }

    let static_bodies = index
        .operation_children
        .get(call_occurrence.operation)
        .cloned()
        .unwrap_or_default();
    let static_expansions = index
        .operation_children
        .get(body_occurrence.operation)
        .cloned()
        .unwrap_or_default();
    consume_proof_budget(
        &mut index.proof_budget,
        static_bodies.len() + static_expansions.len(),
    )?;
    if static_bodies.len() != 1
        || static_bodies[0] != body_occurrence.operation
        || static_expansions.len() != 1
    {
        return Ok(reject(index));
    }
    let sole_static_expansion = static_expansions[0];
    let expansion_operation = operations
        .get(sole_static_expansion)
        .ok_or_else(|| invalid("Semantic occurrence operation reference is invalid"))?;
    let expansion_path = operation_path(expansion_operation)?;
    let expansion_segment = expansion_path
        .last()
        .ok_or_else(|| invalid("Expected a semantic structural path"))?;
    let expansion_segment_parts = segment_parts(expansion_segment);
    if operation_category(expansion_operation)? != "control"
        || operation_name(expansion_operation)? != "$expansion"
        || expansion_segment_parts.0 != Some("control")
        || expansion_segment_parts.1 != Some("$expansion")
        || expansion_segment_parts.2 != Some(0)
    {
        return Ok(reject(index));
    }

    let call_runtime_children = index
        .runtime_children
        .get(call_occurrence_index)
        .cloned()
        .unwrap_or_default();
    let body_runtime_children = index
        .runtime_children
        .get(body_occurrence_index)
        .cloned()
        .unwrap_or_default();
    consume_proof_budget(
        &mut index.proof_budget,
        call_runtime_children.len() + body_runtime_children.len(),
    )?;
    if call_runtime_children.len() != 1
        || call_runtime_children[0] != body_occurrence_index
        || body_runtime_children.len() != 1
        || body_runtime_children[0] != definition_occurrence_index
    {
        return Ok(reject(index));
    }
    let anchor = ModuleAnchor {
        call_occurrence: call_occurrence_index,
        body_occurrence: body_occurrence_index,
        definition_occurrence: definition_occurrence_index,
        expansion_operation: sole_static_expansion,
    };
    index
        .activation_by_definition
        .insert(definition_occurrence_index, Some(anchor));
    Ok(Some(anchor))
}

/// isChildrenExpansionContinuation: an $expansion occurrence row is the exact
/// bounded continuation of its consuming children() call.
pub(super) fn is_children_expansion_continuation(
    occurrence_index: usize,
    operation: usize,
    parent: Option<usize>,
    static_parent: usize,
    slots: &[Value],
    index: &mut OccurrenceIndex,
) -> Result<bool> {
    let Some(parent) = parent else {
        return Ok(false);
    };
    let operations = index.operations;
    let rows = index.rows;
    let expansion_operation = operations
        .get(operation)
        .ok_or_else(|| invalid("Semantic occurrence operation reference is invalid"))?;
    let children_occurrence = &rows[parent];
    let children_operation = operations
        .get(children_occurrence.operation)
        .ok_or_else(|| invalid("Semantic occurrence operation reference is invalid"))?;
    let caller_body_occurrence = &rows[static_parent];
    let expansion_path = operation_path(expansion_operation)?;
    let children_path = operation_path(children_operation)?.to_vec();
    let expansion_segment = expansion_path
        .last()
        .ok_or_else(|| invalid("Expected a semantic structural path"))?;
    let children_segment = children_path
        .last()
        .ok_or_else(|| invalid("Expected a semantic structural path"))?;
    let expansion_segment_parts = segment_parts(expansion_segment);
    let children_segment_parts = segment_parts(children_segment);
    if operation_name(expansion_operation)? != "$expansion"
        || operation_category(expansion_operation)? != "control"
        || expansion_segment_parts.0 != Some("control")
        || expansion_segment_parts.1 != Some("$expansion")
        || expansion_segment_parts.2 != Some(0)
        || expansion_operation["parent"]
            .as_u64()
            .and_then(|v| usize::try_from(v).ok())
            != Some(caller_body_occurrence.operation)
        || operation_name(children_operation)? != "children"
        || operation_category(children_operation)? != "control"
        || children_segment_parts.0 != Some("control")
        || children_segment_parts.1 != Some("children")
    {
        return Ok(false);
    }

    let children_slots = &children_occurrence.slots;
    if children_slots.len() != 1
        || slots.len() != 1
        || children_slots[0]["name"].as_str() != Some("$index")
        || slots[0]["name"].as_str() != Some("$index")
        || slot_value_key(&children_slots[0])? != slot_value_key(&slots[0])?
    {
        return Ok(false);
    }

    // A selected `!children()` continuation hangs directly beneath its
    // consuming children() call; all expanded statements must stay in that
    // exact static subtree.
    if static_parent == parent && caller_body_occurrence.operation == children_occurrence.operation
    {
        let static_children = index
            .operation_children
            .get(children_occurrence.operation)
            .cloned()
            .unwrap_or_default();
        let runtime_children = index
            .runtime_children
            .get(parent)
            .cloned()
            .unwrap_or_default();
        let expanded_children = index
            .runtime_children
            .get(occurrence_index)
            .cloned()
            .unwrap_or_default();
        consume_proof_budget(
            &mut index.proof_budget,
            static_children.len() + runtime_children.len() + expanded_children.len(),
        )?;
        let expanded_stay_in_subtree = expanded_children.iter().all(|candidate| {
            let candidate_operation = &operations[rows[*candidate].operation];
            candidate_operation["parent"]
                .as_u64()
                .and_then(|v| usize::try_from(v).ok())
                == Some(operation)
        });
        return Ok(static_children.len() == 1
            && static_children[0] == operation
            && runtime_children.len() == 1
            && runtime_children[0] == occurrence_index
            && expanded_stay_in_subtree);
    }

    let mut cursor = children_occurrence.parent;
    let mut definition_occurrence: Option<usize> = None;
    while let Some(candidate_index) = cursor {
        if candidate_index == static_parent {
            break;
        }
        let candidate = &rows[candidate_index];
        let candidate_operation = operations
            .get(candidate.operation)
            .ok_or_else(|| invalid("Semantic occurrence operation reference is invalid"))?;
        let candidate_path = operation_path(candidate_operation)?;
        consume_proof_budget(&mut index.proof_budget, candidate_path.len())?;
        let candidate_last = candidate_path
            .last()
            .ok_or_else(|| invalid("Expected a semantic structural path"))?;
        if segment_parts(candidate_last).0 == Some("module")
            && path_strictly_contains(candidate_path, &children_path)
        {
            definition_occurrence = Some(candidate_index);
            break;
        }
        consume_proof_budget(&mut index.proof_budget, 1)?;
        cursor = candidate.parent;
    }
    let Some(definition_index) = definition_occurrence else {
        return Ok(false);
    };
    if rows[definition_index].parent != Some(static_parent) {
        return Ok(false);
    }
    let definition_operation = operations
        .get(rows[definition_index].operation)
        .ok_or_else(|| invalid("Semantic occurrence operation reference is invalid"))?;
    let activation = module_activation_anchor(definition_index, index)?;
    let Some(activation) = activation else {
        return Ok(false);
    };
    if activation.body_occurrence != static_parent || activation.expansion_operation != operation {
        return Ok(false);
    }
    let definition_path = operation_path(definition_operation)?.to_vec();
    consume_proof_budget(&mut index.proof_budget, definition_path.len())?;
    let lexically_contains_children = path_strictly_contains(&definition_path, &children_path);
    let expansion_runtime_children = index
        .runtime_children
        .get(parent)
        .cloned()
        .unwrap_or_default();
    let expanded_children = index
        .runtime_children
        .get(occurrence_index)
        .cloned()
        .unwrap_or_default();
    consume_proof_budget(
        &mut index.proof_budget,
        expansion_runtime_children.len() + expanded_children.len(),
    )?;
    let expanded_children_stay_in_static_subtree = expanded_children.iter().all(|candidate| {
        let candidate_operation = &operations[rows[*candidate].operation];
        candidate_operation["parent"]
            .as_u64()
            .and_then(|v| usize::try_from(v).ok())
            == Some(operation)
    });
    Ok(lexically_contains_children
        && expansion_runtime_children.len() == 1
        && expansion_runtime_children[0] == occurrence_index
        && expanded_children_stay_in_static_subtree)
}
