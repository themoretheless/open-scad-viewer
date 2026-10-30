//! primitives algorithm: torus.
use super::*;
/// Closed watertight torus primitive centered at the origin in the XY plane with major radius
/// `major_radius` and tube radius `minor_radius`.
pub fn torus(
    major_radius: f64,
    minor_radius: f64,
    major_segments: usize,
    minor_segments: usize,
) -> Mesh {
    let maj = major_segments.clamp(3, 256);
    let min = minor_segments.clamp(3, 128);
    if !major_radius.is_finite()
        || !minor_radius.is_finite()
        || minor_radius <= 0.0
        || major_radius <= minor_radius
    {
        return empty();
    }
    let mut m = Mesh {
        positions: Vec::with_capacity(maj * min * 3),
        indices: Vec::with_capacity(maj * min * 6),
        uv: None,
    };
    for i in 0..maj {
        let u = std::f64::consts::TAU * (i as f64 / maj as f64);
        let (su, cu) = u.sin_cos();
        for j in 0..min {
            let v = std::f64::consts::TAU * (j as f64 / min as f64);
            let (sv, cv) = v.sin_cos();
            let r = major_radius + minor_radius * cv;
            m.positions.extend([r * cu, r * su, minor_radius * sv]);
        }
    }
    for i in 0..maj {
        let i_next = (i + 1) % maj;
        for j in 0..min {
            let j_next = (j + 1) % min;
            let a = i * min + j;
            let b = i_next * min + j;
            let c = i_next * min + j_next;
            let d = i * min + j_next;
            m.indices.extend([a, b, c, a, c, d]);
        }
    }
    clean(m).unwrap_or_else(|_| empty())
}
