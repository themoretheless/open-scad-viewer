use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

const INVALID_NAMING: &str = "BREP_INVALID_PERSISTENT_NAMING";

fn error(message: impl Into<String>) -> value_codec::Error {
    value_codec::error(message.into())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TopoKind {
    Vertex,
    Edge,
    Loop,
    Face,
    Shell,
    Body,
    ControlPoint,
}

impl TopoKind {
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Vertex => "v",
            Self::Edge => "e",
            Self::Loop => "l",
            Self::Face => "f",
            Self::Shell => "s",
            Self::Body => "b",
            Self::ControlPoint => "cp",
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Vertex => "vertex",
            Self::Edge => "edge",
            Self::Loop => "loop",
            Self::Face => "face",
            Self::Shell => "shell",
            Self::Body => "body",
            Self::ControlPoint => "control-point",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "vertex" => Self::Vertex,
            "edge" => Self::Edge,
            "loop" => Self::Loop,
            "face" => Self::Face,
            "shell" => Self::Shell,
            "body" => Self::Body,
            "control-point" => Self::ControlPoint,
            _ => return None,
        })
    }

    pub fn from_prefix(value: &str) -> Option<Self> {
        Some(match value {
            "v" => Self::Vertex,
            "e" => Self::Edge,
            "l" => Self::Loop,
            "f" => Self::Face,
            "s" => Self::Shell,
            "b" => Self::Body,
            "cp" => Self::ControlPoint,
            _ => return None,
        })
    }
}

impl value_codec::Serialize for TopoKind {
    fn to_value(&self) -> value_codec::Value {
        value_codec::Value::String(self.as_str().into())
    }
}

impl<'de> value_codec::Deserialize<'de> for TopoKind {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let value = value
            .as_str()
            .ok_or_else(|| error("Expected topology kind"))?;
        Self::parse(value).ok_or_else(|| error("Unknown topology kind"))
    }
}

/// Canonical opaque topology identity. Its wire form is `<kind-prefix>:<32 lowercase hex>`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TopoId {
    kind: TopoKind,
    bytes: [u8; 16],
}

impl TopoId {
    pub const fn kind(self) -> TopoKind {
        self.kind
    }

    pub fn parse(value: &str) -> Result<Self, &'static str> {
        let (prefix, hex) = value
            .split_once(':')
            .ok_or("Topology ID is missing its prefix")?;
        let kind = TopoKind::from_prefix(prefix).ok_or("Unknown topology ID prefix")?;
        if hex.len() != 32
            || !hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("Topology ID payload must be 32 lowercase hexadecimal digits");
        }
        let mut bytes = [0; 16];
        for (index, pair) in hex.as_bytes().as_chunks::<2>().0.iter().enumerate() {
            let digit = |byte| match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                _ => unreachable!(),
            };
            bytes[index] = digit(pair[0]) << 4 | digit(pair[1]);
        }
        Ok(Self { kind, bytes })
    }

    /// Deterministically derives an opaque public identity from authored
    /// provenance plus private geometry-signature evidence.
    pub fn derive(
        kind: TopoKind,
        operation: &str,
        occurrence: &str,
        role: &str,
        geometry_signature: &[u8],
    ) -> Self {
        let mut digest = Sha256::new();
        digest.update(b"open-scad-viewer/topology-id/v2\0");
        for part in [
            kind.as_str().as_bytes(),
            operation.as_bytes(),
            occurrence.as_bytes(),
            role.as_bytes(),
            geometry_signature,
        ] {
            digest.update((part.len() as u64).to_be_bytes());
            digest.update(part);
        }
        let digest = digest.finalize();
        let mut bytes = [0; 16];
        bytes.copy_from_slice(&digest[..16]);
        Self { kind, bytes }
    }

    /// Explicitly migrates the old `<prefix>:<16hex>` representation. Legacy
    /// values are deliberately rejected by normal parsing/deserialization.
    pub fn migrate_legacy(value: &str) -> Result<Self, &'static str> {
        let (prefix, hex) = value
            .split_once(':')
            .ok_or("Legacy topology ID is missing its prefix")?;
        let kind = TopoKind::from_prefix(prefix).ok_or("Unknown legacy topology ID prefix")?;
        if hex.len() != 16
            || !hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("Legacy topology ID payload must be 16 lowercase hexadecimal digits");
        }
        Ok(Self::derive(
            kind,
            "legacy-migration",
            value,
            "legacy",
            hex.as_bytes(),
        ))
    }
}

