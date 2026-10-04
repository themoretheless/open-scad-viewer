//! Optional transport implementations.
use super::*;

impl value_codec::Serialize for CoedgeTrim {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({
            "curveParameter": self.curve_parameter,
            "pcurveParameter": self.pcurve_parameter,
            "periodicLift": self.periodic_lift
        })
    }
}

impl<V: value_codec::Serialize> value_codec::Serialize for Vertex<V> {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "point".into(),
            value_codec::Serialize::to_value(&self.point),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de, V: value_codec::Deserialize<'de>> value_codec::Deserialize<'de> for Vertex<V> {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let point: V = value_codec::Deserialize::from_value(
            object
                .remove("point")
                .ok_or_else(|| value_codec::error("Missing field point"))?,
        )?;
        if let Some(key) = object.keys().next() {
            return Err(value_codec::error(format!("Unknown field {key}")));
        }
        Ok(Self { point })
    }
}

impl<C: value_codec::Serialize> value_codec::Serialize for Edge<C> {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        if self.degenerate {
            object.insert("degenerate".into(), value_codec::Value::Bool(true));
        }
        object.insert(
            "vertices".into(),
            value_codec::Serialize::to_value(&self.vertices),
        );
        object.insert(
            "curve".into(),
            value_codec::Serialize::to_value(&self.curve),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de, C: value_codec::Deserialize<'de>> value_codec::Deserialize<'de> for Edge<C> {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let degenerate = object
            .remove("degenerate")
            .map(value_codec::Deserialize::from_value)
            .transpose()?
            .unwrap_or(false);
        let vertices: [usize; 2] = value_codec::Deserialize::from_value(
            object
                .remove("vertices")
                .ok_or_else(|| value_codec::error("Missing field vertices"))?,
        )?;
        let curve: C = value_codec::Deserialize::from_value(
            object
                .remove("curve")
                .ok_or_else(|| value_codec::error("Missing field curve"))?,
        )?;
        if let Some(key) = object.keys().next() {
            return Err(value_codec::error(format!("Unknown field {key}")));
        }
        Ok(Self {
            degenerate,
            vertices,
            curve,
        })
    }
}

impl<P: value_codec::Serialize> value_codec::Serialize for Coedge<P> {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert("edge".into(), value_codec::Serialize::to_value(&self.edge));
        object.insert(
            "reversed".into(),
            value_codec::Serialize::to_value(&self.reversed),
        );
        object.insert(
            "pcurve".into(),
            value_codec::Serialize::to_value(&self.pcurve),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de, P: value_codec::Deserialize<'de>> value_codec::Deserialize<'de> for Coedge<P> {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let edge: usize = value_codec::Deserialize::from_value(
            object
                .remove("edge")
                .ok_or_else(|| value_codec::error("Missing field edge"))?,
        )?;
        let reversed: bool = value_codec::Deserialize::from_value(
            object
                .remove("reversed")
                .ok_or_else(|| value_codec::error("Missing field reversed"))?,
        )?;
        let pcurve: P = value_codec::Deserialize::from_value(
            object
                .remove("pcurve")
                .ok_or_else(|| value_codec::error("Missing field pcurve"))?,
        )?;
        if let Some(key) = object.keys().next() {
            return Err(value_codec::error(format!("Unknown field {key}")));
        }
        Ok(Self {
            edge,
            reversed,
            pcurve,
        })
    }
}

impl<P: value_codec::Serialize> value_codec::Serialize for Loop<P> {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "coedges".into(),
            value_codec::Serialize::to_value(&self.coedges),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de, P: value_codec::Deserialize<'de>> value_codec::Deserialize<'de> for Loop<P> {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let coedges: Vec<Coedge<P>> = value_codec::Deserialize::from_value(
            object
                .remove("coedges")
                .ok_or_else(|| value_codec::error("Missing field coedges"))?,
        )?;
        if let Some(key) = object.keys().next() {
            return Err(value_codec::error(format!("Unknown field {key}")));
        }
        Ok(Self { coedges })
    }
}

impl<S: value_codec::Serialize> value_codec::Serialize for Face<S> {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "surface".into(),
            value_codec::Serialize::to_value(&self.surface),
        );
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

impl<'de, S: value_codec::Deserialize<'de>> value_codec::Deserialize<'de> for Face<S> {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let surface: S = value_codec::Deserialize::from_value(
            object
                .remove("surface")
                .ok_or_else(|| value_codec::error("Missing field surface"))?,
        )?;
        let outer: usize = value_codec::Deserialize::from_value(
            object
                .remove("outer")
                .ok_or_else(|| value_codec::error("Missing field outer"))?,
        )?;
        let holes: Vec<usize> = value_codec::Deserialize::from_value(
            object
                .remove("holes")
                .ok_or_else(|| value_codec::error("Missing field holes"))?,
        )?;
        if let Some(key) = object.keys().next() {
            return Err(value_codec::error(format!("Unknown field {key}")));
        }
        Ok(Self {
            surface,
            outer,
            holes,
        })
    }
}

