//! Original curve/implicit-quadric roots; finite surface UV admission is separate.
use super::*;
use cad_predicates::{
    rational_bspline_quadric_roots, AuthoredScalar, Limits, OriginalQuadricRootsDecision,
    OriginalQuadricRootsOutcome, PredicateContext, SourceArena, ToleranceContext,
};
#[derive(Debug)]
pub struct OriginalQuadricCarrierRoot {
    pub parameter: [f64; 2],
    pub position_bounds: [[f64; 2]; 3],
    pub source_spans: Vec<usize>,
    pub multiplicities: Vec<usize>,
    pub normalized_original_span_parameters: Vec<[f64; 2]>,
}
#[derive(Debug)]
pub struct OriginalQuadricCarrierRoots {
    pub roots: Vec<OriginalQuadricCarrierRoot>,
    pub scalar_proofs: Vec<(usize, OriginalQuadricRootsDecision)>,
    pub unresolved_spans: Vec<usize>,
    pub spans: usize,
    pub exact_work: u64,
    pub complete: bool,
}
/// Complete isolated roots on an infinite authored quadric carrier, using
/// original nonperiodic source controls/weights/knots and exact Q/products.
/// Multiplicity is the implicit equation order, never a crossing assertion.
/// Coincident spans are reported as unresolved isolated-root coverage.
/// No target surface, finite UV chart, trim, topology or Body is admitted.
pub fn original_quadric_carrier_roots(
    c: &Curve,
    matrix: [[f64; 4]; 4],
    products: &[[f64; 2]],
    tolerance: f64,
    max_spans: usize,
    max_exact_work: u64,
) -> Result<OriginalQuadricCarrierRoots> {
    c.validate()?;
    check(
        !c.periodic
            && c.control_points[0].len() == 3
            && c.degree <= 25
            && c.control_points.len() <= 256,
        "Quadric roots require an original nonperiodic 3D curve of degree 1..25",
    )?;
    check(
        c.degree >= 1
            && tolerance.is_finite()
            && tolerance > 0.
            && (1..=100000).contains(&max_spans),
        "Invalid original quadric carrier root budget/tolerance",
    )?;
    check(
        products.len() <= 16
            && matrix
                .iter()
                .flatten()
                .chain(products.iter().flatten())
                .all(|x| x.is_finite()),
        "Invalid original quadric coefficients",
    )?;
    let [lo, hi] = c.domain();
    check(
        c.knots
            .iter()
            .filter(|&&t| lo < t && t < hi)
            .all(|t| c.knots.iter().filter(|v| *v == t).count() <= c.degree),
        "Original quadric carrier roots require C0 source knots",
    )?;
    let mut values: Vec<_> = c
        .control_points
        .iter()
        .zip(&c.weights)
        .flat_map(|(p, w)| [p[0], p[1], p[2], *w])
        .collect();
    let k = values.len();
    values.extend(&c.knots);
    let q = values.len();
    values.extend(matrix.into_iter().flatten());
    let f = values.len();
    values.extend(products.iter().flatten());
    let arena = SourceArena::authored(
        "nurbs-core/original-quadric-carrier-roots",
        1,
        values
            .into_iter()
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    )
    .map_err(|_| crate::input("Invalid quadric source arena"))?;
    let controls: Vec<_> = (0..c.control_points.len())
        .map(|i| std::array::from_fn(|j| arena.leaf(i * 4 + j).unwrap()))
        .collect();
    let knots: Vec<_> = (0..c.knots.len())
        .map(|i| arena.leaf(k + i).unwrap())
        .collect();
    let matrix =
        std::array::from_fn(|i| std::array::from_fn(|j| arena.leaf(q + i * 4 + j).unwrap()));
    let factors: Vec<_> = (0..products.len())
        .map(|i| std::array::from_fn(|j| arena.leaf(f + i * 2 + j).unwrap()))
        .collect();
    let tol = ToleranceContext::default_valid();
    let mut out = OriginalQuadricCarrierRoots {
        roots: vec![],
        scalar_proofs: vec![],
        unresolved_spans: vec![],
        spans: 0,
        exact_work: 0,
        complete: false,
    };
    for span in c.degree..c.control_points.len() {
        let domain = [c.knots[span], c.knots[span + 1]];
        if domain[0] >= domain[1] {
            continue;
        }
        if out.spans == max_spans || out.exact_work == max_exact_work {
            out.unresolved_spans.push(span);
            continue;
        }
        let width = I::point(domain[1]).sub(I::point(domain[0]))?;
        let normalized = (I::point(tolerance).div(width)?.lo * 0.125).min(0.25);
        if normalized <= 0. || !normalized.is_finite() {
            out.unresolved_spans.push(span);
            continue;
        }
        let mut ctx = PredicateContext::new(
            &arena,
            &tol,
            Limits {
                max_work: max_exact_work - out.exact_work,
                ..Limits::default()
            },
            None,
        );
        let decision = rational_bspline_quadric_roots(
            &mut ctx,
            cad_predicates::SplineLeafRefs {
                controls: &controls,
                knots: &knots,
                degree: c.degree,
            },
            span,
            matrix,
            &factors,
            normalized,
        )
        .map_err(|_| crate::input("Invalid original quadric root query"))?;
        out.spans += 1;
        out.exact_work += decision.work_used;
        match &decision.outcome {
            OriginalQuadricRootsOutcome::Roots(roots) => {
                for r in roots {
                    let t = if r.interval == [0., 0.] {
                        I::point(domain[0])
                    } else if r.interval == [1., 1.] {
                        I::point(domain[1])
                    } else {
                        I::point(domain[0])
                            .add(width.mul(I::new(r.interval[0], r.interval[1])?)?)?
                            .intersect(domain[0], domain[1])?
                    };
                    let p = crate::curve_distance::enclosure(c, span, t)?;
                    let position = std::array::from_fn(|k| [p[k].lo, p[k].hi]);
                    merge(
                        &mut out.roots,
                        OriginalQuadricCarrierRoot {
                            parameter: [t.lo, t.hi],
                            position_bounds: position,
                            source_spans: vec![span],
                            multiplicities: vec![r.multiplicity],
                            normalized_original_span_parameters: vec![r.interval],
                        },
                    );
                }
            }
            _ => out.unresolved_spans.push(span),
        }
        out.scalar_proofs.push((span, decision));
    }
    out.complete = out.unresolved_spans.is_empty();
    Ok(out)
}
#[path = "quadric_carrier/merge.rs"]
mod merging;
use merging::merge;
