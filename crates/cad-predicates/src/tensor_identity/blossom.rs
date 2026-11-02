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
