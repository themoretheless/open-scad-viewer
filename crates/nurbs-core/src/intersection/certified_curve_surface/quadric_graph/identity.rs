//! Exact original tensor membership, sharing the caller's finite total budget.
use super::*;
use cad_predicates::{
    binary_bezier_quadric_with_constant_products_identity, AuthoredScalar, BezierIdentityDecision,
    Limits, PredicateContext, SourceArena, ToleranceContext,
};
pub(super) fn prove(
    s: &Surface,
    matrix: [[f64; 4]; 4],
    products: &[[f64; 2]],
    budget: u64,
) -> Result<BezierIdentityDecision> {
    let mut values = Vec::new();
    for (i, row) in s.control_points.iter().enumerate() {
        for (j, p) in row.iter().enumerate() {
            values.extend(p);
            values.push(s.weights[i][j]);
        }
    }
    let offset = values.len();
    values.extend(matrix.into_iter().flatten());
    values.extend(products.iter().flatten());
    let arena = SourceArena::authored(
        "nurbs-core/original-quadric-graph-membership",
        1,
        values
            .into_iter()
            .map(|x: f64| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    )
    .map_err(|_| crate::input("Invalid graph source arena"))?;
    let nv = s.control_points[0].len();
    let surface: Vec<Vec<_>> = (0..s.control_points.len())
        .map(|i| {
            (0..nv)
                .map(|j| std::array::from_fn(|k| arena.leaf((i * nv + j) * 4 + k).unwrap()))
                .collect()
        })
        .collect();
    let matrix =
        std::array::from_fn(|i| std::array::from_fn(|j| arena.leaf(offset + i * 4 + j).unwrap()));
    let factors: Vec<_> = (0..products.len())
        .map(|i| std::array::from_fn(|j| arena.leaf(offset + 16 + i * 2 + j).unwrap()))
        .collect();
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
    binary_bezier_quadric_with_constant_products_identity(&mut ctx, &surface, matrix, &factors)
        .map_err(|_| crate::input("Invalid original quadric graph membership query"))
}
