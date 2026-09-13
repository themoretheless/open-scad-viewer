//! Native execution and liveness for an ordered B-rep geometry DAG.
//! This is not yet admission of the complete SemanticProgram envelope.
use super::brep_session::{Committed, Outcome, Session};
use super::{Result, Value, field, input};
use value_codec::json;

/// Failure owns only diagnostic data, never a partially committed result.
#[derive(Debug)]
pub struct GraphFailure {
    pub node: Option<usize>,
    pub completed_nodes: usize,
    pub cause: super::Error,
}
impl From<super::Error> for GraphFailure {
    fn from(cause: super::Error) -> Self {
        Self {
            node: None,
            completed_nodes: 0,
            cause,
        }
    }
}

pub fn execute(nodes: Vec<Value>, outputs: &[usize], session: Session) -> Result<Committed> {
    execute_detailed(nodes, outputs, session).map_err(|failure| failure.cause)
}

pub fn execute_detailed(
    nodes: Vec<Value>,
    outputs: &[usize],
    session: Session,
) -> std::result::Result<Committed, GraphFailure> {
    let mut runner = super::brep_graph_runner::Runner::new(nodes, outputs, session)?;
    while runner.completed_nodes() < runner.total_nodes() {
        runner.advance(256)?;
    }
    runner.commit()
}

/// Execute the authored B-rep SemanticProgram schedule after native admission.
/// This accepts the plan component, not a trusted/lowered complete envelope.
pub fn execute_plan(
    nodes: Vec<Value>,
    outputs: &[usize],
    plan: &Value,
    session: Session,
) -> std::result::Result<Committed, GraphFailure> {
    super::brep_execution_plan::validate(plan, nodes.len())?;
    execute_detailed(nodes, outputs, session)
}

pub(crate) struct PreparedGraph {
    pub runner: super::brep_graph_runner::Runner,
    selection: Option<super::brep_result::Selection>,
}
impl PreparedGraph {
    pub fn finish(self) -> std::result::Result<Value, GraphFailure> {
        let completed_nodes = self.runner.completed_nodes();
        let bundle = self.runner.commit()?;
        serialize_bundle(bundle, self.selection, completed_nodes)
    }
}

pub(crate) fn prepare(v: &Value) -> std::result::Result<PreparedGraph, GraphFailure> {
    let nodes: Vec<Value> = field(&v, "nodes")?;
    let max_nodes: usize = field(&v, "maxNodes")?;
    if nodes.len() > max_nodes {
        return Err(input("Native graph node budget exceeded").into());
    }
    let mut session = Session::new(max_nodes, field(&v, "maxBytes")?)?;
    if v.get("maxCharacters").is_some() {
        session = session.with_character_limit(field(&v, "maxCharacters")?)?;
    }
    let selection = if let Some(result) = v.get("result") {
        if v.get("outputs").is_some() {
            return Err(input("Provide either semantic result or graph output indices").into());
        }
        Some(super::brep_result::select(
            result,
            &nodes,
            &field::<Vec<Value>>(&v, "occurrences")?,
        )?)
    } else {
        None
    };
    let outputs: Vec<usize> = match &selection {
        Some(selection) => selection.roots.clone(),
        None => field(&v, "outputs")?,
    };
    if let Some(plan) = v.get("execution") {
        super::brep_execution_plan::validate(plan, nodes.len())?;
    }
    super::brep_envelope::validate(v, &nodes)?;
    let runner = super::brep_graph_runner::Runner::new(nodes, &outputs, session)?;
    Ok(PreparedGraph { runner, selection })
}

fn request(v: Value) -> std::result::Result<Value, GraphFailure> {
    let mut prepared = prepare(&v)?;
    while prepared.runner.completed_nodes() < prepared.runner.total_nodes() {
        prepared.runner.advance(256)?;
    }
    prepared.finish()
}

fn serialize_bundle(
    bundle: Committed,
    selection: Option<super::brep_result::Selection>,
    completed_nodes: usize,
) -> std::result::Result<Value, GraphFailure> {
    let outcomes: Vec<Value> = bundle
        .outcomes()
        .map(|outcome| match outcome {
            Outcome::Empty { value_type } => json!({"tag":"empty","valueType":value_type}),
            Outcome::Value {
                value_type,
                geometry,
            } => json!({"tag":"value","valueType":value_type,"geometry":geometry}),
        })
        .collect();
    let mut report =
        json!({"tag":"committed","completedNodes":completed_nodes,"outcomes":outcomes});
    if let Some(selection) = selection {
        let items: Vec<Value> = selection
            .items
            .into_iter()
            .zip(selection.outcome_indices)
            .map(|(reference, index)| json!({"reference":reference,"outcomeIndex":index}))
            .collect();
        report["resultItems"] = json!(items);
    }
    Ok(report)
}

pub(crate) fn dispatch(v: Value) -> Result<Value> {
    let report = request(v).map_err(|failure| failure.cause)?;
    let mut value = json!({"outcomes":report["outcomes"].clone()});
    if let Some(items) = report.get("resultItems") {
        value["resultItems"] = items.clone();
    }
    Ok(value)
}

