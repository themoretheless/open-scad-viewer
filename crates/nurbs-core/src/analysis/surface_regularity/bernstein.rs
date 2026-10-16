//! Whole rational normal numerator in the ORIGINAL tensor Bernstein basis.
//! W>0, so W(Hu x Hv) - Wv(Hu x H) - Wu(H x Hv) vanishes exactly
//! where Su x Sv does. Derivatives here use normalized positive span axes.
use super::*;
use crate::distance_bounds::Interval as I;
type Net = Vec<Vec<I>>;
type Normals = Vec<Vec<[I; 3]>>;
fn choose(n: usize, k: usize) -> u64 {
    let mut c = 1u64;
    for j in 0..k.min(n - k) {
        c = c * (n - j) as u64 / (j + 1) as u64;
    }
    c
}
fn derivative(a: &Net, axis: usize) -> Result<Net> {
    let p = if axis == 0 {
        a.len() - 1
    } else {
        a[0].len() - 1
    };
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
fn product(a: &Net, b: &Net) -> Result<Net> {
    let [m, n] = [a.len() - 1, a[0].len() - 1];
    let [p, q] = [b.len() - 1, b[0].len() - 1];
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
fn minus(a: Net, b: Net) -> Result<Net> {
    debug_assert_eq!((a.len(), a[0].len()), (b.len(), b[0].len()));
    a.into_iter()
        .zip(b)
        .map(|(a, b)| a.into_iter().zip(b).map(|(a, b)| a.sub(b)).collect())
        .collect()
}
fn cross(a: &[Net; 3], b: &[Net; 3], k: usize) -> Result<Net> {
    let i = (k + 1) % 3;
    let j = (k + 2) % 3;
    minus(product(&a[i], &b[j])?, product(&a[j], &b[i])?)
}
fn normals(s: &Surface) -> Result<Normals> {
    let mut w = vec![vec![I::point(0.); s.degree_v + 1]; s.degree_u + 1];
    let mut h: [Net; 3] = std::array::from_fn(|_| w.clone());
    let origin = &s.control_points[0][0];
    for u in 0..w.len() {
        for v in 0..w[0].len() {
            w[u][v] = I::point(s.weights[u][v]);
            for k in 0..3 {
                h[k][u][v] = I::point(s.control_points[u][v][k])
                    .sub(I::point(origin[k]))?
                    .mul(w[u][v])?;
            }
        }
    }
    let hu = [
        derivative(&h[0], 0)?,
        derivative(&h[1], 0)?,
        derivative(&h[2], 0)?,
    ];
    let hv = [
        derivative(&h[0], 1)?,
        derivative(&h[1], 1)?,
        derivative(&h[2], 1)?,
    ];
    let wu = derivative(&w, 0)?;
    let wv = derivative(&w, 1)?;
    let mut out = vec![vec![[I::point(0.); 3]; 3 * s.degree_v]; 3 * s.degree_u];
    for k in 0..3 {
        let first = product(&cross(&hu, &hv, k)?, &w)?;
        let second = product(&cross(&hu, &h, k)?, &wv)?;
        let third = product(&cross(&h, &hv, k)?, &wu)?;
        let net = minus(minus(first, second)?, third)?;
        for u in 0..out.len() {
            for v in 0..out[0].len() {
                out[u][v][k] = net[u][v];
            }
        }
    }
    Ok(out)
}
fn split_line(mut a: Vec<[I; 3]>) -> Result<(Vec<[I; 3]>, Vec<[I; 3]>)> {
    let mut left = vec![a[0]];
    let mut right = vec![*a.last().unwrap()];
    for n in (1..a.len()).rev() {
        for i in 0..n {
            for k in 0..3 {
                a[i][k] = a[i][k].add(a[i + 1][k])?.mul(I::point(0.5))?;
            }
        }
        left.push(a[0]);
        right.push(a[n - 1]);
    }
    right.reverse();
    Ok((left, right))
}
fn split(a: Normals, axis: usize) -> Result<(Normals, Normals)> {
    let mut left = a.clone();
    let mut right = a.clone();
    if axis == 0 {
        for v in 0..a[0].len() {
            let (l, r) = split_line(a.iter().map(|row| row[v]).collect())?;
            for u in 0..a.len() {
                left[u][v] = l[u];
                right[u][v] = r[u];
            }
        }
    } else {
        for u in 0..a.len() {
            let (l, r) = split_line(a[u].clone())?;
            left[u] = l;
            right[u] = r;
        }
    }
    Ok((left, right))
}
/// Only literal single Bezier patches of bounded degree use this finite
/// coefficient path. Other bases keep the original generic jet verifier.
pub(super) fn inspect(s: &Surface, max: usize) -> Result<Option<Report>> {
    let bezier = |p: usize, n: usize, k: &[f64]| {
        n == p + 1
            && k.len() == 2 * n
            && k[..n].iter().all(|x| *x == k[0])
            && k[n..].iter().all(|x| *x == k[n])
    };
    if s.periodic_u
        || s.periodic_v
        || s.degree_u > 5
        || s.degree_v > 5
        || !bezier(s.degree_u, s.control_points.len(), &s.knots_u)
        || !bezier(s.degree_v, s.control_points[0].len(), &s.knots_v)
    {
        return Ok(None);
    }
    let domain = [
        [s.knots_u[s.degree_u], s.knots_u[s.degree_u + 1]],
        [s.knots_v[s.degree_v], s.knots_v[s.degree_v + 1]],
    ];
    // Coefficients split at exact dyadic half parameters. Keep their parameter
    // labels normalized too; rounded natural-domain midpoints need not be the
    // same half. Depth 100 limits either axis to 50 exact binary subdivisions.
    let mut pending = vec![(normals(s)?, [[0., 1.]; 2], 0usize)];
    let mut out = Report {
        spanwise_regular: false,
        interior_basis_c1: true,
        unresolved: Vec::new(),
        cells: 1,
        proven_cells: 0,
    };
    while let Some((net, normalized, depth)) = pending.pop() {
        let separated = (0..3).any(|k| {
            net.iter().flatten().all(|x| x[k].lo > 0.) || net.iter().flatten().all(|x| x[k].hi < 0.)
        });
        if separated {
            out.proven_cells += 1;
            continue;
        }
        let axis = depth % 2;
        let [a, b] = normalized[axis];
        let m = a * 0.5 + b * 0.5;
        if out.cells + 2 > max || depth >= 100 || !(a < m && m < b) {
            let mut unresolved = domain;
            for k in 0..2 {
                let width = I::point(domain[k][1]).sub(I::point(domain[k][0]))?;
                let lo = I::point(domain[k][0]).add(width.mul(I::point(normalized[k][0]))?)?;
                let hi = I::point(domain[k][0]).add(width.mul(I::point(normalized[k][1]))?)?;
                unresolved[k] = [lo.lo.max(domain[k][0]), hi.hi.min(domain[k][1])];
            }
            out.unresolved.push(UnresolvedCell { domain: unresolved });
            continue;
        }
        let (left, right) = split(net, axis)?;
        let mut l = normalized;
        let mut r = normalized;
        l[axis] = [a, m];
        r[axis] = [m, b];
        pending.push((right, r, depth + 1));
        pending.push((left, l, depth + 1));
        out.cells += 2;
    }
    out.spanwise_regular = out.unresolved.is_empty();
    Ok(Some(out))
}
