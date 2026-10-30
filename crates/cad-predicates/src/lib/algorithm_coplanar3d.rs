//! lib algorithm: coplanar3d.
use super::*;
/// Return true when `d` lies within signed distance `tol` of the plane spanned by
/// `(a, b, c)`, or when `(a, b, c)` are collinear within `tol`. Uses exact
/// orientation when `tol <= 0.0`.
pub fn coplanar3d(a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3], tol: f64) -> bool {
    let coords = [
        a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2], d[0], d[1], d[2],
    ];
    if coords.iter().any(|v| !v.is_finite()) || !tol.is_finite() || tol < 0.0 {
        return false;
    }
    if tol == 0.0 {
        let scalars = coords
            .iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect();
        let Ok(source) = SourceArena::authored("coplanar3d-ephemeral", 0, scalars) else {
            return false;
        };
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
        let pt = |offset: usize| {
            [
                source.leaf(offset).unwrap(),
                source.leaf(offset + 1).unwrap(),
                source.leaf(offset + 2).unwrap(),
            ]
        };
        return matches!(
            orient3d(&mut ctx, pt(0), pt(3), pt(6), pt(9)),
            Ok(Decision {
                outcome: Outcome::Sign(Sign::Zero),
                ..
            })
        );
    }
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let ad = [d[0] - a[0], d[1] - a[1], d[2] - a[2]];
    let n = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    let n_len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if n_len <= tol * tol {
        return true;
    }
    let det = ad[0] * n[0] + ad[1] * n[1] + ad[2] * n[2];
    (det.abs() / n_len) <= tol
}
