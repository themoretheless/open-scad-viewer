//! Controlled deformations (item 50) and refinement (item 43).
//!
//! [`transform`] applies an affine matrix and reverses the winding when the
//! linear part reflects, so a closed outward mesh stays outward. Loop
//! refinement wraps `subdivision_core::Cage::loop_subdivide`; midpoint
//! refinement (linear, shape-preserving) is implemented here because
//! subdivision-core offers only smoothing schemes for triangles.

use crate::geom::{cross, dot, point};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum TransformError {
    InvalidInput(String),
    /// Matrix has a non-finite entry, a projective row, or a (near) singular linear part.
    InvalidMatrix(String),
    /// Refinement output failed the manifold re-check.
    NotPreserved,
}

impl std::fmt::Display for TransformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for TransformError {}

#[derive(Debug, Clone)]
pub struct Transformed {
    pub positions: Vec<f64>,
    pub indices: Vec<usize>,
    /// True when the matrix reflects and the winding was reversed.
    pub orientation_flipped: bool,
    pub determinant: f64,
}

fn buffers(positions: &[f64], indices: &[usize]) -> Result<(), TransformError> {
    if positions.len() % 3 != 0
        || indices.len() % 3 != 0
        || indices.iter().any(|&i| i >= positions.len() / 3)
    {
        return Err(TransformError::InvalidInput(
            "malformed mesh buffers".into(),
        ));
    }
    if positions.iter().any(|v| !v.is_finite()) {
        return Err(TransformError::InvalidInput("non-finite coordinate".into()));
    }
    Ok(())
}

/// Apply a row-major affine 4x4 matrix (`p' = M * [p, 1]`).
pub fn transform(
    positions: &[f64],
    indices: &[usize],
    m: [[f64; 4]; 4],
) -> Result<Transformed, TransformError> {
    buffers(positions, indices)?;
    if m.iter().flatten().any(|v| !v.is_finite()) {
        return Err(TransformError::InvalidMatrix("non-finite entry".into()));
    }
    if m[3] != [0.0, 0.0, 0.0, 1.0] {
        return Err(TransformError::InvalidMatrix(
            "last row must be [0, 0, 0, 1]".into(),
        ));
    }
    let r = |i: usize| [m[i][0], m[i][1], m[i][2]];
    let det = dot(r(0), cross(r(1), r(2)));
    let scale = (0..3).map(|i| dot(r(i), r(i)).sqrt()).product::<f64>();
    if !(det.abs() > 1e-12 * scale) || det.is_nan() {
        return Err(TransformError::InvalidMatrix("singular linear part".into()));
    }
    let out = (0..positions.len() / 3)
        .flat_map(|i| {
            let p = point(positions, i);
            std::array::from_fn::<f64, 3, _>(|k| dot(r(k), p) + m[k][3])
        })
        .collect();
    let flipped = det < 0.0;
    let indices = if flipped {
        indices.chunks(3).flat_map(|t| [t[0], t[2], t[1]]).collect()
    } else {
        indices.to_vec()
    };
    Ok(Transformed {
        positions: out,
        indices,
        orientation_flipped: flipped,
        determinant: det,
    })
}

/// Split every triangle into four through edge midpoints, `levels` times (shape unchanged).
pub fn refine_midpoint(
    positions: &[f64],
    indices: &[usize],
    levels: usize,
) -> Result<(Vec<f64>, Vec<usize>), TransformError> {
    buffers(positions, indices)?;
    if levels > 6 {
        return Err(TransformError::InvalidInput(
            "at most 6 refinement levels".into(),
        ));
    }
    let (mut p, mut t) = (positions.to_vec(), indices.to_vec());
    for _ in 0..levels {
        let mut mid: HashMap<(usize, usize), usize> = HashMap::new();
        let mut next = Vec::with_capacity(t.len() * 4);
        for f in t.chunks(3) {
            let mut m = [0; 3];
            for k in 0..3 {
                let (a, b) = (f[k], f[(k + 1) % 3]);
                m[k] = *mid.entry((a.min(b), a.max(b))).or_insert_with(|| {
                    let (pa, pb) = (point(&p, a), point(&p, b));
                    p.extend((0..3).map(|c| 0.5 * (pa[c] + pb[c])));
                    p.len() / 3 - 1
                });
            }
            next.extend([
                f[0], m[0], m[2], m[0], f[1], m[1], m[2], m[1], f[2], m[0], m[1], m[2],
            ]);
        }
        t = next;
    }
    Ok((p, t))
}

/// Loop subdivision via `subdivision-core`; the output is re-checked to keep
/// the input's manifold class (closed stays closed).
pub fn refine_loop(
    positions: &[f64],
    indices: &[usize],
    levels: usize,
) -> Result<(Vec<f64>, Vec<usize>), TransformError> {
    buffers(positions, indices)?;
    let before = manifold_core::check(positions, indices);
    if !before.is_manifold_with_boundary() {
        return Err(TransformError::InvalidInput(
            "input must be manifold (boundary allowed)".into(),
        ));
    }
    let cage = subdivision_core::Cage::from_faces(
        (0..positions.len() / 3)
            .map(|i| point(positions, i))
            .collect(),
        indices.chunks(3).map(|t| t.to_vec()).collect(),
    )
    .map_err(|e| TransformError::InvalidInput(e.message))?;
    let out = cage
        .loop_subdivide(levels)
        .map_err(|e| TransformError::InvalidInput(e.message))?;
    let p: Vec<f64> = out.vertices.iter().flatten().copied().collect();
    let t: Vec<usize> = out.faces.iter().flatten().copied().collect();
    let after = manifold_core::check(&p, &t);
    let ok = if before.is_manifold() {
        after.is_manifold()
    } else {
        after.is_manifold_with_boundary()
    };
    if !ok {
        return Err(TransformError::NotPreserved);
    }
    Ok((p, t))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::fixtures::{cuboid, sphere, volume};

    #[test]
    fn mirror_keeps_outward_orientation() {
        let (p, t) = cuboid([0.0; 3], [1.0, 2.0, 3.0]);
        let mirror = [
            [-1.0, 0.0, 0.0, 5.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 2.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let out = transform(&p, &t, mirror).unwrap();
        assert!(out.orientation_flipped);
        assert!((volume(&out.positions, &out.indices) - 12.0).abs() < 1e-12);
        assert!(manifold_core::check(&out.positions, &out.indices).is_manifold());
        let rot = [
            [0.0, -1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 1.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        assert!(!transform(&p, &t, rot).unwrap().orientation_flipped);
        let singular = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        assert!(matches!(
            transform(&p, &t, singular),
            Err(TransformError::InvalidMatrix(_))
        ));
        let projective = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0, 1.0],
        ];
        assert!(matches!(
            transform(&p, &t, projective),
            Err(TransformError::InvalidMatrix(_))
        ));
    }

    #[test]
    fn refinements_keep_manifoldness() {
        let (p, t) = cuboid([0.0; 3], [1.0; 3]);
        let (mp, mt) = refine_midpoint(&p, &t, 2).unwrap();
        assert_eq!(mt.len(), t.len() * 16);
        assert!(manifold_core::check(&mp, &mt).is_manifold());
        assert!((volume(&mp, &mt) - 1.0).abs() < 1e-12);
        let (sp, st) = sphere(1.0, 6, 8);
        let (lp, lt) = refine_loop(&sp, &st, 2).unwrap();
        assert_eq!(lt.len(), st.len() * 16);
        assert!(manifold_core::check(&lp, &lt).is_manifold());
        let v = volume(&lp, &lt);
        assert!(v > 0.0 && v < volume(&sp, &st));
    }
}