impl fmt::Display for TopoId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:", self.kind.prefix())?;
        for byte in self.bytes {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for TopoId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl value_codec::Serialize for TopoId {
    fn to_value(&self) -> value_codec::Value {
        value_codec::Value::String(self.to_string())
    }
}

impl<'de> value_codec::Deserialize<'de> for TopoId {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let value = value
            .as_str()
            .ok_or_else(|| error("Expected topology ID"))?;
        Self::parse(value).map_err(error)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeKind {
    Persisted,
    Generated,
    Modified,
    Split,
    Merge,
    Deleted,
}

impl ChangeKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Persisted => "persisted",
            Self::Generated => "generated",
            Self::Modified => "modified",
            Self::Split => "split",
            Self::Merge => "merge",
            Self::Deleted => "deleted",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "persisted" => Self::Persisted,
            "generated" => Self::Generated,
            "modified" => Self::Modified,
            "split" => Self::Split,
            "merge" => Self::Merge,
            "deleted" => Self::Deleted,
            _ => return None,
        })
    }
}

impl value_codec::Serialize for ChangeKind {
    fn to_value(&self) -> value_codec::Value {
        value_codec::Value::String(self.as_str().into())
    }
}

impl<'de> value_codec::Deserialize<'de> for ChangeKind {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let value = value
            .as_str()
            .ok_or_else(|| error("Expected change kind"))?;
        Self::parse(value).ok_or_else(|| error("Unknown topology change kind"))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeProvenance {
    pub operation: String,
    pub operand: Option<String>,
    pub occurrence: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopologyChange {
    pub kind: ChangeKind,
    pub topo_kind: TopoKind,
    pub parents: Vec<TopoId>,
    pub children: Vec<TopoId>,
    pub provenance: ChangeProvenance,
    pub role: String,
    pub anchor: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChangeSet {
    pub nodes: BTreeMap<TopoId, TopoKind>,
    pub changes: Vec<TopologyChange>,
}

impl ChangeSet {
    pub fn validate(&self) -> math_core::Result<()> {
        let fail = |message| math_core::Error::new(INVALID_NAMING, message);
        for (id, kind) in &self.nodes {
            if id.kind() != *kind {
                return Err(fail("Topology node kind conflicts with its ID prefix"));
            }
        }
        let mut edges = BTreeMap::<TopoId, BTreeSet<TopoId>>::new();
        for change in &self.changes {
            if change.provenance.operation.is_empty()
                || change.provenance.occurrence.is_empty()
                || change.role.is_empty()
            {
                return Err(fail(
                    "Topology change provenance and role must be non-empty",
                ));
            }
            let unique_parents: BTreeSet<_> = change.parents.iter().copied().collect();
            let unique_children: BTreeSet<_> = change.children.iter().copied().collect();
            if unique_parents.len() != change.parents.len()
                || unique_children.len() != change.children.len()
            {
                return Err(fail("Topology change endpoints must be unique"));
            }
            let cardinality_ok = match change.kind {
                ChangeKind::Persisted | ChangeKind::Modified => {
                    change.parents.len() == 1 && change.children.len() == 1
                }
                ChangeKind::Generated => change.parents.is_empty() && change.children.len() == 1,
                ChangeKind::Split => change.parents.len() == 1 && change.children.len() >= 2,
                ChangeKind::Merge => change.parents.len() >= 2 && change.children.len() == 1,
                ChangeKind::Deleted => change.parents.len() == 1 && change.children.is_empty(),
            };
            if !cardinality_ok {
                return Err(fail("Topology change cardinality does not match its kind"));
            }
            let identity_ok = match change.kind {
                ChangeKind::Persisted => change.parents[0] == change.children[0],
                ChangeKind::Modified => change.parents[0] != change.children[0],
                // A split may preserve the parent's ID on one branch, and a
                // merge may preserve one operand's ID. Those are legitimate
                // persistent subrelations; cardinality remains strict and
                // self-edges are omitted from the DAG traversal below.
                ChangeKind::Split | ChangeKind::Merge => true,
                ChangeKind::Generated | ChangeKind::Deleted => true,
            };
            if !identity_ok {
                return Err(fail(
                    "Topology change identity overlap does not match its kind",
                ));
            }
            for id in change.parents.iter().chain(&change.children) {
                if self.nodes.get(id) != Some(&change.topo_kind) {
                    return Err(fail(
                        "Topology change endpoint is missing or has the wrong kind",
                    ));
                }
            }
            for parent in &change.parents {
                for child in &change.children {
                    if parent != child {
                        edges.entry(*parent).or_default().insert(*child);
                    }
                }
            }
        }
        fn visit(
            node: TopoId,
            edges: &BTreeMap<TopoId, BTreeSet<TopoId>>,
            visiting: &mut BTreeSet<TopoId>,
            visited: &mut BTreeSet<TopoId>,
        ) -> bool {
            if visited.contains(&node) {
                return true;
            }
            if !visiting.insert(node) {
                return false;
            }
            if edges
                .get(&node)
                .into_iter()
                .flatten()
                .any(|child| !visit(*child, edges, visiting, visited))
            {
                return false;
            }
            visiting.remove(&node);
            visited.insert(node);
            true
        }
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        for node in self.nodes.keys() {
            if !visit(*node, &edges, &mut visiting, &mut visited) {
                return Err(fail("Topology lineage must be acyclic"));
            }
        }
        Ok(())
    }
}

fn parse_object(
    value: value_codec::Value,
    expected: &[&str],
) -> value_codec::Result<value_codec::Map<String, value_codec::Value>> {
    let object = value
        .as_object()
        .ok_or_else(|| error("Expected object"))?
        .clone();
    if object.len() != expected.len() || expected.iter().any(|key| !object.contains_key(*key)) {
        return Err(error("Object fields do not match persistent naming schema"));
    }
    Ok(object)
}

impl value_codec::Serialize for ChangeProvenance {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert("operation".into(), self.operation.to_value());
        object.insert("operand".into(), self.operand.to_value());
        object.insert("occurrence".into(), self.occurrence.to_value());
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for ChangeProvenance {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = parse_object(value, &["operation", "operand", "occurrence"])?;
        Ok(Self {
            operation: value_codec::Deserialize::from_value(object.remove("operation").unwrap())?,
            operand: value_codec::Deserialize::from_value(object.remove("operand").unwrap())?,
            occurrence: value_codec::Deserialize::from_value(object.remove("occurrence").unwrap())?,
        })
    }
}

impl value_codec::Serialize for TopologyChange {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert("kind".into(), self.kind.to_value());
        object.insert("topoKind".into(), self.topo_kind.to_value());
        object.insert("parents".into(), self.parents.to_value());
        object.insert("children".into(), self.children.to_value());
        object.insert("provenance".into(), self.provenance.to_value());
        object.insert("role".into(), self.role.to_value());
        object.insert("anchor".into(), self.anchor.to_value());
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for TopologyChange {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = parse_object(
            value,
            &[
                "kind",
                "topoKind",
                "parents",
                "children",
                "provenance",
                "role",
                "anchor",
            ],
        )?;
        Ok(Self {
            kind: value_codec::Deserialize::from_value(object.remove("kind").unwrap())?,
            topo_kind: value_codec::Deserialize::from_value(object.remove("topoKind").unwrap())?,
            parents: value_codec::Deserialize::from_value(object.remove("parents").unwrap())?,
            children: value_codec::Deserialize::from_value(object.remove("children").unwrap())?,
            provenance: value_codec::Deserialize::from_value(object.remove("provenance").unwrap())?,
            role: value_codec::Deserialize::from_value(object.remove("role").unwrap())?,
            anchor: value_codec::Deserialize::from_value(object.remove("anchor").unwrap())?,
        })
    }
}

impl value_codec::Serialize for ChangeSet {
    fn to_value(&self) -> value_codec::Value {
        let nodes: Vec<_> = self
            .nodes
            .iter()
            .map(|(id, kind)| {
                let mut node = value_codec::Map::new();
                node.insert("id".into(), id.to_value());
                node.insert("kind".into(), kind.to_value());
                value_codec::Value::Object(node)
            })
            .collect();
        let mut object = value_codec::Map::new();
        object.insert("schema".into(), 1_u64.to_value());
        object.insert("nodes".into(), nodes.to_value());
        object.insert("changes".into(), self.changes.to_value());
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for ChangeSet {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = parse_object(value, &["schema", "nodes", "changes"])?;
        let schema: u64 = value_codec::Deserialize::from_value(object.remove("schema").unwrap())?;
        if schema != 1 {
            return Err(error("Unsupported persistent naming schema"));
        }
        let raw_nodes: Vec<value_codec::Value> =
            value_codec::Deserialize::from_value(object.remove("nodes").unwrap())?;
        let mut nodes = BTreeMap::new();
        for node in raw_nodes {
            let mut node = parse_object(node, &["id", "kind"])?;
            let id: TopoId = value_codec::Deserialize::from_value(node.remove("id").unwrap())?;
            let kind: TopoKind =
                value_codec::Deserialize::from_value(node.remove("kind").unwrap())?;
            if nodes.insert(id, kind).is_some() {
                return Err(error("Duplicate topology node ID"));
            }
        }
        let result = Self {
            nodes,
            changes: value_codec::Deserialize::from_value(object.remove("changes").unwrap())?,
        };
        result
            .validate()
            .map_err(|failure| error(failure.message))?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_ids_are_strict_and_legacy_requires_migration() {
        let id = TopoId::derive(TopoKind::Face, "op", "occ", "cap", b"surface");
        assert_eq!(TopoId::parse(&id.to_string()).unwrap(), id);
        assert!(TopoId::parse("f:0123456789abcdef").is_err());
        assert!(TopoId::parse("e:0123456789abcdef0123456789ABCDEF").is_err());
        let migrated = TopoId::migrate_legacy("f:0123456789abcdef").unwrap();
        assert_eq!(migrated.kind(), TopoKind::Face);
        assert_eq!(migrated.to_string().len(), 34);
    }

    #[test]
    fn change_sets_enforce_existence_cardinality_and_dag() {
        let a = TopoId::derive(TopoKind::Face, "op", "a", "cap", b"a");
        let b = TopoId::derive(TopoKind::Face, "op", "b", "cap", b"b");
        let provenance = ChangeProvenance {
            operation: "op".into(),
            operand: None,
            occurrence: "occ".into(),
        };
        let mut set = ChangeSet {
            nodes: [(a, TopoKind::Face), (b, TopoKind::Face)].into(),
            changes: vec![TopologyChange {
                kind: ChangeKind::Modified,
                topo_kind: TopoKind::Face,
                parents: vec![a],
                children: vec![b],
                provenance: provenance.clone(),
                role: "cap".into(),
                anchor: Some("top".into()),
            }],
        };
        set.validate().unwrap();
        set.changes.push(TopologyChange {
            kind: ChangeKind::Modified,
            topo_kind: TopoKind::Face,
            parents: vec![b],
            children: vec![a],
            provenance,
            role: "cap".into(),
            anchor: None,
        });
        assert!(set.validate().is_err());
    }
}
