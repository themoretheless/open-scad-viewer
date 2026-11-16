//! Global injectivity from independent enclosures of projected Jacobian
//! column directions. The determinant test applies to columns from any
//! two points, and therefore also to the Jacobian averaged on a segment.
use super::*;
use crate::numerics::bernstein_tensor::{Net, derivative, minus, product};

fn spend(work: &mut usize, limit: usize, cost: usize) -> Option<()> {
    if cost > limit - *work {
        return None;
    }
    *work += cost;
    Some(())
}

pub(crate) struct Proof {
    pub axes: [usize; 2],
    pub diagonally_dominant: bool,
}

pub(crate) fn certify(s: &Surface, work: &mut usize, limit: usize) -> Option<Proof> {
    let degree = [s.degree_u, s.degree_v];
    if s.periodic_u
        || s.periodic_v
        || degree.iter().any(|d| !(1..=7).contains(d))
        || s.control_points.len() != degree[0] + 1
        || s.control_points[0].len() != degree[1] + 1
    {
        return None;
    }
    for (knots, d) in [(&s.knots_u, degree[0]), (&s.knots_v, degree[1])] {
        spend(work, limit, 1)?;
        if knots
            .iter()
            .enumerate()
            .any(|(i, k)| *k != knots[if i <= d { 0 } else { d + 1 }])
        {
            return None;
        }
    }

    // Corner chords select axes and orientations; all subsequent sign
    // decisions use outward enclosures of the original Bernstein coefficients.
    let origin = &s.control_points[0][0];
    let ends = [
        &s.control_points[degree[0]][0],
        &s.control_points[0][degree[1]],
    ];
    let chords = ends.map(|end| std::array::from_fn::<_, 3, _>(|k| end[k] - origin[k]));
    let axes = chords.map(|chord| {
        (0..3)
            .max_by(|&a, &b| chord[a].abs().total_cmp(&chord[b].abs()))
            .unwrap()
    });
    if axes[0] == axes[1] || (0..2).any(|i| chords[i][axes[i]] == 0.) {
        return None;
    }
    let orientation = std::array::from_fn::<_, 2, _>(|i| chords[i][axes[i]].signum());
    let count = (degree[0] + 1) * (degree[1] + 1);
    spend(work, limit, count.div_ceil(8))?;
    let weights: Net = s
        .weights
        .iter()
        .map(|row| row.iter().map(|&w| I::point(w)).collect())
        .collect();
    let homogeneous = |i: usize| -> Result<Net> {
        let k = axes[i];
        s.control_points
            .iter()
            .zip(&weights)
            .map(|(row, w)| {
                row.iter()
                    .zip(w)
                    .map(|(p, &w)| {
                        I::point(p[k])
                            .sub(I::point(origin[k]))?
                            .mul(I::point(orientation[i]))?
                            .mul(w)
                    })
                    .collect()
            })
            .collect()
    };
    let h = [homogeneous(0).ok()?, homogeneous(1).ok()?];
    let mut slopes = [I::point(0.); 2];
    let mut diagonally_dominant = true;
    for axis in 0..2 {
        let derivative_count = count / (degree[axis] + 1) * degree[axis];
        // Four tensor products: charge in blocks of at most 32 coefficient
        // pair contributions. The degree cap bounds the remaining row work.
        spend(work, limit, (4 * derivative_count * count).div_ceil(32))?;
        let dw = derivative(&weights, axis).ok()?;
        let numerator = |k: usize| -> Result<Net> {
            let dh = derivative(&h[k], axis)?;
            minus(product(&dh, &weights)?, product(&h[k], &dw)?)
        };
        let diagonal = numerator(axis).ok()?;
        let cross = numerator(1 - axis).ok()?;
        for (drow, crow) in diagonal.iter().zip(&cross) {
            spend(work, limit, 1)?;
            for (d, c) in drow.iter().zip(crow) {
                if d.lo <= 0. {
                    return None;
                }
                let dominant = d.lo > c.lo.abs().max(c.hi.abs());
                diagonally_dominant &= dominant;
                let slope = match c.div(*d) {
                    Ok(slope) => slope,
                    // A tiny diagonal may overflow its reciprocal even
                    // though dominance directly encloses the ratio.
                    Err(_) if dominant => I::new(-1., 1.).ok()?,
                    Err(_) => return None,
                };
                slopes[axis] = hull([slopes[axis], slope].into_iter());
            }
        }
    }
    // With strictly positive diagonal coefficients, the slope of each
    // column is a convex combination of the coefficient ratios. W² cancels.
    // Along any segment, averaging the columns preserves the same slopes,
    // with their positive diagonal entries as weights. The averaged matrix
    // has determinant d0*d1*(1-slope0*slope1), with d0,d1>0.
    if !diagonally_dominant && slopes[0].mul(slopes[1]).ok()?.hi >= 1. {
        return None;
    }
    Some(Proof {
        axes,
        diagonally_dominant,
    })
}
