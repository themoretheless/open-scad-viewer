//! Native polygon construction. No spline or external CAD backend is used.
use crate::solid::tessellation::{self, Options, ParametricSurface, Trim};
use crate::{BuiltMesh, Mesh, Result, check, cross, dot, norm, sub};
use planar_geometry::rings::{Rings, area, inside};
pub type Point = math_core::V3;
fn unit(a: Point) -> Result<Point> {
    let l = norm(a);
    check(l > 1e-12, "Direction is zero or numerically singular")?;
    Ok(a.map(|v| v / l))
}
#[derive(Clone, Debug)]
pub struct Profile {
    pub outer: Vec<[f64; 2]>,
    pub holes: Vec<Vec<[f64; 2]>>,
}
impl value_codec::Serialize for Profile {
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
impl<'de> value_codec::Deserialize<'de> for Profile {
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
struct Plane {
    min: [f64; 2],
    max: [f64; 2],
}
impl ParametricSurface for Plane {
    fn domain(&self) -> [f64; 4] {
        [self.min[0], self.max[0], self.min[1], self.max[1]]
    }
    fn point(&self, u: f64, v: f64) -> Result<Point> {
        Ok([u, v, 0.])
    }
}
pub fn profile_mesh(profile: &Profile) -> Result<Mesh> {
    check(
        profile.outer.len() >= 3
            && profile.outer.len() + profile.holes.iter().map(Vec::len).sum::<usize>() <= 512,
        "Profile requires 3..512 points",
    )?;
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    for p in profile.outer.iter().chain(profile.holes.iter().flatten()) {
        for k in 0..2 {
            check(
                p[k].is_finite() && p[k].abs() <= 1e6,
                "Invalid profile coordinates",
            )?;
            min[k] = min[k].min(p[k]);
            max[k] = max[k].max(p[k]);
        }
    }
    Ok(tessellation::tessellate(
        &Plane { min, max },
        &Options {
            segments_u: 1,
            segments_v: 1,
            trim: Some(Trim {
                outer: profile.outer.clone(),
                holes: profile.holes.clone(),
            }),
            max_triangles: Some(20_000),
        },
    )?
    .mesh)
}
pub fn extrude(profile: &Profile, vector: Point) -> Result<BuiltMesh> {
    profile_mesh(profile)?.thicken(vector)
}
fn orient2(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
/// Ear clipping of a simple planar ring; source vertices are preserved exactly.
fn cap(ring: &[Point]) -> Result<Vec<usize>> {
    check(
        (3..=128).contains(&ring.len()),
        "Section requires 3..128 vertices",
    )?;
    check(
        ring.iter()
            .flatten()
            .all(|v| v.is_finite() && v.abs() <= 1e6),
        "Invalid section coordinates",
    )?;
    let a = ring[0];
    let mut normal = [0.; 3];
    for i in 1..ring.len() - 1 {
        let n = cross(sub(ring[i], a), sub(ring[i + 1], a));
        for k in 0..3 {
            normal[k] += n[k];
        }
    }
    let normal = unit(normal)?;
    let u = unit(sub(ring[1], a))?;
    let v = cross(normal, u);
    let size = ring.iter().map(|&p| norm(sub(p, a))).fold(0., f64::max);
    check(
        ring.iter()
            .all(|&p| dot(sub(p, a), normal).abs() <= size * 1e-9),
        "Loft caps require planar sections",
    )?;
    let points: Vec<_> = ring
        .iter()
        .map(|&p| [dot(sub(p, a), u) / size, dot(sub(p, a), v) / size])
        .collect();
    let min = [
        points.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min),
        points.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min),
    ];
    let max = [
        points
            .iter()
            .map(|p| p[0])
            .fold(f64::NEG_INFINITY, f64::max),
        points
            .iter()
            .map(|p| p[1])
            .fold(f64::NEG_INFINITY, f64::max),
    ];
    let normalized = points
        .iter()
        .map(|p| {
            [
                (p[0] - min[0]) / (max[0] - min[0]),
                (p[1] - min[1]) / (max[1] - min[1]),
            ]
        })
        .collect();
    tessellation::validate_loops(&[normalized])?;
    let mut remaining: Vec<_> = (0..ring.len()).collect();
    let mut triangles = Vec::new();
    while remaining.len() > 3 {
        let mut found = false;
        for i in 0..remaining.len() {
            let a = remaining[(i + remaining.len() - 1) % remaining.len()];
            let b = remaining[i];
            let c = remaining[(i + 1) % remaining.len()];
            if orient2(points[a], points[b], points[c]) <= 1e-12 {
                continue;
            }
            if remaining.iter().any(|&j| {
                j != a
                    && j != b
                    && j != c
                    && orient2(points[a], points[b], points[j]) >= -1e-12
                    && orient2(points[b], points[c], points[j]) >= -1e-12
                    && orient2(points[c], points[a], points[j]) >= -1e-12
            }) {
                continue;
            }
            triangles.extend([a, b, c]);
            remaining.remove(i);
            found = true;
            break;
        }
        check(
            found,
            "Cannot triangulate section; remove collinear or degenerate vertices",
        )?;
    }
    triangles.extend(remaining);
    Ok(triangles)
}
fn finish(mesh: Mesh, closed: bool) -> Result<BuiltMesh> {
    let mut mesh = mesh;
    let mut report = mesh.inspect()?;
    check(
        report.degenerate_triangles == 0
            && report.non_manifold_edges == 0
            && report.orientation_conflicts == 0
            && (!closed || report.closed),
        "Construction produced invalid mesh topology",
    )?;
    if closed && report.signed_volume_mm3 < 0. {
        mesh.reverse_winding();
        report = mesh.inspect()?;
    }
    if closed {
        check(
            report.signed_volume_mm3 > 0.,
            "Construction has no positive volume",
        )?;
    }
    Ok(BuiltMesh { mesh, report })
}
pub fn loft(sections: &[Vec<Point>], caps: bool) -> Result<BuiltMesh> {
    check(
        (2..=64).contains(&sections.len()),
        "Loft requires 2..64 sections",
    )?;
    let n = sections[0].len();
    check(
        (3..=128).contains(&n) && sections.iter().all(|s| s.len() == n),
        "Loft sections require identical vertex counts, 3..128",
    )?;
    let triangles = (sections.len() - 1) * n * 2 + if caps { (n - 2) * 2 } else { 0 };
    check(triangles <= 20_000, "Loft triangle budget exceeded")?;
    for ring in sections {
        cap(ring)?;
    }
    let positions = sections.iter().flatten().flatten().copied().collect();
    let mut indices = Vec::new();
    for k in 0..sections.len() - 1 {
        for i in 0..n {
            let a = k * n + i;
            let b = k * n + (i + 1) % n;
            let c = b + n;
            let d = a + n;
            indices.extend([a, b, c, a, c, d]);
        }
    }
    if caps {
        for t in cap(&sections[0])?.as_chunks::<3>().0 {
            indices.extend([t[0], t[2], t[1]]);
        }
        let base = (sections.len() - 1) * n;
        indices.extend(cap(sections.last().unwrap())?.iter().map(|i| base + i));
    }
    finish(
        Mesh {
            positions,
            indices,
            uv: None,
        },
        caps,
    )
}
/// Rotation-minimizing frames on a polyline; 180-degree reversals are rejected.
pub fn sweep_sections(profile: &[[f64; 2]], path: &[Point], up: Point) -> Result<Vec<Vec<Point>>> {
    geometry_ops::sweep_sections(profile, path, up)
}
pub fn sweep(profile: &[[f64; 2]], path: &[Point], up: Point, caps: bool) -> Result<BuiltMesh> {
    loft(&sweep_sections(profile, path, up)?, caps)
}
/// Revolve a closed r/z profile around the Z axis. Full turns are welded;
/// partial turns optionally get planar caps. Axis vertices are welded.
pub fn revolve(
    profile: &[[f64; 2]],
    angle_degrees: f64,
    segments: usize,
    caps: bool,
) -> Result<BuiltMesh> {
    check(
        angle_degrees.is_finite()
            && angle_degrees != 0.
            && angle_degrees.abs() <= 360.
            && (1..=512).contains(&segments),
        "Invalid revolve angle or segments",
    )?;
    check(
        profile
            .iter()
            .all(|p| p[0] >= 0. && p.iter().all(|v| v.is_finite())),
        "Revolve requires nonnegative radii",
    )?;
    let full = angle_degrees.abs() == 360.;
    let n = profile.len();
    check(
        (3..=128).contains(&n) && n * segments * 2 <= 20_000,
        "Revolve profile/budget exceeded",
    )?;
    let mut sections = Vec::new();
    for i in 0..=segments {
        let a = angle_degrees.to_radians() * i as f64 / segments as f64;
        sections.push(
            profile
                .iter()
                .map(|p| [p[0] * a.cos(), p[0] * a.sin(), p[1]])
                .collect::<Vec<_>>(),
        );
    }
    cap(&sections[0])?;
    let rows = if full { segments } else { segments + 1 };
    let positions = sections[..rows]
        .iter()
        .flatten()
        .flatten()
        .copied()
        .collect();
    let mut indices = Vec::new();
    for k in 0..segments {
        for i in 0..n {
            let a = k * n + i;
            let b = k * n + (i + 1) % n;
            let c = ((k + 1) % rows) * n + (i + 1) % n;
            let d = ((k + 1) % rows) * n + i;
            indices.extend([a, b, c, a, c, d]);
        }
    }
    if !full && caps {
        for t in cap(&sections[0])?.as_chunks::<3>().0 {
            indices.extend([t[0], t[2], t[1]]);
        }
        indices.extend(cap(&sections[segments])?.iter().map(|i| segments * n + i));
    }
    let mut mesh = crate::solid::proximity::weld_exact(&Mesh {
        positions,
        indices,
        uv: None,
    })?;
    mesh.indices = mesh
        .indices
        .as_chunks::<3>()
        .0
        .iter()
        .filter(|t| t[0] != t[1] && t[1] != t[2] && t[2] != t[0])
        .flatten()
        .copied()
        .collect();
    finish(mesh, full || caps)
}

/// Linear extrude of closed rings (OpenSCAD-style height / twist / scale).
pub fn extrude_rings(
    rings: &Rings,
    height: f64,
    slices: usize,
    twist: f64,
    scale: [f64; 2],
    center: bool,
) -> Result<Mesh> {
    check(
        height.is_finite() && height > 0. && slices <= 513,
        "Invalid extrusion",
    )?;
    check(
        rings
            .iter()
            .map(Vec::len)
            .sum::<usize>()
            .saturating_mul(if twist != 0. || scale != [1., 1.] {
                slices.max(1)
            } else {
                1
            })
            .saturating_mul(2)
            <= 20000,
        "Extrusion triangle budget exceeded",
    )?;
    let mut pieces = Vec::new();
    for outer in rings.iter().filter(|r| area(r) > 0.) {
        let holes = rings
            .iter()
            .filter(|h| area(h) < 0. && inside(h[0], &vec![outer.clone()]))
            .cloned()
            .collect();
        let profile = Profile {
            outer: outer.clone(),
            holes,
        };
        let mut m = crate::solid::primitives::triangulate(&profile)?
            .thicken([0., 0., height])?
            .mesh;
        if twist != 0. || scale != [1., 1.] {
            // Subdivide every vertical side consistently, retaining triangulated caps.
            let base = crate::solid::primitives::triangulate(&profile)?;
            let n = base.positions.len() / 3;
            let steps = slices.max(1);
            m = crate::solid::primitives::empty();
            for row in 0..=steps {
                let f = row as f64 / steps as f64;
                let a = (-twist * f).to_radians();
                for p in base.positions.as_chunks::<3>().0 {
                    let x = p[0] * (1. + (scale[0] - 1.) * f);
                    let y = p[1] * (1. + (scale[1] - 1.) * f);
                    m.positions.extend([
                        x * a.cos() - y * a.sin(),
                        x * a.sin() + y * a.cos(),
                        height * f,
                    ]);
                }
            }
            for t in base.indices.as_chunks::<3>().0 {
                m.indices.extend([
                    t[2],
                    t[1],
                    t[0],
                    t[0] + steps * n,
                    t[1] + steps * n,
                    t[2] + steps * n,
                ]);
            }
            for ring in base.boundary_loops()? {
                for row in 0..steps {
                    for i in 0..ring.len() - 1 {
                        let a = ring[i] + row * n;
                        let b = ring[i + 1] + row * n;
                        m.indices.extend([a, b, b + n, a, b + n, a + n]);
                    }
                }
            }
        }
        if center {
            for p in m.positions.as_chunks_mut::<3>().0 {
                p[2] -= height / 2.
            }
        }
        pieces.push(m);
    }
    crate::solid::primitives::join(&pieces)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn square() -> Vec<[f64; 2]> {
        vec![[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]]
    }
    #[test]
    fn extrude_hole_and_oblique_vector() {
        let p = Profile {
            outer: square(),
            holes: vec![vec![[-0.5, -0.5], [-0.5, 0.5], [0.5, 0.5], [0.5, -0.5]]],
        };
        let m = extrude(&p, [1., 2., 3.]).unwrap();
        assert!(m.report.closed);
        assert!((m.report.signed_volume_mm3 - 9.).abs() < 1e-9);
    }
    #[test]
    fn loft_concave_sections() {
        let p = [[0., 0.], [2., 0.], [2., 1.], [1., 1.], [1., 2.], [0., 2.]];
        let sections = [0., 3.].map(|z| p.iter().map(|p| [p[0], p[1], z]).collect());
        let m = loft(&sections, true).unwrap();
        assert!((m.report.signed_volume_mm3 - 9.).abs() < 1e-9);
    }
    #[test]
    fn sweep_straight_and_curved() {
        let m = sweep(&square(), &[[0.; 3], [0., 0., 5.]], [1., 0., 0.], true).unwrap();
        assert!((m.report.signed_volume_mm3 - 20.).abs() < 1e-9);
        assert!(
            sweep(
                &square(),
                &[[0.; 3], [0., 0., 5.], [3., 0., 8.]],
                [1., 0., 0.],
                true
            )
            .unwrap()
            .report
            .closed
        );
    }
    #[test]
    fn extrude_rings_difference_volume() {
        let a = vec![vec![[0., 0.], [4., 0.], [4., 4.], [0., 4.]]];
        let b = vec![vec![[1., 1.], [3., 1.], [3., 3.], [1., 3.]]];
        let r = planar_geometry::rings::planar(&a, &b, "difference").unwrap();
        let m = extrude_rings(&r, 2., 1, 0., [1., 1.], false).unwrap();
        assert!((m.inspect().unwrap().signed_volume_mm3 - 24.).abs() < 1e-8);
    }
    #[test]
    fn revolve_full_and_partial() {
        let p = vec![[1., 0.], [2., 0.], [2., 3.], [1., 3.]];
        let full = revolve(&p, 360., 32, true).unwrap();
        assert!(full.report.closed);
        assert!((full.report.signed_volume_mm3 - 9. * std::f64::consts::PI).abs() < 0.2);
        assert!(revolve(&p, 90., 8, true).unwrap().report.closed);
    }
}
