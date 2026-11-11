//! Wire encoding, separate from native calculations.
use super::*;

impl value_codec::Serialize for Surface {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "degreeU".into(),
            value_codec::Serialize::to_value(&self.degree_u),
        );
        object.insert(
            "degreeV".into(),
            value_codec::Serialize::to_value(&self.degree_v),
        );
        object.insert(
            "knotsU".into(),
            value_codec::Serialize::to_value(&self.knots_u),
        );
        object.insert(
            "knotsV".into(),
            value_codec::Serialize::to_value(&self.knots_v),
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
            "periodicU".into(),
            value_codec::Serialize::to_value(&self.periodic_u),
        );
        object.insert(
            "periodicV".into(),
            value_codec::Serialize::to_value(&self.periodic_v),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for Surface {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let degree_u: usize = value_codec::Deserialize::from_value(
            object
                .remove("degreeU")
                .ok_or_else(|| value_codec::error("Missing field degreeU"))?,
        )?;
        let degree_v: usize = value_codec::Deserialize::from_value(
            object
                .remove("degreeV")
                .ok_or_else(|| value_codec::error("Missing field degreeV"))?,
        )?;
        let knots_u: Vec<f64> = value_codec::Deserialize::from_value(
            object
                .remove("knotsU")
                .ok_or_else(|| value_codec::error("Missing field knotsU"))?,
        )?;
        let knots_v: Vec<f64> = value_codec::Deserialize::from_value(
            object
                .remove("knotsV")
                .ok_or_else(|| value_codec::error("Missing field knotsV"))?,
        )?;
        let control_points: Vec<Vec<Vec<f64>>> = value_codec::Deserialize::from_value(
            object
                .remove("controlPoints")
                .ok_or_else(|| value_codec::error("Missing field controlPoints"))?,
        )?;
        let weights: Vec<Vec<f64>> = value_codec::Deserialize::from_value(
            object
                .remove("weights")
                .ok_or_else(|| value_codec::error("Missing field weights"))?,
        )?;
        let periodic_u: bool = if let Some(v) = object.remove("periodicU") {
            value_codec::Deserialize::from_value(v)?
        } else {
            Default::default()
        };
        let periodic_v: bool = if let Some(v) = object.remove("periodicV") {
            value_codec::Deserialize::from_value(v)?
        } else {
            Default::default()
        };
        Ok(Self {
            degree_u,
            degree_v,
            knots_u,
            knots_v,
            control_points,
            weights,
            periodic_u,
            periodic_v,
        })
    }
}

impl<'de> value_codec::Deserialize<'de> for Axis {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        match value.as_str().unwrap_or("") {
            "u" => Ok(Self::U),
            "v" => Ok(Self::V),
            _ => Err(value_codec::error("Unknown enum variant")),
        }
    }
}

impl value_codec::Serialize for Evaluation {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "point".into(),
            value_codec::Serialize::to_value(&self.point),
        );
        object.insert("du".into(), value_codec::Serialize::to_value(&self.du));
        object.insert("dv".into(), value_codec::Serialize::to_value(&self.dv));
        object.insert("duu".into(), value_codec::Serialize::to_value(&self.duu));
        object.insert("duv".into(), value_codec::Serialize::to_value(&self.duv));
        object.insert("dvv".into(), value_codec::Serialize::to_value(&self.dvv));
        object.insert(
            "normal".into(),
            value_codec::Serialize::to_value(&self.normal),
        );
        object.insert(
            "gaussianCurvature".into(),
            value_codec::Serialize::to_value(&self.gaussian_curvature),
        );
        object.insert(
            "meanCurvature".into(),
            value_codec::Serialize::to_value(&self.mean_curvature),
        );
        object.insert(
            "derivative_status".into(),
            value_codec::Serialize::to_value(&self.derivative_status),
        );
        object.insert(
            "derivative_side_u".into(),
            value_codec::Serialize::to_value(&self.derivative_side_u),
        );
        object.insert(
            "derivative_side_v".into(),
            value_codec::Serialize::to_value(&self.derivative_side_v),
        );
        object.insert(
            "domainU".into(),
            value_codec::Serialize::to_value(&self.domain_u),
        );
        object.insert(
            "domainV".into(),
            value_codec::Serialize::to_value(&self.domain_v),
        );
        value_codec::Value::Object(object)
    }
}
