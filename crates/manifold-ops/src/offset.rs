//! Vertex-normal offset and shell (thickening) with honest refusals.
//!
//! Each vertex moves along its angle-weighted normal by `distance` (positive
//! outward). This is not an exact Minkowski offset: distances at sharp
//! features are not uniform. The result is accepted only when every triangle
//! keeps its orientation and stays non-degenerate and the exact
//! self-intersection test of `manifold-core` finds nothing; otherwise a typed
//! [`OffsetError`] is returned. [`shell`] combines the original surface with
//! the reversed offset into one hollow solid and checks that the two walls
//! do not touch.
//!
//! Existing shells were considered: `polygon_core::mesh_shell::shell` is a
//! sampled (grid) shell that needs removed "opening" faces, and
//! `polygon_core::solid_extensions::sphere_shell` is sphere-specific, so
//! neither gives a closed offset of a general manifold.

use crate::geom::{P, add, cross, dot, norm, point, scale, sub, tri_normal};
use manifold_core::{SelfIntersectionOptions, self_intersections};

#[derive(Debug, Clone, PartialEq)]
pub enum OffsetError {
    InvalidInput(String),
    /// Input is not a closed, outward-oriented strict 2-manifold.
    NotClosedManifold,
    /// Triangles that would flip or collapse (indices into the input triangles).
    FlippedTriangles(Vec<usize>),
    /// The offset turned the solid inside out or moved its volume the wrong way.
    Inverted,
    /// Offset surface (or shell walls) intersect themselves; count of pairs.
    SelfIntersection {
        pairs: usize,
    },
    /// Self-intersection check exceeded its work budget.
    BudgetExceeded,
}

impl std::fmt::Display for OffsetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for OffsetError {}

fn admit(positions: &[f64], indices: &[usize], distance: f64) -> Result<(), OffsetError> {
    if !positions.len().is_multiple_of(3)
        || !indices.len().is_multiple_of(3)
        || indices.iter().any(|&i| i >= positions.len() / 3)
    {
        return Err(OffsetError::InvalidInput("malformed mesh buffers".into()));
    }
    if positions.iter().any(|v| !v.is_finite()) || !distance.is_finite() {
        return Err(OffsetError::InvalidInput(
            "non-finite coordinate or distance".into(),
        ));
    }
    if !manifold_core::check(positions, indices).is_manifold() {
        return Err(OffsetError::NotClosedManifold);
    }
    let vol = polygon_core::Mesh {
        positions: positions.to_vec(),
        indices: indices.to_vec(),
        uv: None,
    }
    .volume();
    if vol <= 0.0 {
        return Err(OffsetError::NotClosedManifold);
    }
    Ok(())
}

/// Angle-weighted unit vertex normals.
pub fn vertex_normals(positions: &[f64], indices: &[usize]) -> Vec<[f64; 3]> {
    let mut acc = vec![[0.0; 3]; positions.len() / 3];
    for t in indices.chunks(3) {
        let p: [P; 3] = [t[0], t[1], t[2]].map(|i| point(positions, i));
        let n = tri_normal(p[0], p[1], p[2]);
        let len = norm(n);
        if len == 0.0 {
            continue;
        }
        let n = scale(n, 1.0 / len);
        for k in 0..3 {
            let e1 = sub(p[(k + 1) % 3], p[k]);
            let e2 = sub(p[(k + 2) % 3], p[k]);
            let angle = norm(cross(e1, e2)).atan2(dot(e1, e2));
            acc[t[k]] = add(acc[t[k]], scale(n, angle));
        }
    }
    acc.into_iter()
        .map(|n| {
            let len = norm(n);
            if len > 0.0 { scale(n, 1.0 / len) } else { n }
        })
        .collect()
}

fn self_check(positions: &[f64], indices: &[usize]) -> Result<(), OffsetError> {
    let report = self_intersections(
        positions,
        indices,
        SelfIntersectionOptions {
            max_reported_pairs: 1,
            ..Default::default()
        },
    )
    .map_err(|_| OffsetError::BudgetExceeded)?;
    if report.has_self_intersections() {
        return Err(OffsetError::SelfIntersection {
            pairs: report.pairs.len(),
        });
    }
    Ok(())
}

