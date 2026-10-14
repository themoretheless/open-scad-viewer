//! Optional value-codec adapters for native domain types.
use super::*;

impl value_codec::Serialize for Cage {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "vertices".into(),
            value_codec::Serialize::to_value(&self.vertices),
        );
        object.insert(
            "faces".into(),
            value_codec::Serialize::to_value(&self.faces),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for Cage {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let vertices: Vec<Point> = value_codec::Deserialize::from_value(
            object
                .remove("vertices")
                .ok_or_else(|| value_codec::error("Missing field vertices"))?,
        )?;
        let faces: Vec<Vec<usize>> = value_codec::Deserialize::from_value(
            object
                .remove("faces")
                .ok_or_else(|| value_codec::error("Missing field faces"))?,
        )?;
        Ok(Self { vertices, faces })
    }
}

impl value_codec::Serialize for Refined {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert("cage".into(), value_codec::Serialize::to_value(&self.cage));
        object.insert(
            "faceIds".into(),
            value_codec::Serialize::to_value(&self.face_ids),
        );
        value_codec::Value::Object(object)
    }
}
