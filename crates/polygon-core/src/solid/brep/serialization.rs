//! Optional wire-format implementations, separate from native algorithms.
use super::*;

impl value_codec::Serialize for FaceGeometry {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert("mesh".into(), value_codec::Serialize::to_value(&self.mesh));
        object.insert(
            "sourceFaceId".into(),
            value_codec::Serialize::to_value(&self.source_face_id),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for FaceGeometry {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let mesh: Mesh = value_codec::Deserialize::from_value(
            object
                .remove("mesh")
                .ok_or_else(|| value_codec::error("Missing field mesh"))?,
        )?;
        let source_face_id: usize = value_codec::Deserialize::from_value(
            object
                .remove("sourceFaceId")
                .ok_or_else(|| value_codec::error("Missing field sourceFaceId"))?,
        )?;
        if let Some(key) = object.keys().next() {
            return Err(value_codec::error(format!("Unknown field {key}")));
        }
        Ok(Self {
            mesh,
            source_face_id,
        })
    }
}

impl value_codec::Serialize for Tessellation {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        if let value_codec::Value::Object(fields) = value_codec::Serialize::to_value(&self.mesh) {
            object.extend(fields);
        }
        object.insert(
            "faceIds".into(),
            value_codec::Serialize::to_value(&self.face_ids),
        );
        value_codec::Value::Object(object)
    }
}
