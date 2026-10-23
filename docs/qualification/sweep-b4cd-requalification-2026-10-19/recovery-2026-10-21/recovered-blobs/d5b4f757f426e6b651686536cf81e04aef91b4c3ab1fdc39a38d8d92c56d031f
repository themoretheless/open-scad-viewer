//! BRep wire encoding and explicit legacy identity migration.
use super::*;

impl value_codec::Serialize for TopologyLineageRecord {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "operation".into(),
            value_codec::Serialize::to_value(&self.operation),
        );
        object.insert(
            "entityKind".into(),
            value_codec::Serialize::to_value(&self.entity_kind),
        );
        object.insert(
            "parents".into(),
            value_codec::Serialize::to_value(&self.parents),
        );
        object.insert(
            "children".into(),
            value_codec::Serialize::to_value(&self.children),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for TopologyLineageRecord {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected topology lineage record"))?;
        if object.len() != 4
            || ["operation", "entityKind", "parents", "children"]
                .iter()
                .any(|key| !object.contains_key(*key))
        {
            return Err(value_codec::error(
                "Topology lineage fields do not match the schema",
            ));
        }
        let field = |key: &str| {
            object
                .get(key)
                .cloned()
                .ok_or_else(|| value_codec::error(format!("Missing field {key}")))
        };
        Ok(Self {
            operation: value_codec::Deserialize::from_value(field("operation")?)?,
            entity_kind: value_codec::Deserialize::from_value(field("entityKind")?)?,
            parents: value_codec::Deserialize::from_value(field("parents")?)?,
            children: value_codec::Deserialize::from_value(field("children")?)?,
        })
    }
}

impl value_codec::Serialize for TopologyIds {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "vertices".into(),
            value_codec::Serialize::to_value(&self.vertices),
        );
        object.insert(
            "edges".into(),
            value_codec::Serialize::to_value(&self.edges),
        );
        object.insert(
            "loops".into(),
            value_codec::Serialize::to_value(&self.loops),
        );
        object.insert(
            "faces".into(),
            value_codec::Serialize::to_value(&self.faces),
        );
        object.insert(
            "shells".into(),
            value_codec::Serialize::to_value(&self.shells),
        );
        object.insert(
            "bodies".into(),
            value_codec::Serialize::to_value(&self.bodies),
        );
        object.insert(
            "lineage".into(),
            value_codec::Serialize::to_value(&self.lineage),
        );
        object.insert(
            "changeSet".into(),
            value_codec::Serialize::to_value(&self.change_set),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for TopologyIds {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected topologyIds object"))?;
        if object.keys().any(|key| {
            ![
                "vertices",
                "edges",
                "loops",
                "faces",
                "shells",
                "bodies",
                "lineage",
                "changeSet",
            ]
            .contains(&key.as_str())
        }) {
            return Err(value_codec::error("Unknown topologyIds field"));
        }
        let read = |key: &str| {
            object
                .get(key)
                .cloned()
                .map(value_codec::Deserialize::from_value)
                .transpose()
                .map(|value| value.unwrap_or_default())
        };
        Ok(Self {
            vertices: read("vertices")?,
            edges: read("edges")?,
            loops: read("loops")?,
            faces: read("faces")?,
            shells: read("shells")?,
            bodies: read("bodies")?,
            lineage: object
                .get("lineage")
                .cloned()
                .map(value_codec::Deserialize::from_value)
                .transpose()?
                .unwrap_or_default(),
            change_set: object
                .get("changeSet")
                .cloned()
                .map(value_codec::Deserialize::from_value)
                .transpose()?
                .unwrap_or_default(),
        })
    }
}

impl value_codec::Serialize for Model {
    fn to_value(&self) -> value_codec::Value {
        let mut value = value_codec::Serialize::to_value(&self.0);
        value.as_object_mut().unwrap().insert(
            "topologyIds".into(),
            value_codec::Serialize::to_value(&self.1),
        );
        value
    }
}

