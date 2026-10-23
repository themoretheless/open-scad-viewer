//! Optional wire-format implementations, separate from native algorithms.
use super::*;

impl value_codec::Serialize for Operation {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Union => value_codec::Value::String("union".into()),
            Self::Intersection => value_codec::Value::String("intersection".into()),
            Self::Difference => value_codec::Value::String("difference".into()),
        }
    }
}

impl<'de> value_codec::Deserialize<'de> for Operation {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        match value.as_str().unwrap_or("") {
            "union" => Ok(Self::Union),
            "intersection" => Ok(Self::Intersection),
            "difference" => Ok(Self::Difference),
            _ => Err(value_codec::error("Unknown enum variant")),
        }
    }
}

impl value_codec::Serialize for Options {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "relativeTolerance".into(),
            value_codec::Serialize::to_value(&self.relative_tolerance),
        );
        object.insert(
            "maxWork".into(),
            value_codec::Serialize::to_value(&self.max_work),
        );
        object.insert(
            "maxFragments".into(),
            value_codec::Serialize::to_value(&self.max_fragments),
        );
        object.insert(
            "maxOutputTriangles".into(),
            value_codec::Serialize::to_value(&self.max_output_triangles),
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
        let relative_tolerance: f64 = if let Some(v) = object.remove("relativeTolerance") {
            value_codec::Deserialize::from_value(v)?
        } else {
            default_tolerance()
        };
        let max_work: usize = if let Some(v) = object.remove("maxWork") {
            value_codec::Deserialize::from_value(v)?
        } else {
            default_work()
        };
        let max_fragments: usize = if let Some(v) = object.remove("maxFragments") {
            value_codec::Deserialize::from_value(v)?
        } else {
            default_fragments()
        };
        let max_output_triangles: usize = if let Some(v) = object.remove("maxOutputTriangles") {
            value_codec::Deserialize::from_value(v)?
        } else {
            default_output()
        };
        if let Some(key) = object.keys().next() {
            return Err(value_codec::error(format!("Unknown field {key}")));
        }
        Ok(Self {
            relative_tolerance,
            max_work,
            max_fragments,
            max_output_triangles,
        })
    }
}

impl value_codec::Serialize for BooleanReport {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "operation".into(),
            value_codec::Serialize::to_value(&self.operation),
        );
        object.insert(
            "toleranceMm".into(),
            value_codec::Serialize::to_value(&self.tolerance_mm),
        );
        object.insert("work".into(), value_codec::Serialize::to_value(&self.work));
        object.insert(
            "fragments".into(),
            value_codec::Serialize::to_value(&self.fragments),
        );
        object.insert(
            "inputTriangles".into(),
            value_codec::Serialize::to_value(&self.input_triangles),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for BooleanReport {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let operation: Operation = value_codec::Deserialize::from_value(
            object
                .remove("operation")
                .ok_or_else(|| value_codec::error("Missing field operation"))?,
        )?;
        let tolerance_mm: f64 = value_codec::Deserialize::from_value(
            object
                .remove("toleranceMm")
                .ok_or_else(|| value_codec::error("Missing field toleranceMm"))?,
        )?;
        let work: usize = value_codec::Deserialize::from_value(
            object
                .remove("work")
                .ok_or_else(|| value_codec::error("Missing field work"))?,
        )?;
        let fragments: usize = value_codec::Deserialize::from_value(
            object
                .remove("fragments")
                .ok_or_else(|| value_codec::error("Missing field fragments"))?,
        )?;
        let input_triangles: [usize; 2] = value_codec::Deserialize::from_value(
            object
                .remove("inputTriangles")
                .ok_or_else(|| value_codec::error("Missing field inputTriangles"))?,
        )?;
        Ok(Self {
            operation,
            tolerance_mm,
            work,
            fragments,
            input_triangles,
        })
    }
}
