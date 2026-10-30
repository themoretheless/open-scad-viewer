//! primitives algorithm: capsule.
use super::*;
/// Closed watertight capsule (cylindrical shaft of length `height` centered at the origin along Z
/// capped by two hemispheres of `radius`).
pub fn capsule(radius: f64, height: f64, segments: usize) -> Mesh {
    let n = segments.clamp(4, 256);
    if !radius.is_finite() || radius <= 0.0 || !height.is_finite() || height < 0.0 {
        return empty();
    }
    let hemi_rings = (n / 4).max(2);
    let half_h = 0.5 * height;
    let has_cyl = half_h > 1e-12;
    let total_rings = 2 * hemi_rings + usize::from(has_cyl);
    let mut m = Mesh {
        positions: Vec::with_capacity((total_rings + 1) * n * 3),
        indices: Vec::with_capacity(total_rings * n * 6),
        uv: None,
    };
    for j in 0..=total_rings {
        let (r, z) = if j == 0 {
            (0.0, half_h + radius)
        } else if j == total_rings {
            (0.0, -half_h - radius)
        } else if j <= hemi_rings {
            let theta = std::f64::consts::FRAC_PI_2 * (j as f64 / hemi_rings as f64);
            (radius * theta.sin(), half_h + radius * theta.cos())
        } else {
            let k = j - hemi_rings - usize::from(has_cyl);
            let theta =
                std::f64::consts::FRAC_PI_2 * (1.0 + k as f64 / hemi_rings as f64);
            (radius * theta.sin(), -half_h + radius * theta.cos())
        };
        for i in 0..n {
            let phi = std::f64::consts::TAU * (i as f64 / n as f64);
            m.positions.extend([r * phi.cos(), r * phi.sin(), z]);
        }
    }
    for j in 0..total_rings {
        for i in 0..n {
            let a = j * n + i;
            let b = j * n + (i + 1) % n;
            m.indices.extend([a, a + n, b + n, a, b + n, b]);
        }
    }
    clean(m).unwrap_or_else(|_| empty())
}
