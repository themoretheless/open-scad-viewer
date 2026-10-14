//! Wire encoding, separate from native calculations.
use super::*;

impl value_codec::Serialize for Curve {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "degree".into(),
            value_codec::Serialize::to_value(&self.degree),
        );
        object.insert(
            "knots".into(),
            value_codec::Serialize::to_value(&self.knots),
        );
        object.insert(
            "controlPoints".into(),
            value_codec::Serialize::to_value(&self.control_points),
        );
        object.insert(
            "weights".into(),
            value_codec::Serialize::to_value(&self.weights),
        );
        object.insert(
            "periodic".into(),
            value_codec::Serialize::to_value(&self.periodic),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for Curve {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let degree: usize = value_codec::Deserialize::from_value(
            object
                .remove("degree")
                .ok_or_else(|| value_codec::error("Missing field degree"))?,
        )?;
        let knots: Vec<f64> = value_codec::Deserialize::from_value(
            object
                .remove("knots")
                .ok_or_else(|| value_codec::error("Missing field knots"))?,
        )?;
        let control_points: Vec<Vec<f64>> = value_codec::Deserialize::from_value(
            object
                .remove("controlPoints")
                .ok_or_else(|| value_codec::error("Missing field controlPoints"))?,
        )?;
        let weights: Vec<f64> = value_codec::Deserialize::from_value(
            object
                .remove("weights")
                .ok_or_else(|| value_codec::error("Missing field weights"))?,
        )?;
        let periodic: bool = if let Some(v) = object.remove("periodic") {
            value_codec::Deserialize::from_value(v)?
        } else {
            Default::default()
        };
        Ok(Self {
            degree,
            knots,
            control_points,
            weights,
            periodic,
        })
    }
}

impl value_codec::Serialize for Basis {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "basis".into(),
            value_codec::Serialize::to_value(&self.basis),
        );
        object.insert("d1".into(), value_codec::Serialize::to_value(&self.d1));
        object.insert("d2".into(), value_codec::Serialize::to_value(&self.d2));
        object.insert(
            "domain".into(),
            value_codec::Serialize::to_value(&self.domain),
        );
        object.insert(
            "continuity".into(),
            value_codec::Serialize::to_value(&self.continuity),
        );
        object.insert(
            "derivative_status".into(),
            value_codec::Serialize::to_value(&self.derivative_status),
        );
        object.insert(
            "derivative_side".into(),
            value_codec::Serialize::to_value(&self.derivative_side),
        );
        value_codec::Value::Object(object)
    }
}

impl value_codec::Serialize for Evaluation {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "point".into(),
            value_codec::Serialize::to_value(&self.point),
        );
        object.insert("d1".into(), value_codec::Serialize::to_value(&self.d1));
        object.insert("d2".into(), value_codec::Serialize::to_value(&self.d2));
        object.insert(
            "domain".into(),
            value_codec::Serialize::to_value(&self.domain),
        );
        object.insert(
            "continuity".into(),
            value_codec::Serialize::to_value(&self.continuity),
        );
        object.insert(
            "derivative_status".into(),
            value_codec::Serialize::to_value(&self.derivative_status),
        );
        object.insert(
            "derivative_side".into(),
            value_codec::Serialize::to_value(&self.derivative_side),
        );
        value_codec::Value::Object(object)
    }
}

impl value_codec::Serialize for Segment {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "curve".into(),
            value_codec::Serialize::to_value(&self.curve),
        );
        object.insert(
            "domain".into(),
            value_codec::Serialize::to_value(&self.domain),
        );
        value_codec::Value::Object(object)
    }
}
