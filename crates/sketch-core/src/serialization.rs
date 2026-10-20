//! Optional value-codec adapters for native domain types.
use super::*;

impl value_codec::Serialize for Circle {
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
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for Circle {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = match value {
            value_codec::Value::Object(map) => map,
            _ => return Err(value_codec::error("Expected object")),
        };
        let center: usize = value_codec::Deserialize::from_value(
            object
                .remove("center")
                .ok_or_else(|| value_codec::error("Missing field center"))?,
        )?;
        let radius: f64 = value_codec::Deserialize::from_value(
            object
                .remove("radius")
                .ok_or_else(|| value_codec::error("Missing field radius"))?,
        )?;
        Ok(Self { center, radius })
    }
}

impl value_codec::Serialize for Constraint {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Fix { point, at } => {
                let mut object = value_codec::Map::new();
                object.insert("point".into(), value_codec::Serialize::to_value(point));
                object.insert("at".into(), value_codec::Serialize::to_value(at));
                object.insert("kind".into(), value_codec::Value::String("fix".into()));
                value_codec::Value::Object(object)
            }
            Self::Horizontal { a, b } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("horizontal".into()),
                );
                value_codec::Value::Object(object)
            }
            Self::Vertical { a, b } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert("kind".into(), value_codec::Value::String("vertical".into()));
                value_codec::Value::Object(object)
            }
            Self::Coincident { a, b } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("coincident".into()),
                );
                value_codec::Value::Object(object)
            }
            Self::Distance { a, b, value } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert("value".into(), value_codec::Serialize::to_value(value));
                object.insert("kind".into(), value_codec::Value::String("distance".into()));
                value_codec::Value::Object(object)
            }
            Self::Parallel { a, b, c, d } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert("c".into(), value_codec::Serialize::to_value(c));
                object.insert("d".into(), value_codec::Serialize::to_value(d));
                object.insert("kind".into(), value_codec::Value::String("parallel".into()));
                value_codec::Value::Object(object)
            }
            Self::Perpendicular { a, b, c, d } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert("c".into(), value_codec::Serialize::to_value(c));
                object.insert("d".into(), value_codec::Serialize::to_value(d));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("perpendicular".into()),
                );
                value_codec::Value::Object(object)
            }
            Self::EqualLength { a, b, c, d } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert("c".into(), value_codec::Serialize::to_value(c));
                object.insert("d".into(), value_codec::Serialize::to_value(d));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("equal_length".into()),
                );
                value_codec::Value::Object(object)
            }
            Self::Radius { circle, value } => {
                let mut object = value_codec::Map::new();
                object.insert("circle".into(), value_codec::Serialize::to_value(circle));
                object.insert("value".into(), value_codec::Serialize::to_value(value));
                object.insert("kind".into(), value_codec::Value::String("radius".into()));
                value_codec::Value::Object(object)
            }
            Self::PointOnCircle { point, circle } => {
                let mut object = value_codec::Map::new();
                object.insert("point".into(), value_codec::Serialize::to_value(point));
                object.insert("circle".into(), value_codec::Serialize::to_value(circle));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("point_on_circle".into()),
                );
                value_codec::Value::Object(object)
            }
            Self::TangentLineCircle { a, b, circle } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert("circle".into(), value_codec::Serialize::to_value(circle));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("tangent_line_circle".into()),
                );
                value_codec::Value::Object(object)
            }
            Self::TangentCircles { a, b, internal } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert(
                    "internal".into(),
                    value_codec::Serialize::to_value(internal),
                );
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("tangent_circles".into()),
                );
                value_codec::Value::Object(object)
            }
            Self::Angle {
                a,
                b,
                c,
                d,
                radians,
            } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert("c".into(), value_codec::Serialize::to_value(c));
                object.insert("d".into(), value_codec::Serialize::to_value(d));
                object.insert("radians".into(), value_codec::Serialize::to_value(radians));
                object.insert("kind".into(), value_codec::Value::String("angle".into()));
                value_codec::Value::Object(object)
            }
            Self::PointToLineDistance {
                point,
                a,
                b,
                value,
            } => {
                let mut object = value_codec::Map::new();
                object.insert("point".into(), value_codec::Serialize::to_value(point));
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert("value".into(), value_codec::Serialize::to_value(value));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("point_to_line_distance".into()),
                );
                value_codec::Value::Object(object)
            }
            Self::Midpoint { point, a, b } => {
                let mut object = value_codec::Map::new();
                object.insert("point".into(), value_codec::Serialize::to_value(point));
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert("kind".into(), value_codec::Value::String("midpoint".into()));
                value_codec::Value::Object(object)
            }
            Self::Symmetric {
                a,
                b,
                axis_a,
                axis_b,
            } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert("axis_a".into(), value_codec::Serialize::to_value(axis_a));
                object.insert("axis_b".into(), value_codec::Serialize::to_value(axis_b));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("symmetric".into()),
                );
                value_codec::Value::Object(object)
            }
            Self::Collinear { a, b, c } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert("c".into(), value_codec::Serialize::to_value(c));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("collinear".into()),
                );
                value_codec::Value::Object(object)
            }
            Self::Concentric { a, b } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("concentric".into()),
                );
                value_codec::Value::Object(object)
            }
            Self::EqualRadius { a, b } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("equal_radius".into()),
                );
                value_codec::Value::Object(object)
            }
            Self::PointOnLine { point, a, b } => {
                let mut object = value_codec::Map::new();
                object.insert("point".into(), value_codec::Serialize::to_value(point));
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("point_on_line".into()),
                );
                value_codec::Value::Object(object)
            }
            Self::FixAngle { a, b, radians } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert("radians".into(), value_codec::Serialize::to_value(radians));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("fix_angle".into()),
                );
                value_codec::Value::Object(object)
            }
            Self::EqualAngle {
                a,
                b,
                c,
                d,
                e,
                f,
                g,
                h,
            } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert("c".into(), value_codec::Serialize::to_value(c));
                object.insert("d".into(), value_codec::Serialize::to_value(d));
                object.insert("e".into(), value_codec::Serialize::to_value(e));
                object.insert("f".into(), value_codec::Serialize::to_value(f));
                object.insert("g".into(), value_codec::Serialize::to_value(g));
                object.insert("h".into(), value_codec::Serialize::to_value(h));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("equal_angle".into()),
                );
                value_codec::Value::Object(object)
            }
            Self::PointOnArc {
                point,
                circle,
                start_angle,
                end_angle,
            } => {
                let mut object = value_codec::Map::new();
                object.insert("point".into(), value_codec::Serialize::to_value(point));
                object.insert("circle".into(), value_codec::Serialize::to_value(circle));
                object.insert(
                    "start_angle".into(),
                    value_codec::Serialize::to_value(start_angle),
                );
                object.insert(
                    "end_angle".into(),
                    value_codec::Serialize::to_value(end_angle),
                );
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("point_on_arc".into()),
                );
                value_codec::Value::Object(object)
            }
        }
    }
}

