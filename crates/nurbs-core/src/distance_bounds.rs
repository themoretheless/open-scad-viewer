//! Outward binary64 interval arithmetic shared by curve and surface distance.
use crate::{Result, numeric};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Interval {
    pub(crate) lo: f64,
    pub(crate) hi: f64,
}
impl Interval {
    pub(crate) fn point(x: f64) -> Self {
        Self { lo: x, hi: x }
    }
    pub(crate) fn new(lo: f64, hi: f64) -> Result<Self> {
        numeric(
            lo.is_finite() && hi.is_finite() && lo <= hi,
            "Distance interval exceeded numeric range",
        )?;
        Ok(Self { lo, hi })
    }
    pub(crate) fn add(self, b: Self) -> Result<Self> {
        Self::new((self.lo + b.lo).next_down(), (self.hi + b.hi).next_up())
    }
    pub(crate) fn sub(self, b: Self) -> Result<Self> {
        Self::new((self.lo - b.hi).next_down(), (self.hi - b.lo).next_up())
    }
    pub(crate) fn mul(self, b: Self) -> Result<Self> {
        let p = [
            self.lo * b.lo,
            self.lo * b.hi,
            self.hi * b.lo,
            self.hi * b.hi,
        ];
        numeric(
            p.iter().all(|x| x.is_finite()),
            "Distance product exceeded numeric range",
        )?;
        Self::new(
            p.iter().copied().fold(f64::INFINITY, f64::min).next_down(),
            p.iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max)
                .next_up(),
        )
    }
    pub(crate) fn div(self, b: Self) -> Result<Self> {
        numeric(b.lo > 0., "Distance denominator is not separated from zero")?;
        self.mul(Self::new((1. / b.hi).next_down(), (1. / b.lo).next_up())?)
    }
    pub(crate) fn intersect(self, lo: f64, hi: f64) -> Result<Self> {
        Self::new(self.lo.max(lo), self.hi.min(hi))
    }
}

pub(crate) fn box_distance(a: &[Interval], b: &[Interval]) -> Result<(f64, f64)> {
    let mut low = Interval::point(0.);
    let mut high = Interval::point(0.);
    for (&x, &y) in a.iter().zip(b) {
        let delta = x.sub(y)?;
        let minimum = if delta.lo > 0. {
            delta.lo
        } else if delta.hi < 0. {
            -delta.hi
        } else {
            0.
        };
        let maximum = delta.lo.abs().max(delta.hi.abs());
        low = low.add(Interval::point(minimum).mul(Interval::point(minimum))?)?;
        high = high.add(Interval::point(maximum).mul(Interval::point(maximum))?)?;
    }
    let lo = low.lo.max(0.).sqrt().next_down().max(0.);
    let hi = high.hi.sqrt().next_up();
    numeric(hi.is_finite(), "Distance norm exceeded numeric range")?;
    Ok((lo, hi))
}
