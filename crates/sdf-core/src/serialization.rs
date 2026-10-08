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

impl value_codec::Serialize for Field {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Extrude {
                profile,
                half_height,
            } => {
                let mut object = value_codec::Map::new();
                object.insert("profile".into(), profile_to_value(profile));
                object.insert(
                    "half_height".into(),
                    value_codec::Serialize::to_value(half_height),
                );
                object.insert("kind".into(), value_codec::Value::String("extrude".into()));
                value_codec::Value::Object(object)
            }
            Self::Revolve { profile } => {
                let mut object = value_codec::Map::new();
                object.insert("profile".into(), profile_to_value(profile));
                object.insert("kind".into(), value_codec::Value::String("revolve".into()));
                value_codec::Value::Object(object)
            }
            Self::Deform { input, deformation } => {
                let mut object = value_codec::Map::new();
                object.insert("input".into(), value_codec::Serialize::to_value(input));
                object.insert(
                    "deformation".into(),
                    value_codec::Serialize::to_value(deformation),
                );
                object.insert("kind".into(), value_codec::Value::String("deform".into()));
                value_codec::Value::Object(object)
            }
            Self::MeshDistance { mesh, signed } => {
                let mut object = value_codec::Map::new();
                object.insert("mesh".into(), value_codec::Serialize::to_value(mesh));
                object.insert("signed".into(), value_codec::Serialize::to_value(signed));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("mesh_distance".into()),
                );
                value_codec::Value::Object(object)
            }
            Self::Sphere { center, radius } => {
                let mut object = value_codec::Map::new();
                object.insert("center".into(), value_codec::Serialize::to_value(center));
                object.insert("radius".into(), value_codec::Serialize::to_value(radius));
                object.insert("kind".into(), value_codec::Value::String("sphere".into()));
                value_codec::Value::Object(object)
            }
            Self::Box { center, half_size } => {
                let mut object = value_codec::Map::new();
                object.insert("center".into(), value_codec::Serialize::to_value(center));
                object.insert(
                    "half_size".into(),
                    value_codec::Serialize::to_value(half_size),
                );
                object.insert("kind".into(), value_codec::Value::String("box".into()));
                value_codec::Value::Object(object)
            }
            Self::Torus {
                center,
                major_radius,
                minor_radius,
            } => {
                let mut object = value_codec::Map::new();
                object.insert("center".into(), value_codec::Serialize::to_value(center));
                object.insert(
                    "major_radius".into(),
                    value_codec::Serialize::to_value(major_radius),
                );
                object.insert(
                    "minor_radius".into(),
                    value_codec::Serialize::to_value(minor_radius),
                );
                object.insert("kind".into(), value_codec::Value::String("torus".into()));
                value_codec::Value::Object(object)
            }
            Self::Capsule { a, b, radius } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert("radius".into(), value_codec::Serialize::to_value(radius));
                object.insert("kind".into(), value_codec::Value::String("capsule".into()));
                value_codec::Value::Object(object)
            }
            Self::Union { a, b } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert("kind".into(), value_codec::Value::String("union".into()));
                value_codec::Value::Object(object)
            }
            Self::SmoothDifference { a, b, radius } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert("radius".into(), value_codec::Serialize::to_value(radius));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("smooth_difference".into()),
                );
                value_codec::Value::Object(object)
            }
            Self::Intersection { a, b } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("intersection".into()),
                );
                value_codec::Value::Object(object)
            }
            Self::Difference { a, b } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("difference".into()),
                );
                value_codec::Value::Object(object)
            }
            Self::SmoothUnion { a, b, radius } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert("radius".into(), value_codec::Serialize::to_value(radius));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("smooth_union".into()),
                );
                value_codec::Value::Object(object)
            }
            Self::Offset { input, distance } => {
                let mut object = value_codec::Map::new();
                object.insert("input".into(), value_codec::Serialize::to_value(input));
                object.insert(
                    "distance".into(),
                    value_codec::Serialize::to_value(distance),
                );
                object.insert("kind".into(), value_codec::Value::String("offset".into()));
                value_codec::Value::Object(object)
            }
            Self::Translate { input, vector } => {
                let mut object = value_codec::Map::new();
                object.insert("input".into(), value_codec::Serialize::to_value(input));
                object.insert("vector".into(), value_codec::Serialize::to_value(vector));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("translate".into()),
                );
                value_codec::Value::Object(object)
            }
        }
    }
}