impl<'de> value_codec::Deserialize<'de> for Constraint {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = match value {
            value_codec::Value::Object(map) => map,
            _ => return Err(value_codec::error("Expected object")),
        };
        let kind = object.get("kind").and_then(|v| v.as_str()).unwrap_or("");
        match kind {
            "fix" => {
                let point: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("point")
                        .ok_or_else(|| value_codec::error("Missing field point"))?,
                )?;
                let at: [f64; 2] = value_codec::Deserialize::from_value(
                    object
                        .remove("at")
                        .ok_or_else(|| value_codec::error("Missing field at"))?,
                )?;
                Ok(Self::Fix { point, at })
            }
            "horizontal" => {
                let a: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                Ok(Self::Horizontal { a, b })
            }
            "vertical" => {
                let a: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                Ok(Self::Vertical { a, b })
            }
            "coincident" => {
                let a: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                Ok(Self::Coincident { a, b })
            }
            "distance" => {
                let a: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                let value: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("value")
                        .ok_or_else(|| value_codec::error("Missing field value"))?,
                )?;
                Ok(Self::Distance { a, b, value })
            }
            "parallel" => {
                let a: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                let c: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("c")
                        .ok_or_else(|| value_codec::error("Missing field c"))?,
                )?;
                let d: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("d")
                        .ok_or_else(|| value_codec::error("Missing field d"))?,
                )?;
                Ok(Self::Parallel { a, b, c, d })
            }
            "perpendicular" => {
                let a: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                let c: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("c")
                        .ok_or_else(|| value_codec::error("Missing field c"))?,
                )?;
                let d: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("d")
                        .ok_or_else(|| value_codec::error("Missing field d"))?,
                )?;
                Ok(Self::Perpendicular { a, b, c, d })
            }
            "equal_length" => {
                let a: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                let c: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("c")
                        .ok_or_else(|| value_codec::error("Missing field c"))?,
                )?;
                let d: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("d")
                        .ok_or_else(|| value_codec::error("Missing field d"))?,
                )?;
                Ok(Self::EqualLength { a, b, c, d })
            }
            "radius" => {
                let circle: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("circle")
                        .ok_or_else(|| value_codec::error("Missing field circle"))?,
                )?;
                let value: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("value")
                        .ok_or_else(|| value_codec::error("Missing field value"))?,
                )?;
                Ok(Self::Radius { circle, value })
            }
            "point_on_circle" => {
                let point: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("point")
                        .ok_or_else(|| value_codec::error("Missing field point"))?,
                )?;
                let circle: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("circle")
                        .ok_or_else(|| value_codec::error("Missing field circle"))?,
                )?;
                Ok(Self::PointOnCircle { point, circle })
            }
            "tangent_line_circle" => {
                let a: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                let circle: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("circle")
                        .ok_or_else(|| value_codec::error("Missing field circle"))?,
                )?;
                Ok(Self::TangentLineCircle { a, b, circle })
            }
            "tangent_circles" => {
                let a: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                let internal: bool = value_codec::Deserialize::from_value(
                    object
                        .remove("internal")
                        .ok_or_else(|| value_codec::error("Missing field internal"))?,
                )?;
                Ok(Self::TangentCircles { a, b, internal })
            }
            "angle" => {
                let a: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                let c: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("c")
                        .ok_or_else(|| value_codec::error("Missing field c"))?,
                )?;
                let d: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("d")
                        .ok_or_else(|| value_codec::error("Missing field d"))?,
                )?;
                let radians: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("radians")
                        .ok_or_else(|| value_codec::error("Missing field radians"))?,
                )?;
                Ok(Self::Angle {
                    a,
                    b,
                    c,
                    d,
                    radians,
                })
            }
            "point_to_line_distance" => {
                let point: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("point")
                        .ok_or_else(|| value_codec::error("Missing field point"))?,
                )?;
                let a: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                let value: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("value")
                        .ok_or_else(|| value_codec::error("Missing field value"))?,
                )?;
                Ok(Self::PointToLineDistance {
                    point,
                    a,
                    b,
                    value,
                })
            }
            "midpoint" => {
                let point: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("point")
                        .ok_or_else(|| value_codec::error("Missing field point"))?,
                )?;
                let a: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                Ok(Self::Midpoint { point, a, b })
            }
            "symmetric" => {
                let a: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                let axis_a: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("axis_a")
                        .ok_or_else(|| value_codec::error("Missing field axis_a"))?,
                )?;
                let axis_b: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("axis_b")
                        .ok_or_else(|| value_codec::error("Missing field axis_b"))?,
                )?;
                Ok(Self::Symmetric {
                    a,
                    b,
                    axis_a,
                    axis_b,
                })
            }
            "collinear" => {
                let a: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                let c: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("c")
                        .ok_or_else(|| value_codec::error("Missing field c"))?,
                )?;
                Ok(Self::Collinear { a, b, c })
            }
            "concentric" => {
                let a: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                Ok(Self::Concentric { a, b })
            }
            "equal_radius" => {
                let a: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                Ok(Self::EqualRadius { a, b })
            }
            "point_on_line" => {
                let point: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("point")
                        .ok_or_else(|| value_codec::error("Missing field point"))?,
                )?;
                let a: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                Ok(Self::PointOnLine { point, a, b })
            }
            "fix_angle" => {
                let a: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                let radians: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("radians")
                        .ok_or_else(|| value_codec::error("Missing field radians"))?,
                )?;
                Ok(Self::FixAngle { a, b, radians })
            }
            "equal_angle" => {
                let a: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                let c: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("c")
                        .ok_or_else(|| value_codec::error("Missing field c"))?,
                )?;
                let d: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("d")
                        .ok_or_else(|| value_codec::error("Missing field d"))?,
                )?;
                let e: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("e")
                        .ok_or_else(|| value_codec::error("Missing field e"))?,
                )?;
                let f: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("f")
                        .ok_or_else(|| value_codec::error("Missing field f"))?,
                )?;
                let g: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("g")
                        .ok_or_else(|| value_codec::error("Missing field g"))?,
                )?;
                let h: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("h")
                        .ok_or_else(|| value_codec::error("Missing field h"))?,
                )?;
                Ok(Self::EqualAngle {
                    a,
                    b,
                    c,
                    d,
                    e,
                    f,
                    g,
                    h,
                })
            }
            "point_on_arc" => {
                let point: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("point")
                        .ok_or_else(|| value_codec::error("Missing field point"))?,
                )?;
                let circle: usize = value_codec::Deserialize::from_value(
                    object
                        .remove("circle")
                        .ok_or_else(|| value_codec::error("Missing field circle"))?,
                )?;
                let start_angle: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("start_angle")
                        .ok_or_else(|| value_codec::error("Missing field start_angle"))?,
                )?;
                let end_angle: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("end_angle")
                        .ok_or_else(|| value_codec::error("Missing field end_angle"))?,
                )?;
                Ok(Self::PointOnArc {
                    point,
                    circle,
                    start_angle,
                    end_angle,
                })
            }
            _ => Err(value_codec::error("Unknown enum variant")),
        }
    }
}

