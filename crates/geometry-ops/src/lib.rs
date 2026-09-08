//! Representation-independent point mappings and brush falloffs. Geometry
//! ownership, topology updates and validity checks belong to each consuming kernel.
pub type Point = [f64; 3];
pub type Result<T> = std::result::Result<T, String>;
fn finite(p: Point) -> bool {
    p.iter().all(|v| v.is_finite() && v.abs() <= 1e6)
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
impl value_codec::Serialize for Deformation {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Twist {
                origin,
                radians_per_unit,
            } => {
                let mut object = value_codec::Map::new();
                object.insert("origin".into(), value_codec::Serialize::to_value(origin));
                object.insert(
                    "radians_per_unit".into(),
                    value_codec::Serialize::to_value(radians_per_unit),
                );
                object.insert("kind".into(), value_codec::Value::String("twist".into()));
                value_codec::Value::Object(object)
            }
            Self::Bend { origin, radius } => {
                let mut object = value_codec::Map::new();
                object.insert("origin".into(), value_codec::Serialize::to_value(origin));
                object.insert("radius".into(), value_codec::Serialize::to_value(radius));
                object.insert("kind".into(), value_codec::Value::String("bend".into()));
                value_codec::Value::Object(object)
            }
            Self::Lattice { min, max, controls } => {
                let mut object = value_codec::Map::new();
                object.insert("min".into(), value_codec::Serialize::to_value(min));
                object.insert("max".into(), value_codec::Serialize::to_value(max));
                object.insert(
                    "controls".into(),
                    value_codec::Serialize::to_value(controls),
                );
                object.insert("kind".into(), value_codec::Value::String("lattice".into()));
                value_codec::Value::Object(object)
            }
        }
    }
}
impl<'de> value_codec::Deserialize<'de> for Deformation {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        match value["kind"].as_str().unwrap_or("") {
            "twist" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let origin: Point = value_codec::Deserialize::from_value(
                    object
                        .remove("origin")
                        .ok_or_else(|| value_codec::error("Missing field origin"))?,
                )?;
                let radians_per_unit: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("radians_per_unit")
                        .ok_or_else(|| value_codec::error("Missing field radians_per_unit"))?,
                )?;
                Ok(Self::Twist {
                    origin,
                    radians_per_unit,
                })
            }
            "bend" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let origin: Point = value_codec::Deserialize::from_value(
                    object
                        .remove("origin")
                        .ok_or_else(|| value_codec::error("Missing field origin"))?,
                )?;
                let radius: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("radius")
                        .ok_or_else(|| value_codec::error("Missing field radius"))?,
                )?;
                Ok(Self::Bend { origin, radius })
            }
            "lattice" => {
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
                let controls: Box<[Point; 8]> = value_codec::Deserialize::from_value(
                    object
                        .remove("controls")
                        .ok_or_else(|| value_codec::error("Missing field controls"))?,
                )?;
                Ok(Self::Lattice { min, max, controls })
            }
            _ => Err(value_codec::error("Unknown enum variant")),
        }
    }
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
            Err("Invalid deformation parameters".into())
        }
    }
    pub fn apply(&self, p: Point) -> Result<Point> {
        self.validate()?;
        if !finite(p) {
            return Err("Invalid deformation point".into());
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
                    return Err("Bend leaves its nonfolding domain".into());
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
                    return Err("Point lies outside deformation lattice".into());
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
            Err("Deformation exceeds coordinate limits".into())
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
            _ => Err("This deformation has no supported global inverse for implicit fields".into()),
        }
    }
}
#[derive(Clone, Debug)]
pub struct Brush {
    pub center: Point,
    pub radius: f64,
    pub displacement: Point,
}
impl value_codec::Serialize for Brush {
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
        object.insert(
            "displacement".into(),
            value_codec::Serialize::to_value(&self.displacement),
        );
        value_codec::Value::Object(object)
    }
}
impl<'de> value_codec::Deserialize<'de> for Brush {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
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
        let displacement: Point = value_codec::Deserialize::from_value(
            object
                .remove("displacement")
                .ok_or_else(|| value_codec::error("Missing field displacement"))?,
        )?;
        Ok(Self {
            center,
            radius,
            displacement,
        })
    }
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
            Err("Invalid brush".into())
        }
    }
    pub fn apply(&self, p: Point) -> Result<Point> {
        self.validate()?;
        if !finite(p) {
            return Err("Invalid brush point".into());
        }
        let d = p
            .iter()
            .zip(self.center)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt()
            / self.radius;
        let w = if d >= 1. {
            0.
        } else {
            let t = 1. - d;
            t * t * (3. - 2. * t)
        };
        let q = std::array::from_fn(|k| p[k] + w * self.displacement[k]);
        if finite(q) {
            Ok(q)
        } else {
            Err("Brush exceeds coordinate limits".into())
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
}

/// Common 2D region representation, without a triangulation or geometry kernel.
#[derive(Clone, Debug)]
pub struct Region2 {
    pub outer: Vec<[f64; 2]>,
    pub holes: Vec<Vec<[f64; 2]>>,
}
impl value_codec::Serialize for Region2 {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
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
impl<'de> value_codec::Deserialize<'de> for Region2 {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
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
        Ok(Self { outer, holes })
    }
}
fn area2(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
fn inside2(p: [f64; 2], ring: &[[f64; 2]]) -> bool {
    let mut inside = false;
    for i in 0..ring.len() {
        let a = ring[i];
        let b = ring[(i + 1) % ring.len()];
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
    }
    inside
}
impl Region2 {
    pub fn validate(&self) -> Result<()> {
        let loops: Vec<_> = std::iter::once(&self.outer).chain(&self.holes).collect();
        if loops.len() > 17
            || loops.iter().map(|l| l.len()).sum::<usize>() > 512
            || loops
                .iter()
                .any(|l| l.len() < 3 || l.iter().flatten().any(|v| !v.is_finite() || v.abs() > 1e6))
        {
            return Err("Invalid planar region".into());
        }
        let mut edges = Vec::new();
        for (li, l) in loops.iter().enumerate() {
            let area = (1..l.len() - 1)
                .map(|i| area2(l[0], l[i], l[i + 1]))
                .sum::<f64>();
            if area.abs() < 1e-14 {
                return Err("Zero-area profile loop".into());
            }
            for i in 0..l.len() {
                let a = l[i];
                let b = l[(i + 1) % l.len()];
                if a == b {
                    return Err("Repeated profile vertex".into());
                }
                edges.push((li, i, a, b));
            }
        }
        let on = |p: [f64; 2], a: [f64; 2], b: [f64; 2]| {
            area2(a, b, p).abs() <= 1e-12
                && p[0] >= a[0].min(b[0]) - 1e-12
                && p[0] <= a[0].max(b[0]) + 1e-12
                && p[1] >= a[1].min(b[1]) - 1e-12
                && p[1] <= a[1].max(b[1]) + 1e-12
        };
        for i in 0..edges.len() {
            for j in i + 1..edges.len() {
                let (li, ai, a, b) = edges[i];
                let (lj, bi, c, d) = edges[j];
                if li == lj
                    && ((ai + 1) % loops[li].len() == bi || (bi + 1) % loops[li].len() == ai)
                {
                    continue;
                }
                if on(a, c, d)
                    || on(b, c, d)
                    || on(c, a, b)
                    || on(d, a, b)
                    || (area2(a, b, c) > 0.) != (area2(a, b, d) > 0.)
                        && (area2(c, d, a) > 0.) != (area2(c, d, b) > 0.)
                {
                    return Err("Profile loops intersect or touch".into());
                }
            }
        }
        for (i, h) in self.holes.iter().enumerate() {
            if !inside2(h[0], &self.outer)
                || self
                    .holes
                    .iter()
                    .enumerate()
                    .any(|(j, other)| i != j && inside2(h[0], other))
            {
                return Err("Invalid hole containment".into());
            }
        }
        Ok(())
    }
    pub fn signed_distance(&self, p: [f64; 2]) -> f64 {
        let mut distance = f64::INFINITY;
        for l in std::iter::once(&self.outer).chain(&self.holes) {
            for i in 0..l.len() {
                let a = l[i];
                let b = l[(i + 1) % l.len()];
                let v = [b[0] - a[0], b[1] - a[1]];
                let t = (((p[0] - a[0]) * v[0] + (p[1] - a[1]) * v[1])
                    / (v[0] * v[0] + v[1] * v[1]))
                    .clamp(0., 1.);
                distance = distance.min((p[0] - a[0] - t * v[0]).hypot(p[1] - a[1] - t * v[1]));
            }
        }
        if inside2(p, &self.outer) && !self.holes.iter().any(|h| inside2(p, h)) {
            -distance
        } else {
            distance
        }
    }
    pub fn work(&self) -> usize {
        self.outer.len() + self.holes.iter().map(Vec::len).sum::<usize>()
    }
}
