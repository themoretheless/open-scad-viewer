//! Native polygon construction. No spline or external CAD backend is used.
use crate::tessellation::{self, Options, ParametricSurface, Trim};
use crate::{check, cross, norm, sub, BuiltMesh, Mesh, Result};
use serde::{Deserialize, Serialize};
pub type Point = [f64; 3];
fn dot(a: Point, b: Point) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn unit(a: Point) -> Result<Point> {
    let l = norm(&a);
    check(l > 1e-12, "Direction is zero or numerically singular")?;
    Ok(a.map(|v| v / l))
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    pub outer: Vec<[f64; 2]>,
    #[serde(default)]
    pub holes: Vec<Vec<[f64; 2]>>,
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
    let size = ring.iter().map(|&p| norm(&sub(p, a))).fold(0., f64::max);
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
        for t in cap(&sections[0])?.chunks_exact(3) {
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
    check(
        (2..=64).contains(&path.len()) && path.iter().flatten().all(|v| v.is_finite()),
        "Sweep requires 2..64 finite path points",
    )?;
    let tangents: Vec<_> = path
        .windows(2)
        .map(|w| unit(sub(w[1], w[0])))
        .collect::<Result<_>>()?;
    let mut normal = unit(sub(up, tangents[0].map(|v| v * dot(up, tangents[0]))))?;
    let mut previous = tangents[0];
    let mut sections = Vec::new();
    for i in 0..path.len() {
        let tangent = if i == 0 {
            tangents[0]
        } else if i == path.len() - 1 {
            *tangents.last().unwrap()
        } else {
            unit(std::array::from_fn(|k| tangents[i - 1][k] + tangents[i][k]))?
        };
        let axis = cross(previous, tangent);
        let sine = norm(&axis);
        let cosine = dot(previous, tangent).clamp(-1., 1.);
        check(cosine > -1. + 1e-9, "Sweep path reverses direction")?;
        if sine > 1e-12 {
            let axis = axis.map(|v| v / sine);
            let b = cross(axis, normal);
            let d = dot(axis, normal);
            normal = std::array::from_fn(|k| {
                normal[k] * cosine + b[k] * sine + axis[k] * d * (1. - cosine)
            });
        }
        normal = unit(sub(normal, tangent.map(|v| v * dot(normal, tangent))))?;
        let binormal = cross(tangent, normal);
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
            && (3..=64).contains(&segments),
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
        for t in cap(&sections[0])?.chunks_exact(3) {
            indices.extend([t[0], t[2], t[1]]);
        }
        indices.extend(cap(&sections[segments])?.iter().map(|i| segments * n + i));
    }
    let mut mesh = crate::proximity::weld_exact(&Mesh {
        positions,
        indices,
        uv: None,
    })?;
    mesh.indices = mesh
        .indices
        .chunks_exact(3)
        .filter(|t| t[0] != t[1] && t[1] != t[2] && t[2] != t[0])
        .flatten()
        .copied()
        .collect();
    finish(mesh, full || caps)
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
    fn revolve_full_and_partial() {
        let p = vec![[1., 0.], [2., 0.], [2., 3.], [1., 3.]];
        let full = revolve(&p, 360., 32, true).unwrap();
        assert!(full.report.closed);
        assert!((full.report.signed_volume_mm3 - 9. * std::f64::consts::PI).abs() < 0.2);
        assert!(revolve(&p, 90., 8, true).unwrap().report.closed);
    }
}
