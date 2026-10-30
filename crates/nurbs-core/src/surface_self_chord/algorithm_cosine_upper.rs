//! surface self chord algorithm: cosine_upper.
use super::*;
pub fn cosine_upper(j: [[I; 2]; 3]) -> Result<f64> {
    let mut variation = I::point(0.);
    let mut width_gram = [I::point(0.);3];
    for row in j {
        let mut widths=[I::point(0.);2];
        for (axis,x) in row.into_iter().enumerate() {
            let width = I::point(x.hi).sub(I::point(x.lo))?;
            variation = variation.add(width.mul(width)?)?;
            widths[axis]=I::point(width.hi);
        }
        width_gram[0]=width_gram[0].add(widths[0].mul(widths[0])?)?;
        width_gram[1]=width_gram[1].add(widths[0].mul(widths[1])?)?;
        width_gram[2]=width_gram[2].add(widths[1].mul(widths[1])?)?;
    }
    // For every difference E between two enclosed differentials, |E| <= W
    // componentwise, where W contains the complete interval widths. Thus
    // ||E||_2^2 <= rho(W^T W) <= max row sum(W^T W). Take the smaller
    // of this outward bound and the existing Frobenius bound. No midpoint
    // derivative or sampled normal enters this whole-enclosure inequality.
    let row_upper=width_gram[0].add(width_gram[1])?.hi
        .max(width_gram[2].add(width_gram[1])?.hi);
    variation=I::point(variation.hi.min(row_upper));
    let mut best = 1_f64;
    let (_, lower, off) = gram_bounds(j)?;
    let least = direct_gram_lower(lower, off.lo.abs().max(off.hi.abs()))?;
    if least > 0. {
        best = best.min(variation.div(I::point(least))?.hi);
    }
    let mut gram_determinant = I::point(0.);
    for [a, b] in [[0, 1], [0, 2], [1, 2]] {
        let det = j[a][0].mul(j[b][1])?.sub(j[a][1].mul(j[b][0])?)?;
        let Some(det_squared) = separated_square(det)? else {
            continue;
        };
        if det_squared.lo <= 0. {
            continue;
        }
        gram_determinant = gram_determinant.add(det_squared)?;
        let mut norm = I::point(0.);
        for axis in [a, b] {
            for direction in 0..2 {
                let upper = j[axis][direction].lo.abs().max(j[axis][direction].hi.abs());
                norm = norm.add(I::point(upper).mul(I::point(upper))?)?;
            }
        }
        let ratio = variation.mul(norm)?.div(det_squared)?;
        best = best.min(ratio.hi);
    }
    // For every enclosed 3x2 mean differential M, Cauchy--Binet gives
    // det(M^T M) = sum of its three squared coordinate minors. Hence
    // sigma_min(M)^2 >= det(M^T M) / trace(M^T M). Summing separated
    // minor bounds preserves all original differentials and can strengthen
    // the single-coordinate lower bound above.
    if gram_determinant.lo > 0. {
        let mut trace = I::point(0.);
        for row in j {
            for x in row {
                let upper = x.lo.abs().max(x.hi.abs());
                trace = trace.add(I::point(upper).mul(I::point(upper))?)?;
            }
        }
        best = best.min(variation.mul(trace)?.div(gram_determinant)?.hi);
    }
    Ok(best)
}
