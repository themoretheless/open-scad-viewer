//! Native original periodic/nonclamped border ownership, not sampled lifting.
use super::*;
pub(super) fn verify(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    reversed: bool,
    max_work: u64,
) -> Result<Option<cad_predicates::BezierIdentityDecision>> {
    use cad_predicates::{
        AuthoredScalar, BezierIdentity, BezierIdentityDecision, BsplineStripLocation, Limits,
        ParameterIdentity, PredicateContext, Reason, SourceArena, ToleranceContext,
    };
    if p.degree != 1
        || p.control_points.len() != 2
        || p.weights[0] != p.weights[1]
        || p.periodic
        || c.control_points.len() > 33
        || s.control_points.len() > 33
        || s.control_points[0].len() > 33
    {
        return Ok(None);
    }
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let degrees = [s.degree_u, s.degree_v];
    let knots = [&s.knots_u, &s.knots_v];
    let periodic = [s.periodic_u, s.periodic_v];
    for fixed in 0..2 {
        let free = 1 - fixed;
        let a = &p.control_points[0];
        let b = &p.control_points[1];
        if a[fixed] != b[fixed]
            || !matches!(a[fixed],v if v==knots[fixed][degrees[fixed]]||v==knots[fixed][counts[fixed]])
        {
            continue;
        }
        let forward = a[free] == knots[free][degrees[free]] && b[free] == knots[free][counts[free]];
        let backward =
            b[free] == knots[free][degrees[free]] && a[free] == knots[free][counts[free]];
        if !forward && !backward {
            continue;
        }
        let clamped = knots[fixed][..=degrees[fixed]]
            .iter()
            .all(|&v| v == knots[fixed][degrees[fixed]])
            && knots[fixed][counts[fixed]..]
                .iter()
                .all(|&v| v == knots[fixed][counts[fixed]]);
        let original_bezier = |k: &[f64], p: usize, n: usize| {
            n == p + 1 && k[..=p].iter().all(|&x| x == k[p]) && k[n..].iter().all(|&x| x == k[n])
        };
        if clamped
            && !periodic[0]
            && !periodic[1]
            && !c.periodic
            && ((original_bezier(&c.knots, c.degree, c.control_points.len())
                && original_bezier(knots[free], degrees[free], counts[free]))
                || (c.degree == degrees[free]
                    && c.control_points.len() == counts[free]
                    && c.knots == *knots[free]))
        {
            continue;
        } // preserve existing Bezier composition and exact mismatch witnesses
        let reverse = backward ^ reversed;
        let mut values = Vec::new();
        for i in 0..c.control_points.len() {
            let j = if reverse {
                c.control_points.len() - 1 - i
            } else {
                i
            };
            values.extend_from_slice(&c.control_points[j]);
            values.push(c.weights[j]);
        }
        let si = values.len();
        for i in 0..counts[free] {
            for j in 0..counts[fixed] {
                let (u, v) = if fixed == 0 { (j, i) } else { (i, j) };
                values.extend_from_slice(&s.control_points[u][v]);
                values.push(s.weights[u][v]);
            }
        }
        let cki = values.len();
        values.extend(if reverse {
            c.knots.iter().rev().map(|v| -v).collect::<Vec<_>>()
        } else {
            c.knots.clone()
        });
        let aki = values.len();
        values.extend_from_slice(knots[free]);
        let ki = values.len();
        values.extend_from_slice(knots[fixed]);
        let ti = values.len();
        values.push(a[fixed]);
        values.extend(p.control_points.iter().flatten().copied());
        values.extend_from_slice(&p.weights);
        values.extend_from_slice(&p.knots);
        let source = SourceArena::authored(
            "original-bspline-isoparametric-trace",
            1,
            values
                .iter()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .map_err(|e| crate::input(e.to_string()))?;
        let refs = |at: usize, len: usize| {
            (0..len)
                .map(|i| source.leaf(at + i).unwrap())
                .collect::<Vec<_>>()
        };
        let curve = (0..c.control_points.len())
            .map(|i| std::array::from_fn(|k| source.leaf(4 * i + k).unwrap()))
            .collect::<Vec<_>>();
        let strip = (0..counts[free])
            .map(|i| {
                (0..counts[fixed])
                    .map(|j| {
                        std::array::from_fn(|k| {
                            source.leaf(si + 4 * (i * counts[fixed] + j) + k).unwrap()
                        })
                    })
                    .collect()
            })
            .collect::<Vec<_>>();
        let ck = refs(cki, c.knots.len());
        let ak = refs(aki, knots[free].len());
        let k = refs(ki, knots[fixed].len());
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
        let common = if c.degree == degrees[free] && c.control_points.len() == counts[free] {
            cad_predicates::affine_bspline_knot_basis_identity(&mut ctx, [&ck, &ak], c.degree)
                .map_err(|e| crate::input(e.to_string()))?
                .outcome
                == ParameterIdentity::Equal
        } else {
            false
        };
        // Either declaration requires ORIGINAL endpoint closure, even if the
        // equivalent other representation stores a clamped closed chain.
        if c.periodic || periodic[free] {
            let closure = cad_predicates::rational_bspline_cartesian_endpoint_jet_identity(
                &mut ctx, &curve, &ck, c.degree, 0,
            )
            .map_err(|e| crate::input(e.to_string()))?;
            if closure.outcome != BezierIdentity::Equal {
                return Ok(Some(BezierIdentityDecision {
                    outcome: BezierIdentity::Indeterminate(Reason::MissingProof),
                    work_used: ctx.work_used(),
                    context: ctx.identity(),
                }));
            }
        }
        let span = if a[fixed] == knots[fixed][counts[fixed]] {
            (degrees[fixed]..counts[fixed])
                .rev()
                .find(|&i| knots[fixed][i] < knots[fixed][i + 1])
        } else {
            (degrees[fixed]..counts[fixed]).find(|&i| knots[fixed][i] < knots[fixed][i + 1])
        }
        .unwrap();
        let location = BsplineStripLocation {
            span,
            parameter: source.leaf(ti).unwrap(),
            inward_positive: true,
        };
        let proof = if common {
            cad_predicates::rational_bspline_isoparametric_trace_identity(
                &mut ctx,
                &curve,
                &strip,
                &k,
                degrees[fixed],
                location,
                periodic[fixed],
            )
        } else {
            cad_predicates::rational_bspline_independent_trace_identity(
                &mut ctx,
                &curve,
                &strip,
                &k,
                degrees[fixed],
                location,
                periodic[fixed],
                [&ck, &ak],
                [c.degree, degrees[free]],
            )
        }
        .map_err(|e| crate::input(e.to_string()))?;
        return Ok(Some(proof));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_predicates::BezierIdentity;
    #[test]
    fn independent_original_world_curve_binds_periodic_cross_trace_and_budget() {
        let xy = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]];
        let s = Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: (0..9).map(|i| i as f64).collect(),
            control_points: (0..2)
                .map(|z| {
                    (0..6)
                        .map(|j| vec![xy[j % 4][0], xy[j % 4][1], z as f64])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 6]; 2],
            periodic_u: false,
            periodic_v: true,
        };
        let p = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0., 2.], vec![1., 2.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let c = Curve {
            degree: 2,
            knots: vec![4., 4., 4., 5., 6., 6., 6.],
            control_points: [0., 0.25, 0.75, 1.]
                .iter()
                .map(|&z| vec![0.5, 0.5, z])
                .collect(),
            weights: vec![3.; 4],
            periodic: false,
        };
        let saved = (c.clone(), p.clone(), s.clone());
        let run = |c: &Curve, rev, budget| {
            super::super::verify_exact(c, &p, &s, rev, budget)
                .unwrap()
                .unwrap()
        };
        let proof = run(&c, false, 1000000);
        assert_eq!(proof.outcome, BezierIdentity::Equal, "{proof:?}");
        assert_ne!(
            run(&c, false, proof.work_used - 1).outcome,
            BezierIdentity::Equal
        );
        assert_ne!(run(&c, false, 0).outcome, BezierIdentity::Equal);
        let mut reverse = c.clone();
        reverse.control_points.reverse();
        reverse.weights.reverse();
        assert_eq!(run(&reverse, true, 1000000).outcome, BezierIdentity::Equal);
        let mut bad = c.clone();
        bad.control_points[1][0] = f64::from_bits(0.5f64.to_bits() + 1);
        assert_ne!(run(&bad, false, 1000000).outcome, BezierIdentity::Equal);
        assert_eq!((c, p, s), saved);
    }
}
