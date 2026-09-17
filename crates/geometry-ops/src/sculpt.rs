//! Representation-independent sculpt brushes. The caller supplies vertex
//! positions together with per-vertex unit normals and one-ring adjacency; the
//! engine returns displaced positions and never touches topology. Kernels own
//! normal/adjacency construction and post-edit validity checks.
use crate::{Point, Result, fail, finite};
use math_core::{add, dot, norm, scale, sub, unit};
use value_codec::{Deserialize, Map, Serialize, Value, error};

/// Radial weight profile over the normalized distance `d = |p - center| / radius`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Falloff {
    /// Hermite smoothstep of `1 - d` (the historical brush profile).
    Smooth,
    Linear,
    /// `(1 - d)^2`: tight core, long tail.
    Sharp,
    /// `sqrt(1 - d)`: wide plateau, abrupt edge.
    Root,
    /// Hemisphere `sqrt(1 - d^2)`.
    Sphere,
    /// Full weight everywhere inside the radius.
    Constant,
}
impl Falloff {
    pub const ALL: [Self; 6] = [
        Self::Smooth,
        Self::Linear,
        Self::Sharp,
        Self::Root,
        Self::Sphere,
        Self::Constant,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Smooth => "smooth",
            Self::Linear => "linear",
            Self::Sharp => "sharp",
            Self::Root => "root",
            Self::Sphere => "sphere",
            Self::Constant => "constant",
        }
    }
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.name() == name)
    }
    /// Weight for normalized distance `d`; zero at and beyond the radius, one at the center.
    pub fn weight(self, d: f64) -> f64 {
        if d.is_nan() || !(0. ..1.).contains(&d) {
            return 0.;
        }
        let t = 1. - d;
        match self {
            Self::Smooth => t * t * (3. - 2. * t),
            Self::Linear => t,
            Self::Sharp => t * t,
            Self::Root => t.sqrt(),
            Self::Sphere => (1. - d * d).sqrt(),
            Self::Constant => 1.,
        }
    }
}
impl Serialize for Falloff {
    fn to_value(&self) -> Value {
        Value::String(self.name().into())
    }
}
impl<'de> Deserialize<'de> for Falloff {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        value
            .as_str()
            .and_then(Self::parse)
            .ok_or_else(|| error("Unknown falloff"))
    }
}

