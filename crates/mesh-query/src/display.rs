//! Display-space bounds and triangle measurements on borrowed render buffers.
type P = [f64; 3];
pub type Matrix = [f64; 16];
fn transform(m: &Matrix, [x, y, z]: P) -> Option<P> {
    let w = m[12] * x + m[13] * y + m[14] * z + m[15];
    let iw = if w.is_finite() && w.abs() > 1e-12 {
        1. / w
    } else {
        1.
    };
    let p = [
        (m[0] * x + m[1] * y + m[2] * z + m[3]) * iw,
        (m[4] * x + m[5] * y + m[6] * z + m[7]) * iw,
        (m[8] * x + m[9] * y + m[10] * z + m[11]) * iw,
    ];
    p.iter().all(|v| v.is_finite()).then_some(p)
}
fn vertex(vertices: &[f32], i: usize, m: &Matrix) -> Option<P> {
    let o = i.checked_mul(6)?;
    let p = vertices.get(o..o.checked_add(3)?)?;
    if p.iter().any(|v| !v.is_finite()) {
        return None;
    }
    transform(m, [p[0] as f64, p[1] as f64, p[2] as f64])
}
pub fn bounds(vertices: &[f32], m: &Matrix) -> Option<(P, P)> {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    let mut found = false;
    for i in 0..vertices.len().div_ceil(6) {
        if let Some(p) = vertex(vertices, i, m) {
            found = true;
            for k in 0..3 {
                min[k] = min[k].min(p[k]);
                max[k] = max[k].max(p[k]);
            }
        }
    }
    found.then_some((min, max))
}
fn triangle(vertices: &[f32], indices: &[u32], face: usize, m: &Matrix) -> Option<[P; 3]> {
    let o = face.checked_mul(3)?;
    let t = indices.get(o..o.checked_add(3)?)?;
    Some([
        vertex(vertices, t[0] as usize, m)?,
        vertex(vertices, t[1] as usize, m)?,
        vertex(vertices, t[2] as usize, m)?,
    ])
}
pub fn hit_point(
    vertices: &[f32],
    indices: &[u32],
    face: usize,
    m: &Matrix,
    weights: P,
) -> Option<P> {
    if weights.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let [a, b, c] = triangle(vertices, indices, face, m)?;
    let p = std::array::from_fn::<_, 3, _>(|k| {
        a[k] * weights[0] + b[k] * weights[1] + c[k] * weights[2]
    });
    p.iter().all(|v| v.is_finite()).then_some(p)
}
pub fn hit_normal(vertices: &[f32], indices: &[u32], face: usize, m: &Matrix) -> Option<P> {
    let [a, b, c] = triangle(vertices, indices, face, m)?;
    let ab = math_core::sub(b, a);
    let ac = math_core::sub(c, a);
    let n = math_core::cross(ab, ac);
    let length = n[0].hypot(n[1]).hypot(n[2]);
    let scale = ab[0] * ab[0]
        + ab[1] * ab[1]
        + ab[2] * ab[2]
        + ac[0] * ac[0]
        + ac[1] * ac[1]
        + ac[2] * ac[2];
    if !length.is_finite() || length <= scale * f64::EPSILON * 16. {
        return None;
    }
    Some(n.map(|x| x / length))
}
