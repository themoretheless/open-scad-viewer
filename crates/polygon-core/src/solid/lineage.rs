//! Topology split/merge lineage and durable selection transfer.
//!
//! Identifiers are snapshot-local 128-bit values. Process handles are not
//! durable IDs. A split with more than one child is ambiguous; a merge follows
//! the unique result. Missing identities are lost rather than guessed.

use std::collections::BTreeMap;

use crate::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TopoId(u128);

impl TopoId {
    pub const fn from_parts(high: u64, low: u64) -> Self {
        Self(((high as u128) << 64) | low as u128)
    }

    pub fn to_hex(self) -> String {
        format!("{:032x}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopoKind {
    Face,
    Edge,
    Vertex,
    ControlPoint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineageKind {
    Persist,
    Split,
    Merge,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineageRecord {
    pub kind: LineageKind,
    pub parents: Vec<TopoId>,
    pub children: Vec<TopoId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectionTransfer {
    Persistent(TopoId),
    Followed(TopoId),
    Ambiguous(Vec<TopoId>),
    Lost,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DurableSnapshot {
    pub schema: u32,
    pub nodes: Vec<(TopoId, TopoKind)>,
    pub records: Vec<LineageRecord>,
    pub selection: Option<TopoId>,
}

#[derive(Clone, Debug, Default)]
pub struct TopologyLineage {
    kinds: BTreeMap<TopoId, TopoKind>,
    records: Vec<LineageRecord>,
}

fn invalid(message: &str) -> Error {
    Error {
        code: "TOPOLOGY_LINEAGE_INVALID",
        message: message.into(),
    }
}

impl TopologyLineage {
    pub fn introduce(&mut self, id: TopoId, kind: TopoKind) -> Result<()> {
        self.kinds.insert(id, kind);
        Ok(())
    }

    pub fn persist(&mut self, parent: TopoId, child: TopoId) -> Result<()> {
        self.record(LineageKind::Persist, &[parent], &[child])
    }

    pub fn split(&mut self, parent: TopoId, children: &[TopoId]) -> Result<()> {
        if children.len() < 2 {
            return Err(invalid("Split requires at least two children"));
        }
        self.record(LineageKind::Split, &[parent], children)
    }

    pub fn merge(&mut self, parents: &[TopoId], child: TopoId) -> Result<()> {
        if parents.len() < 2 {
            return Err(invalid("Merge requires at least two parents"));
        }
        self.record(LineageKind::Merge, parents, &[child])
    }

    pub fn transfer(&self, selection: TopoId) -> SelectionTransfer {
        if !self.kinds.contains_key(&selection) {
            return SelectionTransfer::Lost;
        }
        let latest = self
            .records
            .iter()
            .rev()
            .find(|record| record.parents.iter().any(|id| *id == selection));
        match latest {
            None => SelectionTransfer::Persistent(selection),
            Some(record) if record.kind == LineageKind::Split && record.children.len() != 1 => {
                SelectionTransfer::Ambiguous(record.children.clone())
            }
            Some(record) => SelectionTransfer::Followed(record.children[0]),
        }
    }

    pub fn snapshot(&self, selection: Option<TopoId>) -> Result<DurableSnapshot> {
        if let Some(id) = selection {
            if !self.kinds.contains_key(&id) {
                return Err(invalid("Snapshot selection is not in this lineage"));
            }
        }
        Ok(DurableSnapshot {
            schema: 1,
            nodes: self.kinds.iter().map(|(id, kind)| (*id, *kind)).collect(),
            records: self.records.clone(),
            selection,
        })
    }

    pub fn restore(snapshot: &DurableSnapshot) -> Result<Self> {
        if snapshot.schema != 1 {
            return Err(invalid("Unsupported topology snapshot schema"));
        }
        let mut lineage = Self::default();
        for (id, kind) in &snapshot.nodes {
            lineage.introduce(*id, *kind)?;
        }
        lineage.records = snapshot.records.clone();
        Ok(lineage)
    }

    fn record(&mut self, kind: LineageKind, parents: &[TopoId], children: &[TopoId]) -> Result<()> {
        let parent_kind = *parents
            .iter()
            .find_map(|id| self.kinds.get(id))
            .ok_or_else(|| invalid("Unknown lineage parent"))?;
        for parent in parents {
            if !self.kinds.contains_key(parent) {
                return Err(invalid("Unknown lineage parent"));
            }
        }
        for child in children {
            self.kinds.insert(*child, parent_kind);
        }
        self.records.push(LineageRecord {
            kind,
            parents: parents.to_vec(),
            children: children.to_vec(),
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_is_ambiguous_and_merge_follows() {
        let a = TopoId::from_parts(1, 1);
        let b = TopoId::from_parts(1, 2);
        let c = TopoId::from_parts(1, 3);
        let d = TopoId::from_parts(1, 4);
        let mut lineage = TopologyLineage::default();
        lineage.introduce(a, TopoKind::Edge).unwrap();
        lineage.split(a, &[b, c]).unwrap();
        assert_eq!(
            lineage.transfer(a),
            SelectionTransfer::Ambiguous(vec![b, c])
        );
        lineage.merge(&[b, c], d).unwrap();
        assert_eq!(lineage.transfer(b), SelectionTransfer::Followed(d));
        let snapshot = lineage.snapshot(Some(d)).unwrap();
        let restored = TopologyLineage::restore(&snapshot).unwrap();
        assert_eq!(restored.transfer(d), SelectionTransfer::Persistent(d));
        assert_eq!(
            restored.transfer(TopoId::from_parts(9, 9)),
            SelectionTransfer::Lost
        );
    }
}
