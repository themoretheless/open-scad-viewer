//! Division-free restriction of a whole homogeneous vector-valued B-spline.
//! Every blossom and EVERY component keep the same positive gauge. In a tensor
//! restriction components include the complete other-axis net, never one row.
use super::*;
pub(super) fn restrict(
    points: &[Vec<Expansion>],
    knots: &[Expansion],
    p: usize,
    span: usize,
    range: [&Expansion; 2],
    ctx: &mut PredicateContext<'_>,
) -> Result<Vec<Vec<Expansion>>, Reason> {
    ctx.charge(1)?;
    let components = points[0].len();
    let base = points[span - p..=span].to_vec();
    // A complete C0 Bernstein cell already IS this original control vector.
    // Multiplicity p at both ends suffices (adjacent cells share one pole).
    // Retain its single common homogeneous gauge across every component.
    let lo = &knots[span];
    let hi = &knots[span + 1];
    let mut original_cell =
        range[0].sub(lo, ctx)?.sign() == Sign::Zero && range[1].sub(hi, ctx)?.sign() == Sign::Zero;
    if original_cell {
        for k in &knots[span - p + 1..=span] {
            original_cell &= k.sub(lo, ctx)?.sign() == Sign::Zero;
        }
        for k in &knots[span + 1..span + p + 1] {
            original_cell &= k.sub(hi, ctx)?.sign() == Sign::Zero;
        }
        if original_cell && hi.sub(lo, ctx)?.sign() == Sign::Positive {
            return Ok(base);
        }
    }
    let mut rows = vec![base; p + 1];
    for r in 1..=p {
        let m = p - r + 1;
        let mut coefficients = Vec::new();
        for j in r..=p {
            let i = span - p + j;
            let upper = &knots[i + p - r + 1];
            let lower = &knots[i];
            coefficients.extend([
                upper.sub(lower, ctx)?,
                upper.sub(range[0], ctx)?,
                range[0].sub(lower, ctx)?,
                upper.sub(range[1], ctx)?,
                range[1].sub(lower, ctx)?,
            ]);
        }
        Expansion::normalize_common(&mut coefficients, ctx)?;
        let mut prefix = vec![Expansion::scalar(1.)];
        for j in 0..m {
            if coefficients[5 * j].sign() != Sign::Positive {
                return Err(Reason::MissingProof);
            }
            prefix.push(prefix[j].mul(&coefficients[5 * j], ctx)?);
        }
        let mut suffix = vec![Expansion::scalar(1.); m + 1];
        for j in (0..m).rev() {
            suffix[j] = suffix[j + 1].mul(&coefficients[5 * j], ctx)?;
        }
        for (blossom, row) in rows.iter_mut().enumerate() {
            let end = usize::from(r > p - blossom);
            for j in (r..=p).rev() {
                let c = &coefficients[5 * (j - r)..5 * (j - r) + 5];
                let omit = prefix[j - r].mul(&suffix[j - r + 1], ctx)?;
                for axis in 0..components {
                    row[j][axis] = row[j - 1][axis]
                        .mul(&c[1 + 2 * end], ctx)?
                        .add(&row[j][axis].mul(&c[2 + 2 * end], ctx)?, ctx)?
                        .mul(&omit, ctx)?;
                }
            }
        }
        let mut active = rows
            .iter()
            .flat_map(|row| row[r..].iter().flatten().cloned())
            .collect::<Vec<_>>();
        Expansion::normalize_common(&mut active, ctx)?;
        let mut values = active.into_iter();
        for row in &mut rows {
            for point in &mut row[r..] {
                for x in point {
                    *x = values.next().unwrap();
                }
            }
        }
    }
    Ok(rows.into_iter().map(|row| row[p].clone()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    #[test]
    fn full_c0_cell_is_original_but_partial_and_c1_cells_are_restricted() {
        let arena = SourceArena::authored(
            "original-blossom-cell",
            1,
            vec![AuthoredScalar::Binary64Bits(0f64.to_bits())],
        )
        .unwrap();
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        let points = [0., 1., 2., 4., 8.].map(|x| vec![Expansion::scalar(x), Expansion::scalar(1.)]);
        let knots = [0., 0., 0., 0.5, 0.5, 1., 1., 1.].map(Expansion::scalar);
        let full = restrict(&points, &knots, 2, 2, [&knots[2], &knots[3]], &mut ctx).unwrap();
        let check =
            |rows: &[Vec<Expansion>], expected: [f64; 3], ctx: &mut PredicateContext<'_>| {
                for (row, x) in rows.iter().zip(expected) {
                    assert_eq!(
                        row[0]
                            .sub(&row[1].mul(&Expansion::scalar(x), ctx).unwrap(), ctx)
                            .unwrap()
                            .sign(),
                        Sign::Zero
                    );
                    assert_eq!(row[1].sign(), Sign::Positive);
                }
            };
        check(&full, [0., 1., 2.], &mut ctx);
        let quarter = Expansion::scalar(0.25);
        let partial = restrict(&points, &knots, 2, 2, [&knots[2], &quarter], &mut ctx).unwrap();
        check(&partial, [0., 0.5, 1.], &mut ctx);
        let c1 = [0., 0., 0., 0.5, 1., 1., 1.].map(Expansion::scalar);
        let restricted = restrict(&points[..4], &c1, 2, 2, [&c1[2], &c1[3]], &mut ctx).unwrap();
        check(&restricted, [0., 1., 1.5], &mut ctx);
    }
}
