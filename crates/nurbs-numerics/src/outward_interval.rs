//! Outward-rounded intervals with Bernstein knot insertion and binomials, shared by seam
//! regularity and surface deviation certificates.
use crate::{Result, check};
use math_core::{next_down, next_up};
#[derive(Clone, Copy, Debug)]
pub struct I {
    pub lo: f64,
    pub hi: f64,
}
impl I {
    pub fn point(x: f64) -> Self {
        Self { lo: x, hi: x }
    }
    pub fn add(self, b: Self) -> Self {
        Self {
            lo: next_down(self.lo + b.lo),
            hi: next_up(self.hi + b.hi),
        }
    }
    pub fn sub(self, b: Self) -> Self {
        Self {
            lo: next_down(self.lo - b.hi),
            hi: next_up(self.hi - b.lo),
        }
    }
    pub fn mul(self, b: Self) -> Self {
        let v = [
            self.lo * b.lo,
            self.lo * b.hi,
            self.hi * b.lo,
            self.hi * b.hi,
        ];
        Self {
            lo: next_down(v.iter().copied().fold(f64::INFINITY, f64::min)),
            hi: next_up(v.iter().copied().fold(f64::NEG_INFINITY, f64::max)),
        }
    }
    pub fn div_positive(self, b: Self) -> Result<Self> {
        check(
            b.lo > 0. && b.hi.is_finite(),
            "Seam certificate denominator is not separated from zero",
        )?;
        Ok(self.mul(Self {
            lo: next_down(1. / b.hi),
            hi: next_up(1. / b.lo),
        }))
    }
    pub fn finite(self) -> bool {
        self.lo.is_finite() && self.hi.is_finite()
    }
}
pub fn insert<const N: usize>(
    knots: &mut Vec<f64>,
    controls: &mut Vec<[I; N]>,
    p: usize,
    u: f64,
) -> Result<()> {
    check(
        controls.len() < 8192,
        "Seam certificate knot refinement exceeds 8192 controls",
    )?;
    let n = controls.len() - 1;
    let s = knots.iter().filter(|&&k| k == u).count();
    let mut k = p;
    while k + 1 < knots.len() && knots[k + 1] <= u {
        k += 1;
    }
    check(
        s <= p && k >= p && k - s <= n,
        "Seam certificate knot insertion is invalid",
    )?;
    let mut next = vec![[I::point(0.); N]; n + 2];
    next[..=k - p].copy_from_slice(&controls[..=k - p]);
    next[k - s + 1..n + 2].copy_from_slice(&controls[k - s..n + 1]);
    for i in k - p + 1..=k - s {
        let alpha = I::point(u)
            .sub(I::point(knots[i]))
            .div_positive(I::point(knots[i + p]).sub(I::point(knots[i])))?;
        let beta = I::point(1.).sub(alpha);
        next[i] =
            std::array::from_fn(|a| alpha.mul(controls[i][a]).add(beta.mul(controls[i - 1][a])));
    }
    knots.insert(k + 1, u);
    *controls = next;
    Ok(())
}
pub fn choose(n: usize, k: usize) -> I {
    let mut x = I::point(1.);
    for i in 0..k.min(n - k) {
        let d = (i + 1) as f64;
        x = x.mul(I::point((n - i) as f64)).mul(I {
            lo: next_down(1. / d),
            hi: next_up(1. / d),
        });
    }
    x
}
