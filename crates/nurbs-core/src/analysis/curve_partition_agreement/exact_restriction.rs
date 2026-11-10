//! Whole-interval rational root identity; independent from topology admission.
use crate::{Result, check, curve::Curve};
use cad_predicates::{
    AuthoredScalar, BezierIdentityDecision, Limits, PredicateContext, SourceArena, ToleranceContext,
};

/// Reprove a clamped rational Bezier child against an interval on its ORIGINAL
/// world curve. Different child degree/gauge are supported. None is an
/// unsupported representation; resource exhaustion remains Indeterminate.
/// Candidate domain is normalized independently, and reverse reverses candidate
/// traversal only. This alone proves neither regularity nor shared incidence.
pub fn verify_exact_bezier_restriction(
    original: &Curve,
    child: &Curve,
    range: [f64; 2],
    reverse: bool,
    max_work: u64,
) -> Result<Option<BezierIdentityDecision>> {
    original.validate()?;
    child.validate()?;
    check(
        range.iter().all(|x| x.is_finite())
            && range[0] < range[1]
            && max_work > 0
            && max_work <= cad_predicates::MAX_WORK,
        "Invalid exact restriction range or work allowance",
    )?;
    let bezier = |c: &Curve| {
        let d = c.domain();
        !c.periodic
            && (1..=32).contains(&c.degree)
            && c.control_points.len() == c.degree + 1
            && c.control_points
                .iter()
                .all(|p| p.len() == c.control_points[0].len())
            && (2..=3).contains(&c.control_points[0].len())
            && c.knots[..=c.degree].iter().all(|k| *k == d[0])
            && c.knots[c.degree + 1..].iter().all(|k| *k == d[1])
    };
    if !bezier(original)
        || !bezier(child)
        || original.control_points[0].len() != child.control_points[0].len()
    {
        return Ok(None);
    }
    let mut values = Vec::new();
    for c in [original, child] {
        for (p, w) in c.control_points.iter().zip(&c.weights) {
            values.extend(p.iter().copied());
            if p.len() == 2 {
                values.push(0.);
            }
            values.push(*w);
        }
    }
    values.extend(original.domain());
    values.extend(range);
    let arena = SourceArena::authored(
        "original-rational-root-restriction",
        1,
        values
            .iter()
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    )
    .map_err(|_| crate::Error::new("NURBS_INVALID_INPUT", "Invalid exact root source leaves"))?;
    let refs = (0..values.len())
        .map(|i| arena.leaf(i))
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| crate::Error::new("NURBS_INVALID_INPUT", "Invalid exact root leaf index"))?;
    let n = original.control_points.len();
    let m = child.control_points.len();
    let a = refs[..4 * n]
        .chunks_exact(4)
        .map(|p| p.try_into().unwrap())
        .collect::<Vec<_>>();
    let b = refs[4 * n..4 * (n + m)]
        .chunks_exact(4)
        .map(|p| p.try_into().unwrap())
        .collect::<Vec<_>>();
    let end = 4 * (n + m);
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work,
            ..Limits::default()
        },
        None,
    );
    cad_predicates::rational_bezier_restriction_identity(
        &mut ctx,
        &a,
        &b,
        [refs[end], refs[end + 1]],
        [refs[end + 2], refs[end + 3]],
        reverse,
    )
    .map(Some)
    .map_err(|_| {
        crate::Error::new(
            "NURBS_INVALID_INPUT",
            "Invalid exact rational root restriction",
        )
    })
}
