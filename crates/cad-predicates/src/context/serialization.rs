//! Optional transport implementations.
use super::*;

impl value_codec::Serialize for ToleranceSpecIdentity {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "version".into(),
            value_codec::Serialize::to_value(&self.version),
        );
        object.insert(
            "canonical".into(),
            value_codec::Serialize::to_value(&self.canonical),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for ToleranceSpecIdentity {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected tolerance identity object"))?
            .clone();
        let version = value_codec::Deserialize::from_value(
            object
                .remove("version")
                .ok_or_else(|| value_codec::error("Missing tolerance identity version"))?,
        )?;
        let canonical = value_codec::Deserialize::from_value(
            object
                .remove("canonical")
                .ok_or_else(|| value_codec::error("Missing canonical tolerance identity"))?,
        )?;
        if version != TOLERANCE_SPEC_VERSION || !object.is_empty() {
            return Err(value_codec::error("Unsupported tolerance identity"));
        }
        Ok(Self { version, canonical })
    }
}

impl value_codec::Serialize for ToleranceSpec {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "version".into(),
            value_codec::Serialize::to_value(&TOLERANCE_SPEC_VERSION),
        );
        object.insert(
            "linearAbsMm".into(),
            value_codec::Serialize::to_value(&self.linear_abs),
        );
        object.insert(
            "linearRelative".into(),
            value_codec::Serialize::to_value(&self.linear_rel),
        );
        object.insert(
            "onMm".into(),
            value_codec::Serialize::to_value(&self.on_tol),
        );
        object.insert(
            "clearMm".into(),
            value_codec::Serialize::to_value(&self.clear_tol),
        );
        object.insert(
            "angularRadians".into(),
            value_codec::Serialize::to_value(&self.angular),
        );
        object.insert(
            "parametricFloor".into(),
            value_codec::Serialize::to_value(&self.param_floor),
        );
        object.insert(
            "ulpGuard".into(),
            value_codec::Serialize::to_value(&self.ulp_guard),
        );
        object.insert(
            "maxEntityErrorMm".into(),
            value_codec::Serialize::to_value(&self.max_entity_error),
        );
        object.insert(
            "policy".into(),
            value_codec::Serialize::to_value(&self.policy),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for ToleranceSpec {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected tolerance specification object"))?
            .clone();
        macro_rules! take {
            ($name:literal) => {
                value_codec::Deserialize::from_value(
                    object
                        .remove($name)
                        .ok_or_else(|| value_codec::error(concat!("Missing field ", $name)))?,
                )?
            };
        }
        let version: u32 = take!("version");
        if version != TOLERANCE_SPEC_VERSION {
            return Err(value_codec::error(
                "Unsupported tolerance specification version",
            ));
        }
        let spec = Self {
            linear_abs: take!("linearAbsMm"),
            linear_rel: take!("linearRelative"),
            on_tol: take!("onMm"),
            clear_tol: take!("clearMm"),
            angular: take!("angularRadians"),
            param_floor: take!("parametricFloor"),
            ulp_guard: take!("ulpGuard"),
            max_entity_error: take!("maxEntityErrorMm"),
            policy: take!("policy"),
        };
        if !object.is_empty() {
            return Err(value_codec::error("Unknown tolerance specification field"));
        }
        ToleranceContext::new(spec.clone())
            .map_err(|_| value_codec::error("Invalid tolerance specification"))?;
        Ok(spec)
    }
}

impl value_codec::Serialize for ToleranceContext {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert("spec".into(), value_codec::Serialize::to_value(&self.spec));
        object.insert(
            "identity".into(),
            value_codec::Serialize::to_value(&self.spec_identity()),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for ToleranceContext {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected tolerance context object"))?
            .clone();
        let spec: ToleranceSpec = value_codec::Deserialize::from_value(
            object
                .remove("spec")
                .ok_or_else(|| value_codec::error("Missing tolerance context spec"))?,
        )?;
        let identity: ToleranceSpecIdentity = value_codec::Deserialize::from_value(
            object
                .remove("identity")
                .ok_or_else(|| value_codec::error("Missing tolerance context identity"))?,
        )?;
        if !object.is_empty() {
            return Err(value_codec::error("Unknown tolerance context field"));
        }
        let context = Self::new(spec)
            .map_err(|_| value_codec::error("Invalid tolerance context specification"))?;
        if context.spec_identity() != identity {
            return Err(value_codec::error("Tolerance context identity mismatch"));
        }
        Ok(context)
    }
}
