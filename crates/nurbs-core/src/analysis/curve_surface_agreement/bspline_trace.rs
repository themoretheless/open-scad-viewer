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
        if (!forward && !backward)
            || c.degree != degrees[free]
            || c.control_points.len() != counts[free]
            || c.periodic != periodic[free]
        {
            continue;
        }
        let clamped = knots[fixed][..=degrees[fixed]]
            .iter()
            .all(|&v| v == knots[fixed][degrees[fixed]])
            && knots[fixed][counts[fixed]..]
                .iter()
                .all(|&v| v == knots[fixed][counts[fixed]]);
        if clamped && !periodic[0] && !periodic[1] && !c.periodic {
            continue;
        } // preserve all existing Bezier paths
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
        let curve = (0..counts[free])
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
        let basis =
            cad_predicates::affine_bspline_knot_basis_identity(&mut ctx, [&ck, &ak], c.degree)
                .map_err(|e| crate::input(e.to_string()))?;
        if basis.outcome != ParameterIdentity::Equal {
            return Ok(Some(BezierIdentityDecision {
                outcome: BezierIdentity::Indeterminate(Reason::MissingProof),
                work_used: ctx.work_used(),
                context: ctx.identity(),
            }));
        }
        if c.periodic {
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
        let proof = cad_predicates::rational_bspline_isoparametric_trace_identity(
            &mut ctx,
            &curve,
            &strip,
            &k,
            degrees[fixed],
            BsplineStripLocation {
                span,
                parameter: source.leaf(ti).unwrap(),
                inward_positive: true,
            },
            periodic[fixed],
        )
        .map_err(|e| crate::input(e.to_string()))?;
        return Ok(Some(proof));
    }
    Ok(None)
}
