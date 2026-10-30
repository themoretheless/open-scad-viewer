//! iges interchange v2 algorithm: filter_discarded_entities.
use super::*;
pub fn filter_discarded_entities(
    entities: &mut BTreeMap<usize, Entity>,
) -> Result<IgesV2DiscardReport> {
    let mut work = 0;
    let mut live = BTreeSet::new();
    let mut blanked = 0;
    for (&de, entity) in entities.iter() {
        graph_work(&mut work, 1)?;
        if entity.directory.status.blanked {
            blanked += 1;
        } else {
            live.insert(de);
        }
    }

    let mut references = BTreeMap::new();
    let mut physical_owners = BTreeMap::<usize, BTreeSet<usize>>::new();
    let mut logical_owners = BTreeMap::<usize, BTreeSet<usize>>::new();
    let mut dependents_by_owner = BTreeMap::<usize, BTreeSet<usize>>::new();
    for &de in &live {
        let entity = &entities[&de];
        let refs = entity_references(de, entity)?;
        graph_work(&mut work, refs.len())?;
        for reference in &refs {
            let owners = if entity.directory.ty == 402 {
                Some(&mut logical_owners)
            } else if entity.directory.ty == 406 {
                None
            } else {
                Some(&mut physical_owners)
            };
            if let Some(owners) = owners {
                owners.entry(reference.target).or_default().insert(de);
                dependents_by_owner
                    .entry(de)
                    .or_default()
                    .insert(reference.target);
            }
        }
        references.insert(de, refs);
    }

    let mut worklist = live
        .iter()
        .copied()
        .filter(|&de| entities[&de].directory.status.subordinate != Subordinate::Independent)
        .collect::<BTreeSet<_>>();
    let mut dependent_without_owner = 0;
    while let Some(de) = worklist.pop_first() {
        graph_work(&mut work, 1)?;
        if !live.contains(&de) {
            continue;
        }
        let subordinate = entities[&de].directory.status.subordinate;
        let no_physical = physical_owners.get(&de).is_none_or(BTreeSet::is_empty);
        let no_logical = logical_owners.get(&de).is_none_or(BTreeSet::is_empty);
        if !(subordinate.requires_physical() && no_physical)
            && !(subordinate.requires_logical() && no_logical)
        {
            continue;
        }
        live.remove(&de);
        dependent_without_owner += 1;
        let dependents = dependents_by_owner.get(&de).cloned().unwrap_or_default();
        for dependent in dependents {
            graph_work(&mut work, 1)?;
            if let Some(owners) = physical_owners.get_mut(&dependent) {
                owners.remove(&de);
            }
            if let Some(owners) = logical_owners.get_mut(&dependent) {
                owners.remove(&de);
            }
            worklist.insert(dependent);
        }
    }

    for &de in &live {
        for reference in &references[&de] {
            graph_work(&mut work, 1)?;
            if !live.contains(&reference.target) {
                let state = if entities.contains_key(&reference.target) {
                    "discarded"
                } else {
                    "missing"
                };
                let message = format!(
                    "Retained IGES DE {de} references {state} DE {} as {}",
                    reference.target, reference.role
                );
                return Err(if entities[&de].directory.ty == 102 {
                    composite_error("BREP_IGES_V2_COMPOSITE_DANGLING", message)
                } else {
                    refuse(message)
                });
            }
        }
    }

    let group_nodes = live
        .iter()
        .copied()
        .filter(|&de| entities[&de].directory.ty == 402)
        .collect::<BTreeSet<_>>();
    let mut indegree = group_nodes
        .iter()
        .map(|&de| (de, 0usize))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing = BTreeMap::<usize, Vec<usize>>::new();
    for &de in &group_nodes {
        for reference in &references[&de] {
            if group_nodes.contains(&reference.target) {
                graph_work(&mut work, 1)?;
                *indegree.get_mut(&reference.target).unwrap() += 1;
                outgoing.entry(de).or_default().push(reference.target);
            }
        }
    }
    let mut ready = indegree
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(&de, _)| de)
        .collect::<BTreeSet<_>>();
    let mut processed = 0;
    while let Some(de) = ready.pop_first() {
        graph_work(&mut work, 1)?;
        processed += 1;
        if let Some(targets) = outgoing.get(&de) {
            for &target in targets {
                graph_work(&mut work, 1)?;
                let count = indegree.get_mut(&target).unwrap();
                *count -= 1;
                if *count == 0 {
                    ready.insert(target);
                }
            }
        }
    }
    if processed != group_nodes.len() {
        let de = indegree
            .iter()
            .find(|(_, count)| **count > 0)
            .map(|(&de, _)| de)
            .unwrap();
        return Err(refuse(format!(
            "IGES 402 associativity cycle involving DE {de}"
        )));
    }

    entities.retain(|de, _| live.contains(de));
    Ok(IgesV2DiscardReport {
        blanked,
        dependent_without_owner,
    })
}
