//! Cooperative native graph execution. A step boundary is between nodes;
//! interruption inside one synchronous geometry operation still needs isolation.
use super::brep_graph::GraphFailure;
use super::brep_session::{Committed, Lease, Session, references};
use super::{Result, Value, field, input};
use std::collections::BTreeSet;

pub struct Runner {
    session: Session,
    pending: std::vec::IntoIter<Value>,
    outputs: Vec<usize>,
    retained: BTreeSet<usize>,
    uses: Vec<usize>,
    leases: Vec<Option<Lease>>,
    total: usize,
    completed: usize,
    closed: bool,
}
impl Runner {
    pub fn new(
        nodes: Vec<Value>,
        outputs: &[usize],
        session: Session,
    ) -> std::result::Result<Self, GraphFailure> {
        let mut active = None;
        let prepared = (|| -> Result<_> {
            if nodes.len() > 65536 || outputs.len() > nodes.len() {
                return Err(input("Native graph size exceeded"));
            }
            let retained: BTreeSet<_> = outputs.iter().copied().collect();
            if retained.len() != outputs.len() || outputs.iter().any(|id| *id >= nodes.len()) {
                return Err(input("Invalid or duplicate native graph output"));
            }
            let mut uses = vec![0usize; nodes.len()];
            for (index, node) in nodes.iter().enumerate() {
                active = Some(index);
                if field::<usize>(node, "id")? != index {
                    return Err(input(
                        "Native graph requires contiguous ordered node identities",
                    ));
                }
                let refs = references(node)?;
                if refs.len() > 512 || refs.iter().any(|id| *id >= index) {
                    return Err(input("Invalid native graph operands or forward reference"));
                }
                for id in refs {
                    uses[id] += 1;
                }
            }
            Ok((retained, uses))
        })()
        .map_err(|cause| GraphFailure {
            node: active,
            completed_nodes: 0,
            cause,
        })?;
        let total = nodes.len();
        Ok(Self {
            session,
            pending: nodes.into_iter(),
            outputs: outputs.to_vec(),
            retained: prepared.0,
            uses: prepared.1,
            leases: vec![None; total],
            total,
            completed: 0,
            closed: false,
        })
    }
    pub fn total_nodes(&self) -> usize {
        self.total
    }
    pub fn completed_nodes(&self) -> usize {
        self.completed
    }
    pub fn retained_bytes(&self) -> usize {
        self.session.retained_bytes()
    }
    pub fn abort(&mut self) {
        self.session.abort();
        self.pending = Vec::new().into_iter();
        self.leases.clear();
        self.uses.clear();
        self.outputs.clear();
        self.retained.clear();
        self.closed = true;
    }
    fn fail(&mut self, node: Option<usize>, cause: super::Error) -> GraphFailure {
        self.abort();
        GraphFailure {
            node,
            completed_nodes: self.completed,
            cause,
        }
    }
    /// Evaluate at most limit nodes and leave all ownership in this runner.
    pub fn advance(&mut self, limit: usize) -> std::result::Result<usize, GraphFailure> {
        if self.closed || !(1..=65536).contains(&limit) {
            return Err(self.fail(None, input("Closed native graph or invalid step limit")));
        }
        for _ in 0..limit {
            let Some(node) = self.pending.next() else {
                break;
            };
            let index = self.completed;
            if let Err(cause) = self.evaluate_one(index, node) {
                return Err(self.fail(Some(index), cause));
            }
            self.completed += 1;
        }
        Ok(self.completed)
    }
    fn evaluate_one(&mut self, index: usize, node: Value) -> Result<()> {
        let refs = references(&node)?;
        let inputs = refs
            .iter()
            .map(|id| {
                self.leases[*id]
                    .clone()
                    .ok_or_else(|| input("Missing native graph operand"))
            })
            .collect::<Result<Vec<_>>>()?;
        self.leases[index] = Some(self.session.evaluate(node, &inputs)?);
        for id in refs {
            self.uses[id] -= 1;
            if self.uses[id] == 0 && !self.retained.contains(&id) {
                self.session.release(
                    &self.leases[id]
                        .take()
                        .ok_or_else(|| input("Missing native graph lease"))?,
                )?;
            }
        }
        if self.uses[index] == 0 && !self.retained.contains(&index) {
            self.session.release(
                &self.leases[index]
                    .take()
                    .ok_or_else(|| input("Missing native graph result"))?,
            )?;
        }
        Ok(())
    }
    /// Consume the runner: no caller can publish a prefix or commit twice.
    pub fn commit(mut self) -> std::result::Result<Committed, GraphFailure> {
        if self.closed || self.completed != self.total {
            return Err(self.fail(
                None,
                input("Cannot commit incomplete or closed native graph"),
            ));
        }
        let selected = self
            .outputs
            .iter()
            .map(|id| {
                self.leases[*id]
                    .clone()
                    .ok_or_else(|| input("Missing native graph output"))
            })
            .collect::<Result<Vec<_>>>()?;
        self.session
            .commit(&selected)
            .map_err(|cause| GraphFailure {
                node: None,
                completed_nodes: self.completed,
                cause,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use value_codec::json;
    fn cube(id: usize) -> Value {
        json!({"id":id,"kind":"box","size":[1,1,1],"center":false,"valueType":{"space":"d3","geometryKind":"solid","representation":"analytic-brep","evidence":{"tag":"representation-preserving"}}})
    }
    fn runner() -> Runner {
        Runner::new(
            vec![cube(0), cube(1), cube(2)],
            &[0, 2],
            Session::new(8, 1_000_000).unwrap(),
        )
        .unwrap()
    }
    #[test]
    fn bounded_steps_match_one_shot_output_and_cannot_commit_prefix() {
        let baseline = super::super::brep_graph::execute(
            vec![cube(0), cube(1), cube(2)],
            &[0, 2],
            Session::new(8, 1_000_000).unwrap(),
        )
        .unwrap();
        for size in [1, 2, 65536] {
            let mut runner = runner();
            let mut previous = 0;
            while runner.completed_nodes() < runner.total_nodes() {
                let completed = runner.advance(size).unwrap();
                assert!(completed > previous && completed - previous <= size);
                previous = completed;
            }
            assert_eq!(runner.advance(1).unwrap(), 3);
            assert_eq!(
                runner.commit().unwrap().results().collect::<Vec<_>>(),
                baseline.results().collect::<Vec<_>>()
            );
        }
        let mut partial = runner();
        partial.advance(1).unwrap();
        assert_eq!(partial.commit().unwrap_err().completed_nodes, 1);
    }
    #[test]
    fn abort_and_failed_steps_release_owned_geometry() {
        let mut cancelled = runner();
        cancelled.advance(1).unwrap();
        assert!(cancelled.retained_bytes() > 0);
        cancelled.abort();
        cancelled.abort();
        assert_eq!(cancelled.retained_bytes(), 0);
        assert_eq!(cancelled.advance(1).unwrap_err().completed_nodes, 1);
        assert!(cancelled.commit().is_err());
        let mut bad = cube(1);
        bad["size"] = json!([-1, 1, 1]);
        let mut failing = Runner::new(
            vec![cube(0), bad],
            &[0],
            Session::new(8, 1_000_000).unwrap(),
        )
        .unwrap();
        failing.advance(1).unwrap();
        let error = failing.advance(1).unwrap_err();
        assert_eq!(error.node, Some(1));
        assert_eq!(error.completed_nodes, 1);
        assert_eq!(failing.retained_bytes(), 0);
        assert!(failing.commit().is_err());
        assert!(runner().advance(0).is_err());
    }
}
