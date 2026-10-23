//! Optional wire-format implementations, separate from native algorithms.
use super::*;

impl value_codec::Serialize for Trim {
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

impl<'de> value_codec::Deserialize<'de> for Trim {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let outer: Vec<UV> = value_codec::Deserialize::from_value(
            object
                .remove("outer")
                .ok_or_else(|| value_codec::error("Missing field outer"))?,
        )?;
        let holes: Vec<Vec<UV>> = if let Some(v) = object.remove("holes") {
            value_codec::Deserialize::from_value(v)?
        } else {
            Default::default()
        };
        Ok(Self { outer, holes })
    }
}

impl value_codec::Serialize for Options {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "segmentsU".into(),
            value_codec::Serialize::to_value(&self.segments_u),
        );
        object.insert(
            "segmentsV".into(),
            value_codec::Serialize::to_value(&self.segments_v),
        );
        object.insert("trim".into(), value_codec::Serialize::to_value(&self.trim));
        object.insert(
            "maxTriangles".into(),
            value_codec::Serialize::to_value(&self.max_triangles),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for Options {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let segments_u: usize = value_codec::Deserialize::from_value(
            object
                .remove("segmentsU")
                .ok_or_else(|| value_codec::error("Missing field segmentsU"))?,
        )?;
        let segments_v: usize = value_codec::Deserialize::from_value(
            object
                .remove("segmentsV")
                .ok_or_else(|| value_codec::error("Missing field segmentsV"))?,
        )?;
        let trim: Option<Trim> = if let Some(v) = object.remove("trim") {
            value_codec::Deserialize::from_value(v)?
        } else {
            Default::default()
        };
        let max_triangles: Option<usize> = if let Some(v) = object.remove("maxTriangles") {
            value_codec::Deserialize::from_value(v)?
        } else {
            Default::default()
        };
        Ok(Self {
            segments_u,
            segments_v,
            trim,
            max_triangles,
        })
    }
}