/// Mirror the stroke across axis-aligned planes through `origin`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Symmetry {
    pub axes: [bool; 3],
    pub origin: Point,
}
impl Default for Symmetry {
    fn default() -> Self {
        Self {
            axes: [false; 3],
            origin: [0.; 3],
        }
    }
}
impl Symmetry {
    pub fn validate(&self) -> Result<()> {
        if finite(self.origin) {
            Ok(())
        } else {
            Err(fail("Invalid symmetry origin"))
        }
    }
    fn mirror_point(&self, mask: [bool; 3], p: Point) -> Point {
        std::array::from_fn(|k| {
            if mask[k] {
                2. * self.origin[k] - p[k]
            } else {
                p[k]
            }
        })
    }
    fn mirror_vector(mask: [bool; 3], v: Point) -> Point {
        std::array::from_fn(|k| if mask[k] { -v[k] } else { v[k] })
    }
    /// Every distinct mirrored (center, vector) pair, identity first. Centers
    /// lying on a mirror plane are not duplicated.
    fn passes(&self, center: Point, vector: Point) -> Vec<(Point, Point)> {
        let mut out: Vec<(Point, Point)> = Vec::new();
        for bits in 0..8u8 {
            let mask = [bits & 1 != 0, bits & 2 != 0, bits & 4 != 0];
            if (0..3).any(|k| mask[k] && !self.axes[k]) {
                continue;
            }
            let c = self.mirror_point(mask, center);
            let v = Self::mirror_vector(mask, vector);
            if !out.iter().any(|(pc, _)| *pc == c) {
                out.push((c, v));
            }
        }
        out
    }
}
impl Serialize for Symmetry {
    fn to_value(&self) -> Value {
        let mut object = Map::new();
        object.insert("axes".into(), self.axes.to_value());
        object.insert("origin".into(), self.origin.to_value());
        Value::Object(object)
    }
}
impl<'de> Deserialize<'de> for Symmetry {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| error("Expected object"))?
            .clone();
        let axes: [bool; 3] = match object.remove("axes") {
            Some(v) => Deserialize::from_value(v)?,
            None => [false; 3],
        };
        let origin: Point = match object.remove("origin") {
            Some(v) => Deserialize::from_value(v)?,
            None => [0.; 3],
        };
        Ok(Self { axes, origin })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SculptKind {
    /// Translate by a fixed vector (the classic displacement brush).
    Grab { displacement: Point },
    /// Push along the area normal (weighted mean of affected vertex normals) by `strength`.
    Draw { strength: f64 },
    /// Push every vertex along its own normal by `strength`.
    Inflate { strength: f64 },
    /// Move each vertex toward the mean of its one-ring by fraction `strength` in `[0, 1]`.
    Smooth { strength: f64 },
    /// Project toward the weighted best-fit plane through the area centroid by fraction `strength`.
    Flatten { strength: f64 },
    /// Pull toward the brush axis, tangentially to the area normal, by fraction `strength`.
    Pinch { strength: f64 },
}
impl SculptKind {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Grab { .. } => "grab",
            Self::Draw { .. } => "draw",
            Self::Inflate { .. } => "inflate",
            Self::Smooth { .. } => "smooth",
            Self::Flatten { .. } => "flatten",
            Self::Pinch { .. } => "pinch",
        }
    }
    /// Whether this brush needs per-vertex normals from the caller.
    pub fn needs_normals(&self) -> bool {
        !matches!(self, Self::Grab { .. } | Self::Smooth { .. })
    }
    /// Whether this brush needs one-ring adjacency from the caller.
    pub fn needs_adjacency(&self) -> bool {
        matches!(self, Self::Smooth { .. })
    }
    pub fn validate(&self) -> Result<()> {
        let ok = match self {
            Self::Grab { displacement } => finite(*displacement),
            Self::Draw { strength } | Self::Inflate { strength } => {
                strength.is_finite() && strength.abs() <= 1e6
            }
            Self::Smooth { strength } | Self::Flatten { strength } | Self::Pinch { strength } => {
                strength.is_finite() && (0. ..=1.).contains(strength)
            }
        };
        if ok {
            Ok(())
        } else {
            Err(fail("Invalid sculpt brush strength"))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SculptBrush {
    pub kind: SculptKind,
    pub center: Point,
    pub radius: f64,
    pub falloff: Falloff,
    pub symmetry: Symmetry,
}
impl SculptBrush {
    pub fn new(kind: SculptKind, center: Point, radius: f64) -> Self {
        Self {
            kind,
            center,
            radius,
            falloff: Falloff::Smooth,
            symmetry: Symmetry::default(),
        }
    }
    pub fn validate(&self) -> Result<()> {
        self.kind.validate()?;
        self.symmetry.validate()?;
        if finite(self.center) && self.radius.is_finite() && self.radius > 0. && self.radius <= 1e6
        {
            Ok(())
        } else {
            Err(fail("Invalid sculpt brush"))
        }
    }
    /// Falloff weight of a point for the unmirrored stroke.
    pub fn weight(&self, p: Point) -> f64 {
        self.falloff.weight(norm(sub(p, self.center)) / self.radius)
    }
}
impl Serialize for SculptBrush {
    fn to_value(&self) -> Value {
        let mut object = Map::new();
        object.insert("kind".into(), Value::String(self.kind.name().into()));
        match &self.kind {
            SculptKind::Grab { displacement } => {
                object.insert("displacement".into(), displacement.to_value());
            }
            SculptKind::Draw { strength }
            | SculptKind::Inflate { strength }
            | SculptKind::Smooth { strength }
            | SculptKind::Flatten { strength }
            | SculptKind::Pinch { strength } => {
                object.insert("strength".into(), strength.to_value());
            }
        }
        object.insert("center".into(), self.center.to_value());
        object.insert("radius".into(), self.radius.to_value());
        object.insert("falloff".into(), self.falloff.to_value());
        object.insert("symmetry".into(), self.symmetry.to_value());
        Value::Object(object)
    }
}
impl<'de> Deserialize<'de> for SculptBrush {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| error("Expected object"))?
            .clone();
        let kind_name = object
            .remove("kind")
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(|| error("Missing field kind"))?;
        let mut take = |name: &str| {
            object
                .remove(name)
                .ok_or_else(|| error(format!("Missing field {name}")))
        };
        let kind = match kind_name.as_str() {
            "grab" => SculptKind::Grab {
                displacement: Deserialize::from_value(take("displacement")?)?,
            },
            "draw" => SculptKind::Draw {
                strength: Deserialize::from_value(take("strength")?)?,
            },
            "inflate" => SculptKind::Inflate {
                strength: Deserialize::from_value(take("strength")?)?,
            },
            "smooth" => SculptKind::Smooth {
                strength: Deserialize::from_value(take("strength")?)?,
            },
            "flatten" => SculptKind::Flatten {
                strength: Deserialize::from_value(take("strength")?)?,
            },
            "pinch" => SculptKind::Pinch {
                strength: Deserialize::from_value(take("strength")?)?,
            },
            _ => return Err(error("Unknown sculpt brush kind")),
        };
        let center: Point = Deserialize::from_value(take("center")?)?;
        let radius: f64 = Deserialize::from_value(take("radius")?)?;
        let falloff = match object.remove("falloff") {
            Some(Value::Null) | None => Falloff::Smooth,
            Some(v) => Deserialize::from_value(v)?,
        };
        let symmetry = match object.remove("symmetry") {
            Some(Value::Null) | None => Symmetry::default(),
            Some(v) => Deserialize::from_value(v)?,
        };
        Ok(Self {
            kind,
            center,
            radius,
            falloff,
            symmetry,
        })
    }
}

