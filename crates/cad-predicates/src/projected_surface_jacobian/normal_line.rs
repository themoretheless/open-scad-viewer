//! Exact normal-line identities on ORIGINAL tensor Bernstein coefficient rows.
//! No claim about boundary collapse, nonzero normals or pole regularity follows.
use super::*;
pub struct NormalLineDecision {
    pub parallel: Option<bool>,
    pub reason: Option<Reason>,
    pub work_used: u64,
    pub context: ContextIdentity,
}
fn normal_row(
    ctx: &mut PredicateContext<'_>,
    surface: &[Vec<[LeafRef; 4]>],
    row: usize,
    column: Option<usize>,
) -> Result<Result<[Vec<Expansion>; 3], Reason>, InputError> {
    if !(2..=9).contains(&surface.len())
        || !(2..=9).contains(&surface[0].len())
        || surface.iter().any(|r| r.len() != surface[0].len())
        || row >= 3 * (surface.len() - 1)
        || column.is_some_and(|c| c >= 3 * (surface[0].len() - 1))
    {
        return Err(InputError::InvalidInput("Invalid original normal row"));
    }
    let mut values = surface
        .iter()
        .flatten()
        .flat_map(|p| p.iter())
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    values.push(AuthoredScalar::Binary64Bits(1f64.to_bits()));
    Ok((|| {
        ctx.charge(values.len() as u64)?;
        let x = exact_inputs(&values, ctx)?;
        let unit = x.last().unwrap();
        if x[..x.len() - 1]
            .chunks_exact(4)
            .any(|p| p[3].sign() != Sign::Positive)
        {
            return Err(Reason::MissingProof);
        }
        let mut h = (0..4)
            .map(|_| zeros(surface.len(), surface[0].len()))
            .collect::<Vec<_>>();
        for (u, r) in surface.iter().enumerate() {
            for v in 0..r.len() {
                let at = (u * r.len() + v) * 4;
                for k in 0..3 {
                    h[k][u][v] = x[at + k].sub(&x[k], ctx)?.mul(&x[at + 3], ctx)?;
                }
                h[3][u][v] = unit.mul(&x[at + 3], ctx)?;
            }
        }
        let mut out: [Vec<Expansion>; 3] = std::array::from_fn(|_| Vec::new());
        for k in 0..3 {
            let projected = vec![h[(k + 1) % 3].clone(), h[(k + 2) % 3].clone(), h[3].clone()];
            let grid = determinant_grid(
                ctx,
                &projected,
                [surface.len() - 1, surface[0].len() - 1],
                Some(row),
                column,
            )?;
            out[k] = column.map_or_else(|| grid[row].clone(), |c| vec![grid[row][c].clone()]);
        }
        Ok(out)
    })())
}
fn finish(ctx: &mut PredicateContext<'_>, result: Result<bool, Reason>) -> NormalLineDecision {
    let result = result.and_then(|b| {
        ctx.charge(0)?;
        Ok(b)
    });
    let (parallel, reason) = match result {
        Ok(b) => (Some(b), None),
        Err(e) => (None, Some(e)),
    };
    NormalLineDecision {
        parallel,
        reason,
        work_used: ctx.work_used(),
        context: ctx.identity(),
    }
}
fn parallel(
    ctx: &mut PredicateContext<'_>,
    a: [&Expansion; 3],
    b: [&Expansion; 3],
) -> Result<bool, Reason> {
    for k in 0..3 {
        let (i, j) = ((k + 1) % 3, (k + 2) % 3);
        if a[i].mul(b[j], ctx)?.sub(&a[j].mul(b[i], ctx)?, ctx)?.sign() != Sign::Zero {
            return Ok(false);
        }
    }
    Ok(true)
}
/// Whole selected normal row is parallel to its first coefficient, which must
/// be nonzero. Some(column) proves only that coefficient against the same first
/// coefficient; callers must cover EVERY column before claiming a row identity.
pub fn rational_surface_normal_row_line(
    ctx: &mut PredicateContext<'_>,
    surface: &[Vec<[LeafRef; 4]>],
    row: usize,
    column: Option<usize>,
) -> Result<NormalLineDecision, InputError> {
    let target = normal_row(ctx, surface, row, column)?;
    let reference = if column.is_some_and(|c| c != 0) {
        Some(normal_row(ctx, surface, row, Some(0))?)
    } else {
        None
    };
    let result = (|| {
        let target = target?;
        let reference = match reference {
            Some(r) => r?,
            None => std::array::from_fn(|k| vec![target[k][0].clone()]),
        };
        let first = std::array::from_fn(|k| &reference[k][0]);
        if first.iter().all(|x| x.sign() == Sign::Zero) {
            return Ok(false);
        }
        for v in 0..target[0].len() {
            if !parallel(ctx, first, std::array::from_fn(|k| &target[k][v]))? {
                return Ok(false);
            }
        }
        Ok(true)
    })();
    Ok(finish(ctx, result))
}
/// Parallelism of two ORIGINAL normal-row first coefficients. Row factors,
/// collapse and nonzero reference hypotheses must be admitted independently.
pub fn rational_surface_normal_rows_parallel(
    ctx: &mut PredicateContext<'_>,
    surfaces: [&[Vec<[LeafRef; 4]>]; 2],
    rows: [usize; 2],
) -> Result<NormalLineDecision, InputError> {
    let a = normal_row(ctx, surfaces[0], rows[0], Some(0))?;
    let b = normal_row(ctx, surfaces[1], rows[1], Some(0))?;
    let result = (|| {
        let (a, b) = (a?, b?);
        let a = std::array::from_fn(|k| &a[k][0]);
        let b = std::array::from_fn(|k| &b[k][0]);
        if a.iter().all(|x| x.sign() == Sign::Zero) || b.iter().all(|x| x.sign() == Sign::Zero) {
            return Ok(false);
        }
        parallel(ctx, a, b)
    })();
    Ok(finish(ctx, result))
}

#[cfg(test)]
mod tests;
