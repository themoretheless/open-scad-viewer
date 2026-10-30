//! lib algorithm: segment_intersects_segment_2d.
use super::*;
/// Exact 2D closed segment intersection test using adaptive `orient2d` predicates.
/// Returns `true` if closed segments `[a0, a1]` and `[b0, b1]` intersect (either by proper
/// crossing or by endpoint/collinear touch).
pub fn segment_intersects_segment_2d(
    a0: [f64; 2],
    a1: [f64; 2],
    b0: [f64; 2],
    b1: [f64; 2],
) -> bool {
    let coords = [a0[0], a0[1], a1[0], a1[1], b0[0], b0[1], b1[0], b1[1]];
    if coords.iter().any(|v| !v.is_finite()) {
        return false;
    }
    let min_a = [a0[0].min(a1[0]), a0[1].min(a1[1])];
    let max_a = [a0[0].max(a1[0]), a0[1].max(a1[1])];
    let min_b = [b0[0].min(b1[0]), b0[1].min(b1[1])];
    let max_b = [b0[0].max(b1[0]), b0[1].max(b1[1])];
    if aabb2_disjoint(min_a, max_a, min_b, max_b) {
        return false;
    }
    let scalars = coords
        .iter()
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let Ok(source) = SourceArena::authored("seg-inter-2d-ephemeral", 0, scalars) else {
        return false;
    };
    let tolerance = ToleranceContext::default_valid();
    let pair = |offset: usize| [source.leaf(offset).unwrap(), source.leaf(offset + 1).unwrap()];
    let orient = |p: usize, q: usize, r: usize| -> Option<Sign> {
        let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
        match orient2d(&mut ctx, pair(p), pair(q), pair(r)) {
            Ok(Decision {
                outcome: Outcome::Sign(s),
                ..
            }) => Some(s),
            _ => None,
        }
    };
    let (Some(o1), Some(o2), Some(o3), Some(o4)) = (
        orient(0, 2, 4),
        orient(0, 2, 6),
        orient(4, 6, 0),
        orient(4, 6, 2),
    ) else {
        return false;
    };
    if o1 != o2 && o3 != o4 && o1 != Sign::Zero && o2 != Sign::Zero && o3 != Sign::Zero && o4 != Sign::Zero {
        return true;
    }
    let on_segment = |p: [f64; 2], s0: [f64; 2], s1: [f64; 2]| -> bool {
        p[0] >= s0[0].min(s1[0])
            && p[0] <= s0[0].max(s1[0])
            && p[1] >= s0[1].min(s1[1])
            && p[1] <= s0[1].max(s1[1])
    };
    (o1 == Sign::Zero && on_segment(b0, a0, a1))
        || (o2 == Sign::Zero && on_segment(b1, a0, a1))
        || (o3 == Sign::Zero && on_segment(a0, b0, b1))
        || (o4 == Sign::Zero && on_segment(a1, b0, b1))
}
