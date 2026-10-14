//! Exact supporting-line admission for a shared endpoint, including oblique
//! higher-order contacts. This never applies to an interior line/curve event.
use cad_predicates::{
    AuthoredScalar, Limits, Outcome, PredicateContext, Sign, SourceArena, ToleranceContext,
    orient2d,
};
fn separates(flat: &[[f64; 2]], curved: &[[f64; 2]], point: [f64; 2]) -> Option<bool> {
    if flat[0] == *flat.last()? {
        return Some(false);
    }
    let values = flat
        .iter()
        .chain(curved)
        .flatten()
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let source = SourceArena::authored("represented-profile-joint-hulls", 1, values).ok()?;
    let refs = |i: usize| std::array::from_fn(|k| source.leaf(i * 2 + k).unwrap());
    let tolerance = ToleranceContext::default_valid();
    let mut limits = Limits::default();
    limits.max_work = 10000;
    let mut ctx = PredicateContext::new(&source, &tolerance, limits, None);
    let mut sign = |i: usize| match orient2d(&mut ctx, refs(0), refs(flat.len() - 1), refs(i))
        .ok()?
        .outcome
    {
        Outcome::Sign(s) => Some(s),
        Outcome::Indeterminate(_) => None,
    };
    for i in 0..flat.len() {
        if sign(i)? != Sign::Zero {
            return Some(false);
        }
    }
    let other = if curved[0] == point {
        curved.len() - 1
    } else {
        0
    };
    let side = sign(flat.len() + other)?;
    if side == Sign::Zero {
        return Some(false);
    }
    for i in 0..curved.len() {
        let s = sign(flat.len() + i)?;
        if s != side && s != Sign::Zero {
            return Some(false);
        }
    }
    Some(true)
}
pub(super) fn shared_line_separates(a: &[[f64; 2]], b: &[[f64; 2]], point: [f64; 2]) -> bool {
    separates(a, b, point) == Some(true) || separates(b, a, point) == Some(true)
}
/// Opposite closed control half-planes imply strict separation of the open
/// positive-weight Bernstein spans, even with nonshared controls on the plane.
pub(super) fn opposite_halfplanes(a: &[[f64; 2]], b: &[[f64; 2]], point: [f64; 2]) -> bool {
    fn inspect(a: &[[f64; 2]], b: &[[f64; 2]], point: [f64; 2]) -> Option<bool> {
        let points = std::iter::once(point)
            .chain(a.iter().copied())
            .chain(b.iter().copied())
            .collect::<Vec<_>>();
        let source = SourceArena::authored(
            "represented-profile-shared-halfplanes",
            1,
            points
                .iter()
                .flatten()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .ok()?;
        let refs = |i: usize| std::array::from_fn(|k| source.leaf(i * 2 + k).unwrap());
        let tolerance = ToleranceContext::default_valid();
        let mut limits = Limits::default();
        limits.max_work = 10000;
        let mut ctx = PredicateContext::new(&source, &tolerance, limits, None);
        let other_a = 1 + if a[0] == point { a.len() - 1 } else { 0 };
        let other_b = 1 + a.len() + if b[0] == point { b.len() - 1 } else { 0 };
        for candidate in 1..points.len() {
            if points[candidate] == point {
                continue;
            }
            let mut sign = |i: usize| match orient2d(&mut ctx, refs(0), refs(candidate), refs(i))
                .ok()?
                .outcome
            {
                Outcome::Sign(s) => Some(s),
                Outcome::Indeterminate(_) => None,
            };
            let sa = sign(other_a)?;
            let sb = sign(other_b)?;
            if sa == Sign::Zero || sb == Sign::Zero || sa == sb {
                continue;
            }
            let mut valid = true;
            for i in 0..a.len() {
                let s = sign(1 + i)?;
                if s != Sign::Zero && s != sa {
                    valid = false;
                    break;
                }
            }
            if !valid {
                continue;
            }
            for i in 0..b.len() {
                let s = sign(1 + a.len() + i)?;
                if s != Sign::Zero && s != sb {
                    valid = false;
                    break;
                }
            }
            if valid {
                return Some(true);
            }
        }
        Some(false)
    }
    inspect(a, b, point) == Some(true)
}
