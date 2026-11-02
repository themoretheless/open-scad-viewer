//! Exact nonplanar graph reconstruction over a proved triangular projection.
//! Full source identities and a positive target denominator prove image equality.
use super::*;
fn monomials(h: &Net, degree: usize, work: &mut Work) -> Result<Vec<Scalar>> {
    let coordinates = [
        bernstein::scalar(h, 0),
        bernstein::scalar(h, 1),
        bernstein::scalar(h, 3),
    ];
    let mut powers = Vec::new();
    for coordinate in &coordinates {
        let mut values = vec![vec![vec![Q::one()]]];
        for k in 0..degree {
            values.push(bernstein::product(&values[k], coordinate, work)?);
        }
        powers.push(values);
    }
    (0..=degree)
        .flat_map(|i| (0..=degree - i).map(move |j| (i, j)))
        .map(|(i, j)| {
            bernstein::product(
                &bernstein::product(&powers[0][i], &powers[1][j], work)?,
                &powers[2][degree - i - j],
                work,
            )
        })
        .collect()
}
fn combination(terms: &[Scalar], c: &[Q]) -> Scalar {
    terms[0]
        .iter()
        .enumerate()
        .map(|(i, row)| {
            row.iter()
                .enumerate()
                .map(|(j, _)| terms.iter().zip(c).map(|(t, c)| &t[i][j] * c).sum())
                .collect()
        })
        .collect()
}
pub(super) fn canonical(p: &Patch, work: &mut Work) -> Result<Option<Net>> {
    for axis in 0..3 {
        let coordinates = (0..3).filter(|k| *k != axis).collect::<Vec<_>>();
        let projected =
            p.h.iter()
                .map(|row| {
                    row.iter()
                        .map(|h| {
                            [
                                h[coordinates[0]].clone(),
                                h[coordinates[1]].clone(),
                                Q::zero(),
                                h[3].clone(),
                            ]
                        })
                        .collect()
                })
                .collect::<Net>();
        let projection = Patch {
            ids: p.ids,
            derivatives: bernstein::derivatives(&projected, work)?,
            motion: bernstein::homotopy(&projected, work)?,
            h: projected.clone(),
        };
        let Some(target) = planar::canonical(&projection, work)? else {
            continue;
        };
        let z = bernstein::scalar(&p.h, axis);
        let w = bernstein::scalar(&p.h, 3);
        for degree in 1..=3 {
            if (p.h.len() - 1) * (degree + 1) > 33 {
                continue;
            }
            let terms = monomials(&projected, degree, work)?;
            let count = terms.len();
            let nw = terms
                .iter()
                .map(|t| bernstein::product(&w, t, work))
                .collect::<Result<Vec<_>>>()?;
            let dz = terms
                .iter()
                .map(|t| bernstein::product(&z, t, work))
                .collect::<Result<Vec<_>>>()?;
            let rows = nw[0]
                .iter()
                .enumerate()
                .flat_map(|(i, row)| (0..row.len()).map(move |j| (i, j)))
                .map(|(i, j)| {
                    nw.iter()
                        .map(|t| -t[i][j].clone())
                        .chain(dz.iter().map(|t| t[i][j].clone()))
                        .collect()
                })
                .collect();
            let target_terms = monomials(&target, degree, work)?;
            for c in linear::kernel(rows, 2 * count, work)? {
                let mut numerator = combination(&target_terms, &c[..count]);
                let mut denominator = combination(&target_terms, &c[count..]);
                if denominator.iter().flatten().all(|q| *q < Q::zero()) {
                    for q in numerator
                        .iter_mut()
                        .flatten()
                        .chain(denominator.iter_mut().flatten())
                    {
                        *q = -q.clone();
                    }
                }
                if !denominator.iter().flatten().all(|q| *q > Q::zero()) {
                    continue;
                }
                // Recheck the complete source identity independently of elimination.
                for (i, row) in nw[0].iter().enumerate() {
                    for j in 0..row.len() {
                        work.charge(count)?;
                        let residual: Q = (0..count)
                            .map(|k| &dz[k][i][j] * &c[count + k] - &nw[k][i][j] * &c[k])
                            .sum();
                        if !residual.is_zero() {
                            return Err(fail("nonplanar graph source identity is false"));
                        }
                    }
                }
                let one = vec![vec![Q::one(), Q::one()], vec![Q::one()]];
                let height = bernstein::product(&numerator, &one, work)?;
                let weight = bernstein::product(&denominator, &one, work)?;
                let x = bernstein::product(&denominator, &bernstein::scalar(&target, 0), work)?;
                let y = bernstein::product(&denominator, &bernstein::scalar(&target, 1), work)?;
                let out = weight
                    .iter()
                    .enumerate()
                    .map(|(i, row)| {
                        row.iter()
                            .enumerate()
                            .map(|(j, w)| {
                                let mut h = [Q::zero(), Q::zero(), Q::zero(), w.clone()];
                                h[axis] = height[i][j].clone();
                                h[coordinates[0]] = x[i][j].clone();
                                h[coordinates[1]] = y[i][j].clone();
                                h
                            })
                            .collect()
                    })
                    .collect::<Net>();
                transport::check(&out)?;
                if bernstein::corners(&out) != bernstein::corners(&p.h) {
                    return Err(fail(
                        "nonplanar graph reconstruction changed source vertices",
                    ));
                }
                return Ok(Some(out));
            }
        }
    }
    Ok(None)
}
#[cfg(test)]
mod closed_tests;
#[cfg(test)]
mod tests;