impl<'de> value_codec::Deserialize<'de> for Model {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let value_codec::Value::Object(mut object) = value else {
            return Err(value_codec::error("Expected B-rep object"));
        };
        let ids = object
            .remove("topologyIds")
            .map(TopologyIds::from_value)
            .transpose()?;
        let topology =
            <brep_topology::Model<Curve, Surface, Curve> as value_codec::Deserialize>::from_value(
                value_codec::Value::Object(object),
            )?;
        let generate_ids = ids.is_none();
        let mut model = Self(topology, ids.unwrap_or_default());
        if generate_ids {
            // Legacy documents omit the optional identity tables. Validate
            // every index and rational definition before deriving signatures;
            // malformed documents must return errors rather than index traps.
            let decode_error = |e: Error| value_codec::error(format!("{}: {}", e.code, e.message));
            model.0.validate_topology().map_err(decode_error)?;
            for edge in &model.edges {
                edge.curve.validate().map_err(decode_error)?;
            }
            for wire in &model.loops {
                for use_ in &wire.coedges {
                    use_.pcurve.validate().map_err(decode_error)?;
                }
            }
            for face in &model.faces {
                face.surface.validate().map_err(decode_error)?;
            }
            model.rebuild_topology_ids();
        } else if model.1.change_set.nodes.is_empty() && model.1.change_set.changes.is_empty() {
            // Compatibility for canonical-ID snapshots written before the
            // authoritative changeSet field was introduced.
            model.refresh_change_set(&[]);
        }
        Ok(model)
    }
}

impl value_codec::Serialize for Report {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "vertexCount".into(),
            value_codec::Serialize::to_value(&self.vertex_count),
        );
        object.insert(
            "edgeCount".into(),
            value_codec::Serialize::to_value(&self.edge_count),
        );
        object.insert(
            "loopCount".into(),
            value_codec::Serialize::to_value(&self.loop_count),
        );
        object.insert(
            "faceCount".into(),
            value_codec::Serialize::to_value(&self.face_count),
        );
        object.insert(
            "shellCount".into(),
            value_codec::Serialize::to_value(&self.shell_count),
        );
        object.insert(
            "bodyCount".into(),
            value_codec::Serialize::to_value(&self.body_count),
        );
        object.insert(
            "boundaryEdgeCount".into(),
            value_codec::Serialize::to_value(&self.boundary_edge_count),
        );
        object.insert(
            "topologyValid".into(),
            value_codec::Serialize::to_value(&self.topology_valid),
        );
        object.insert(
            "geometryAgreement".into(),
            value_codec::Serialize::to_value(&self.geometry_agreement),
        );
        object.insert(
            "solidGeometryStatus".into(),
            value_codec::Serialize::to_value(&self.solid_geometry_status),
        );
        value_codec::Value::Object(object)
    }
}

impl Model {
    /// Explicit migration entry point for old `<prefix>:<16hex>` identity
    /// tables. Normal deserialization intentionally rejects those values.
    pub fn from_legacy_topology_value(mut value: value_codec::Value) -> value_codec::Result<Self> {
        let ids = value
            .get_mut("topologyIds")
            .and_then(value_codec::Value::as_object_mut)
            .ok_or_else(|| value_codec::error("Missing legacy topologyIds object"))?;
        let mut migrated = BTreeMap::<String, String>::new();
        for field in ["vertices", "edges", "loops", "faces", "shells", "bodies"] {
            let values = ids
                .get_mut(field)
                .and_then(value_codec::Value::as_array_mut)
                .ok_or_else(|| value_codec::error(format!("Missing legacy field {field}")))?;
            for value in values {
                let old = value
                    .as_str()
                    .ok_or_else(|| value_codec::error("Legacy topology ID must be a string"))?;
                let id = TopoId::migrate_legacy(old).map_err(value_codec::error)?;
                migrated.insert(old.into(), id.to_string());
                *value = value_codec::Value::String(id.to_string());
            }
        }
        if let Some(records) = ids
            .get_mut("lineage")
            .and_then(value_codec::Value::as_array_mut)
        {
            for record in records {
                for endpoint in ["parents", "children"] {
                    let values = record
                        .get_mut(endpoint)
                        .and_then(value_codec::Value::as_array_mut)
                        .ok_or_else(|| value_codec::error("Invalid legacy lineage endpoints"))?;
                    for value in values {
                        let old = value.as_str().ok_or_else(|| {
                            value_codec::error("Legacy lineage ID must be a string")
                        })?;
                        let replacement = match migrated.get(old) {
                            Some(id) => id.clone(),
                            None => TopoId::migrate_legacy(old)
                                .map_err(value_codec::error)?
                                .to_string(),
                        };
                        *value = value_codec::Value::String(replacement);
                    }
                }
            }
        }
        ids.remove("changeSet");
        <Self as value_codec::Deserialize>::from_value(value)
    }
}
