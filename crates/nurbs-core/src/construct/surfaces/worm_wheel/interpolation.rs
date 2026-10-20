//! Quintic Lagrange-to-Bernstein intervals and rigorous operator/remainder norms.
use super::*;
pub(super) const NODES: [f64; 6] = [0., 0.125, 0.375, 0.625, 0.875, 1.];
pub(super) struct Operator {
    pub matrix: [[I; 6]; 6],
    pub lebesgue: f64,
    pub weight: f64,
}
fn choose(n: usize, k: usize) -> f64 {
    let mut a = 1usize;
    for i in 0..k {
        a = a * (n - i) / (i + 1);
    }
    a as f64
}
fn roots(values: &[f64]) -> Result<Vec<I>> {
    let mut a = vec![I::point(1.)];
    for &root in values {
        let mut b = vec![I::point(0.); a.len() + 1];
        for i in 0..a.len() {
            b[i] = b[i].sub(a[i].mul(I::point(root))?)?;
            b[i + 1] = b[i + 1].add(a[i])?;
        }
        a = b;
    }
    Ok(a)
}
fn bernstein(a: &[I]) -> Result<Vec<I>> {
    let n = a.len() - 1;
    let mut b = vec![I::point(0.); n + 1];
    for i in 0..=n {
        for k in 0..=i {
            b[i] = b[i].add(
                a[k].mul(I::point(choose(i, k)))?
                    .div(I::point(choose(n, k)))?,
            )?;
        }
    }
    Ok(b)
}
fn split(a: &[I]) -> Result<(Vec<I>, Vec<I>)> {
    let n = a.len();
    let mut level = a.to_vec();
    let mut left = Vec::with_capacity(n);
    let mut right = Vec::with_capacity(n);
    left.push(a[0]);
    right.push(*a.last().unwrap());
    while level.len() > 1 {
        level = level
            .windows(2)
            .map(|p| p[0].add(p[1])?.mul(I::point(0.5)))
            .collect::<Result<_>>()?;
        left.push(level[0]);
        right.push(*level.last().unwrap());
    }
    right.reverse();
    Ok((left, right))
}
fn abs(a: I) -> f64 {
    a.lo.abs().max(a.hi.abs())
}
fn norm(polynomials: Vec<Vec<I>>) -> Result<f64> {
    let mut cells = vec![polynomials];
    for _ in 0..4 {
        let mut next = Vec::with_capacity(cells.len() * 2);
        for p in cells {
            let pairs = p.iter().map(|p| split(p)).collect::<Result<Vec<_>>>()?;
            next.push(pairs.iter().map(|p| p.0.clone()).collect());
            next.push(pairs.into_iter().map(|p| p.1).collect());
        }
        cells = next;
    }
    let mut bound = 0_f64;
    for cell in cells {
        let mut sum = I::point(0.);
        for p in cell {
            sum = sum.add(I::point(p.into_iter().map(abs).fold(0_f64, f64::max)))?;
        }
        bound = bound.max(sum.hi);
    }
    Ok(bound)
}
pub(super) fn operator() -> Result<Operator> {
    let mut matrix = [[I::point(0.); 6]; 6];
    let mut polynomials = Vec::with_capacity(6);
    for i in 0..6 {
        let others = NODES
            .iter()
            .enumerate()
            .filter_map(|(j, x)| (i != j).then_some(*x))
            .collect::<Vec<_>>();
        let mut denominator = I::point(1.);
        for &x in &others {
            denominator = denominator.mul(I::point(NODES[i]).sub(I::point(x))?)?;
        }
        let power = roots(&others)?
            .into_iter()
            .map(|a| a.div_signed(denominator))
            .collect::<Result<Vec<_>>>()?;
        let mut b = bernstein(&power)?;
        // Exact cardinal endpoint identities follow from the original product.
        b[0] = I::point(if i == 0 { 1. } else { 0. });
        b[5] = I::point(if i == 5 { 1. } else { 0. });
        for k in 0..6 {
            matrix[k][i] = b[k];
        }
        polynomials.push(b);
    }
    Ok(Operator {
        matrix,
        lebesgue: norm(polynomials)?,
        weight: norm(vec![bernstein(&roots(&NODES)?)?])?,
    })
}
pub(super) fn transform(data: &[[I; 3]; 6], op: &Operator) -> Result<[[I; 3]; 6]> {
    let mut out = [[I::point(0.); 3]; 6];
    out[0] = data[0];
    out[5] = data[5];
    // Copy exact endpoint identities; do not accumulate rounded zero terms.
    for k in 1..5 {
        for i in 0..6 {
            for d in 0..3 {
                out[k][d] = out[k][d].add(op.matrix[k][i].mul(data[i][d])?)?;
            }
        }
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cardinal_quintic_basis_and_operator_bounds_are_independent_of_source_geometry() {
        let op = operator().unwrap();
        assert!(op.lebesgue >= 1. && op.lebesgue < 4.);
        assert!(op.weight > 0. && op.weight < 0.02);
        let samples = NODES.map(|x| [I::point(x.powi(5)); 3]);
        let b = transform(&samples, &op).unwrap();
        for i in 0..6 {
            let expected = if i == 5 { 1. } else { 0. };
            assert!(b[i][0].lo <= expected && expected <= b[i][0].hi);
        }
    }
}
