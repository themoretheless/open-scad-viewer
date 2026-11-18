use super::*;



pub const NURBS_BOOLEAN_CAPABILITY_V1: &str = "nurbs-boolean-bezier-le3/1";
pub const NURBS_BOOLEAN_CAPABILITY: &str = "nurbs-boolean-bezier-le3/2";
pub const NURBS_BOOLEAN_CAPABILITY_V3: &str = "nurbs-boolean-bezier-le3/3";
pub const NURBS_BOOLEAN_CAPABILITY_V4: &str = "nurbs-boolean-bezier-le3/4";
pub const NURBS_BOOLEAN_CAPABILITY_V5: &str = "nurbs-boolean-bezier-le3/5";
pub const NURBS_BOOLEAN_CAPABILITY_V7: &str = "nurbs-boolean-bezier-le3/7";
pub const NURBS_BOOLEAN_CAPABILITY_V8: &str = "nurbs-boolean-bezier-le3/8";
/// Product Boolean successor over nurbs-ss/1 for the admitted multispan×affine cell.
pub const NURBS_BOOLEAN_SS_CAPABILITY: &str = "nurbs-boolean/1";
/// Authority for nurbs-boolean/1 general authorship. Distinct from finite `/8`.
pub const GENERAL_NURBS_BOOLEAN_AUTHORITY: &str = "author-general-nurbs-boolean";
pub const NURBS_BOOLEAN_V1_MATURITY: G6Maturity = G6Maturity::Unavailable;














pub(crate) fn is_nurbs_boolean_candidate(a: &Model, b: &Model) -> bool {
    a.faces
        .iter()
        .chain(&b.faces)
        .all(|face| is_bezier_le3_positive(&face.surface))
}


pub(super) fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub(super) fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub(super) fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub(super) fn norm(a: [f64; 3]) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
pub(super) fn normalize(a: [f64; 3]) -> Option<[f64; 3]> {
    let n = norm(a);
    if !n.is_finite() || n == 0. {
        None
    } else {
        Some([a[0] / n, a[1] / n, a[2] / n])
    }
}








