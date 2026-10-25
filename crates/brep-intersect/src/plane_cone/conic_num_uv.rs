use super::*;

/// Homogeneous plane-UV numerator of the conic (x, y) = num / den.
pub(super) fn conic_num_uv(
    plane: &CanonicalPlane,
    q: [f64; 3],
    e1: [f64; 3],
    e2: [f64; 3],
    num: [[f64; 3]; 2],
    den: [f64; 3],
) -> [[f64; 3]; 2] {
    let u2 = plane.u_len * plane.u_len;
    let v2 = plane.v_len * plane.v_len;
    let au = dot(e1, plane.u) / u2;
    let bu = dot(e2, plane.u) / u2;
    let av = dot(e1, plane.v) / v2;
    let bv = dot(e2, plane.v) / v2;
    let u_q = dot(sub(q, plane.origin), plane.u) / u2;
    let v_q = dot(sub(q, plane.origin), plane.v) / v2;
    [
        std::array::from_fn(|i| u_q * den[i] + au * num[0][i] + bu * num[1][i]),
        std::array::from_fn(|i| v_q * den[i] + av * num[0][i] + bv * num[1][i]),
    ]
}
