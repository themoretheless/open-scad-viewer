//! Optional value-codec adapters for native domain types.
use super::*;

fn profile_to_value(rings: &Rings) -> value_codec::Value {
    let mut object = value_codec::Map::new();
    let (outer, holes) = match rings.split_first() {
        Some((outer, holes)) => (outer.clone(), holes.to_vec()),
        None => (Vec::new(), Vec::new()),
    };
    object.insert("outer".into(), value_codec::Serialize::to_value(&outer));
    object.insert("holes".into(), value_codec::Serialize::to_value(&holes));
    value_codec::Value::Object(object)
}

fn profile_from_value(value: value_codec::Value) -> value_codec::Result<Rings> {
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
    Ok(rings::from_outer_holes(outer, holes))
}





impl value_codec::Serialize for Grid {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert("min".into(), value_codec::Serialize::to_value(&self.min));
        object.insert("max".into(), value_codec::Serialize::to_value(&self.max));
        object.insert(
            "cells".into(),
            value_codec::Serialize::to_value(&self.cells),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for Grid {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let min: Point = value_codec::Deserialize::from_value(
            object
                .remove("min")
                .ok_or_else(|| value_codec::error("Missing field min"))?,
        )?;
        let max: Point = value_codec::Deserialize::from_value(
            object
                .remove("max")
                .ok_or_else(|| value_codec::error("Missing field max"))?,
        )?;
        let cells: [usize; 3] = value_codec::Deserialize::from_value(
            object
                .remove("cells")
                .ok_or_else(|| value_codec::error("Missing field cells"))?,
        )?;
        Ok(Self { min, max, cells })
    }
}

impl value_codec::Serialize for SdfTool {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        match self {
            Self::Sphere { center, radius } => {
                object.insert("shape".into(), value_codec::Value::String("sphere".into()));
                object.insert("center".into(), value_codec::Serialize::to_value(center));
                object.insert("radius".into(), value_codec::Serialize::to_value(radius));
            }
            Self::Box { center, half_size } => {
                object.insert("shape".into(), value_codec::Value::String("box".into()));
                object.insert("center".into(), value_codec::Serialize::to_value(center));
                object.insert(
                    "half_size".into(),
                    value_codec::Serialize::to_value(half_size),
                );
            }
            Self::Capsule { a, b, radius } => {
                object.insert("shape".into(), value_codec::Value::String("capsule".into()));
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert("radius".into(), value_codec::Serialize::to_value(radius));
            }
        }
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for SdfTool {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let shape = object
            .remove("shape")
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(|| value_codec::error("Missing field shape"))?;
        let mut take = |name: &str| {
            object
                .remove(name)
                .ok_or_else(|| value_codec::error(format!("Missing field {name}")))
        };
        Ok(match shape.as_str() {
            "sphere" => Self::Sphere {
                center: value_codec::Deserialize::from_value(take("center")?)?,
                radius: value_codec::Deserialize::from_value(take("radius")?)?,
            },
            "box" => Self::Box {
                center: value_codec::Deserialize::from_value(take("center")?)?,
                half_size: value_codec::Deserialize::from_value(take("half_size")?)?,
            },
            "capsule" => Self::Capsule {
                a: value_codec::Deserialize::from_value(take("a")?)?,
                b: value_codec::Deserialize::from_value(take("b")?)?,
                radius: value_codec::Deserialize::from_value(take("radius")?)?,
            },
            _ => return Err(value_codec::error("Unknown sculpt tool shape")),
        })
    }
}

impl value_codec::Serialize for SdfStroke {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert("tool".into(), value_codec::Serialize::to_value(&self.tool));
        object.insert(
            "remove".into(),
            value_codec::Serialize::to_value(&self.remove),
        );
        object.insert(
            "blend".into(),
            value_codec::Serialize::to_value(&self.blend),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for SdfStroke {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let tool: SdfTool = value_codec::Deserialize::from_value(
            object
                .remove("tool")
                .ok_or_else(|| value_codec::error("Missing field tool"))?,
        )?;
        let remove: bool = match object.remove("remove") {
            Some(value_codec::Value::Null) | None => false,
            Some(v) => value_codec::Deserialize::from_value(v)?,
        };
        let blend: Option<f64> = match object.remove("blend") {
            Some(value_codec::Value::Null) | None => None,
            Some(v) => Some(value_codec::Deserialize::from_value(v)?),
        };
        Ok(Self {
            tool,
            remove,
            blend,
        })
    }
}

#[path="serialization/methods_Field_to_value.rs"]
mod methods_field_to_value;
#[path="serialization/methods_Field_from_value.rs"]
mod methods_field_from_value;
