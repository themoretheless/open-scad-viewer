//! Original projected-coordinate root signs in the remaining total allowance.
use super::*;
use cad_predicates::{
    rational_bspline_quadric_projected_edge_sign, AuthoredScalar, Limits,
    OriginalQuadricRootSignDecision, PredicateContext, Sign, SourceArena, ToleranceContext,
};
#[derive(Debug)]
pub struct GraphBoundaryProof {
    pub span: usize,
    pub normalized_root: [f64; 2],
    pub edge: usize,
    /// Per-query cost; decision.work_used is cumulative within this root context.
    pub work_delta: u64,
    pub decision: OriginalQuadricRootSignDecision,
}
pub(super) struct Proofs {
    pub signs: [Option<Sign>; 4],
    pub records: Vec<GraphBoundaryProof>,
    pub work: u64,
}
pub(super) fn prove(
    c: &Curve,
    s: &Surface,
    chart: &chart::Chart,
    matrix: [[f64; 4]; 4],
    products: &[[f64; 2]],
    root: &OriginalQuadricCarrierRoot,
    eligible: [bool; 4],
    budget: u64,
) -> Result<Proofs> {
    let mut out = Proofs {
        signs: [None; 4],
        records: vec![],
        work: 0,
    };
    if budget == 0 {
        return Ok(out);
    }
    let mut values: Vec<_> = c
        .control_points
        .iter()
        .zip(&c.weights)
        .flat_map(|(p, w)| [p[0], p[1], p[2], *w])
        .collect();
    let k = values.len();
    values.extend(&c.knots);
    let m = values.len();
    values.extend(matrix.into_iter().flatten());
    let f = values.len();
    values.extend(products.iter().flatten());
    let a = values.len();
    for (i, j) in [(0, 0), (s.degree_u, 0), (0, s.degree_v)] {
        values.extend(&s.control_points[i][j]);
    }
    let arena = SourceArena::authored(
        "nurbs-core/original-quadric-graph-coordinate-sign",
        1,
        values
            .into_iter()
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    )
    .map_err(|_| crate::input("Invalid quadric coordinate source arena"))?;
    let controls: Vec<_> = (0..c.control_points.len())
        .map(|i| std::array::from_fn(|j| arena.leaf(i * 4 + j).unwrap()))
        .collect();
    let knots: Vec<_> = (0..c.knots.len())
        .map(|i| arena.leaf(k + i).unwrap())
        .collect();
    let matrix =
        std::array::from_fn(|i| std::array::from_fn(|j| arena.leaf(m + i * 4 + j).unwrap()));
    let factors: Vec<_> = (0..products.len())
        .map(|i| std::array::from_fn(|j| arena.leaf(f + i * 2 + j).unwrap()))
        .collect();
    let frame =
        std::array::from_fn(|i| std::array::from_fn(|j| arena.leaf(a + i * 3 + j).unwrap()));
    let tol = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tol,
        Limits {
            max_work: budget,
            ..Limits::default()
        },
        None,
    );
    let span = root.source_spans[0];
    let normalized = root.normalized_original_span_parameters[0];
    for edge in 0..4 {
        if !eligible[edge] {
            continue;
        }
        if ctx.work_used() == budget {
            break;
        }
        let before = ctx.work_used();
        let decision = rational_bspline_quadric_projected_edge_sign(
            &mut ctx, &controls, &knots, c.degree, span, matrix, &factors, frame, chart.axes, edge,
            normalized,
        )
        .map_err(|_| crate::input("Invalid original quadric edge-sign query"))?;
        if let Ok(sign) = &decision.sign {
            out.signs[edge] = Some(*sign);
        }
        let outside = matches!(&decision.sign,Ok(Sign::Negative) if edge%2==0)
            || matches!(&decision.sign,Ok(Sign::Positive) if edge%2==1);
        out.records.push(GraphBoundaryProof {
            span,
            normalized_root: normalized,
            edge,
            work_delta: ctx.work_used() - before,
            decision,
        });
        if outside {
            break;
        }
    }
    out.work = ctx.work_used();
    Ok(out)
}
