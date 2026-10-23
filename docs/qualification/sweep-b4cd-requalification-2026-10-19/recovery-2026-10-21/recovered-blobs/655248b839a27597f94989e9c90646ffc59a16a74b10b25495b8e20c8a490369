//! Instance-local wire handles for native B-rep sessions. No handle reuse.
use super::brep_session::{Committed, Lease, Outcome, Session};
use super::{Result, Value, field, input};
use std::{cell::RefCell, collections::BTreeMap};
use value_codec::json;
const MAX_SLOTS: usize = 64;
const MAX_RESERVED: usize = 64 * 1024 * 1024;
struct Active {
    session: Session,
    leases: BTreeMap<String, Lease>,
}
enum State {
    Active(Active),
    Graph(super::brep_graph::PreparedGraph),
    Committed {
        bundle: Committed,
        leases: Vec<String>,
    },
}
struct Slot {
    reserved: usize,
    state: State,
}
#[derive(Default)]
struct Registry {
    next: u64,
    slots: BTreeMap<String, Slot>,
    reserved: usize,
}
fn outcome(value: Outcome<'_>) -> Value {
    match value {
        Outcome::Empty { value_type } => json!({"tag":"empty","valueType":value_type}),
        Outcome::Value {
            value_type,
            geometry,
        } => json!({"tag":"value","valueType":value_type,"geometry":geometry}),
    }
}
impl Registry {
    fn token(&mut self) -> Result<String> {
        self.next = self
            .next
            .checked_add(1)
            .ok_or_else(|| input("Native handle space exhausted"))?;
        Ok(self.next.to_string())
    }
    fn dispatch(&mut self, v: Value) -> Result<Value> {
        let op: String = field(&v, "action")?;
        if op == "begin" || op == "graph-begin" {
            let bytes: usize = field(&v, "maxBytes")?;
            let nodes: usize = field(&v, "maxNodes")?;
            if self.slots.len() >= MAX_SLOTS || bytes > MAX_RESERVED - self.reserved {
                return Err(input("Native session registry budget exceeded"));
            }
            let state = if op == "graph-begin" {
                match super::brep_graph::prepare(&v) {
                    Ok(graph) => State::Graph(graph),
                    Err(failure) => return Ok(super::brep_graph::failure_report(failure)),
                }
            } else {
                let mut session = Session::new(nodes, bytes)?;
                if v.get("maxCharacters").is_some() {
                    session = session.with_character_limit(field(&v, "maxCharacters")?)?;
                }
                State::Active(Active {
                    session,
                    leases: BTreeMap::new(),
                })
            };
            let id = self.token()?;
            self.slots.insert(
                id.clone(),
                Slot {
                    reserved: bytes,
                    state,
                },
            );
            self.reserved += bytes;
            return Ok(json!({"session":id}));
        }
        let id: String = field(&v, "session")?;
        if op == "dispose" {
            let slot = self
                .slots
                .remove(&id)
                .ok_or_else(|| input("Unknown native session handle"))?;
            self.reserved -= slot.reserved;
            return Ok(json!({"tag":"disposed"}));
        }
        if op == "graph-commit" {
            if !matches!(
                self.slots.get(&id).map(|slot| &slot.state),
                Some(State::Graph(_))
            ) {
                return Err(input("Unknown native graph handle"));
            }
            let slot = self.slots.remove(&id).unwrap();
            self.reserved -= slot.reserved;
            let State::Graph(graph) = slot.state else {
                unreachable!()
            };
            return Ok(match graph.finish() {
                Ok(report) => report,
                Err(failure) => super::brep_graph::failure_report(failure),
            });
        }
        // Allocate before borrowing a slot; IDs are deliberately consumed on failure.
        let result_id = if op == "evaluate" {
            Some(self.token()?)
        } else {
            None
        };
        let slot = self
            .slots
            .get_mut(&id)
            .ok_or_else(|| input("Unknown native session handle"))?;
        if op == "graph-advance" {
            let State::Graph(graph) = &mut slot.state else {
                return Err(input("Expected native graph handle"));
            };
            let limit = match field::<usize>(&v, "limit") {
                Ok(limit) => limit,
                Err(error) => {
                    graph.runner.abort();
                    return Err(error);
                }
            };
            return Ok(match graph.runner.advance(limit) {
                Ok(completed) => {
                    json!({"tag":if completed == graph.runner.total_nodes() {"ready"} else {"progress"},"completedNodes":completed,"totalNodes":graph.runner.total_nodes()})
                }
                Err(failure) => super::brep_graph::failure_report(failure),
            });
        }
        if op == "snapshot" {
            let lease: String = field(&v, "lease")?;
            return match &slot.state {
                State::Graph(_) => Err(input("Graph results require graph commit")),
                State::Active(active) => Ok(outcome(
                    active.session.outcome(
                        active
                            .leases
                            .get(&lease)
                            .ok_or_else(|| input("Unknown native result handle"))?,
                    )?,
                )),
                State::Committed { bundle, leases } => {
                    let index = leases
                        .iter()
                        .position(|x| x == &lease)
                        .ok_or_else(|| input("Result was not retained at commit"))?;
                    Ok(outcome(
                        bundle
                            .outcomes()
                            .nth(index)
                            .ok_or_else(|| input("Invalid committed result index"))?,
                    ))
                }
            };
        }
        let State::Active(active) = &mut slot.state else {
            return Err(input("Native session is already committed"));
        };
        match op.as_str() {
            "evaluate" => {
                let result =
                    (|| {
                        let node: Value = field(&v, "node")?;
                        let refs: Vec<String> = field(&v, "inputs")?;
                        if refs.len() > 512 {
                            return Err(input("Native operand handle budget exceeded"));
                        }
                        let leases =
                            refs.iter()
                                .map(|key| {
                                    active.leases.get(key).cloned().ok_or_else(|| {
                                        input("Foreign or unknown native result handle")
                                    })
                                })
                                .collect::<Result<Vec<_>>>()?;
                        let reduced = if v.get("inputNodeIndices").is_some() {
                            Some(field::<Vec<usize>>(&v, "inputNodeIndices")?)
                        } else {
                            None
                        };
                        active.session.evaluate_reduced(node, reduced, &leases)
                    })();
                match result {
                    Ok(lease) => {
                        let key = result_id.unwrap();
                        active.leases.insert(key.clone(), lease);
                        Ok(json!({"lease":key}))
                    }
                    Err(error) => {
                        active.session.abort();
                        Err(error)
                    }
                }
            }
            "release" => {
                let key: String = field(&v, "lease")?;
                active.session.release(
                    active
                        .leases
                        .get(&key)
                        .ok_or_else(|| input("Unknown native result handle"))?,
                )?;
                Ok(json!({"tag":"released"}))
            }
            "commit" => {
                let keys: Vec<String> = field(&v, "retained")?;
                let leases = keys
                    .iter()
                    .map(|key| {
                        active
                            .leases
                            .get(key)
                            .cloned()
                            .ok_or_else(|| input("Unknown retained native result handle"))
                    })
                    .collect::<Result<Vec<_>>>();
                let leases = match leases {
                    Ok(value) => value,
                    Err(error) => {
                        active.session.abort();
                        return Err(error);
                    }
                };
                let bundle = active.session.commit(&leases)?;
                slot.state = State::Committed {
                    bundle,
                    leases: keys,
                };
                Ok(json!({"tag":"committed"}))
            }
            _ => Err(input("Unknown native session action")),
        }
    }
}
thread_local! { static REGISTRY:RefCell<Registry>=RefCell::new(Registry::default()); }
pub fn dispatch(v: Value) -> Result<Value> {
    REGISTRY.with(|registry| {
        registry
            .try_borrow_mut()
            .map_err(|_| input("Reentrant native session request"))?
            .dispatch(v)
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn graph(r: &mut Registry, bytes: usize) -> String {
        let mut second = cube();
        second["id"] = json!(1);
        r.dispatch(json!({"action":"graph-begin","maxBytes":bytes,"maxNodes":16,"nodes":[cube(),second],"outputs":[1]})).unwrap()["session"].as_str().unwrap().into()
    }
    #[test]
    fn graph_steps_commit_once_and_share_registry_budget() {
        let mut registry = Registry::default();
        let ordinary = begin(&mut registry);
        let handle = graph(&mut registry, MAX_RESERVED - 1_000_000);
        assert_eq!(registry.reserved, MAX_RESERVED);
        assert!(
            registry
                .dispatch(json!({"action":"begin","maxNodes":1,"maxBytes":1}))
                .is_err()
        );
        assert_eq!(
            registry
                .dispatch(json!({"action":"graph-advance","session":handle,"limit":1}))
                .unwrap()["tag"]
                .as_str(),
            Some("progress")
        );
        assert_eq!(
            registry
                .dispatch(json!({"action":"graph-advance","session":handle,"limit":1}))
                .unwrap()["tag"]
                .as_str(),
            Some("ready")
        );
        let committed = registry
            .dispatch(json!({"action":"graph-commit","session":handle}))
            .unwrap();
        assert_eq!(committed["tag"].as_str(), Some("committed"));
        assert_eq!(committed["outcomes"].as_array().unwrap().len(), 1);
        assert_eq!(registry.reserved, 1_000_000);
        assert!(
            registry
                .dispatch(json!({"action":"graph-commit","session":handle}))
                .is_err()
        );
        registry
            .dispatch(json!({"action":"dispose","session":ordinary}))
            .unwrap();
        assert_eq!(registry.reserved, 0);
    }
    #[test]
    fn incomplete_graph_commit_revokes_handle_and_dispose_cancels() {
        let mut registry = Registry::default();
        let first = graph(&mut registry, 1_000_000);
        registry
            .dispatch(json!({"action":"graph-advance","session":first,"limit":1}))
            .unwrap();
        let failed = registry
            .dispatch(json!({"action":"graph-commit","session":first}))
            .unwrap();
        assert_eq!(failed["tag"].as_str(), Some("failed"));
        assert_eq!(failed["completedNodes"].as_u64(), Some(1));
        assert!(failed.get("outcomes").is_none());
        assert_eq!(registry.reserved, 0);
        let second = graph(&mut registry, 1_000_000);
        assert_ne!(first, second);
        registry
            .dispatch(json!({"action":"dispose","session":second}))
            .unwrap();
        assert!(
            registry
                .dispatch(json!({"action":"graph-advance","session":second,"limit":1}))
                .is_err()
        );
        assert_eq!(registry.reserved, 0);
    }
    fn begin(r: &mut Registry) -> String {
        r.dispatch(json!({"action":"begin","maxBytes":1_000_000,"maxNodes":16}))
            .unwrap()["session"]
            .as_str()
            .unwrap()
            .into()
    }
    fn cube() -> Value {
        json!({"id":0,"kind":"box","valueType":{"space":"d3","geometryKind":"solid","representation":"analytic-brep","evidence":{"tag":"representation-preserving"}},"size":[1,1,1],"center":false})
    }
    #[test]
    fn wire_handles_survive_commit_and_are_revoked_on_dispose() {
        let mut r = Registry::default();
        let id = begin(&mut r);
        let lease = r
            .dispatch(json!({"action":"evaluate","session":id,"node":cube(),"inputs":[]}))
            .unwrap()["lease"]
            .as_str()
            .unwrap()
            .to_owned();
        let before = r
            .dispatch(json!({"action":"snapshot","session":id,"lease":lease}))
            .unwrap();
        r.dispatch(json!({"action":"commit","session":id,"retained":[lease]}))
            .unwrap();
        assert_eq!(
            r.dispatch(json!({"action":"snapshot","session":id,"lease":lease}))
                .unwrap(),
            before
        );
        r.dispatch(json!({"action":"dispose","session":id}))
            .unwrap();
        assert_eq!(r.reserved, 0);
        let next = begin(&mut r);
        assert_ne!(next, id);
        assert!(
            r.dispatch(json!({"action":"snapshot","session":id,"lease":lease}))
                .is_err()
        );
    }
    #[test]
    fn foreign_handles_fail_without_revoking_their_owner() {
        let mut r = Registry::default();
        let a = begin(&mut r);
        let b = begin(&mut r);
        let lease = r
            .dispatch(json!({"action":"evaluate","session":a,"node":cube(),"inputs":[]}))
            .unwrap()["lease"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(
            r.dispatch(json!({"action":"evaluate","session":b,"node":cube(),"inputs":[lease]}))
                .is_err()
        );
        assert!(
            r.dispatch(json!({"action":"snapshot","session":a,"lease":lease}))
                .is_ok()
        );
        assert!(
            r.dispatch(json!({"action":"evaluate","session":b,"node":cube(),"inputs":[]}))
                .is_err()
        );
    }
}
