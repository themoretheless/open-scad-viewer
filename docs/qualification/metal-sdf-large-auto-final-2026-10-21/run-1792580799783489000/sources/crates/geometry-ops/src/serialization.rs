//! Optional value-codec adapters for native domain types.
use super::*;

impl value_codec::Serialize for Deformation {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Twist {
                origin,
                radians_per_unit,
            } => {
                let mut object = value_codec::Map::new();
                object.insert("origin".into(), value_codec::Serialize::to_value(origin));
                object.insert(
                    "radians_per_unit".into(),
                    value_codec::Serialize::to_value(radians_per_unit),
                );
                object.insert("kind".into(), value_codec::Value::String("twist".into()));
                value_codec::Value::Object(object)
            }
            Self::Bend { origin, radius } => {
                let mut object = value_codec::Map::new();
                object.insert("origin".into(), value_codec::Serialize::to_value(origin));
                object.insert("radius".into(), value_codec::Serialize::to_value(radius));
                object.insert("kind".into(), value_codec::Value::String("bend".into()));
                value_codec::Value::Object(object)
            }
            Self::Lattice { min, max, controls } => {
                let mut object = value_codec::Map::new();
                object.insert("min".into(), value_codec::Serialize::to_value(min));
                object.insert("max".into(), value_codec::Serialize::to_value(max));
                object.insert(
                    "controls".into(),
                    value_codec::Serialize::to_value(controls),
                );
                object.insert("kind".into(), value_codec::Value::String("lattice".into()));
                value_codec::Value::Object(object)
            }
        }
    }
}

impl<'de> value_codec::Deserialize<'de> for Deformation {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = codec::object(value)?;
        let kind: String = codec::required(&mut object, "kind")?;
        match kind.as_str() {
            "twist" => Ok(Self::Twist {
                origin: codec::required(&mut object, "origin")?,
                radians_per_unit: codec::required(&mut object, "radians_per_unit")?,
            }),
            "bend" => Ok(Self::Bend {
                origin: codec::required(&mut object, "origin")?,
                radius: codec::required(&mut object, "radius")?,
            }),
            "lattice" => Ok(Self::Lattice {
                min: codec::required(&mut object, "min")?,
                max: codec::required(&mut object, "max")?,
                controls: codec::required(&mut object, "controls")?,
            }),
            _ => Err(value_codec::error("Unknown deformation kind")),
        }
    }
}

impl value_codec::Serialize for Brush {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "center".into(),
            value_codec::Serialize::to_value(&self.center),
        );
        object.insert(
            "radius".into(),
            value_codec::Serialize::to_value(&self.radius),
        );
        object.insert(
            "displacement".into(),
            value_codec::Serialize::to_value(&self.displacement),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for Brush {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = codec::object(value)?;
        Ok(Self {
            center: codec::required(&mut object, "center")?,
            radius: codec::required(&mut object, "radius")?,
            displacement: codec::required(&mut object, "displacement")?,
        })
    }
}

impl value_codec::Serialize for Triangles {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "positions".into(),
            value_codec::Serialize::to_value(&self.positions),
        );
        object.insert(
            "indices".into(),
            value_codec::Serialize::to_value(&self.indices),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for Triangles {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let positions = value_codec::Deserialize::from_value(
            object
                .remove("positions")
                .ok_or_else(|| value_codec::error("Missing field positions"))?,
        )?;
        let indices = value_codec::Deserialize::from_value(
            object
                .remove("indices")
                .ok_or_else(|| value_codec::error("Missing field indices"))?,
        )?;
        Ok(Self { positions, indices })
    }
}
