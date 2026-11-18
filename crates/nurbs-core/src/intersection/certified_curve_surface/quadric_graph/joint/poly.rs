//! Bounded exact tensor Bernstein algebra for the ORIGINAL projected Jacobian.
use super::*;
#[derive(Clone)]
pub(super) struct Poly(pub Vec<Vec<Q>>);
pub(super) const LIMIT: u64 = 4_000_000;
pub(super) struct Work(u64);
impl Work {
    pub(super) fn new(n: u64) -> Self {
        Self(n)
    }
    pub(super) fn charge(&mut self, n: u64) -> Option<()> {
        self.0 = self.0.checked_sub(n)?;
        Some(())
    }
    pub(super) fn keep(&mut self, x: Q) -> Option<Q> {
        self.charge(1)?;
        if x.numer().bits() > 32768 || x.denom().bits() > 32768 {
            None
        } else {
            Some(x)
        }
    }
}
fn choose(n: usize, k: usize) -> Q {
    q((0..k).fold(1u64, |a, j| a * (n - j) as u64 / (j + 1) as u64) as f64)
}
impl Poly {
    fn degrees(&self) -> [usize; 2] {
        [self.0.len() - 1, self.0[0].len() - 1]
    }
    pub(super) fn derivative(&self, axis: usize, w: &mut Work) -> Option<Self> {
        let [p, r] = self.degrees();
        let (n, m) = (
            p + 1 - usize::from(axis == 0),
            r + 1 - usize::from(axis == 1),
        );
        let mut out = vec![vec![q(0.); m]; n];
        for i in 0..n {
            for j in 0..m {
                out[i][j] = w.keep(
                    (&self.0[i + usize::from(axis == 0)][j + usize::from(axis == 1)]
                        - &self.0[i][j])
                        * q(if axis == 0 { p } else { r } as f64),
                )?;
            }
        }
        Some(Self(out))
    }
    pub(super) fn mul(&self, b: &Self, w: &mut Work) -> Option<Self> {
        let [p, r] = self.degrees();
        let [n, m] = b.degrees();
        let mut out = vec![vec![q(0.); r + m + 1]; p + n + 1];
        for i in 0..=p {
            for j in 0..=r {
                for k in 0..=n {
                    for l in 0..=m {
                        let factor = choose(p, i) * choose(n, k) / choose(p + n, i + k)
                            * choose(r, j)
                            * choose(m, l)
                            / choose(r + m, j + l);
                        let t = w.keep(&self.0[i][j] * &b.0[k][l] * factor)?;
                        out[i + k][j + l] = w.keep(&out[i + k][j + l] + t)?;
                    }
                }
            }
        }
        Some(Self(out))
    }
    pub(super) fn sub(&self, b: &Self, w: &mut Work) -> Option<Self> {
        if self.degrees() != b.degrees() {
            return None;
        }
        let mut out = self.clone();
        for (i, row) in out.0.iter_mut().enumerate() {
            for (j, x) in row.iter_mut().enumerate() {
                *x = w.keep(&self.0[i][j] - &b.0[i][j])?;
            }
        }
        Some(out)
    }
    pub(super) fn add(&self, b: &Self, w: &mut Work) -> Option<Self> {
        if self.degrees() != b.degrees() {
            return None;
        }
        let mut out = self.clone();
        for (i, row) in out.0.iter_mut().enumerate() {
            for (j, x) in row.iter_mut().enumerate() {
                *x = w.keep(&self.0[i][j] + &b.0[i][j])?;
            }
        }
        Some(out)
    }
    pub(super) fn positive(&self, w: &mut Work, depth: usize) -> Option<bool> {
        w.charge((self.0.len() * self.0[0].len()) as u64)?;
        if self.0.iter().flatten().all(|x| *x > q(0.)) {
            return Some(true);
        }
        let [p, r] = self.degrees();
        if [
            self.0[0][0].clone(),
            self.0[p][0].clone(),
            self.0[0][r].clone(),
            self.0[p][r].clone(),
        ]
        .iter()
        .any(|x| *x <= q(0.))
            || depth == 8
        {
            return Some(false);
        }
        let (a, b) = split(
            self.0
                .iter()
                .map(|row| row.iter().map(|x| [x.clone()]).collect())
                .collect(),
            depth % 2,
            w,
        )?;
        let scalar = |h: Vec<Vec<[Q; 1]>>| {
            Self(
                h.into_iter()
                    .map(|row| row.into_iter().map(|x| x[0].clone()).collect())
                    .collect(),
            )
        };
        Some(scalar(a).positive(w, depth + 1)? && scalar(b).positive(w, depth + 1)?)
    }
}
pub(super) fn split<const N: usize>(
    h: Vec<Vec<[Q; N]>>,
    axis: usize,
    w: &mut Work,
) -> Option<(Vec<Vec<[Q; N]>>, Vec<Vec<[Q; N]>>)> {
    let (p, r) = (h.len() - 1, h[0].len() - 1);
    let mut left = h.clone();
    let mut right = h.clone();
    let (n, m) = if axis == 0 { (p, r) } else { (r, p) };
    for j in 0..=m {
        let mut d: Vec<_> = (0..=n)
            .map(|i| {
                if axis == 0 {
                    h[i][j].clone()
                } else {
                    h[j][i].clone()
                }
            })
            .collect();
        let mut a = vec![d[0].clone()];
        let mut b = vec![d[n].clone()];
        for level in 1..=n {
            for i in 0..=n - level {
                for k in 0..N {
                    d[i][k] = w.keep((&d[i][k] + &d[i + 1][k]) / q(2.))?;
                }
            }
            a.push(d[0].clone());
            b.push(d[n - level].clone());
        }
        b.reverse();
        for i in 0..=n {
            if axis == 0 {
                left[i][j] = a[i].clone();
                right[i][j] = b[i].clone();
            } else {
                left[j][i] = a[i].clone();
                right[j][i] = b[i].clone();
            }
        }
    }
    Some((left, right))
}

#[cfg(test)]
mod tests;