/// Offset a closed outward-oriented manifold by `distance` along vertex normals.
/// Returns new positions with the input `indices` unchanged.
pub fn offset(
    positions: &[f64],
    indices: &[usize],
    distance: f64,
) -> Result<Vec<f64>, OffsetError> {
    admit(positions, indices, distance)?;
    let normals = vertex_normals(positions, indices);
    let out: Vec<f64> = (0..positions.len() / 3)
        .flat_map(|i| add(point(positions, i), scale(normals[i], distance)))
        .collect();
    let mut flipped = Vec::new();
    for (f, t) in indices.chunks(3).enumerate() {
        let old = tri_normal(
            point(positions, t[0]),
            point(positions, t[1]),
            point(positions, t[2]),
        );
        let new = tri_normal(point(&out, t[0]), point(&out, t[1]), point(&out, t[2]));
        if dot(old, new) <= 1e-12 * dot(old, old) {
            flipped.push(f);
        }
    }
    if !flipped.is_empty() {
        return Err(OffsetError::FlippedTriangles(flipped));
    }
    // A point reflection keeps every triangle normal but turns the solid inside out.
    let v0 = polygon_core::Mesh {
        positions: positions.to_vec(),
        indices: indices.to_vec(),
        uv: None,
    }
    .volume();
    let v1 = polygon_core::Mesh {
        positions: out.clone(),
        indices: indices.to_vec(),
        uv: None,
    }
    .volume();
    if v1 <= 0.0 || (distance < 0.0 && v1 >= v0) || (distance > 0.0 && v1 <= v0) {
        return Err(OffsetError::Inverted);
    }
    self_check(&out, indices)?;
    Ok(out)
}

/// Hollow solid with wall `thickness`: `inward` keeps the outer surface and
/// adds an inner wall, otherwise the input becomes the inner wall.
pub fn shell(
    positions: &[f64],
    indices: &[usize],
    thickness: f64,
    inward: bool,
) -> Result<(Vec<f64>, Vec<usize>), OffsetError> {
    if thickness.is_nan() || thickness <= 0.0 {
        return Err(OffsetError::InvalidInput(
            "shell thickness must be positive".into(),
        ));
    }
    let moved = offset(
        positions,
        indices,
        if inward { -thickness } else { thickness },
    )?;
    let (outer, inner) = if inward {
        (positions.to_vec(), moved)
    } else {
        (moved, positions.to_vec())
    };
    let n = outer.len() / 3;
    let mut p = outer;
    p.extend_from_slice(&inner);
    let mut t = indices.to_vec();
    t.extend(
        indices
            .chunks(3)
            .flat_map(|f| [f[0] + n, f[2] + n, f[1] + n]),
    );
    self_check(&p, &t)?;
    Ok((p, t))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::fixtures::{cuboid, sphere, volume};
    use std::f64::consts::PI;

    #[test]
    fn sphere_offsets_scale_volume() {
        let (p, t) = sphere(5.0, 24, 48);
        let v0 = volume(&p, &t);
        let grown = offset(&p, &t, 1.0).unwrap();
        let ratio = volume(&grown, &t) / v0;
        assert!((ratio - (6.0f64 / 5.0).powi(3)).abs() < 1e-3, "{ratio}");
        let shrunk = offset(&p, &t, -2.0).unwrap();
        let ratio = volume(&shrunk, &t) / v0;
        assert!((ratio - (3.0f64 / 5.0).powi(3)).abs() < 1e-3, "{ratio}");
        assert!(manifold_core::check(&grown, &t).is_manifold());
    }

    #[test]
    fn unsafe_offsets_are_refused() {
        let (p, t) = sphere(5.0, 16, 32);
        // Beyond the centre the sphere turns inside out.
        assert!(matches!(
            offset(&p, &t, -6.0),
            Err(OffsetError::FlippedTriangles(_) | OffsetError::Inverted)
        ));
        // Thin plate: walls cross before triangles flip.
        let (p, t) = crate::transform::refine_midpoint(
            &cuboid([0.0; 3], [10.0, 10.0, 1.0]).0,
            &cuboid([0.0; 3], [10.0, 10.0, 1.0]).1,
            2,
        )
        .unwrap();
        let err = offset(&p, &t, -0.8).unwrap_err();
        assert!(
            matches!(
                err,
                OffsetError::SelfIntersection { .. } | OffsetError::FlippedTriangles(_)
            ),
            "{err:?}"
        );
        // Open mesh.
        let (p, t) = sphere(1.0, 6, 8);
        assert_eq!(
            offset(&p, &t[3..], 0.1).unwrap_err(),
            OffsetError::NotClosedManifold
        );
        assert!(matches!(
            offset(&p, &t, f64::NAN),
            Err(OffsetError::InvalidInput(_))
        ));
    }

    #[test]
    fn shell_volume_matches_wall() {
        let (p, t) = sphere(5.0, 24, 48);
        let (sp, st) = shell(&p, &t, 1.0, true).unwrap();
        assert!(manifold_core::check(&sp, &st).is_manifold());
        let expected = 4.0 / 3.0 * PI * (125.0 - 64.0);
        let got = volume(&sp, &st);
        assert!(
            (got - expected).abs() / expected < 0.02,
            "{got} vs {expected}"
        );
        let (op, ot) = shell(&p, &t, 0.5, false).unwrap();
        assert!(volume(&op, &ot) > 0.0);
        assert!(matches!(
            shell(&p, &t, 6.0, true),
            Err(OffsetError::FlippedTriangles(_) | OffsetError::Inverted)
        ));
    }
}
