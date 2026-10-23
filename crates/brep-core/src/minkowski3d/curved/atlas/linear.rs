//! Bounded exact rational candidate generation; callers recheck identities.
use super::*;
pub(super) fn kernel(
    mut rows: Vec<Vec<Q>>,
    columns: usize,
    work: &mut Work,
) -> Result<Vec<Vec<Q>>> {
    let mut pivots = Vec::new();
    for col in 0..columns {
        let rank = pivots.len();
        let Some(pivot) = (rank..rows.len()).find(|&i| !rows[i][col].is_zero()) else {
            continue;
        };
        rows.swap(rank, pivot);
        let scale = rows[rank][col].clone();
        for q in &mut rows[rank] {
            *q /= &scale;
        }
        for i in 0..rows.len() {
            if i == rank {
                continue;
            }
            let scale = rows[i][col].clone();
            if scale.is_zero() {
                continue;
            }
            for j in 0..columns {
                work.charge(1)?;
                let q = &scale * &rows[rank][j];
                rows[i][j] -= q;
                if rows[i][j].numer().bits() > 8192 || rows[i][j].denom().bits() > 8192 {
                    return Err(fail("exact atlas elimination bit budget exceeded"));
                }
            }
        }
        pivots.push(col);
    }
    let basis = (0..columns)
        .filter(|c| !pivots.contains(c))
        .map(|free| {
            let mut v = vec![Q::zero(); columns];
            v[free] = Q::one();
            for (i, &c) in pivots.iter().enumerate() {
                v[c] = -rows[i][free].clone();
            }
            v
        })
        .collect::<Vec<_>>();
    // Basis order is stable for old successful recipes. Positive-denominator
    // inverse candidates may require a linear combination of two basis vectors.
    let mut out = basis.clone();
    'pairs: for i in 0..basis.len() {
        for j in i + 1..basis.len() {
            for sign in [-1, 1] {
                if out.len() >= basis.len() + 64 {
                    break 'pairs;
                }
                out.push(
                    basis[i]
                        .iter()
                        .zip(&basis[j])
                        .map(|(a, b)| a + integer(sign) * b)
                        .collect(),
                );
            }
        }
    }
    Ok(out)
}
