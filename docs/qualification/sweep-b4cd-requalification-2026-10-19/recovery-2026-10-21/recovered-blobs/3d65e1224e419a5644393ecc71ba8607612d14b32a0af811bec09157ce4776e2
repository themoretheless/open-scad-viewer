//! Native result selection and identity-lineage admission for B-rep graphs.
//! Operation/occurrence digest admission lives in brep_identity.rs.
use super::{Result, Value, field};
use std::collections::BTreeMap;

pub struct Selection {
    pub roots: Vec<usize>,
    pub items: Vec<Value>,
    pub outcome_indices: Vec<usize>,
}
fn invalid(message: &str) -> super::Error {
    super::Error::new("BREP_SEMANTIC_CONTRACT", message)
}
fn exact(value: &Value, keys: &[&str]) -> Result<()> {
    let map = value
        .as_object()
        .ok_or_else(|| invalid("Expected semantic result object"))?;
    if map.len() != keys.len() || keys.iter().any(|key| !map.contains_key(*key)) {
        return Err(invalid("Unexpected semantic result fields"));
    }
    Ok(())
}
fn optional_index(value: &Value, limit: usize) -> Result<Option<usize>> {
    if value.is_null() {
        return Ok(None);
    }
    let index = value
        .as_u64()
        .and_then(|x| usize::try_from(x).ok())
        .filter(|x| *x < limit)
        .ok_or_else(|| invalid("Invalid semantic occurrence reference"))?;
    Ok(Some(index))
}
fn ancestor(mut child: usize, parent: usize, parents: &[Option<usize>]) -> bool {
    loop {
        if child == parent {
            return true;
        }
        match parents[child] {
            Some(next) => child = next,
            None => return false,
        }
    }
}
fn root(mut index: usize, parents: &[Option<usize>]) -> usize {
    while let Some(parent) = parents[index] {
        index = parent;
    }
    index
}
fn preserves(mut node: usize, target: usize, nodes: &[Value]) -> Result<bool> {
    loop {
        if node == target {
            return Ok(true);
        }
        if nodes[node]["kind"].as_str() != Some("transform") {
            return Ok(false);
        }
        let next: usize = field(&nodes[node], "input")?;
        if next >= node {
            return Err(invalid("Invalid transform identity lineage"));
        }
        node = next;
    }
}
pub fn select(result: &Value, nodes: &[Value], occurrences: &[Value]) -> Result<Selection> {
    if nodes.len() > 25_000 || occurrences.len() > 100_000 {
        return Err(invalid("Semantic result context limit exceeded"));
    }
    let mut parents = Vec::with_capacity(occurrences.len());
    let mut occurrence_nodes = Vec::with_capacity(occurrences.len());
    let mut canonical = Vec::with_capacity(occurrences.len());
    let mut first = BTreeMap::new();
    for (index, occurrence) in occurrences.iter().enumerate() {
        if field::<usize>(occurrence, "id")? != index {
            return Err(invalid("Invalid occurrence row identity"));
        }
        let parent = occurrence
            .get("parent")
            .ok_or_else(|| invalid("Missing occurrence parent"))?;
        parents.push(optional_index(parent, index)?);
        occurrence_nodes.push(optional_index(
            occurrence
                .get("node")
                .ok_or_else(|| invalid("Missing occurrence node"))?,
            nodes.len(),
        )?);
        let identity: String = field(occurrence, "occurrenceId")?;
        if identity.is_empty() {
            return Err(invalid("Missing occurrence identity"));
        }
        canonical.push(*first.entry(identity).or_insert(index));
    }
    let items = match result["tag"].as_str() {
        Some("empty") => {
            exact(result, &["tag", "type"])?;
            if result["type"].as_str() != Some("never") {
                return Err(invalid("Empty result requires never type"));
            }
            Vec::new()
        }
        Some("single") => {
            exact(result, &["tag", "item"])?;
            vec![result["item"].clone()]
        }
        Some("multi") => {
            exact(result, &["tag", "items"])?;
            let items: Vec<Value> = field(result, "items")?;
            if !(2..=1000).contains(&items.len()) {
                return Err(invalid("Multi result requires 2..1000 outputs"));
            }
            items
        }
        _ => return Err(invalid("Unknown semantic result tag")),
    };
    let mut roots = Vec::new();
    let mut root_indices = BTreeMap::new();
    let mut outcome_indices = Vec::with_capacity(items.len());
    let mut prior_identity = None;
    for item in &items {
        exact(
            item,
            &["node", "producerOccurrence", "identityOccurrence", "color"],
        )?;
        let node: usize = field(item, "node")?;
        let producer: usize = field(item, "producerOccurrence")?;
        let identity: usize = field(item, "identityOccurrence")?;
        if node >= nodes.len() || producer >= occurrences.len() || identity >= occurrences.len() {
            return Err(invalid(
                "Semantic result references absent node or occurrence",
            ));
        }
        if prior_identity.is_some_and(|prior| identity <= prior) {
            return Err(invalid("Semantic outputs must follow occurrence order"));
        }
        prior_identity = Some(identity);
        if occurrence_nodes[producer] != Some(node) {
            return Err(invalid("Output node differs from producer occurrence"));
        }
        if !occurrences[identity]["sceneEntityId"]
            .as_str()
            .is_some_and(|id| !id.is_empty())
        {
            return Err(invalid("Output identity must own a scene entity"));
        }
        let identity_node =
            occurrence_nodes[identity].ok_or_else(|| invalid("Output identity has no node"))?;
        if !preserves(node, identity_node, nodes)?
            || root(canonical[producer], &parents) != root(identity, &parents)
            || !(ancestor(identity, producer, &parents)
                || ancestor(identity, canonical[producer], &parents))
        {
            return Err(invalid(
                "Output identity does not belong to its producing branch and lineage",
            ));
        }
        let color: [f64; 4] = field(item, "color")?;
        if color
            .iter()
            .any(|x| !x.is_finite() || !(0.0..=1.0).contains(x))
        {
            return Err(invalid("Color channels must be in [0, 1]"));
        }
        let index = *root_indices.entry(node).or_insert_with(|| {
            let index = roots.len();
            roots.push(node);
            index
        });
        outcome_indices.push(index);
    }
    Ok(Selection {
        roots,
        items,
        outcome_indices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use value_codec::json;
    fn nodes() -> Vec<Value> {
        vec![json!({"kind":"box"}), json!({"kind":"transform","input":0})]
    }
    fn occurrences() -> Vec<Value> {
        vec![
            json!({"id":0,"parent":null,"node":1,"occurrenceId":"root","sceneEntityId":null}),
            json!({"id":1,"parent":0,"node":0,"occurrenceId":"first","sceneEntityId":"entity:first"}),
            json!({"id":2,"parent":0,"node":0,"occurrenceId":"second","sceneEntityId":"entity:second"}),
        ]
    }
    fn item(identity: usize) -> Value {
        json!({"node":1,"producerOccurrence":0,"identityOccurrence":identity,"color":[1,0,0,1]})
    }
    #[test]
    fn output_slots_share_geometry_and_preserve_references() {
        let items = vec![item(1), item(2)];
        let selected = select(
            &json!({"tag":"multi","items":items.clone()}),
            &nodes(),
            &occurrences(),
        )
        .unwrap();
        assert_eq!(selected.roots, vec![1]);
        assert_eq!(selected.outcome_indices, vec![0, 0]);
        assert_eq!(selected.items, items);
        assert!(
            select(&json!({"tag":"empty","type":"never"}), &[], &[])
                .unwrap()
                .roots
                .is_empty()
        );
    }
    #[test]
    fn rejects_wrong_branch_lineage_order_and_color() {
        let mut unrelated = occurrences();
        unrelated[1]["parent"] = Value::Null;
        assert!(
            select(
                &json!({"tag":"single","item":item(1)}),
                &nodes(),
                &unrelated
            )
            .is_err()
        );
        let mut cycle = occurrences();
        cycle[1]["parent"] = json!(1);
        assert!(select(&json!({"tag":"single","item":item(1)}), &nodes(), &cycle).is_err());
        let mut wrong_lineage = nodes();
        wrong_lineage[1] = json!({"kind":"boolean"});
        assert!(
            select(
                &json!({"tag":"single","item":item(1)}),
                &wrong_lineage,
                &occurrences()
            )
            .is_err()
        );
        for items in [
            vec![item(2), item(1)],
            vec![item(1), item(1)],
            vec![item(1)],
        ] {
            assert!(
                select(
                    &json!({"tag":"multi","items":items}),
                    &nodes(),
                    &occurrences()
                )
                .is_err()
            );
        }
        for (key, value) in [
            ("color", json!([2, 0, 0, 1])),
            ("node", json!(0)),
            ("identityOccurrence", json!(99)),
            ("extra", json!(true)),
        ] {
            let mut invalid = item(1);
            invalid[key] = value;
            assert!(
                select(
                    &json!({"tag":"single","item":invalid}),
                    &nodes(),
                    &occurrences()
                )
                .is_err()
            );
        }
    }
}
