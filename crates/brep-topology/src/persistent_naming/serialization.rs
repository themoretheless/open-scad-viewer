//! Optional transport implementations.
use super::*;

fn error(message: impl Into<String>) -> value_codec::Error {
    value_codec::error(message.into())
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
