//! Exact monotonicity with unordered poles and isolated stationary points.
use super::*;
mod poly;
use poly::*;
const LIMIT: u64 = 65_536;
const MAX_BITS: u64 = 32_768;
pub(super) struct Work {
    left: u64,
}
impl Work {
    pub(super) fn new() -> Self {
        Self { left: LIMIT }
    }
    fn keep(&mut self, x: Q) -> Option<Q> {
        self.left = self.left.checked_sub(1)?;
        if x.numer().bits() > MAX_BITS || x.denom().bits() > MAX_BITS {
            None
        } else {
            Some(x)
        }
    }
    pub(super) fn prove(&mut self, h: &[[Q; 2]]) -> bool {
        self.proof(h).unwrap_or(false)
    }
    fn proof(&mut self, h: &[[Q; 2]]) -> Option<bool> {
        if !(2..=9).contains(&h.len()) || h.iter().any(|x| x[1] <= q(0.)) {
            return Some(false);
        }
        for x in h {
            self.keep(x[0].clone())?;
            self.keep(x[1].clone())?;
        }
        let last = h.last()?;
        if &h[0][0] * &last[1] >= &last[0] * &h[0][1] {
            return Some(false);
        }
        if h.windows(2)
            .all(|x| &x[0][0] * &x[1][1] <= &x[1][0] * &x[0][1])
        {
            return Some(true);
        }
        let p = h.len() - 1;
        let d = 2 * p - 1;
        let mut bern = vec![q(0.); d + 1];
        // G=(N'D-ND')/p. W>0, so the derivative has exactly its sign.
        for k in 0..=d {
            for i in 0..p {
                if k < i || k - i > p {
                    continue;
                }
                let j = k - i;
                let dn = self.keep(&h[i + 1][0] - &h[i][0])?;
                let dw = self.keep(&h[i + 1][1] - &h[i][1])?;
                let a = self.keep(dn * &h[j][1])?;
                let b = self.keep(dw * &h[j][0])?;
                let diff = self.keep(a - b)?;
                let c = choose(p - 1, i) * choose(p, j) / choose(d, k);
                let term = self.keep(c * diff)?;
                bern[k] = self.keep(&bern[k] + term)?;
            }
        }
        let mut power = vec![q(0.); d + 1];
        for k in 0..=d {
            for (i, b) in bern.iter().enumerate().take(k + 1) {
                let term = self.keep(choose(k, i) * b)?;
                power[k] = self.keep(if (k - i) % 2 == 0 {
                    &power[k] + term
                } else {
                    &power[k] - term
                })?;
            }
            power[k] = self.keep(&power[k] * choose(d, k))?;
        }
        trim(&mut power);
        nonnegative(&power, self)
    }
}
fn choose(n: usize, k: usize) -> Q {
    q((0..k).fold(1u64, |a, j| a * (n - j) as u64 / (j + 1) as u64) as f64)
}
/// Yun's square-free decomposition retains only odd-multiplicity factors.
/// G = leading(G) * S² * R, with monic S,R and square-free R. Thus
/// isolated even zeros may be retained without numeric root separation.
fn odd_part(p: &Poly, w: &mut Work) -> Option<Poly> {
    let lead = p.last()?.clone();
    if p.len() == 1 {
        return Some(p.clone());
    }
    let f = normalize(p, false, w)?;
    let df = derivative(&f, w)?;
    let mut c = gcd(&f, &df, w)?;
    let mut a = quotient(&f, &c, w)?;
    let mut out = vec![q(1.)];
    for i in 1..p.len() {
        if a.len() == 1 {
            break;
        }
        let y = gcd(&a, &c, w)?;
        let z = quotient(&a, &y, w)?;
        if i % 2 == 1 {
            out = multiply(&out, &z, w)?;
        }
        c = quotient(&c, &y, w)?;
        a = y;
    }
    if a.len() != 1 {
        return None;
    }
    for x in &mut out {
        *x = w.keep(x.clone() * &lead)?;
    }
    Some(out)
}
fn nonnegative(p: &Poly, w: &mut Work) -> Option<bool> {
    if p.is_empty() {
        return Some(false);
    }
    let mut r = odd_part(p, w)?;
    // Remove endpoint factors. On (0,1), t is positive and t-1 negative.
    while r.first().is_some_and(|x| x.is_zero()) {
        r.remove(0);
    }
    while r.len() > 1 && at_one(&r, w)?.is_zero() {
        r = quotient(&r, &vec![q(-1.), q(1.)], w)?;
        for x in &mut r {
            *x = -x.clone();
        }
    }
    if r.is_empty() || r[0] <= q(0.) {
        return Some(false);
    }
    if r.len() == 1 {
        return Some(true);
    }
    let mut a = normalize(&r, true, w)?;
    let mut b = derivative(&a, w)?;
    b = normalize(&b, true, w)?;
    let mut previous = [0i8; 2];
    let mut variations = [0usize; 2];
    observe(&a, w, &mut previous, &mut variations)?;
    observe(&b, w, &mut previous, &mut variations)?;
    while !b.is_empty() {
        let mut next = divide(&a, &b, w)?.1;
        if next.is_empty() {
            break;
        }
        for x in &mut next {
            *x = -x.clone();
        }
        next = normalize(&next, true, w)?;
        observe(&next, w, &mut previous, &mut variations)?;
        a = b;
        b = next;
    }
    // After endpoint removal, ordinary Sturm variations count all roots in
    // (0,1). The square-free odd part must have none and must be positive.
    Some(variations[0] == variations[1])
}
fn observe(
    p: &Poly,
    w: &mut Work,
    previous: &mut [i8; 2],
    variations: &mut [usize; 2],
) -> Option<()> {
    for (axis, x) in [p[0].clone(), at_one(p, w)?].into_iter().enumerate() {
        let sign = if x > q(0.) {
            1
        } else if x < q(0.) {
            -1
        } else {
            0
        };
        if sign != 0 {
            if previous[axis] != 0 && previous[axis] != sign {
                variations[axis] += 1;
            }
            previous[axis] = sign;
        }
    }
    Some(())
}

#[cfg(test)]
mod tests;
