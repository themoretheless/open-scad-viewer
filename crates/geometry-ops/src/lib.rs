//! Representation-independent point mappings and brush falloffs. Geometry
//! ownership, topology updates and validity checks belong to each consuming kernel.
#[cfg(feature = "codec")]
mod serialization;
pub use math_core::{Error, Result, V3, finite};
pub type Point = V3;
#[cfg(feature = "codec")]
mod codec;
pub mod sculpt;
pub mod keypoints;
pub mod transparency;
pub mod solid_program;
pub mod program_graph;
pub mod profile_program;
pub mod extrude_slices;
pub mod fragment_resolution;
pub mod revolution;
pub mod resize;
pub mod polygon_mesh;
pub use sculpt::{Falloff, SculptBrush, SculptKind, SculptTarget, Symmetry, unit_or_zero};
const INVALID_INPUT: &str = "GEOMETRY_INVALID_INPUT";
pub(crate) fn fail(message: impl Into<String>) -> Error {
    Error::new(INVALID_INPUT, message)
}
#[derive(Clone, Debug)]
pub enum Deformation {
    Twist {
        origin: Point,
        radians_per_unit: f64,
    },
    Bend {
        origin: Point,
        radius: f64,
    },
    Lattice {
        min: Point,
        max: Point,
        controls: Box<[Point; 8]>,
    },
}

impl Deformation {
    pub fn validate(&self) -> Result<()> {
        let valid = match self {
            Self::Twist {
                origin,
                radians_per_unit,
            } => finite(*origin) && radians_per_unit.is_finite() && radians_per_unit.abs() <= 1e3,
            Self::Bend { origin, radius } => {
                finite(*origin) && radius.is_finite() && *radius > 1e-8 && *radius <= 1e6
            }
            Self::Lattice { min, max, controls } => {
                finite(*min)
                    && finite(*max)
                    && (0..3).all(|k| max[k] > min[k])
                    && controls.iter().all(|&p| finite(p))
            }
        };
        if valid {
            Ok(())
        } else {
            Err(fail("Invalid deformation parameters"))
        }
    }
    pub fn apply(&self, p: Point) -> Result<Point> {
        self.validate()?;
        if !finite(p) {
            return Err(fail("Invalid deformation point"));
        }
        let q = match self {
            Self::Twist {
                origin,
                radians_per_unit,
            } => {
                let [x, y, z] = std::array::from_fn(|k| p[k] - origin[k]);
                let a = z * radians_per_unit;
                [
                    origin[0] + x * a.cos() - y * a.sin(),
                    origin[1] + x * a.sin() + y * a.cos(),
                    p[2],
                ]
            }
            Self::Bend { origin, radius: r } => {
                let y = p[1] - origin[1];
                let z = p[2] - origin[2];
                if r + y <= 0. || (z / r).abs() >= std::f64::consts::PI {
                    return Err(fail("Bend leaves its nonfolding domain"));
                }
                let a = z / r;
                [
                    p[0],
                    origin[1] + (r + y) * a.cos() - r,
                    origin[2] + (r + y) * a.sin(),
                ]
            }
            Self::Lattice { min, max, controls } => {
                let t: Point = std::array::from_fn(|k| (p[k] - min[k]) / (max[k] - min[k]));
                if t.iter().any(|&v| !(-1e-12..=1. + 1e-12).contains(&v)) {
                    return Err(fail("Point lies outside deformation lattice"));
                }
                let mut q = [0.; 3];
                for (i, c) in controls.iter().enumerate() {
                    let w = (0..3)
                        .map(|k| if i & (1 << k) != 0 { t[k] } else { 1. - t[k] })
                        .product::<f64>();
                    for k in 0..3 {
                        q[k] += w * c[k];
                    }
                }
                q
            }
        };
        if finite(q) {
            Ok(q)
        } else {
            Err(fail("Deformation exceeds coordinate limits"))
        }
    }
    /// Twist has a global analytic inverse. Other fields must explicitly implement
    /// an inverse with domain checks rather than reuse a forward point mapping.
    pub fn inverse(&self, p: Point) -> Result<Point> {
        match self {
            Self::Twist {
                origin,
                radians_per_unit,
            } => Self::Twist {
                origin: *origin,
                radians_per_unit: -radians_per_unit,
            }
            .apply(p),
            _ => Err(fail(
                "This deformation has no supported global inverse for implicit fields",
            )),
        }
    }
}
#[derive(Clone, Debug)]
pub struct Brush {
    pub center: Point,
    pub radius: f64,
    pub displacement: Point,
}