impl<'de> value_codec::Deserialize<'de> for Field {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        match value["kind"].as_str().unwrap_or("") {
            "extrude" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let profile = profile_from_value(
                    object
                        .remove("profile")
                        .ok_or_else(|| value_codec::error("Missing field profile"))?,
                )?;
                let half_height: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("half_height")
                        .ok_or_else(|| value_codec::error("Missing field half_height"))?,
                )?;
                Ok(Self::Extrude {
                    profile,
                    half_height,
                })
            }
            "revolve" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let profile = profile_from_value(
                    object
                        .remove("profile")
                        .ok_or_else(|| value_codec::error("Missing field profile"))?,
                )?;
                Ok(Self::Revolve { profile })
            }
            "deform" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let input: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("input")
                        .ok_or_else(|| value_codec::error("Missing field input"))?,
                )?;
                let deformation: geometry_ops::Deformation = value_codec::Deserialize::from_value(
                    object
                        .remove("deformation")
                        .ok_or_else(|| value_codec::error("Missing field deformation"))?,
                )?;
                Ok(Self::Deform { input, deformation })
            }
            "mesh_distance" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let mesh: Triangles = value_codec::Deserialize::from_value(
                    object
                        .remove("mesh")
                        .ok_or_else(|| value_codec::error("Missing field mesh"))?,
                )?;
                let signed: bool = value_codec::Deserialize::from_value(
                    object
                        .remove("signed")
                        .ok_or_else(|| value_codec::error("Missing field signed"))?,
                )?;
                Ok(Self::MeshDistance { mesh, signed })
            }
            "sphere" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let center: Point = value_codec::Deserialize::from_value(
                    object
                        .remove("center")
                        .ok_or_else(|| value_codec::error("Missing field center"))?,
                )?;
                let radius: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("radius")
                        .ok_or_else(|| value_codec::error("Missing field radius"))?,
                )?;
                Ok(Self::Sphere { center, radius })
            }
            "box" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let center: Point = value_codec::Deserialize::from_value(
                    object
                        .remove("center")
                        .ok_or_else(|| value_codec::error("Missing field center"))?,
                )?;
                let half_size: Point = value_codec::Deserialize::from_value(
                    object
                        .remove("half_size")
                        .ok_or_else(|| value_codec::error("Missing field half_size"))?,
                )?;
                Ok(Self::Box { center, half_size })
            }
            "torus" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let center: Point = value_codec::Deserialize::from_value(
                    object
                        .remove("center")
                        .ok_or_else(|| value_codec::error("Missing field center"))?,
                )?;
                let major_radius: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("major_radius")
                        .ok_or_else(|| value_codec::error("Missing field major_radius"))?,
                )?;
                let minor_radius: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("minor_radius")
                        .ok_or_else(|| value_codec::error("Missing field minor_radius"))?,
                )?;
                Ok(Self::Torus {
                    center,
                    major_radius,
                    minor_radius,
                })
            }
            "union" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let a: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                Ok(Self::Union { a, b })
            }
            "intersection" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let a: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                Ok(Self::Intersection { a, b })
            }
            "difference" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let a: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                Ok(Self::Difference { a, b })
            }
            "capsule" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let a: Point = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: Point = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                let radius: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("radius")
                        .ok_or_else(|| value_codec::error("Missing field radius"))?,
                )?;
                Ok(Self::Capsule { a, b, radius })
            }
            "smooth_difference" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let a: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                let radius: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("radius")
                        .ok_or_else(|| value_codec::error("Missing field radius"))?,
                )?;
                Ok(Self::SmoothDifference { a, b, radius })
            }
            "smooth_union" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let a: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                let radius: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("radius")
                        .ok_or_else(|| value_codec::error("Missing field radius"))?,
                )?;
                Ok(Self::SmoothUnion { a, b, radius })
            }
            "offset" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let input: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("input")
                        .ok_or_else(|| value_codec::error("Missing field input"))?,
                )?;
                let distance: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("distance")
                        .ok_or_else(|| value_codec::error("Missing field distance"))?,
                )?;
                Ok(Self::Offset { input, distance })
            }
            "translate" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let input: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("input")
                        .ok_or_else(|| value_codec::error("Missing field input"))?,
                )?;
                let vector: Point = value_codec::Deserialize::from_value(
                    object
                        .remove("vector")
                        .ok_or_else(|| value_codec::error("Missing field vector"))?,
                )?;
                Ok(Self::Translate { input, vector })
            }
            _ => Err(value_codec::error("Unknown enum variant")),
        }
    }
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
