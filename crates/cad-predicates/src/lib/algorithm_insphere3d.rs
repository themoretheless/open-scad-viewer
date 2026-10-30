//! lib algorithm: insphere3d.
use super::*;
/// Adaptive exact 3D insphere predicate over finite binary64 coordinates.
/// Returns `Sign::Positive` when `e` is strictly inside the sphere through `(a, b, c, d)`
/// (assuming `orient3d(a, b, c, d) > 0`), `Sign::Negative` when strictly outside, and `Sign::Zero` when cospherical.
pub fn insphere3d(
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
    d: [f64; 3],
    e: [f64; 3],
) -> Sign {
    let coords = [
        a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2], d[0], d[1], d[2], e[0], e[1], e[2],
    ];
    if coords.iter().any(|v| !v.is_finite()) {
        return Sign::Zero;
    }
    let scalars = coords
        .iter()
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let Ok(source) = SourceArena::authored("insphere3d-ephemeral", 0, scalars) else {
        return Sign::Zero;
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
    match insphere3d_authored(&mut ctx, pt(0), pt(3), pt(6), pt(9), pt(12)) {
        Ok(Decision {
            outcome: Outcome::Sign(sign),
            ..
        }) => sign,
        _ => Sign::Zero,
    }
}
