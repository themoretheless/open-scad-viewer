//! Shared outward tensor-Bernstein polynomial arithmetic. No sampled geometry
//! or public proof callback. Product degrees are bounded before integer choose.
use crate::{Result, check, distance_bounds::Interval as I};
pub(crate) type Net = Vec<Vec<I>>;
fn choose(n: usize, k: usize) -> u64 {
    let mut c = 1u64;
    for j in 0..k.min(n - k) {
        c = c * (n - j) as u64 / (j + 1) as u64;
    }
    c
}
pub(crate) fn derivative(a: &Net, axis: usize) -> Result<Net> {
    let p = if axis == 0 {
        a.len() - 1
    } else {
        a[0].len() - 1
    };
    if p == 0 {
        return Ok(vec![vec![I::point(0.); a[0].len()]; a.len()]);
    }
    let mut out = vec![
        vec![I::point(0.); a[0].len() - usize::from(axis == 1)];
        a.len() - usize::from(axis == 0)
    ];
    for u in 0..out.len() {
        for v in 0..out[0].len() {
            out[u][v] = a[u + usize::from(axis == 0)][v + usize::from(axis == 1)]
                .sub(a[u][v])?
                .mul(I::point(p as f64))?;
        }
    }
    Ok(out)
}
pub(crate) fn product(a: &Net, b: &Net) -> Result<Net> {
    let [m, n] = [a.len() - 1, a[0].len() - 1];
    let [p, q] = [b.len() - 1, b[0].len() - 1];
    check(
        m + p <= 15 && n + q <= 15,
        "Tensor Bernstein product degree exceeds finite kernel ceiling",
    )?;
    let mut out = vec![vec![I::point(0.); n + q + 1]; m + p + 1];
    for k in 0..=m + p {
        for l in 0..=n + q {
            for i in k.saturating_sub(p)..=k.min(m) {
                for j in l.saturating_sub(q)..=l.min(n) {
                    let u = I::point(choose(m, i) as f64)
                        .mul(I::point(choose(p, k - i) as f64))?
                        .div(I::point(choose(m + p, k) as f64))?;
                    let v = I::point(choose(n, j) as f64)
                        .mul(I::point(choose(q, l - j) as f64))?
                        .div(I::point(choose(n + q, l) as f64))?;
                    out[k][l] = out[k][l].add(a[i][j].mul(b[k - i][l - j])?.mul(u)?.mul(v)?)?;
                }
            }
        }
    }
    Ok(out)
}
pub(crate) fn minus(a: Net, b: Net) -> Result<Net> {
    check(
        (a.len(), a[0].len()) == (b.len(), b[0].len()),
        "Tensor Bernstein subtraction requires equal degrees",
    )?;
    a.into_iter()
        .zip(b)
        .map(|(a, b)| a.into_iter().zip(b).map(|(a, b)| a.sub(b)).collect())
        .collect()
}

pub(crate) fn evaluate_line(mut a: Vec<I>, t: I) -> Result<I> {
    let hull = I::new(
        a.iter().map(|x| x.lo).fold(f64::INFINITY, f64::min),
        a.iter().map(|x| x.hi).fold(f64::NEG_INFINITY, f64::max),
    )?;
    let beta = I::point(1.).sub(t)?.intersect(0., 1.)?;
    for n in (1..a.len()).rev() {
        for i in 0..n {
            let x = a[i];
            let y = a[i + 1];
            let direct = x.mul(beta)?.add(y.mul(t)?)?;
            let left = x.add(y.sub(x)?.mul(t)?)?;
            let right = y.add(x.sub(y)?.mul(beta)?)?;
            a[i] = I::new(
                direct.lo.max(left.lo).max(right.lo),
                direct.hi.min(left.hi).min(right.hi),
            )?;
        }
    }
    a[0].intersect(hull.lo, hull.hi)
}
pub(crate) fn evaluate(a: &Net, t: [I; 2]) -> Result<I> {
    let rows = a
        .iter()
        .map(|r| evaluate_line(r.clone(), t[1]))
        .collect::<Result<Vec<_>>>()?;
    evaluate_line(rows, t[0])
}
