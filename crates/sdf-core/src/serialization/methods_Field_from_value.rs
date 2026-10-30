//! serialization operations: from_value.
use super::*;
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
            "cylinder" => {
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
                let half_height: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("half_height")
                        .ok_or_else(|| value_codec::error("Missing field half_height"))?,
                )?;
                Ok(Self::Cylinder {
                    center,
                    radius,
                    half_height,
                })
            }
            "gyroid" | "schwarz_p" | "diamond" | "neovius" | "lidinoid" | "schoen_iwp" => {
                let kind = value["kind"].as_str().unwrap_or("gyroid").to_owned();
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let origin: Point = value_codec::Deserialize::from_value(
                    object
                        .remove("origin")
                        .ok_or_else(|| value_codec::error("Missing field origin"))?,
                )?;
                let period: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("period")
                        .ok_or_else(|| value_codec::error("Missing field period"))?,
                )?;
                let thickness: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("thickness")
                        .ok_or_else(|| value_codec::error("Missing field thickness"))?,
                )?;
                Ok(match kind.as_str() {
                    "gyroid" => Self::Gyroid {
                        origin,
                        period,
                        thickness,
                    },
                    "schwarz_p" => Self::SchwarzP {
                        origin,
                        period,
                        thickness,
                    },
                    "diamond" => Self::Diamond {
                        origin,
                        period,
                        thickness,
                    },
                    "neovius" => Self::Neovius {
                        origin,
                        period,
                        thickness,
                    },
                    "lidinoid" => Self::Lidinoid {
                        origin,
                        period,
                        thickness,
                    },
                    _ => Self::SchoenIWP {
                        origin,
                        period,
                        thickness,
                    },
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
            "octahedron" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let center: Point = value_codec::Deserialize::from_value(
                    object
                        .remove("center")
                        .ok_or_else(|| value_codec::error("Missing field center"))?,
                )?;
                let size: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("size")
                        .ok_or_else(|| value_codec::error("Missing field size"))?,
                )?;
                Ok(Self::Octahedron { center, size })
            }
            "hex_prism" | "triangular_prism" => {
                let kind = value["kind"].as_str().unwrap_or("hex_prism").to_owned();
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
                let half_height: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("half_height")
                        .ok_or_else(|| value_codec::error("Missing field half_height"))?,
                )?;
                Ok(if kind == "hex_prism" {
                    Self::HexPrism {
                        center,
                        radius,
                        half_height,
                    }
                } else {
                    Self::TriangularPrism {
                        center,
                        radius,
                        half_height,
                    }
                })
            }
            "smooth_intersection" => {
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
                Ok(Self::SmoothIntersection { a, b, radius })
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
            "chamfer_union" | "chamfer_intersection" | "chamfer_difference" => {
                let kind = value["kind"].as_str().unwrap_or("chamfer_union").to_owned();
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
                Ok(match kind.as_str() {
                    "chamfer_union" => Self::ChamferUnion { a, b, radius },
                    "chamfer_intersection" => Self::ChamferIntersection { a, b, radius },
                    _ => Self::ChamferDifference { a, b, radius },
                })
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
            "onion" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let input: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("input")
                        .ok_or_else(|| value_codec::error("Missing field input"))?,
                )?;
                let thickness: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("thickness")
                        .ok_or_else(|| value_codec::error("Missing field thickness"))?,
                )?;
                Ok(Self::Onion { input, thickness })
            }
            "elongate" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let input: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("input")
                        .ok_or_else(|| value_codec::error("Missing field input"))?,
                )?;
                let half_extents: Point = value_codec::Deserialize::from_value(
                    object
                        .remove("half_extents")
                        .ok_or_else(|| value_codec::error("Missing field half_extents"))?,
                )?;
                Ok(Self::Elongate {
                    input,
                    half_extents,
                })
            }
            "mirror" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let input: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("input")
                        .ok_or_else(|| value_codec::error("Missing field input"))?,
                )?;
                let axis: [bool; 3] = value_codec::Deserialize::from_value(
                    object
                        .remove("axis")
                        .ok_or_else(|| value_codec::error("Missing field axis"))?,
                )?;
                Ok(Self::Mirror { input, axis })
            }
            "taper" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let input: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("input")
                        .ok_or_else(|| value_codec::error("Missing field input"))?,
                )?;
                let height: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("height")
                        .ok_or_else(|| value_codec::error("Missing field height"))?,
                )?;
                let factor: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("factor")
                        .ok_or_else(|| value_codec::error("Missing field factor"))?,
                )?;
                Ok(Self::Taper {
                    input,
                    height,
                    factor,
                })
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
