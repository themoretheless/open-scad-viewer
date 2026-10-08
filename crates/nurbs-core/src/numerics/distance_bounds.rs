//! Outward binary64 interval arithmetic shared by curve and surface distance.
use crate::{Result, check, numeric};

/// Outward-rounded binary64 interval `[lo, hi]`, shared by curve and surface
/// distance machinery and promoted for public certified-enclosure APIs
/// (`crate::interval_eval`). Every arithmetic operation rounds `lo` down and
/// `hi` up, so the true result always lies inside.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    pub lo: f64,
    pub hi: f64,
}
impl Interval {
    /// Degenerate interval containing exactly `x`.
    pub fn point(x: f64) -> Self {
        assert!(x.is_finite(), "Interval::point requires finite coordinate");
        Self { lo: x, hi: x }
    }
    /// Construct a validated interval; rejects non-finite or inverted bounds.
    pub fn new(lo: f64, hi: f64) -> Result<Self> {
        numeric(
            lo.is_finite() && hi.is_finite() && lo <= hi,
            "Distance interval exceeded numeric range",
        )?;
        Ok(Self { lo, hi })
    }
    /// Outward-rounded sum.
    pub fn add(self, b: Self) -> Result<Self> {
        Self::new((self.lo + b.lo).next_down(), (self.hi + b.hi).next_up())
    }
    /// Outward-rounded difference.
    pub fn sub(self, b: Self) -> Result<Self> {
        Self::new((self.lo - b.hi).next_down(), (self.hi - b.lo).next_up())
    }
    /// Outward-rounded product.
    pub fn mul(self, b: Self) -> Result<Self> {
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
    /// Outward-rounded quotient; `b` must be separated above zero.
    pub fn div(self, b: Self) -> Result<Self> {
        numeric(b.lo > 0., "Distance denominator is not separated from zero")?;
        self.mul(Self::new((1. / b.hi).next_down(), (1. / b.lo).next_up())?)
    }
    /// Outward-rounded quotient for a nonzero-signed denominator.
    pub fn div_signed(self, b: Self) -> Result<Self> {
        if b.hi < 0. {
            Self::new(-self.hi, -self.lo)?.div(Self::new(-b.hi, -b.lo)?)
        } else {
            self.div(b)
        }
    }
    /// Tighten against the plain scalar bounds `lo..=hi`.
    pub fn intersect(self, lo: f64, hi: f64) -> Result<Self> {
        Self::new(self.lo.max(lo), self.hi.min(hi))
    }
    /// Optional tightening against scalar bounds; returns `Ok(None)` if disjoint.
    pub fn try_intersect(self, lo: f64, hi: f64) -> Result<Option<Self>> {
        numeric(
            lo.is_finite() && hi.is_finite() && lo <= hi,
            "Scalar bounds exceeded numeric range",
        )?;
        let n_lo = self.lo.max(lo);
        let n_hi = self.hi.min(hi);
        if n_lo <= n_hi {
            Ok(Some(Self { lo: n_lo, hi: n_hi }))
        } else {
            Ok(None)
        }
    }
    /// Width `hi - lo` (an upper bound on the enclosure uncertainty).
    pub fn width(self) -> f64 {
        self.hi - self.lo
    }
    /// Whether `x` lies inside the interval.
    pub fn contains(self, x: f64) -> bool {
        self.lo <= x && x <= self.hi
    }
}

pub(crate) fn box_distance(a: &[Interval], b: &[Interval]) -> Result<(f64, f64)> {
    check(
        a.len() == b.len(),
        "Box distance requires identical spatial dimensions",
    )?;
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

#[cfg(feature = "codec")]
mod serialization;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DistanceStopReason {
    Separated,
    Tolerance,
    WorkLimit,
    PrecisionLimit,
    EmptyDomain,
    DomainWorkLimit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_rejects_nan_and_infinity() {
        let nan_result = std::panic::catch_unwind(|| Interval::point(f64::NAN));
        assert!(nan_result.is_err());
        let inf_result = std::panic::catch_unwind(|| Interval::point(f64::INFINITY));
        assert!(inf_result.is_err());
    }

    #[test]
    fn box_distance_dimension_mismatch_fails() {
        let a = [Interval::point(0.), Interval::point(1.)];
        let b = [Interval::point(0.)];
        assert!(box_distance(&a, &b).is_err());
    }

    #[test]
    fn try_intersect_disjoint_and_overlapping() {
        let iv = Interval::new(0., 2.).unwrap();
        assert_eq!(iv.try_intersect(3., 4.).unwrap(), None);
        assert_eq!(iv.try_intersect(-1., 1.).unwrap(), Some(Interval { lo: 0., hi: 1. }));
    }
}
