//! surface self chord algorithm: mean_matrix_lower.
use super::*;
/// For chord=M z, |n.chord|² <= ||n^T M||² |z|² while
/// |chord|² >= sigma_min(M)² |z|². Cauchy--Binet bounds sigma_min
/// from all separated minors. No endpoint normal is inferred from samples.
pub fn mean_matrix_lower(j: [[I; 2]; 3]) -> Result<f64> {
    let mut determinant = I::point(0.);
    for [a, b] in [[0, 1], [0, 2], [1, 2]] {
        let d = j[a][0].mul(j[b][1])?.sub(j[a][1].mul(j[b][0])?)?;
        if let Some(x) = separated_square(d)? {
            determinant = determinant.add(x)?;
        }
    }
    let mut trace = I::point(0.);
    for k in 0..3 {
        for a in 0..2 {
            let upper = j[k][a].lo.abs().max(j[k][a].hi.abs());
            trace = trace.add(I::point(upper).mul(I::point(upper))?)?;
        }
    }
    // Gershgorin also bounds the largest eigenvalue of the 2x2 Gram
    // matrix. Nearly orthogonal columns need not pay the full trace bound.
    let (diagonal, diagonal_lower, off_diagonal) = gram_bounds(j)?;
    let off_upper = off_diagonal.lo.abs().max(off_diagonal.hi.abs());
    let gershgorin = diagonal[0].hi.max(diagonal[1].hi);
    let mut largest = I::point(gershgorin).add(I::point(off_upper))?.hi.min(trace.hi);
    // For each enclosed Gram matrix, increasing its diagonals and replacing
    // its off-diagonal by an absolute upper bound increases lambda_max.
    // Evaluate that dominating 2x2 matrix with outward arithmetic.
    let difference = I::point(diagonal[0].hi).sub(I::point(diagonal[1].hi))?;
    let difference_upper = difference.lo.abs().max(difference.hi.abs());
    let discriminant = I::point(difference_upper).mul(I::point(difference_upper))?
        .add(I::point(off_upper).mul(I::point(off_upper))?.mul(I::point(4.))?)?;
    let root_upper = discriminant.hi.sqrt().next_up();
    let eigenvalue_upper = I::point(diagonal[0].hi).add(I::point(diagonal[1].hi))?
        .add(I::point(root_upper))?.mul(I::point(0.5))?.hi;
    largest = largest.min(eigenvalue_upper);
    // lambda_min increases with either diagonal and decreases with |b|.
    // Thus lower diagonal norms and the absolute off-diagonal upper bound
    // yield a second independent lower bound for every enclosed matrix.
    let direct_lower = direct_gram_lower(diagonal_lower, off_upper)?;
    let minor_lower = if determinant.lo > 0. && largest > 0. {
        determinant.div(I::point(largest))?.lo.max(0.)
    } else { 0. };
    Ok(direct_lower.max(minor_lower))
}
