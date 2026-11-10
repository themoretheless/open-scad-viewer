//! Outward Bernstein algebra; budgets count every convolution product.
use super::*;
pub(super) type Polynomial = Vec<Vec<I>>;
pub(super) struct Work {
    pub used: u64,
    pub limit: u64,
}
fn choose(n: usize, k: usize) -> u64 {
    (0..k).fold(1, |v, i| v * (n - i) as u64 / (i + 1) as u64)
}
impl Work {
    pub fn mul(&mut self, a: &Polynomial, b: &Polynomial) -> Result<Polynomial> {
        let (p, q, r, s) = (a.len() - 1, a[0].len() - 1, b.len() - 1, b[0].len() - 1);
        let weights = |p: usize, r: usize| -> Result<Vec<Vec<I>>> {
            (0..=p)
                .map(|i| {
                    (0..=r)
                        .map(|k| {
                            I::point(choose(p, i) as f64)
                                .mul(I::point(choose(r, k) as f64))?
                                .div(I::point(choose(p + r, i + k) as f64))
                        })
                        .collect()
                })
                .collect()
        };
        let u = weights(p, r)?;
        let v = weights(q, s)?;
        let mut out = vec![vec![I::point(0.); q + s + 1]; p + r + 1];
        for i in 0..=p {
            for j in 0..=q {
                for k in 0..=r {
                    for l in 0..=s {
                        if self.used == self.limit {
                            return Err(crate::resource("Polar graph polynomial product budget"));
                        }
                        self.used += 1;
                        let term = a[i][j].mul(b[k][l])?.mul(u[i][k])?.mul(v[j][l])?;
                        out[i + k][j + l] = out[i + k][j + l].add(term)?;
                    }
                }
            }
        }
        Ok(out)
    }
    pub fn difference_products(
        &mut self,
        a: &Polynomial,
        b: &Polynomial,
        c: &Polynomial,
        d: &Polynomial,
    ) -> Result<Polynomial> {
        let mut out = self.mul(a, b)?;
        let rhs = self.mul(c, d)?;
        assert_eq!(out.len(), rhs.len());
        assert_eq!(out[0].len(), rhs[0].len());
        for (a, b) in out.iter_mut().flatten().zip(rhs.iter().flatten()) {
            *a = a.sub(*b)?;
        }
        Ok(out)
    }
}
pub(super) fn derivative(a: &Polynomial, axis: usize) -> Result<Polynomial> {
    let (p, q) = (a.len() - 1, a[0].len() - 1);
    let degree = if axis == 0 { p } else { q };
    if degree == 0 {
        return Ok(vec![
            vec![I::point(0.); if axis == 1 { 1 } else { q + 1 }];
            if axis == 0 { 1 } else { p + 1 }
        ]);
    }
    (0..=p - usize::from(axis == 0))
        .map(|i| {
            (0..=q - usize::from(axis == 1))
                .map(|j| {
                    a[i + usize::from(axis == 0)][j + usize::from(axis == 1)]
                        .sub(a[i][j])?
                        .mul(I::point(degree as f64))
                })
                .collect()
        })
        .collect()
}
/// Used only when a private source identity has ALREADY proved the first row
/// exactly zero. Dropping a merely small interval row would be unsound.
pub(super) fn divide_known_radial_zero(a: &Polynomial) -> Result<Polynomial> {
    let degree = a.len() - 1;
    check(
        degree > 0,
        "A known radial factor needs positive polynomial degree",
    )?;
    (1..=degree)
        .map(|i| {
            a[i].iter()
                .map(|x| x.mul(I::point(degree as f64).div(I::point(i as f64))?))
                .collect()
        })
        .collect()
}
pub(super) fn bounds(a: &Polynomial, range: [[I; 2]; 2], point: [bool; 2]) -> Result<I> {
    let mut net = a.clone();
    for v in 0..net[0].len() {
        let column = net
            .iter()
            .map(|r| [r[v], I::point(0.), I::point(0.)])
            .collect::<Vec<_>>();
        let restricted = restrict(&column, range[0], point[0])?;
        for (row, p) in net.iter_mut().zip(restricted) {
            row[v] = p[0];
        }
    }
    for row in &mut net {
        let source = row
            .iter()
            .map(|&x| [x, I::point(0.), I::point(0.)])
            .collect::<Vec<_>>();
        *row = restrict(&source, range[1], point[1])?
            .into_iter()
            .map(|p| p[0])
            .collect();
    }
    I::new(
        net.iter()
            .flatten()
            .map(|x| x.lo)
            .fold(f64::INFINITY, f64::min),
        net.iter()
            .flatten()
            .map(|x| x.hi)
            .fold(f64::NEG_INFINITY, f64::max),
    )
}
