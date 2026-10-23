//! Endpoint separation: preserves the original arithmetic and proof budgets.
use super::*;

/// Direct Bernstein conic separation. A proper original quadratic is the
/// carrier; the original rational query has degree 1..8. No power eliminant,
/// cropped curve, inverse fit or sampled sign grants admission.
pub fn planar_bezier_endpoint_separation(
    ctx: &mut PredicateContext<'_>,
    a: &[[LeafRef; 3]],
    b: &[[LeafRef; 3]],
) -> Result<PlanarEndpointSeparation, InputError> {
    if a.len() != 3 || !(2..=9).contains(&b.len()) {
        return Err(InputError::InvalidInput(
            "Quadratic carrier and degree 1..8 query required",
        ));
    }
    let input = a
        .iter()
        .chain(b)
        .flatten()
        .map(|r| ctx.resolve(*r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    let computed = (|| -> Result<bool, Reason> {
        let exact = exact_inputs(&input, ctx)?;
        let points = exact
            .chunks_exact(3)
            .map(|p| std::array::from_fn(|k| p[k].clone()))
            .collect::<Vec<[Expansion; 3]>>();
        if points.iter().any(|p| p[2].sign() != Sign::Positive) {
            return Err(Reason::MissingProof);
        }
        let (aa, bb) = points.split_at(3);
        let homogeneous = |p: &[[Expansion; 3]],
                           ctx: &mut PredicateContext<'_>|
         -> Result<Vec<[Expansion; 3]>, Reason> {
            let mut common = true;
            for row in &p[1..] {
                if row[2].sub(&p[0][2], ctx)?.sign() != Sign::Zero {
                    common = false;
                    break;
                }
            }
            p.iter()
                .map(|row| {
                    Ok(if common {
                        [row[0].clone(), row[1].clone(), Expansion::scalar(1.)]
                    } else {
                        [
                            row[0].mul(&row[2], ctx)?,
                            row[1].mul(&row[2], ctx)?,
                            row[2].clone(),
                        ]
                    })
                })
                .collect()
        };
        let h = homogeneous(aa, ctx)?;
        let query = homogeneous(bb, ctx)?;
        let mut dual = Vec::new();
        for k in 0..3 {
            let (u, v) = ((k + 1) % 3, (k + 2) % 3);
            dual.push(
                (0..3)
                    .map(|j| {
                        let (x, y) = ((j + 1) % 3, (j + 2) % 3);
                        h[u][x]
                            .mul(&h[v][y], ctx)?
                            .sub(&h[u][y].mul(&h[v][x], ctx)?, ctx)
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            );
        }
        let scalar = |row: &[Expansion],
                      point: &[Expansion; 3],
                      ctx: &mut PredicateContext<'_>|
         -> Result<Expansion, Reason> {
            let mut value = Expansion::scalar(0.);
            for k in 0..3 {
                value = value.add(&row[k].mul(&point[k], ctx)?, ctx)?;
            }
            Ok(value)
        };
        if scalar(&dual[0], &h[0], ctx)?.sign() == Sign::Zero {
            return Ok(false);
        }
        let traces = dual
            .iter()
            .map(|row| {
                query
                    .iter()
                    .map(|point| scalar(row, point, ctx))
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let n = b.len() - 1;
        let degree = 2 * n;
        let gcd = |mut x: u64, mut y: u64| {
            while y != 0 {
                let z = x % y;
                x = y;
                y = z;
            }
            x
        };
        let mut scale = 1u64;
        for k in 0..=degree {
            let c = choose(degree, k);
            scale = (scale / gcd(scale, c))
                .checked_mul(c)
                .ok_or(Reason::ResourceLimit)?;
        }
        let mut coefficients = Vec::new();
        for k in 0..=degree {
            let mut value = Expansion::scalar(0.);
            for i in k.saturating_sub(n)..=k.min(n) {
                let j = k - i;
                let factor = choose(n, i) * choose(n, j) * (scale / choose(degree, k));
                let term = traces[1][i].mul(&traces[1][j], ctx)?.sub(
                    &traces[0][i]
                        .mul(&traces[2][j], ctx)?
                        .mul(&Expansion::scalar(4.), ctx)?,
                    ctx,
                )?;
                value = value.add(&term.mul(&Expansion::integer(factor, ctx)?, ctx)?, ctx)?;
            }
            coefficients.push(value);
        }
        for end in [0, 1] {
            let q = if end == 0 { 0 } else { n };
            let k = if end == 0 { 0 } else { degree };
            if coefficients[k].sign() == Sign::Zero {
                let mut shared = false;
                for s in [0, 2] {
                    if bb[q][0].sub(&aa[s][0], ctx)?.sign() == Sign::Zero
                        && bb[q][1].sub(&aa[s][1], ctx)?.sign() == Sign::Zero
                    {
                        shared = true;
                    }
                }
                if !shared {
                    return Ok(false);
                }
            }
        }
        conic_bernstein_no_interior_roots(&coefficients, ctx)
    })()
    .and_then(|v| ctx.charge(0).map(|_| v));
    Ok(PlanarEndpointSeparation {
        proven: matches!(computed, Ok(true)),
        reason: match computed {
            Ok(true) => None,
            Ok(false) => Some(Reason::MissingProof),
            Err(e) => Some(e),
        },
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
