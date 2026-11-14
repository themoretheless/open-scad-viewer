//! Mass, center of mass and inertia tensor at a given density.

use super::{DiagnosticsError, Mesh, P};

/// Rigid-body properties of a closed mesh at uniform density.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MassProperties {
    /// Enclosed volume (positive regardless of orientation).
    pub volume: f64,
    /// `volume * density`.
    pub mass: f64,
    pub center_of_mass: P,
    /// Inertia tensor about `center_of_mass`, scaled by density
    /// (`I = sum m (|r|^2 E - r r^T)`).
    pub inertia: [[f64; 3]; 3],
}

fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: P, b: P) -> P {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Mass properties from the signed-tetrahedron volume integrals (the same
/// decomposition as [`crate::metrics`] volumes). The mesh must be closed
/// (no boundary edges); orientation may be inward or outward.
pub fn mass_properties(
    positions: &[f64],
    indices: &[usize],
    density: f64,
) -> Result<MassProperties, DiagnosticsError> {
    if !density.is_finite() || density <= 0.0 {
        return Err(DiagnosticsError::InvalidParameter("density"));
    }
    let mesh = Mesh::new(positions, indices)?;
    if mesh.triangles.is_empty() || mesh_topology::EdgeUses::new(indices).counts().boundary > 0 {
        return Err(DiagnosticsError::OpenMesh);
    }
    // First moments about the bounding-box centre for conditioning.
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for t in 0..mesh.triangles.len() {
        for p in mesh.corners(t) {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
    }
    let origin: P = std::array::from_fn(|k| lo[k] * 0.5 + hi[k] * 0.5);
    let local = |t: usize| {
        mesh.corners(t)
            .map(|p| std::array::from_fn(|k| p[k] - origin[k]))
    };
    let mut v6 = 0.0;
    let mut first = [0.0; 3];
    for t in 0..mesh.triangles.len() {
        let [a, b, c] = local(t);
        let det = dot(a, cross(b, c));
        v6 += det;
        for k in 0..3 {
            first[k] += det * (a[k] + b[k] + c[k]);
        }
    }
    let scale = hi
        .iter()
        .zip(&lo)
        .map(|(h, l)| h - l)
        .fold(0.0f64, f64::max);
    if v6.abs() <= f64::EPSILON * scale.powi(3) {
        return Err(DiagnosticsError::ZeroVolume);
    }
    let com_local: P = first.map(|m| m / (4.0 * v6));
    // Second moments (covariance) about the centre of mass.
    let mut cov = [[0.0; 3]; 3];
    for t in 0..mesh.triangles.len() {
        let pts = local(t).map(|p| std::array::from_fn::<f64, 3, _>(|k| p[k] - com_local[k]));
        let det = dot(pts[0], cross(pts[1], pts[2]));
        for (i, row) in cov.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                let mut sum = 0.0;
                for (p, pp) in pts.iter().enumerate() {
                    for (q, qq) in pts.iter().enumerate() {
                        sum += if p == q { 2.0 } else { 1.0 } * pp[i] * qq[j];
                    }
                }
                *cell += det * sum / 120.0;
            }
        }
    }
    // Inward orientation flips every det; normalise so mass is positive.
    let sign = v6.signum();
    let trace = (cov[0][0] + cov[1][1] + cov[2][2]) * sign;
    let inertia = std::array::from_fn(|i| {
        std::array::from_fn(|j| density * (if i == j { trace } else { 0.0 } - cov[i][j] * sign))
    });
    let volume = v6.abs() / 6.0;
    Ok(MassProperties {
        volume,
        mass: volume * density,
        center_of_mass: std::array::from_fn(|k| com_local[k] + origin[k]),
        inertia,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::tests::{box_mesh, flip};

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * b.abs().max(1.0)
    }

    #[test]
    fn box_matches_closed_form() {
        // 2 x 4 x 6 box at density 3: m = 144, Ixx = m (b^2 + c^2) / 12.
        let (p, i) = box_mesh([1., 2., 3.], [3., 6., 9.]);
        let m = mass_properties(&p, &i, 3.0).unwrap();
        assert!(close(m.volume, 48.0) && close(m.mass, 144.0));
        assert!(
            m.center_of_mass
                .iter()
                .zip([2., 4., 6.])
                .all(|(a, b)| close(*a, b))
        );
        assert!(close(m.inertia[0][0], 144.0 * (16.0 + 36.0) / 12.0));
        assert!(close(m.inertia[1][1], 144.0 * (4.0 + 36.0) / 12.0));
        assert!(close(m.inertia[2][2], 144.0 * (4.0 + 16.0) / 12.0));
        assert!(m.inertia[0][1].abs() < 1e-9 && m.inertia[1][2].abs() < 1e-9);
        let inv = mass_properties(&p, &flip(&i), 3.0).unwrap();
        assert!(close(inv.mass, m.mass) && close(inv.inertia[0][0], m.inertia[0][0]));
    }

    #[test]
    fn rejects_open_mesh_bad_density_and_flat_mesh() {
        let (p, i) = box_mesh([0.; 3], [1.; 3]);
        assert_eq!(
            mass_properties(&p, &i, 0.0),
            Err(DiagnosticsError::InvalidParameter("density"))
        );
        assert_eq!(
            mass_properties(&p, &i[..33], 1.0),
            Err(DiagnosticsError::OpenMesh)
        );
        // Two coincident opposite triangles: closed, zero volume.
        let flat = [0., 0., 0., 1., 0., 0., 0., 1., 0.];
        assert_eq!(
            mass_properties(&flat, &[0, 1, 2, 0, 2, 1], 1.0),
            Err(DiagnosticsError::ZeroVolume)
        );
    }
}
