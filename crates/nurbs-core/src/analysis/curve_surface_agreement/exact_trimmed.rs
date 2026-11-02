//! Exact composition using original trim clocks, never rounded extracted poles.
use super::*;
use cad_predicates::{
    AuthoredScalar, BezierIdentityDecision, Limits, PredicateContext, SourceArena, ToleranceContext,
};
/// Exact agreement on the two declared physical source intervals. Reversal
/// reflects the world clock about its ORIGINAL domain, exactly once. A proof
/// includes restricted UV hull membership but not loop/topology/solid validity.
pub fn verify_exact_trimmed(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    reversed: bool,
    curve_range: [f64; 2],
    pcurve_range: [f64; 2],
    max_work: u64,
) -> Result<Option<BezierIdentityDecision>> {
    c.validate()?;
    p.validate()?;
    s.validate()?;
    check(
        max_work > 0 && max_work <= cad_predicates::MAX_WORK,
        "Invalid exact trimmed lift work allowance",
    )?;
    for (d, r) in [(c.domain(), curve_range), (p.domain(), pcurve_range)] {
        check(
            r[0] != r[1] && r.iter().all(|t| t.is_finite() && *t >= d[0] && *t <= d[1]),
            "Exact trimmed lift interval is outside its original domain",
        )?;
    }
    if let Some(proof) =
        super::trimmed_boundary::verify(c, p, s, reversed, curve_range, pcurve_range, max_work)?
    {
        return Ok(Some(proof));
    }
    let bezier = |c: &Curve, max: usize, dim: usize| {
        let d = c.domain();
        !c.periodic
            && (1..=max).contains(&c.degree)
            && c.control_points.len() == c.degree + 1
            && c.control_points.iter().all(|p| p.len() == dim)
            && c.knots[..=c.degree].iter().all(|k| *k == d[0])
            && c.knots[c.degree + 1..].iter().all(|k| *k == d[1])
    };
    let domain = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    if !bezier(c, 32, 3)
        || !bezier(p, 8, 2)
        || s.periodic_u
        || s.periodic_v
        || !(1..=8).contains(&s.degree_u)
        || !(1..=8).contains(&s.degree_v)
        || s.control_points.len() != s.degree_u + 1
        || s.control_points[0].len() != s.degree_v + 1
        || s.control_points.iter().flatten().any(|p| p.len() != 3)
        || s.knots_u[..=s.degree_u].iter().any(|k| *k != domain[0][0])
        || s.knots_u[s.degree_u + 1..]
            .iter()
            .any(|k| *k != domain[0][1])
        || s.knots_v[..=s.degree_v].iter().any(|k| *k != domain[1][0])
        || s.knots_v[s.degree_v + 1..]
            .iter()
            .any(|k| *k != domain[1][1])
    {
        return Ok(None);
    }
    let mut values = Vec::new();
    for (p, w) in c.control_points.iter().zip(&c.weights) {
        values.extend(p);
        values.push(*w);
    }
    for (p, w) in p.control_points.iter().zip(&p.weights) {
        values.extend(p);
        values.push(*w);
    }
    for (row, weights) in s.control_points.iter().zip(&s.weights) {
        for (p, w) in row.iter().zip(weights) {
            values.extend(p);
            values.push(*w);
        }
    }
    values.extend(domain.into_iter().flatten());
    values.extend(c.domain());
    values.extend(curve_range);
    values.extend(p.domain());
    values.extend(pcurve_range);
    let arena = SourceArena::authored(
        "original-trimmed-edge-lift",
        1,
        values
            .iter()
            .map(|v: &f64| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| crate::input("Invalid exact trimmed lift source"))?;
    let mut i = 0;
    let mut leaf = || {
        let r = arena.leaf(i).unwrap();
        i += 1;
        r
    };
    let cc = (0..c.control_points.len())
        .map(|_| std::array::from_fn(|_| leaf()))
        .collect::<Vec<_>>();
    let pp = (0..p.control_points.len())
        .map(|_| std::array::from_fn(|_| leaf()))
        .collect::<Vec<_>>();
    let ss = s
        .control_points
        .iter()
        .map(|r| {
            r.iter()
                .map(|_| std::array::from_fn(|_| leaf()))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let dd = std::array::from_fn(|_| std::array::from_fn(|_| leaf()));
    let clocks = std::array::from_fn(|_| std::array::from_fn(|_| std::array::from_fn(|_| leaf())));
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
    cad_predicates::rational_bezier_trimmed_composition_identity(
        &mut ctx, &cc, &pp, &ss, dd, clocks, reversed,
    )
    .map(Some)
    .map_err(|_| crate::input("Invalid exact trimmed lift predicate"))
}