impl value_codec::Serialize for Sketch {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "points".into(),
            value_codec::Serialize::to_value(&self.points),
        );
        object.insert(
            "circles".into(),
            value_codec::Serialize::to_value(&self.circles),
        );
        object.insert(
            "constraints".into(),
            value_codec::Serialize::to_value(&self.constraints),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for Sketch {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = match value {
            value_codec::Value::Object(map) => map,
            _ => return Err(value_codec::error("Expected object")),
        };
        let points: Vec<[f64; 2]> = value_codec::Deserialize::from_value(
            object
                .remove("points")
                .ok_or_else(|| value_codec::error("Missing field points"))?,
        )?;
        let circles: Vec<Circle> = if let Some(v) = object.remove("circles") {
            value_codec::Deserialize::from_value(v)?
        } else {
            Default::default()
        };
        let constraints: Vec<Constraint> = value_codec::Deserialize::from_value(
            object
                .remove("constraints")
                .ok_or_else(|| value_codec::error("Missing field constraints"))?,
        )?;
        Ok(Self {
            points,
            circles,
            constraints,
        })
    }
}

impl value_codec::Serialize for Solution {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "sketch".into(),
            value_codec::Serialize::to_value(&self.sketch),
        );
        object.insert(
            "status".into(),
            value_codec::Serialize::to_value(&self.status),
        );
        object.insert(
            "iterations".into(),
            value_codec::Serialize::to_value(&self.iterations),
        );
        object.insert(
            "maxResidual".into(),
            value_codec::Serialize::to_value(&self.max_residual),
        );
        object.insert(
            "residuals".into(),
            value_codec::Serialize::to_value(&self.residuals),
        );
        object.insert(
            "degreesOfFreedom".into(),
            value_codec::Serialize::to_value(&self.degrees_of_freedom),
        );
        object.insert(
            "convergenceCertified".into(),
            value_codec::Serialize::to_value(&self.convergence_certified),
        );
        value_codec::Value::Object(object)
    }
}
