//! Exact natural tensor boundary rows, without extracted/reclocked curves.
use super::*;
use cad_predicates::{AuthoredScalar, Limits, PredicateContext, SourceArena, ToleranceContext};
pub(super) fn verify(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    reversed: bool,
    cr: [f64; 2],
    pr: [f64; 2],
    max_work: u64,
) -> Result<Option<cad_predicates::BezierIdentityDecision>> {
    let pd = p.domain();
    if c.periodic
        || p.periodic
        || s.periodic_u
        || s.periodic_v
        || p.degree != 1
        || p.control_points.len() != 2
        || p.weights[0] != p.weights[1]
        || c.control_points.iter().any(|p| p.len() != 3)
        || p.control_points.iter().any(|p| p.len() != 2)
        || !p.knots[..=1].iter().all(|k| *k == pd[0])
        || !p.knots[2..].iter().all(|k| *k == pd[1])
    {
        return Ok(None);
    }
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let degrees = [s.degree_u, s.degree_v];
    let knots = [&s.knots_u, &s.knots_v];
    for fixed in 0..2 {
        let free = 1 - fixed;
        let a = &p.control_points[0];
        let z = &p.control_points[1];
        if a[fixed] != z[fixed] || a[free] == z[free] {
            continue;
        }
        let low = knots[fixed][degrees[fixed]];
        let high = knots[fixed][counts[fixed]];
        let index = if a[fixed] == low && knots[fixed][..=degrees[fixed]].iter().all(|k| *k == low)
        {
            0
        } else if a[fixed] == high && knots[fixed][counts[fixed]..].iter().all(|k| *k == high) {
            counts[fixed] - 1
        } else {
            continue;
        };
        let mut values = Vec::new();
        for (p, w) in c.control_points.iter().zip(&c.weights) {
            values.extend(p);
            values.push(*w);
        }
        for i in 0..counts[free] {
            let (u, v) = if fixed == 0 { (index, i) } else { (i, index) };
            values.extend(&s.control_points[u][v]);
            values.push(s.weights[u][v]);
        }
        let ck = values.len();
        values.extend(&c.knots);
        let bk = values.len();
        values.extend(knots[free]);
        let line = values.len();
        values.extend([a[free], z[free]]);
        values.extend(pd);
        values.extend(pr);
        values.extend(cr);
        let arena = SourceArena::authored(
            "original-trimmed-tensor-boundary",
            1,
            values
                .iter()
                .map(|v: &f64| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .map_err(|_| crate::input("Invalid original tensor boundary source"))?;
        let rows = |start: usize, n: usize| {
            (0..n)
                .map(|i| std::array::from_fn(|k| arena.leaf(4 * (start + i) + k).unwrap()))
                .collect::<Vec<_>>()
        };
        let refs = |start: usize, n: usize| {
            (start..start + n)
                .map(|i| arena.leaf(i).unwrap())
                .collect::<Vec<_>>()
        };
        let aa = rows(0, c.control_points.len());
        let bb = rows(c.control_points.len(), counts[free]);
        let ak = refs(ck, c.knots.len());
        let bkk = refs(bk, knots[free].len());
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
        return cad_predicates::rational_bspline_trimmed_boundary_identity(
            &mut ctx,
            [&aa, &bb],
            [&ak, &bkk],
            [c.degree, degrees[free]],
            [arena.leaf(line).unwrap(), arena.leaf(line + 1).unwrap()],
            std::array::from_fn(|i| arena.leaf(line + 2 + i).unwrap()),
            [arena.leaf(line + 6).unwrap(), arena.leaf(line + 7).unwrap()],
            reversed,
        )
        .map(Some)
        .map_err(|_| crate::input("Invalid original tensor boundary clock proof"));
    }
    Ok(None)
}