/// Vertex data a kernel exposes for sculpting. `normals` and `adjacency` may be
/// empty when the brush does not need them (see `SculptKind::needs_*`).
/// Owned `(positions, unit normals, one-ring adjacency)` triple a kernel extracts from its control data.
pub type SculptData = (Vec<[f64; 3]>, Vec<[f64; 3]>, Vec<Vec<usize>>);

pub struct SculptTarget<'a> {
    pub positions: &'a [Point],
    pub normals: &'a [Point],
    pub adjacency: &'a [Vec<usize>],
}

/// Applies the brush (and its mirrored passes, in order) and returns the new
/// positions. Vertex count and order are preserved.
pub fn sculpt(target: &SculptTarget<'_>, brush: &SculptBrush) -> Result<Vec<Point>> {
    brush.validate()?;
    let n = target.positions.len();
    if n == 0 {
        return Err(fail("Sculpt target has no vertices"));
    }
    if target.positions.iter().any(|p| !finite(*p)) {
        return Err(fail("Sculpt target has invalid coordinates"));
    }
    if brush.kind.needs_normals()
        && (target.normals.len() != n || target.normals.iter().any(|p| !finite(*p)))
    {
        return Err(fail("Sculpt target normals do not match vertices"));
    }
    if brush.kind.needs_adjacency()
        && (target.adjacency.len() != n || target.adjacency.iter().flatten().any(|&i| i >= n))
    {
        return Err(fail("Sculpt target adjacency does not match vertices"));
    }
    let vector = match brush.kind {
        SculptKind::Grab { displacement } => displacement,
        _ => [0.; 3],
    };
    let mut out = target.positions.to_vec();
    for (center, vector) in brush.symmetry.passes(brush.center, vector) {
        let weights: Vec<f64> = out
            .iter()
            .map(|p| brush.falloff.weight(norm(sub(*p, center)) / brush.radius))
            .collect();
        let affected: Vec<usize> = (0..n).filter(|&i| weights[i] > 0.).collect();
        if affected.is_empty() {
            continue;
        }
        let area_normal = || {
            let sum = affected.iter().fold([0.; 3], |acc, &i| {
                add(acc, scale(unit(target.normals[i]), weights[i]))
            });
            (norm(sum) > 1e-12).then(|| unit(sum))
        };
        match brush.kind {
            SculptKind::Grab { .. } => {
                for &i in &affected {
                    out[i] = add(out[i], scale(vector, weights[i]));
                }
            }
            SculptKind::Draw { strength } => {
                let Some(nrm) = area_normal() else { continue };
                for &i in &affected {
                    out[i] = add(out[i], scale(nrm, weights[i] * strength));
                }
            }
            SculptKind::Inflate { strength } => {
                for &i in &affected {
                    let nrm = target.normals[i];
                    if norm(nrm) > 1e-12 {
                        out[i] = add(out[i], scale(unit(nrm), weights[i] * strength));
                    }
                }
            }
            SculptKind::Smooth { strength } => {
                let snapshot = out.clone();
                for &i in &affected {
                    let ring = &target.adjacency[i];
                    if ring.is_empty() {
                        continue;
                    }
                    let mean = scale(
                        ring.iter().fold([0.; 3], |acc, &j| add(acc, snapshot[j])),
                        1. / ring.len() as f64,
                    );
                    out[i] = add(out[i], scale(sub(mean, out[i]), weights[i] * strength));
                }
            }
            SculptKind::Flatten { strength } => {
                let Some(nrm) = area_normal() else { continue };
                let total: f64 = affected.iter().map(|&i| weights[i]).sum();
                let centroid = scale(
                    affected
                        .iter()
                        .fold([0.; 3], |acc, &i| add(acc, scale(out[i], weights[i]))),
                    1. / total,
                );
                for &i in &affected {
                    let height = dot(sub(centroid, out[i]), nrm);
                    out[i] = add(out[i], scale(nrm, height * weights[i] * strength));
                }
            }
            SculptKind::Pinch { strength } => {
                let Some(nrm) = area_normal() else { continue };
                for &i in &affected {
                    let to_axis = sub(center, out[i]);
                    let tangent = sub(to_axis, scale(nrm, dot(to_axis, nrm)));
                    out[i] = add(out[i], scale(tangent, weights[i] * strength));
                }
            }
        }
    }
    if out.iter().any(|p| !finite(*p)) {
        return Err(fail("Sculpt exceeds coordinate limits"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn near(a: Point, b: Point) {
        for k in 0..3 {
            assert!((a[k] - b[k]).abs() < 1e-9, "{a:?} != {b:?}");
        }
    }
    /// 3x3 grid in the z=0 plane, normals +z, 4-neighborhood adjacency.
    fn grid() -> (Vec<Point>, Vec<Point>, Vec<Vec<usize>>) {
        let mut positions = Vec::new();
        for j in 0..3 {
            for i in 0..3 {
                positions.push([i as f64 - 1., j as f64 - 1., 0.]);
            }
        }
        let normals = vec![[0., 0., 1.]; 9];
        let mut adjacency = vec![Vec::new(); 9];
        for j in 0..3i32 {
            for i in 0..3i32 {
                let v = (j * 3 + i) as usize;
                for (di, dj) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (ni, nj) = (i + di, j + dj);
                    if (0..3).contains(&ni) && (0..3).contains(&nj) {
                        adjacency[v].push((nj * 3 + ni) as usize);
                    }
                }
            }
        }
        (positions, normals, adjacency)
    }
    fn target<'a>(g: &'a (Vec<Point>, Vec<Point>, Vec<Vec<usize>>)) -> SculptTarget<'a> {
        SculptTarget {
            positions: &g.0,
            normals: &g.1,
            adjacency: &g.2,
        }
    }
    #[test]
    fn falloff_profiles_are_bounded_monotone_and_named() {
        for f in Falloff::ALL {
            assert_eq!(Falloff::parse(f.name()), Some(f));
            assert_eq!(f.weight(0.), 1.);
            assert_eq!(f.weight(1.), 0.);
            assert_eq!(f.weight(2.), 0.);
            assert_eq!(f.weight(f64::NAN), 0.);
            let mut last = 1.;
            for i in 1..=100 {
                let w = f.weight(i as f64 / 100.);
                assert!((0. ..=1.).contains(&w));
                assert!(w <= last + 1e-12, "{f:?} not monotone");
                last = w;
            }
        }
        assert_eq!(Falloff::Smooth.weight(0.5), 0.5);
        assert_eq!(Falloff::Linear.weight(0.25), 0.75);
        assert_eq!(Falloff::Constant.weight(0.999), 1.);
        assert!((Falloff::Sphere.weight(0.6) - 0.8).abs() < 1e-12);
        assert!(Falloff::parse("gaussian").is_none());
    }
    #[test]
    fn grab_matches_legacy_brush_and_is_local() {
        let g = grid();
        let brush = SculptBrush::new(
            SculptKind::Grab {
                displacement: [0., 0., 2.],
            },
            [0.; 3],
            1.5,
        );
        let out = sculpt(&target(&g), &brush).unwrap();
        near(out[4], [0., 0., 2.]);
        let legacy = crate::Brush {
            center: [0.; 3],
            radius: 1.5,
            displacement: [0., 0., 2.],
        };
        for (p, q) in g.0.iter().zip(&out) {
            near(legacy.apply(*p).unwrap(), *q);
        }
        assert!(out[4][2] > out[1][2] && out[1][2] > out[0][2] && out[0][2] > 0.);
    }
    #[test]
    fn draw_and_inflate_follow_normals() {
        let mut g = grid();
        let brush = SculptBrush::new(SculptKind::Draw { strength: 1. }, [0.; 3], 1.5);
        let out = sculpt(&target(&g), &brush).unwrap();
        near(out[4], [0., 0., 1.]);
        assert!(out[0][2] > 0. && out[0][2] < out[1][2]);
        // Negative strength carves.
        let carve = SculptBrush::new(SculptKind::Draw { strength: -1. }, [0.; 3], 1.5);
        assert_eq!(sculpt(&target(&g), &carve).unwrap()[4][2], -1.);
        // Tilt one normal: draw uses the area normal, inflate uses each vertex's own.
        g.1[4] = [1., 0., 0.];
        let drawn = sculpt(&target(&g), &brush).unwrap();
        assert!(
            drawn[4][2] > 0.5 && drawn[4][0] > 0.,
            "area normal tilts slightly"
        );
        let inflate = SculptBrush::new(SculptKind::Inflate { strength: 1. }, [0.; 3], 1.5);
        let inflated = sculpt(&target(&g), &inflate).unwrap();
        near(inflated[4], [1., 0., 0.]);
        near(inflated[1], [0., -1., Falloff::Smooth.weight(1. / 1.5)]);
    }
    #[test]
    fn smooth_relaxes_toward_ring_mean() {
        let mut g = grid();
        g.0[4] = [0., 0., 1.];
        let brush = SculptBrush::new(SculptKind::Smooth { strength: 1. }, [0., 0., 1.], 0.5);
        let out = sculpt(&target(&g), &brush).unwrap();
        near(out[4], [0., 0., 0.]);
        for i in (0..9).filter(|&i| i != 4) {
            near(out[i], g.0[i]);
        }
        let half = SculptBrush::new(SculptKind::Smooth { strength: 0.5 }, [0., 0., 1.], 0.5);
        near(sculpt(&target(&g), &half).unwrap()[4], [0., 0., 0.5]);
        assert!(
            sculpt(
                &target(&g),
                &SculptBrush::new(SculptKind::Smooth { strength: 1.5 }, [0.; 3], 1.)
            )
            .is_err()
        );
    }
    #[test]
    fn flatten_projects_onto_area_plane() {
        let mut g = grid();
        g.0[4] = [0., 0., 1.];
        g.0[0] = [-1., -1., -1.];
        let brush = SculptBrush::new(SculptKind::Flatten { strength: 1. }, [0.; 3], 10.);
        brush.validate().unwrap();
        let flat = SculptBrush {
            falloff: Falloff::Constant,
            ..brush
        };
        let out = sculpt(&target(&g), &flat).unwrap();
        let z = out[0][2];
        for p in &out {
            assert!(
                (p[2] - z).abs() < 1e-9,
                "all vertices share the plane height"
            );
            assert!((p[0] - g.0[out.iter().position(|q| q == p).unwrap()][0]).abs() < 1e-9);
        }
        assert!(
            (z - 0.).abs() < 1e-9,
            "weighted mean height of +1 and -1 spikes is zero"
        );
    }
    #[test]
    fn pinch_pulls_tangentially_toward_center() {
        let g = grid();
        let brush = SculptBrush {
            falloff: Falloff::Constant,
            ..SculptBrush::new(SculptKind::Pinch { strength: 0.5 }, [0.; 3], 10.)
        };
        let out = sculpt(&target(&g), &brush).unwrap();
        near(out[0], [-0.5, -0.5, 0.]);
        near(out[4], [0., 0., 0.]);
        near(out[5], [0.5, 0., 0.]);
        let full = SculptBrush {
            kind: SculptKind::Pinch { strength: 1. },
            ..brush
        };
        for p in sculpt(&target(&g), &full).unwrap() {
            near(p, [0.; 3]);
        }
    }
    #[test]
    fn symmetry_mirrors_center_and_displacement() {
        let g = grid();
        let mut brush = SculptBrush::new(
            SculptKind::Grab {
                displacement: [1., 0., 1.],
            },
            [1., 0., 0.],
            0.5,
        );
        brush.symmetry = Symmetry {
            axes: [true, false, false],
            origin: [0.; 3],
        };
        let out = sculpt(&target(&g), &brush).unwrap();
        near(out[5], [2., 0., 1.]);
        near(out[3], [-2., 0., 1.]);
        near(out[4], [0., 0., 0.]);
        // Center on the mirror plane: the pass is not doubled.
        brush.center = [0.; 3];
        let once = sculpt(&target(&g), &brush).unwrap();
        near(once[4], [1., 0., 1.]);
        // Three axes: eight passes touch every corner of a cube of vertices.
        let cube: Vec<Point> = (0..8)
            .map(|b| std::array::from_fn(|k| if b >> k & 1 == 0 { -1. } else { 1. }))
            .collect();
        let normals = cube.iter().map(|p| unit(*p)).collect::<Vec<_>>();
        let t = SculptTarget {
            positions: &cube,
            normals: &normals,
            adjacency: &[],
        };
        let mut inflate = SculptBrush::new(SculptKind::Inflate { strength: 1. }, [1., 1., 1.], 0.5);
        inflate.symmetry.axes = [true; 3];
        let out = sculpt(&t, &inflate).unwrap();
        for (p, q) in cube.iter().zip(&out) {
            near(*q, add(*p, unit(*p)));
        }
    }
    #[test]
    fn validation_rejects_bad_brushes_and_targets() {
        let g = grid();
        let ok = SculptBrush::new(SculptKind::Draw { strength: 1. }, [0.; 3], 1.);
        for radius in [0., -1., f64::NAN, 1e6 + 1.] {
            assert!(
                SculptBrush {
                    radius,
                    ..ok.clone()
                }
                .validate()
                .is_err()
            );
        }
        assert!(
            SculptBrush {
                center: [f64::INFINITY, 0., 0.],
                ..ok.clone()
            }
            .validate()
            .is_err()
        );
        assert!(
            SculptBrush {
                symmetry: Symmetry {
                    axes: [true; 3],
                    origin: [f64::NAN; 3]
                },
                ..ok.clone()
            }
            .validate()
            .is_err()
        );
        assert!(SculptKind::Draw { strength: f64::NAN }.validate().is_err());
        assert!(SculptKind::Inflate { strength: 1e7 }.validate().is_err());
        assert!(SculptKind::Flatten { strength: -0.1 }.validate().is_err());
        assert!(SculptKind::Pinch { strength: 1.1 }.validate().is_err());
        assert!(
            SculptKind::Grab {
                displacement: [1e7, 0., 0.]
            }
            .validate()
            .is_err()
        );
        // Missing normals for a normal-based brush; missing adjacency for smooth.
        let no_normals = SculptTarget {
            positions: &g.0,
            normals: &[],
            adjacency: &g.2,
        };
        assert!(sculpt(&no_normals, &ok).is_err());
        assert!(
            sculpt(
                &no_normals,
                &SculptBrush::new(
                    SculptKind::Grab {
                        displacement: [0., 0., 1.]
                    },
                    [0.; 3],
                    1.
                )
            )
            .is_ok()
        );
        let no_ring = SculptTarget {
            positions: &g.0,
            normals: &g.1,
            adjacency: &[],
        };
        assert!(
            sculpt(
                &no_ring,
                &SculptBrush::new(SculptKind::Smooth { strength: 1. }, [0.; 3], 1.)
            )
            .is_err()
        );
        let empty = SculptTarget {
            positions: &[],
            normals: &[],
            adjacency: &[],
        };
        assert!(sculpt(&empty, &ok).is_err());
        let bad_ring = vec![vec![99usize]; 9];
        let t = SculptTarget {
            positions: &g.0,
            normals: &g.1,
            adjacency: &bad_ring,
        };
        assert!(
            sculpt(
                &t,
                &SculptBrush::new(SculptKind::Smooth { strength: 1. }, [0.; 3], 1.)
            )
            .is_err()
        );
    }
    #[test]
    fn brush_round_trips_through_codec_with_defaults() {
        let brush = SculptBrush {
            kind: SculptKind::Flatten { strength: 0.3 },
            center: [1., 2., 3.],
            radius: 4.,
            falloff: Falloff::Root,
            symmetry: Symmetry {
                axes: [true, false, true],
                origin: [0., 1., 0.],
            },
        };
        let back: SculptBrush =
            value_codec::from_value(value_codec::to_value(brush.clone()).unwrap()).unwrap();
        assert_eq!(back, brush);
        let mut minimal = Map::new();
        minimal.insert("kind".into(), Value::String("grab".into()));
        minimal.insert("displacement".into(), [0., 0., 1.].to_value());
        minimal.insert("center".into(), [0.; 3].to_value());
        minimal.insert("radius".into(), 2f64.to_value());
        let parsed: SculptBrush = value_codec::from_value(Value::Object(minimal.clone())).unwrap();
        assert_eq!(parsed.falloff, Falloff::Smooth);
        assert_eq!(parsed.symmetry, Symmetry::default());
        minimal.insert("kind".into(), Value::String("twirl".into()));
        assert!(value_codec::from_value::<SculptBrush>(Value::Object(minimal.clone())).is_err());
        minimal.insert("kind".into(), Value::String("draw".into()));
        assert!(
            value_codec::from_value::<SculptBrush>(Value::Object(minimal)).is_err(),
            "draw needs strength"
        );
    }
}