pub(crate) fn failure_report(failure: GraphFailure) -> Value {
    json!({"tag":"failed","node":failure.node,"completedNodes":failure.completed_nodes,"code":failure.cause.code,"message":failure.cause.message})
}

pub(crate) fn report(v: Value) -> Value {
    match request(v) {
        Ok(value) => value,
        Err(failure) => failure_report(failure),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ty() -> Value {
        json!({"space":"d3","geometryKind":"solid-set","representation":"analytic-brep","evidence":{"tag":"representation-preserving"}})
    }
    fn cube(id: usize) -> Value {
        json!({"id":id,"kind":"box","size":[1,1,1],"center":false,"valueType":ty()})
    }
    fn session(bytes: usize) -> Session {
        Session::new(16, bytes).unwrap()
    }
    #[test]
    fn graph_owns_empty_reuse_and_requested_output_order() {
        let nodes = vec![
            cube(0),
            json!({"id":1,"kind":"boolean","operation":"difference","inputs":[0,0],"valueType":ty()}),
            json!({"id":2,"kind":"boolean","operation":"union","inputs":[1,0],"valueType":ty()}),
        ];
        let committed = execute(nodes, &[2, 1], session(1_000_000)).unwrap();
        let mut results = committed.outcomes();
        assert!(matches!(results.next(), Some(Outcome::Value { .. })));
        assert!(matches!(results.next(), Some(Outcome::Empty { .. })));
        assert!(results.next().is_none());
    }
    #[test]
    fn liveness_releases_unretained_geometry_under_aggregate_budget() {
        let mut sizing = session(1_000_000);
        sizing.evaluate(cube(0), &[]).unwrap();
        let bytes = sizing.retained_bytes();
        execute((0..8).map(cube).collect(), &[7], session(bytes)).unwrap();
        assert!(execute(vec![cube(0), cube(1)], &[0, 1], session(bytes)).is_err());
    }
    #[test]
    fn graph_refuses_invalid_references_outputs_and_partial_success() {
        assert!(execute(vec![cube(1)], &[0], session(1_000_000)).is_err());
        assert!(execute(vec![cube(0)], &[1], session(1_000_000)).is_err());
        assert!(execute(vec![cube(0)], &[0, 0], session(1_000_000)).is_err());
        let forward =
            json!({"id":1,"kind":"boolean","operation":"union","inputs":[2],"valueType":ty()});
        assert!(execute(vec![cube(0), forward, cube(2)], &[0], session(1_000_000)).is_err());
        let mut invalid = cube(1);
        invalid["size"] = json!([-1, 1, 1]);
        assert!(execute(vec![cube(0), invalid], &[0], session(1_000_000)).is_err());
    }
    #[test]
    fn repeated_edges_at_arity_limit_preserve_liveness() {
        for count in [512, 513] {
            let empty =
                json!({"id":0,"kind":"boolean","operation":"union","inputs":[],"valueType":ty()});
            let fold = json!({"id":1,"kind":"boolean","operation":"union","inputs":vec![0usize;count],"valueType":ty()});
            let result = execute(vec![empty, fold], &[1], session(1_000_000));
            if count == 512 {
                assert!(matches!(
                    result.unwrap().outcomes().next(),
                    Some(Outcome::Empty { .. })
                ));
            } else {
                assert!(result.is_err());
            }
        }
    }
    #[test]
    fn reports_failure_node_and_completed_prefix_without_outputs() {
        let mut invalid = cube(1);
        invalid["size"] = json!([-1, 1, 1]);
        let failed = report(
            json!({"maxNodes":16,"maxBytes":1_000_000,"nodes":[cube(0),invalid],"outputs":[0]}),
        );
        assert_eq!(failed["tag"].as_str(), Some("failed"));
        assert_eq!(failed["node"].as_u64(), Some(1));
        assert_eq!(failed["completedNodes"].as_u64(), Some(1));
        assert!(failed["code"].as_str().is_some_and(|code| !code.is_empty()));
        assert!(failed.get("outcomes").is_none());
        let bad_reference =
            json!({"id":1,"kind":"boolean","operation":"union","inputs":[2],"valueType":ty()});
        let preflight =
            execute_detailed(vec![cube(0), bad_reference], &[0], session(1_000_000)).unwrap_err();
        assert_eq!(preflight.node, Some(1));
        assert_eq!(preflight.completed_nodes, 0);
        let malformed = report(json!({}));
        assert_eq!(malformed["tag"].as_str(), Some("failed"));
        assert!(malformed["node"].is_null());
        let success =
            report(json!({"maxNodes":16,"maxBytes":1_000_000,"nodes":[cube(0)],"outputs":[0]}));
        assert_eq!(success["tag"].as_str(), Some("committed"));
        assert_eq!(success["completedNodes"].as_u64(), Some(1));
        assert_eq!(success["outcomes"].as_array().unwrap().len(), 1);
    }
}
