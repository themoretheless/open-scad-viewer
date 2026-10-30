//! lib algorithm: point_in_triangle_2d.
use super::*;
/// Exact 2D point-in-triangle classification using adaptive `orient2d` predicates.
/// Returns `Sign::Positive` when `p` is strictly inside triangle `(a, b, c)`,
/// `Sign::Zero` when `p` lies on a boundary edge or vertex, and `Sign::Negative`
/// when `p` is outside (or if `(a, b, c)` is degenerate), independent of whether
/// `(a, b, c)` is ordered counter-clockwise or clockwise.
pub fn point_in_triangle_2d(p: [f64; 2], a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> Sign {
    let coords = [p[0], p[1], a[0], a[1], b[0], b[1], c[0], c[1]];
    if coords.iter().any(|v| !v.is_finite()) {
        return Sign::Negative;
    }
    let scalars = coords
        .iter()
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let Ok(source) = SourceArena::authored("pt-in-tri-2d-ephemeral", 0, scalars) else {
        return Sign::Negative;
    };
    let tolerance = ToleranceContext::default_valid();
    let pair = |offset: usize| [source.leaf(offset).unwrap(), source.leaf(offset + 1).unwrap()];
    let orient = |q0: usize, q1: usize, q2: usize| -> Option<Sign> {
        let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
        match orient2d(&mut ctx, pair(q0), pair(q1), pair(q2)) {
            Ok(Decision {
                outcome: Outcome::Sign(s),
                ..
            }) => Some(s),
            _ => None,
        }
    };
    let Some(tri_orient) = orient(2, 4, 6) else {
        return Sign::Negative;
    };
    if tri_orient == Sign::Zero {
        return Sign::Negative;
    }
    let (Some(s0), Some(s1), Some(s2)) = (orient(2, 4, 0), orient(4, 6, 0), orient(6, 2, 0)) else {
        return Sign::Negative;
    };
    if [s0, s1, s2].iter().all(|&s| s == tri_orient) {
        return Sign::Positive;
    }
    if [s0, s1, s2]
        .iter()
        .all(|&s| s == tri_orient || s == Sign::Zero)
    {
        return Sign::Zero;
    }
    Sign::Negative
}
