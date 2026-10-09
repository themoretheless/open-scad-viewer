//! Interval Lagrange-to-Bernstein reconstruction on exact binary64 nodes.
//! The caller must establish the polynomial degree. Samples alone cannot
//! establish that an arbitrary expression is polynomial or issue admission.
use crate::{check, distance_bounds::Interval as I, Result};

/// Each row encloses the Bernstein coefficients of one nodal basis function.
/// Insufficient work refuses before allocating a coefficient basis.
pub(crate) fn basis(nodes: &[f64], max_work: usize) -> Result<(Option<Vec<Vec<I>>>, usize)> {
    check(
        !nodes.is_empty()
            && nodes.len() <= 17
            && nodes
                .iter()
                .enumerate()
                .all(|(i, x)| x.is_finite() && *x >= 0. && *x <= 1. && !nodes[..i].contains(x)),
        "Choose distinct finite polynomial nodes in [0,1]",
    )?;
    let n = nodes.len() - 1;
    let required = nodes.len() * n * (n + 3) / 2;
    if required > max_work {
        return Ok((None, 0));
    }
    let mut rows = Vec::with_capacity(nodes.len());
    for (i, &node) in nodes.iter().enumerate() {
        let mut coefficients = vec![I::point(1.)];
        for (j, &other) in nodes.iter().enumerate() {
            if i == j {
                continue;
            }
            let denominator = I::point(node).sub(I::point(other))?;
            let factor = [
                I::point(0.).sub(I::point(other))?.div_signed(denominator)?,
                I::point(1.).sub(I::point(other))?.div_signed(denominator)?,
            ];
            let degree = coefficients.len();
            let mut product = vec![I::point(0.); degree + 1];
            for k in 0..=degree {
                if k < degree {
                    let weight = I::point((degree - k) as f64).div(I::point(degree as f64))?;
                    product[k] = product[k].add(coefficients[k].mul(factor[0])?.mul(weight)?)?;
                }
                if k > 0 {
                    let weight = I::point(k as f64).div(I::point(degree as f64))?;
                    product[k] =
                        product[k].add(coefficients[k - 1].mul(factor[1])?.mul(weight)?)?;
                }
            }
            coefficients = product;
        }
        rows.push(coefficients);
    }
    Ok((Some(rows), required))
}

/// Enclose coefficients of a polynomial of degree at most samples.len()-1.
pub(crate) fn reconstruct(samples: &[I], basis: &[Vec<I>]) -> Result<Vec<I>> {
    check(
        !samples.is_empty()
            && basis.len() == samples.len()
            && basis.iter().all(|r| r.len() == samples.len()),
        "Match polynomial samples and nodal basis",
    )?;
    (0..samples.len())
        .map(|k| {
            samples
                .iter()
                .zip(basis)
                .try_fold(I::point(0.), |sum, (&sample, row)| {
                    sum.add(sample.mul(row[k])?)
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn degree_eight_reconstructs_independent_de_casteljau_coefficients() {
        let nodes: Vec<_> = (0..=8).map(|i| i as f64 / 8.).collect();
        let authored: Vec<_> = (0..=8).map(|i| ((i * 7 + 3) % 11) as f64 / 4.).collect();
        let samples: Vec<_> = nodes
            .iter()
            .map(|&t| {
                let mut level: Vec<_> = authored.iter().map(|&v| I::point(v)).collect();
                while level.len() > 1 {
                    level = level
                        .windows(2)
                        .map(|p| {
                            p[0].mul(I::point(1. - t))
                                .unwrap()
                                .add(p[1].mul(I::point(t)).unwrap())
                                .unwrap()
                        })
                        .collect();
                }
                level[0]
            })
            .collect();
        let matrix = basis(&nodes, 10000).unwrap().0.unwrap();
        let reconstructed = reconstruct(&samples, &matrix).unwrap();
        for (bound, exact) in reconstructed.iter().zip(authored) {
            assert!(bound.contains(exact));
        }
    }
    #[test]
    fn quadratic_coefficients_and_interval_uncertainty_are_enclosed() {
        let nodes = [0., 0.5, 1.];
        let (matrix, work) = basis(&nodes, 100).unwrap();
        assert_eq!(work, 15);
        let matrix = matrix.unwrap();
        let samples = nodes.map(|x| I::point(x * x + 2. * x + 3.));
        let coefficients = reconstruct(&samples, &matrix).unwrap();
        for (bound, exact) in coefficients.iter().zip([3., 4., 6.]) {
            assert!(bound.contains(exact));
            assert!(bound.width() < 1e-11);
        }
        let uncertain = samples.map(|v| I::new(v.lo - 0.01, v.hi + 0.01).unwrap());
        let bounds = reconstruct(&uncertain, &matrix).unwrap();
        for (i, bound) in bounds.iter().enumerate() {
            assert!(bound.lo <= coefficients[i].lo && bound.hi >= coefficients[i].hi);
        }
        assert!(basis(&nodes, 14).unwrap().0.is_none());
        assert!(basis(&[0., 0., 1.], 100).is_err());
        assert!(basis(&[-0.1, 1.], 100).is_err());
    }
}
