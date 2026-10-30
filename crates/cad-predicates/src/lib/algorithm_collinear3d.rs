//! lib algorithm: collinear3d.
use super::*;
/// Return true when `(a, b, c)` are collinear within perpendicular distance `tol`
/// (exact cross-product zero check when `tol <= 0.0`).
pub fn collinear3d(a: [f64; 3], b: [f64; 3], c: [f64; 3], tol: f64) -> bool {
    let coords = [a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2]];
    if coords.iter().any(|v| !v.is_finite()) || !tol.is_finite() || tol < 0.0 {
        return false;
    }
    if tol == 0.0 {
        let scalars = [
            a[0], a[1], b[0], b[1], c[0], c[1],
            a[1], a[2], b[1], b[2], c[1], c[2],
            a[2], a[0], b[2], b[0], c[2], c[0],
        ]
        .iter()
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
        let Ok(source) = SourceArena::authored("collinear3d-ephemeral", 0, scalars) else {
            return false;
        };
        let tolerance = ToleranceContext::default_valid();
        for plane in 0..3 {
            let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
            let base = plane * 6;
            let p = |k: usize| [source.leaf(base + k).unwrap(), source.leaf(base + k + 1).unwrap()];
            let Ok(decision) = orient2d(&mut ctx, p(0), p(2), p(4)) else {
                return false;
            };
            if decision.outcome != Outcome::Sign(Sign::Zero) {
                return false;
            }
        }
        return true;
    }
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let bc = [c[0] - b[0], c[1] - b[1], c[2] - b[2]];
    let cross = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    let area2 = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
    let max_len = (ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2])
        .max(ac[0] * ac[0] + ac[1] * ac[1] + ac[2] * ac[2])
        .max(bc[0] * bc[0] + bc[1] * bc[1] + bc[2] * bc[2])
        .sqrt();
    if max_len <= tol {
        return true;
    }
    (area2 / max_len) <= tol
}
