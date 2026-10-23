use super::*;

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
