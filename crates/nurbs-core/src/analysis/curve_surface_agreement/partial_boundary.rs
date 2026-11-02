//! Exact partial traversal of a literal original clamped tensor border.
//! No knot insertion, fitted border or sampled curve supplies a premise.
use super::*;
pub(super) fn verify(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    reversed: bool,
    max_work: u64,
) -> Result<Option<cad_predicates::BezierIdentityDecision>> {
    use cad_predicates::{
        AuthoredScalar, BsplineParameterMap, Limits, PredicateContext, SourceArena,
        ToleranceContext,
    };
    if max_work == 0
        || c.periodic
        || p.periodic
        || s.periodic_u
        || s.periodic_v
        || c.degree > 32
        || p.degree > 32
        || c.control_points.len() > 33
        || p.control_points.len() > 33
        || s.control_points.len() * s.control_points[0].len() > 4096
    {
        return Ok(None);
    }
    let sizes = [s.control_points.len(), s.control_points[0].len()];
    let degrees = [s.degree_u, s.degree_v];
    let knots = [&s.knots_u, &s.knots_v];
    for fixed in 0..2 {
        let free = 1 - fixed;
        // The map predicate consumes original normalized A coordinates. Do not
        // round a general physical-domain normalization into a false premise.
        if sizes[free] > 33
            || degrees[free] > 32
            || knots[free][degrees[free]] != 0.
            || knots[free][sizes[free]] != 1.
        {
            continue;
        }
        for end in 0..2 {
            let at = if end == 0 { 0 } else { sizes[fixed] - 1 };
            let value = knots[fixed][if end == 0 {
                degrees[fixed]
            } else {
                sizes[fixed]
            }];
            let clamped = if end == 0 {
                &knots[fixed][..=degrees[fixed]]
            } else {
                &knots[fixed][sizes[fixed]..]
            };
            if !clamped.iter().all(|&k| k == value)
                || !p
                    .control_points
                    .iter()
                    .all(|q| q[fixed] == value && q[free] >= 0. && q[free] <= 1.)
            {
                continue;
            }
            // Bind the full ORIGINAL surface, not only the selected row, and
            // both original curves/knots/traversal to this native context.
            let mut values = Vec::new();
            for (row, weights) in s.control_points.iter().zip(&s.weights) {
                for (q, w) in row.iter().zip(weights) {
                    values.extend_from_slice(q);
                    values.push(*w);
                }
            }
            let ski = [values.len(), values.len() + s.knots_u.len()];
            values.extend_from_slice(&s.knots_u);
            values.extend_from_slice(&s.knots_v);
            let ci = values.len();
            for (q, w) in c.control_points.iter().zip(&c.weights) {
                values.extend_from_slice(q);
                values.push(*w);
            }
            values.extend_from_slice(&c.knots);
            let pi = values.len();
            for (q, w) in p.control_points.iter().zip(&p.weights) {
                values.extend_from_slice(q);
                values.push(*w);
            }
            let pki = values.len();
            values.extend_from_slice(&p.knots);
            let cki = values.len();
            if reversed {
                values.extend(c.knots.iter().rev().map(|&k| -k));
            } else {
                values.extend_from_slice(&c.knots);
            }
            values.extend([
                s.degree_u as f64,
                s.degree_v as f64,
                c.degree as f64,
                p.degree as f64,
                fixed as f64,
                end as f64,
                reversed as u8 as f64,
            ]);
            let source = SourceArena::authored(
                "original-clamped-partial-border",
                1,
                values
                    .iter()
                    .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
                    .collect(),
            )
            .map_err(|e| crate::input(e.to_string()))?;
            let refs = |at: usize, n: usize| {
                (0..n)
                    .map(|i| source.leaf(at + i).unwrap())
                    .collect::<Vec<_>>()
            };
            let border = (0..sizes[free])
                .map(|i| {
                    let (u, v) = if fixed == 0 { (at, i) } else { (i, at) };
                    std::array::from_fn(|k| source.leaf(4 * (u * sizes[1] + v) + k).unwrap())
                })
                .collect::<Vec<_>>();
            let curve = (0..c.control_points.len())
                .map(|i| {
                    let i = if reversed {
                        c.control_points.len() - 1 - i
                    } else {
                        i
                    };
                    std::array::from_fn(|k| source.leaf(ci + 4 * i + k).unwrap())
                })
                .collect::<Vec<_>>();
            let map = (0..p.control_points.len())
                .map(|i| {
                    [
                        source.leaf(pi + 3 * i + free).unwrap(),
                        source.leaf(pi + 3 * i + 2).unwrap(),
                    ]
                })
                .collect::<Vec<_>>();
            let ak = refs(ski[free], knots[free].len());
            let bk = refs(cki, c.knots.len());
            let pk = refs(pki, p.knots.len());
            let tolerance = ToleranceContext::default_valid();
            let mut ctx = PredicateContext::new(
                &source,
                &tolerance,
                Limits {
                    max_work,
                    ..Limits::default()
                },
                None,
            );
            return cad_predicates::rational_bspline_mapped_curve_restriction_identity(
                &mut ctx,
                [&border, &curve],
                [&ak, &bk],
                [degrees[free], c.degree],
                BsplineParameterMap {
                    controls: &map,
                    knots: &pk,
                    degree: p.degree,
                },
            )
            .map(Some)
            .map_err(|e| crate::input(e.to_string()));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn high_degree_multispan_surface_border_keeps_exact_partial_original_clocks() {
        let s = Surface {
            degree_u: 12,
            degree_v: 4,
            knots_u: [vec![0.; 13], vec![0.5], vec![1.; 13]].concat(),
            knots_v: [vec![0.; 5], vec![1.; 5]].concat(),
            control_points: (0..14)
                .map(|i| {
                    (0..5)
                        .map(|j| vec![i as f64 / 13., j as f64 / 4., 0.])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 5]; 14],
            periodic_u: false,
            periodic_v: false,
        };
        let mut c = Curve::from_polyline(vec![vec![0., 0., 0.], vec![0., 0.5, 0.]]).unwrap();
        c.knots = vec![2., 2., 7., 7.];
        let p = Curve {
            degree: 1,
            knots: vec![-4., -4., 0., 0.],
            control_points: vec![vec![0., 0.], vec![0., 0.5]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let before = (s.clone(), c.clone(), p.clone());
        let report = verify(&c, &p, &s, false, 1000000).unwrap().unwrap();
        assert_eq!(report.outcome, cad_predicates::BezierIdentity::Equal);
        assert!(report.work_used > 0);
        assert_eq!(
            super::super::verify_exact(&c, &p, &s, false, 1000000)
                .unwrap()
                .unwrap()
                .outcome,
            cad_predicates::BezierIdentity::Equal
        );
        let mut backward = c.clone();
        backward.control_points.reverse();
        backward.weights.reverse();
        assert_eq!(
            verify(&backward, &p, &s, true, 1000000)
                .unwrap()
                .unwrap()
                .outcome,
            cad_predicates::BezierIdentity::Equal
        );
        assert_ne!(
            verify(&backward, &p, &s, false, 1000000)
                .unwrap()
                .unwrap()
                .outcome,
            cad_predicates::BezierIdentity::Equal
        );
        let mut back_map = p.clone();
        back_map.control_points.reverse();
        back_map.weights.reverse();
        assert_eq!(
            verify(&backward, &back_map, &s, false, 1000000)
                .unwrap()
                .unwrap()
                .outcome,
            cad_predicates::BezierIdentity::Equal
        );
        let mut rational = c.clone();
        rational.weights[1] = 2.;
        let mut rational_map = p.clone();
        rational_map.weights[1] = 2.;
        assert_eq!(
            verify(&rational, &rational_map, &s, false, 1000000)
                .unwrap()
                .unwrap()
                .outcome,
            cad_predicates::BezierIdentity::Equal
        );
        let mut changed = c.clone();
        changed.control_points[1][1] = 0.5f64.next_up();
        assert_ne!(
            verify(&changed, &p, &s, false, 1000000)
                .unwrap()
                .unwrap()
                .outcome,
            cad_predicates::BezierIdentity::Equal
        );
        let small = verify(&c, &p, &s, false, 1).unwrap().unwrap();
        assert!(small.work_used <= 1);
        assert_ne!(small.outcome, cad_predicates::BezierIdentity::Equal);
        assert_eq!((s, c, p), before);
    }
}