impl value_codec::Serialize for FaceUse {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert("face".into(), value_codec::Serialize::to_value(&self.face));
        object.insert(
            "reversed".into(),
            value_codec::Serialize::to_value(&self.reversed),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for FaceUse {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let face: usize = value_codec::Deserialize::from_value(
            object
                .remove("face")
                .ok_or_else(|| value_codec::error("Missing field face"))?,
        )?;
        let reversed: bool = value_codec::Deserialize::from_value(
            object
                .remove("reversed")
                .ok_or_else(|| value_codec::error("Missing field reversed"))?,
        )?;
        if let Some(key) = object.keys().next() {
            return Err(value_codec::error(format!("Unknown field {key}")));
        }
        Ok(Self { face, reversed })
    }
}

impl value_codec::Serialize for Shell {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "faces".into(),
            value_codec::Serialize::to_value(&self.faces),
        );
        object.insert(
            "closed".into(),
            value_codec::Serialize::to_value(&self.closed),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for Shell {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let faces: Vec<FaceUse> = value_codec::Deserialize::from_value(
            object
                .remove("faces")
                .ok_or_else(|| value_codec::error("Missing field faces"))?,
        )?;
        let closed: bool = value_codec::Deserialize::from_value(
            object
                .remove("closed")
                .ok_or_else(|| value_codec::error("Missing field closed"))?,
        )?;
        if let Some(key) = object.keys().next() {
            return Err(value_codec::error(format!("Unknown field {key}")));
        }
        Ok(Self { faces, closed })
    }
}

impl value_codec::Serialize for Body {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "outerShell".into(),
            value_codec::Serialize::to_value(&self.outer_shell),
        );
        object.insert(
            "innerShells".into(),
            value_codec::Serialize::to_value(&self.inner_shells),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for Body {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let outer_shell: usize = value_codec::Deserialize::from_value(
            object
                .remove("outerShell")
                .ok_or_else(|| value_codec::error("Missing field outerShell"))?,
        )?;
        let inner_shells: Vec<usize> = value_codec::Deserialize::from_value(
            object
                .remove("innerShells")
                .ok_or_else(|| value_codec::error("Missing field innerShells"))?,
        )?;
        if let Some(key) = object.keys().next() {
            return Err(value_codec::error(format!("Unknown field {key}")));
        }
        Ok(Self {
            outer_shell,
            inner_shells,
        })
    }
}

impl<
    C: value_codec::Serialize,
    S: value_codec::Serialize,
    P: value_codec::Serialize,
    V: value_codec::Serialize,
> value_codec::Serialize for Model<C, S, P, V>
{
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "vertices".into(),
            value_codec::Serialize::to_value(&self.vertices),
        );
        object.insert(
            "edges".into(),
            value_codec::Serialize::to_value(&self.edges),
        );
        object.insert(
            "loops".into(),
            value_codec::Serialize::to_value(&self.loops),
        );
        object.insert(
            "faces".into(),
            value_codec::Serialize::to_value(&self.faces),
        );
        object.insert(
            "shells".into(),
            value_codec::Serialize::to_value(&self.shells),
        );
        object.insert(
            "bodies".into(),
            value_codec::Serialize::to_value(&self.bodies),
        );
        object.insert(
            "toleranceMm".into(),
            value_codec::Serialize::to_value(&self.tolerance_mm),
        );
        if let Ok(context) =
            cad_predicates::ToleranceContext::from_brep_tolerance_mm(self.tolerance_mm)
        {
            object.insert(
                "toleranceContext".into(),
                value_codec::Serialize::to_value(&context),
            );
        }
        value_codec::Value::Object(object)
    }
}
impl<
    'de,
    C: value_codec::Deserialize<'de>,
    S: value_codec::Deserialize<'de>,
    P: value_codec::Deserialize<'de>,
    V: value_codec::Deserialize<'de>,
> value_codec::Deserialize<'de> for Model<C, S, P, V>
{
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let vertices: Vec<Vertex<V>> = value_codec::Deserialize::from_value(
            object
                .remove("vertices")
                .ok_or_else(|| value_codec::error("Missing field vertices"))?,
        )?;
        let edges: Vec<Edge<C>> = value_codec::Deserialize::from_value(
            object
                .remove("edges")
                .ok_or_else(|| value_codec::error("Missing field edges"))?,
        )?;
        let loops: Vec<Loop<P>> = value_codec::Deserialize::from_value(
            object
                .remove("loops")
                .ok_or_else(|| value_codec::error("Missing field loops"))?,
        )?;
        let faces: Vec<Face<S>> = value_codec::Deserialize::from_value(
            object
                .remove("faces")
                .ok_or_else(|| value_codec::error("Missing field faces"))?,
        )?;
        let shells: Vec<Shell> = value_codec::Deserialize::from_value(
            object
                .remove("shells")
                .ok_or_else(|| value_codec::error("Missing field shells"))?,
        )?;
        let bodies: Vec<Body> = value_codec::Deserialize::from_value(
            object
                .remove("bodies")
                .ok_or_else(|| value_codec::error("Missing field bodies"))?,
        )?;
        let legacy_tolerance_mm = object
            .remove("toleranceMm")
            .map(value_codec::Deserialize::from_value)
            .transpose()?;
        let context: Option<cad_predicates::ToleranceContext> = object
            .remove("toleranceContext")
            .map(value_codec::Deserialize::from_value)
            .transpose()?;
        let tolerance_mm = match (context, legacy_tolerance_mm) {
            (Some(context), legacy) => {
                let on_mm = context.spatial_bounds().on_mm;
                if legacy.is_some_and(|value: f64| value.to_bits() != on_mm.to_bits()) {
                    return Err(value_codec::error(
                        "toleranceMm conflicts with toleranceContext",
                    ));
                }
                on_mm
            }
            (None, Some(value)) => {
                cad_predicates::ToleranceContext::from_brep_tolerance_mm(value)
                    .map_err(|_| value_codec::error("Invalid legacy toleranceMm"))?;
                value
            }
            (None, None) => {
                return Err(value_codec::error(
                    "Missing toleranceContext or legacy toleranceMm",
                ));
            }
        };
        if let Some(key) = object.keys().next() {
            return Err(value_codec::error(format!("Unknown field {key}")));
        }
        Ok(Self {
            vertices,
            edges,
            loops,
            faces,
            shells,
            bodies,
            tolerance_mm,
        })
    }
}
