//! Negative-inside implicit fields and bounded marching-tetrahedra extraction.
//! CSG fields preserve the zero set but are not generally exact signed distances.
use polygon_kernel::{Error, Mesh, Result};
use std::collections::BTreeMap;
pub type Point = [f64; 3];
fn sub(a: Point, b: Point) -> Point {
    std::array::from_fn(|i| a[i] - b[i])
}
fn dot(a: Point, b: Point) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn cross(a: Point, b: Point) -> Point {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn length(a: Point) -> f64 {
    dot(a, a).sqrt()
}
#[derive(Clone, Debug)]
pub enum Field {
    Extrude {
        profile: geometry_ops::Region2,
        half_height: f64,
    },
    Revolve {
        profile: geometry_ops::Region2,
    },
    Deform {
        input: Box<Field>,
        deformation: geometry_ops::Deformation,
    },
    MeshDistance {
        mesh: Mesh,
        signed: bool,
    },
    Sphere {
        center: Point,
        radius: f64,
    },
    Box {
        center: Point,
        half_size: Point,
    },
    Torus {
        center: Point,
        major_radius: f64,
        minor_radius: f64,
    },
    Union {
        a: Box<Field>,
        b: Box<Field>,
    },
    Intersection {
        a: Box<Field>,
        b: Box<Field>,
    },
    Difference {
        a: Box<Field>,
        b: Box<Field>,
    },
    SmoothUnion {
        a: Box<Field>,
        b: Box<Field>,
        radius: f64,
    },
    Offset {
        input: Box<Field>,
        distance: f64,
    },
    Translate {
        input: Box<Field>,
        vector: Point,
    },
}
impl value_codec::Serialize for Field {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Extrude {
                profile,
                half_height,
            } => {
                let mut object = value_codec::Map::new();
                object.insert("profile".into(), value_codec::Serialize::to_value(profile));
                object.insert(
                    "half_height".into(),
                    value_codec::Serialize::to_value(half_height),
                );
                object.insert("kind".into(), value_codec::Value::String("extrude".into()));
                value_codec::Value::Object(object)
            }
            Self::Revolve { profile } => {
                let mut object = value_codec::Map::new();
                object.insert("profile".into(), value_codec::Serialize::to_value(profile));
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
            Self::Union { a, b } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert("kind".into(), value_codec::Value::String("union".into()));
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
                let profile: geometry_ops::Region2 = value_codec::Deserialize::from_value(
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
                let profile: geometry_ops::Region2 = value_codec::Deserialize::from_value(
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
                let mesh: Mesh = value_codec::Deserialize::from_value(
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
fn finite(p: Point) -> bool {
    p.iter().all(|x| x.is_finite() && x.abs() <= 1e6)
}
fn positive(x: f64) -> bool {
    x.is_finite() && x > 0. && x <= 1e6
}
impl Field {
    pub fn validate(&self) -> Result<()> {
        fn walk(f: &Field, depth: usize, budget: &mut usize) -> bool {
            if depth > 32 || *budget >= 256 {
                return false;
            }
            *budget += 1;
            match f {
                Field::Extrude {
                    profile,
                    half_height,
                } => profile.validate().is_ok() && positive(*half_height),
                Field::Revolve { profile } => {
                    profile.validate().is_ok()
                        && profile
                            .outer
                            .iter()
                            .chain(profile.holes.iter().flatten())
                            .all(|p| p[0] >= 0.)
                }
                Field::Deform { input, deformation } => {
                    matches!(deformation, geometry_ops::Deformation::Twist { .. })
                        && deformation.validate().is_ok()
                        && walk(input, depth + 1, budget)
                }
                Field::MeshDistance { mesh, signed } => {
                    mesh.indices.len() / 3 <= 4096
                        && mesh.inspect().is_ok_and(|r| {
                            r.triangle_count > 0
                                && r.degenerate_triangles == 0
                                && (!signed || r.closed)
                        })
                }
                Field::Sphere { center, radius } => finite(*center) && positive(*radius),
                Field::Box { center, half_size } => {
                    finite(*center) && half_size.iter().all(|&x| positive(x))
                }
                Field::Torus {
                    center,
                    major_radius,
                    minor_radius,
                } => {
                    finite(*center)
                        && positive(*major_radius)
                        && positive(*minor_radius)
                        && minor_radius < major_radius
                }
                Field::Union { a, b }
                | Field::Intersection { a, b }
                | Field::Difference { a, b } => {
                    walk(a, depth + 1, budget) && walk(b, depth + 1, budget)
                }
                Field::SmoothUnion { a, b, radius } => {
                    positive(*radius) && walk(a, depth + 1, budget) && walk(b, depth + 1, budget)
                }
                Field::Offset { input, distance } => {
                    distance.is_finite() && distance.abs() <= 1e6 && walk(input, depth + 1, budget)
                }
                Field::Translate { input, vector } => {
                    finite(*vector) && walk(input, depth + 1, budget)
                }
            }
        }
        if walk(self, 0, &mut 0) && self.sample_work() <= 4096 {
            Ok(())
        } else {
            Err(Error::new("Invalid field or depth/node budget exceeded"))
        }
    }
    fn sample(&self, p: Point) -> f64 {
        match self {
            Self::Extrude {
                profile,
                half_height,
            } => {
                let a = profile.signed_distance([p[0], p[1]]);
                let b = p[2].abs() - half_height;
                a.max(0.).hypot(b.max(0.)) + a.max(b).min(0.)
            }
            Self::Revolve { profile } => profile.signed_distance([p[0].hypot(p[1]), p[2]]),
            Self::Deform { input, deformation } => match deformation.inverse(p) {
                Ok(q) => input.sample(q),
                Err(_) => f64::NAN,
            },
            Self::MeshDistance { mesh, signed } => {
                if *signed {
                    polygon_kernel::proximity::signed_distance(mesh, p)
                } else {
                    polygon_kernel::proximity::closest_point(mesh, p).1
                }
            }
            Self::Sphere { center, radius } => length(sub(p, *center)) - radius,
            Self::Box { center, half_size } => {
                let q: Point = std::array::from_fn(|i| (p[i] - center[i]).abs() - half_size[i]);
                length(q.map(|x| x.max(0.))) + q[0].max(q[1]).max(q[2]).min(0.)
            }
            Self::Torus {
                center,
                major_radius,
                minor_radius,
            } => {
                let q = sub(p, *center);
                (q[0].hypot(q[1]) - major_radius).hypot(q[2]) - minor_radius
            }
            Self::Union { a, b } => a.sample(p).min(b.sample(p)),
            Self::Intersection { a, b } => a.sample(p).max(b.sample(p)),
            Self::Difference { a, b } => a.sample(p).max(-b.sample(p)),
            Self::SmoothUnion { a, b, radius: k } => {
                let a = a.sample(p);
                let b = b.sample(p);
                let h = (0.5 + 0.5 * (b - a) / k).clamp(0., 1.);
                b * (1. - h) + a * h - k * h * (1. - h)
            }
            Self::Offset { input, distance } => input.sample(p) - distance,
            Self::Translate { input, vector } => input.sample(sub(p, *vector)),
        }
    }
    pub fn from_mesh(mesh: &Mesh, signed: bool) -> Result<Self> {
        let field = Self::MeshDistance {
            mesh: polygon_kernel::proximity::valid_source(mesh, 4096)?,
            signed,
        };
        field.validate()?;
        Ok(field)
    }
    fn sample_work(&self) -> usize {
        match self {
            Self::Extrude { profile, .. } | Self::Revolve { profile } => profile.work(),
            Self::MeshDistance { mesh, .. } => mesh.indices.len() / 3,
            Self::Union { a, b }
            | Self::Intersection { a, b }
            | Self::Difference { a, b }
            | Self::SmoothUnion { a, b, .. } => a.sample_work() + b.sample_work(),
            Self::Deform { input, .. }
            | Self::Offset { input, .. }
            | Self::Translate { input, .. } => input.sample_work(),
            _ => 1,
        }
    }
    pub fn evaluate(&self, p: Point) -> Result<f64> {
        self.validate()?;
        if !finite(p) {
            return Err(Error::new("Invalid sample point"));
        }
        let value = self.sample(p);
        if value.is_finite() {
            Ok(value)
        } else {
            Err(Error::new("Field evaluation left its numeric domain"))
        }
    }
}
#[derive(Clone, Debug)]
pub struct Grid {
    pub min: Point,
    pub max: Point,
    pub cells: [usize; 3],
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
/// Also accepts user-defined scalar fields. Sampling is bounded; sub-cell features
/// can be missed. A negative boundary sample is rejected rather than capped.
pub fn polygonize_with(field: impl Fn(Point) -> f64, grid: &Grid) -> Result<Mesh> {
    if !finite(grid.min)
        || !finite(grid.max)
        || grid.cells.iter().any(|&n| n == 0 || n > 64)
        || (0..3).any(|i| grid.max[i] <= grid.min[i])
    {
        return Err(Error::new(
            "Invalid grid: each axis requires 1..64 cells and ordered finite bounds",
        ));
    }
    let [nx, ny, nz] = grid.cells;
    let index = |x: usize, y: usize, z: usize| (z * (ny + 1) + y) * (nx + 1) + x;
    let mut points = Vec::new();
    let mut values = Vec::new();
    for z in 0..=nz {
        for y in 0..=ny {
            for x in 0..=nx {
                let p = std::array::from_fn(|i| {
                    grid.min[i]
                        + (grid.max[i] - grid.min[i]) * [x, y, z][i] as f64 / grid.cells[i] as f64
                });
                let v = field(p);
                let v = if v.abs()
                    <= 32.
                        * f64::EPSILON
                        * (0..3).map(|i| grid.max[i] - grid.min[i]).fold(0., f64::max)
                {
                    0.
                } else {
                    v
                };
                if !v.is_finite() {
                    return Err(Error::new("Field returned a non-finite value"));
                }
                if (x == 0 || y == 0 || z == 0 || x == nx || y == ny || z == nz) && v <= 0. {
                    return Err(Error::new("Surface touches grid boundary; enlarge bounds"));
                }
                points.push(p);
                values.push(v);
            }
        }
    }
    let mut mesh = Mesh {
        positions: Vec::new(),
        indices: Vec::new(),
        uv: None,
    };
    let mut cache = BTreeMap::new();
    for z in 0..nz {
        for y in 0..ny {
            for x in 0..nx {
                let cube = [
                    index(x, y, z),
                    index(x + 1, y, z),
                    index(x + 1, y + 1, z),
                    index(x, y + 1, z),
                    index(x, y, z + 1),
                    index(x + 1, y, z + 1),
                    index(x + 1, y + 1, z + 1),
                    index(x, y + 1, z + 1),
                ];
                for tet in [
                    [0, 1, 2, 6],
                    [0, 2, 3, 6],
                    [0, 3, 7, 6],
                    [0, 7, 4, 6],
                    [0, 4, 5, 6],
                    [0, 5, 1, 6],
                ] {
                    let t = tet.map(|i| cube[i]);
                    let mut polygon = Vec::new();
                    let mut outward = [0.; 3];
                    let mut ni = 0.;
                    let mut no = 0.;
                    let mut inside = [0.; 3];
                    let mut outside = [0.; 3];
                    for &i in &t {
                        if values[i] < 0. {
                            ni += 1.;
                            for (k, v) in inside.iter_mut().enumerate() {
                                *v += points[i][k];
                            }
                        } else {
                            no += 1.;
                            for (k, v) in outside.iter_mut().enumerate() {
                                *v += points[i][k];
                            }
                        }
                    }
                    if ni == 0. || no == 0. {
                        continue;
                    }
                    for k in 0..3 {
                        outward[k] = outside[k] / no - inside[k] / ni;
                    }
                    for [a, b] in [[0, 1], [0, 2], [0, 3], [1, 2], [1, 3], [2, 3]] {
                        let a = t[a];
                        let b = t[b];
                        if (values[a] < 0.) == (values[b] < 0.) {
                            continue;
                        }
                        let key = if values[a] == 0. {
                            (a, a)
                        } else if values[b] == 0. {
                            (b, b)
                        } else {
                            (a.min(b), a.max(b))
                        };
                        let id = *cache.entry(key).or_insert_with(|| {
                            let ratio = values[a] / (values[a] - values[b]);
                            let id = mesh.positions.len() / 3;
                            mesh.positions.extend(
                                points[a]
                                    .iter()
                                    .zip(points[b])
                                    .map(|(&a, b)| a + ratio * (b - a)),
                            );
                            id
                        });
                        if !polygon.contains(&id) {
                            polygon.push(id);
                        }
                    }
                    if polygon.len() < 3 {
                        continue;
                    }
                    let point = |i: usize| {
                        [
                            mesh.positions[i * 3],
                            mesh.positions[i * 3 + 1],
                            mesh.positions[i * 3 + 2],
                        ]
                    };
                    let mut center = [0.; 3];
                    for &i in &polygon {
                        for (k, v) in center.iter_mut().enumerate() {
                            *v += point(i)[k] / polygon.len() as f64;
                        }
                    }
                    let u = sub(point(polygon[0]), center);
                    let v = cross(outward, u);
                    polygon.sort_by(|&a, &b| {
                        let a = sub(point(a), center);
                        let b = sub(point(b), center);
                        dot(a, v)
                            .atan2(dot(a, u))
                            .total_cmp(&dot(b, v).atan2(dot(b, u)))
                    });
                    for i in 1..polygon.len() - 1 {
                        let a = polygon[0];
                        let mut b = polygon[i];
                        let mut c = polygon[i + 1];
                        let normal = cross(sub(point(b), point(a)), sub(point(c), point(a)));
                        if length(normal) == 0. {
                            continue;
                        }
                        if dot(normal, outward) < 0. {
                            std::mem::swap(&mut b, &mut c);
                        }
                        mesh.indices.extend([a, b, c]);
                    }
                    if mesh.indices.len() / 3 > 100_000 {
                        return Err(Error::new("Implicit mesh exceeds 100000 triangles"));
                    }
                }
            }
        }
    }
    mesh.validate()?;
    Ok(mesh)
}
pub fn polygonize(field: &Field, grid: &Grid) -> Result<Mesh> {
    field.validate()?;
    if grid.cells.iter().any(|&n| n > 64)
        || grid
            .cells
            .iter()
            .map(|n| n + 1)
            .product::<usize>()
            .saturating_mul(field.sample_work())
            > 8_000_000
    {
        return Err(Error::new(
            "SDF extraction exceeds 8000000 sample work units",
        ));
    }
    polygonize_with(|p| field.sample(p), grid)
}

impl Field {
    pub fn deform(&self, deformation: geometry_ops::Deformation) -> Result<Self> {
        self.validate()?;
        let out = Self::Deform {
            input: Box::new(self.clone()),
            deformation,
        };
        out.validate()?;
        Ok(out)
    }
    pub fn sculpt_sphere(&self, center: Point, radius: f64, remove: bool) -> Result<Self> {
        self.validate()?;
        let sphere = Self::Sphere { center, radius };
        sphere.validate()?;
        let out = if remove {
            Self::Difference {
                a: Box::new(self.clone()),
                b: Box::new(sphere),
            }
        } else {
            Self::Union {
                a: Box::new(self.clone()),
                b: Box::new(sphere),
            }
        };
        out.validate()?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sphere() -> Field {
        Field::Sphere {
            center: [0.; 3],
            radius: 1.,
        }
    }
    fn grid(n: usize) -> Grid {
        Grid {
            min: [-1.5; 3],
            max: [1.5; 3],
            cells: [n; 3],
        }
    }
    #[test]
    fn primitives_and_csg() {
        assert_eq!(sphere().evaluate([0.; 3]).unwrap(), -1.);
        let f = Field::Difference {
            a: Box::new(sphere()),
            b: Box::new(Field::Sphere {
                center: [0.; 3],
                radius: 0.5,
            }),
        };
        assert_eq!(f.evaluate([0.; 3]).unwrap(), 0.5);
        assert!(f.evaluate([0.75, 0., 0.]).unwrap() < 0.);
    }
    #[test]
    fn closed_outward_sphere_and_convergence() {
        let coarse = polygonize(&sphere(), &grid(8)).unwrap().inspect().unwrap();
        let fine = polygonize(&sphere(), &grid(16)).unwrap().inspect().unwrap();
        assert!(fine.closed);
        assert_eq!(fine.orientation_conflicts, 0);
        assert_eq!(fine.degenerate_triangles, 0);
        let exact = 4. * std::f64::consts::PI / 3.;
        assert!((fine.signed_volume_mm3 - exact).abs() < (coarse.signed_volume_mm3 - exact).abs());
        assert!((fine.signed_volume_mm3 - exact).abs() < 0.1);
    }
    #[test]
    fn exact_grid_vertices_and_empty() {
        let mut g = grid(12);
        g.min = [-2.; 3];
        g.max = [2.; 3];
        assert!(polygonize(&sphere(), &g).unwrap().inspect().unwrap().closed);
        assert!(polygonize_with(|_| 1., &g).unwrap().indices.is_empty());
        assert!(polygonize_with(|_| f64::NAN, &g).is_err());
        assert!(polygonize_with(|_| -1., &g).is_err());
    }
}
