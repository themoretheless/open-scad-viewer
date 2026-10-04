//! Optional wire-format implementations, separate from native algorithms.
use super::*;

impl value_codec::Serialize for Profile {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "outer".into(),
            value_codec::Serialize::to_value(&self.outer),
        );
        object.insert(
            "holes".into(),
            value_codec::Serialize::to_value(&self.holes),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for Profile {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let outer: Vec<[f64; 2]> = value_codec::Deserialize::from_value(
            object
                .remove("outer")
                .ok_or_else(|| value_codec::error("Missing field outer"))?,
        )?;
        let holes: Vec<Vec<[f64; 2]>> = if let Some(v) = object.remove("holes") {
            value_codec::Deserialize::from_value(v)?
        } else {
            Default::default()
        };
        Ok(Self { outer, holes })
    }
}