impl Brush {
    pub fn validate(&self) -> Result<()> {
        if finite(self.center)
            && finite(self.displacement)
            && self.radius.is_finite()
            && self.radius > 0.
            && self.radius <= 1e6
        {
            Ok(())
        } else {
            Err(fail("Invalid brush"))
        }
    }
    pub fn apply(&self, p: Point) -> Result<Point> {
        self.validate()?;
        if !finite(p) {
            return Err(fail("Invalid brush point"));
        }
        let w =
            Falloff::Smooth.weight(math_core::norm(math_core::sub(p, self.center)) / self.radius);
        let q = std::array::from_fn(|k| p[k] + w * self.displacement[k]);
        if finite(q) {
            Ok(q)
        } else {
            Err(fail("Brush exceeds coordinate limits"))
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn twist_roundtrip() {
        let d = Deformation::Twist {
            origin: [1., 2., 3.],
            radians_per_unit: 0.7,
        };
        let p = [2., 4., 5.];
        let q = d.inverse(d.apply(p).unwrap()).unwrap();
        for i in 0..3 {
            assert!((p[i] - q[i]).abs() < 1e-12);
        }
    }
    #[test]
    fn identity_lattice_and_brush_boundary() {
        let c = std::array::from_fn(|i| {
            std::array::from_fn(|k| if i & (1 << k) == 0 { 0. } else { 1. })
        });
        let d = Deformation::Lattice {
            min: [0.; 3],
            max: [1.; 3],
            controls: Box::new(c),
        };
        assert_eq!(d.apply([0.5; 3]).unwrap(), [0.5; 3]);
        let b = Brush {
            center: [0.; 3],
            radius: 1.,
            displacement: [0., 0., 2.],
        };
        assert_eq!(b.apply([0.; 3]).unwrap(), [0., 0., 2.]);
        assert_eq!(b.apply([1., 0., 0.]).unwrap(), [1., 0., 0.]);
    }
    #[test]
    fn brush_falloff_is_smoothstep_monotone_and_isotropic() {
        let b = Brush {
            center: [1., 2., 3.],
            radius: 2.,
            displacement: [0., 0., 4.],
        };
        // Half radius: t = 0.5, w = 0.25 * (3 - 1) = 0.5.
        let q = b.apply([2., 2., 3.]).unwrap();
        assert!((q[2] - 5.).abs() < 1e-12 && q[0] == 2. && q[1] == 2.);
        let mut last = f64::INFINITY;
        for i in 0..=20 {
            let d = i as f64 / 10.;
            let w = b.apply([1. + d, 2., 3.]).unwrap()[2] - 3.;
            assert!(w <= last + 1e-12, "falloff must not increase with distance");
            last = w;
        }
        for axis in 0..3 {
            let mut p = b.center;
            p[axis] += 1.;
            let q = b.apply(p).unwrap();
            assert!(
                (q[2] - p[2] - 2.).abs() < 1e-12,
                "same weight along every axis"
            );
        }
        assert_eq!(b.apply([3., 2., 3.]).unwrap(), [3., 2., 3.]);
        assert_eq!(b.apply([10., 2., 3.]).unwrap(), [10., 2., 3.]);
    }
    #[test]
    fn brush_rejects_invalid_parameters_and_points() {
        let ok = Brush {
            center: [0.; 3],
            radius: 1.,
            displacement: [1., 0., 0.],
        };
        assert!(ok.validate().is_ok());
        for radius in [0., -1., f64::NAN, f64::INFINITY, 1e6 + 1.] {
            assert!(
                Brush {
                    radius,
                    ..ok.clone()
                }
                .validate()
                .is_err(),
                "{radius}"
            );
        }
        assert!(
            Brush {
                radius: 1e6,
                ..ok.clone()
            }
            .validate()
            .is_ok()
        );
        assert!(
            Brush {
                center: [f64::NAN, 0., 0.],
                ..ok.clone()
            }
            .validate()
            .is_err()
        );
        assert!(
            Brush {
                displacement: [0., f64::INFINITY, 0.],
                ..ok.clone()
            }
            .validate()
            .is_err()
        );
        assert!(ok.apply([f64::NAN, 0., 0.]).is_err());
        // Coordinates are bounded by 1e6: out-of-range inputs and results are rejected.
        assert!(
            Brush {
                center: [1e6 + 1., 0., 0.],
                ..ok.clone()
            }
            .validate()
            .is_err()
        );
        assert!(
            Brush {
                displacement: [1e6 + 1., 0., 0.],
                ..ok.clone()
            }
            .validate()
            .is_err()
        );
        assert!(ok.apply([1e6 + 1., 0., 0.]).is_err());
        let edge = Brush {
            center: [1e6, 0., 0.],
            displacement: [1., 0., 0.],
            radius: 1.,
        };
        assert!(edge.validate().is_ok());
        assert!(
            edge.apply([1e6, 0., 0.]).is_err(),
            "result exceeds coordinate limits"
        );
        assert_eq!(edge.apply([1e6 - 1., 0., 0.]).unwrap(), [1e6 - 1., 0., 0.]);
    }
    #[cfg(feature = "codec")]
    #[test]
    fn brush_round_trips_through_value_codec() {
        let b = Brush {
            center: [1., 2., 3.],
            radius: 4.,
            displacement: [5., 6., 7.],
        };
        let v = value_codec::to_value(b.clone()).unwrap();
        let back: Brush = value_codec::from_value(v).unwrap();
        assert_eq!(back.center, b.center);
        assert_eq!(back.radius, b.radius);
        assert_eq!(back.displacement, b.displacement);
        let mut missing = value_codec::Map::new();
        missing.insert("center".into(), value_codec::to_value(b.center).unwrap());
        assert!(value_codec::from_value::<Brush>(value_codec::Value::Object(missing)).is_err());
    }
}

/// Neutral triangle buffer. Not a mesh kernel: no validate/inspect/boolean.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Triangles {
    pub positions: Vec<f64>,
    pub indices: Vec<usize>,
}

fn vunit(a: Point) -> Result<Point> {
    let n = math_core::norm(a);
    if n <= 1e-12 {
        return Err(fail("Direction is zero or numerically singular"));
    }
    Ok(math_core::scale(a, 1. / n))
}

/// Rotation-minimizing frames on a polyline; 180-degree reversals are rejected.
pub fn sweep_sections(profile: &[[f64; 2]], path: &[Point], up: Point) -> Result<Vec<Vec<Point>>> {
    if !(2..=64).contains(&path.len()) || path.iter().flatten().any(|v| !v.is_finite()) {
        return Err(fail("Sweep requires 2..64 finite path points"));
    }
    let tangents: Vec<Point> = path
        .array_windows()
        .map(|[a, b]| vunit(math_core::sub(*b, *a)))
        .collect::<Result<_>>()?;
    let mut normal = vunit(math_core::sub(
        up,
        tangents[0].map(|v| v * math_core::dot(up, tangents[0])),
    ))?;
    let mut previous = tangents[0];
    let mut sections = Vec::new();
    for i in 0..path.len() {
        let tangent = if i == 0 {
            tangents[0]
        } else if i == path.len() - 1 {
            *tangents.last().unwrap()
        } else {
            vunit(std::array::from_fn(|k| tangents[i - 1][k] + tangents[i][k]))?
        };
        let axis = math_core::cross(previous, tangent);
        let sine = math_core::norm(axis);
        let cosine = math_core::dot(previous, tangent).clamp(-1., 1.);
        if cosine <= -1. + 1e-9 {
            return Err(fail("Sweep path reverses direction"));
        }
        if sine > 1e-12 {
            let axis = axis.map(|v| v / sine);
            let b = math_core::cross(axis, normal);
            let d = math_core::dot(axis, normal);
            normal = std::array::from_fn(|k| {
                normal[k] * cosine + b[k] * sine + axis[k] * d * (1. - cosine)
            });
        }
        normal = vunit(math_core::sub(
            normal,
            tangent.map(|v| v * math_core::dot(normal, tangent)),
        ))?;
        let binormal = math_core::cross(tangent, normal);
        sections.push(
            profile
                .iter()
                .map(|p| {
                    std::array::from_fn(|k| path[i][k] + p[0] * normal[k] + p[1] * binormal[k])
                })
                .collect(),
        );
        previous = tangent;
    }
    Ok(sections)
}

pub mod force_markers;

pub mod point_transform;

pub mod path_sampling;
