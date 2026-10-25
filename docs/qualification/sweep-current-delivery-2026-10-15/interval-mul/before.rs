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
            p[0].is_finite() && p[1].is_finite() && p[2].is_finite() && p[3].is_finite(),
            "Distance product exceeded numeric range",
        )?;
        Self::new(
            f64::INFINITY
                .min(p[0])
                .min(p[1])
                .min(p[2])
                .min(p[3])
                .next_down(),
            f64::NEG_INFINITY
                .max(p[0])
                .max(p[1])
                .max(p[2])
                .max(p[3])
                .next_up(),
        )
    }
    /// Outward-rounded quotient; `b` must be separated above zero.
    pub fn div(self, b: Self) -> Result<Self> {
        numeric(b.lo > 0., "Distance denominator is not separated from zero")?;
        let reciprocal = [(1. / b.hi).next_down(), (1. / b.lo).next_up()];
        if reciprocal.iter().all(|v| v.is_finite()) {
            return self.mul(Self::new(reciprocal[0], reciprocal[1])?);
        }
        // A reciprocal can overflow although every requested ratio is finite
        // (for example, subnormal/subnormal). With a positive denominator,
        // the exact quotient extrema occur at the four endpoint combinations.
        let ratios = [
            self.lo / b.lo,
            self.lo / b.hi,
            self.hi / b.lo,
            self.hi / b.hi,
        ];
        numeric(
            ratios.iter().all(|v| v.is_finite()),
            "Distance quotient exceeded numeric range",
        )?;
        Self::new(
            ratios
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min)
                .next_down(),
            ratios
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max)
                .next_up(),
        )
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

#[cfg(test)]
mod product_regression {
    use super::*;
    fn previous_product(a: Interval, b: Interval) -> Result<Interval> {
        let p = [a.lo * b.lo, a.lo * b.hi, a.hi * b.lo, a.hi * b.hi];
        numeric(
            p.iter().all(|x| x.is_finite()),
            "Distance product exceeded numeric range",
        )?;
        Interval::new(
            p.iter().copied().fold(f64::INFINITY, f64::min).next_down(),
            p.iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max)
                .next_up(),
        )
    }
    #[test]
    fn product_is_bit_identical_for_signed_zero_subnormals_large_bounds_and_refusals() {
        let values = [
            -f64::MAX,
            -1e100,
            -1.,
            -f64::MIN_POSITIVE,
            -f64::from_bits(1),
            -0.,
            0.,
            f64::from_bits(1),
            f64::MIN_POSITIVE,
            1.,
            1e100,
            f64::MAX,
        ];
        let mut intervals = Vec::new();
        for lo in values {
            for hi in values {
                if lo <= hi {
                    intervals.push(Interval::new(lo, hi).unwrap());
                }
            }
        }
        intervals.extend([
            Interval::point(f64::NAN),
            Interval::point(f64::INFINITY),
            Interval::point(f64::NEG_INFINITY),
        ]);
        for &a in &intervals {
            for &b in &intervals {
                let old = previous_product(a, b);
                let new = a.mul(b);
                match (old, new) {
                    (Ok(old), Ok(new)) => {
                        assert_eq!(old.lo.to_bits(), new.lo.to_bits(), "{a:?} {b:?}");
                        assert_eq!(old.hi.to_bits(), new.hi.to_bits(), "{a:?} {b:?}");
                    }
                    (Err(_), Err(_)) => {}
                    _ => panic!("Changed numeric refusal for {a:?} {b:?}"),
                }
            }
        }
    }
    #[test]
    #[ignore = "Explicit paired debug microbenchmark; not application latency"]
    fn paired_product_microbenchmark() {
        use std::{hint::black_box, time::Instant};
        let a = Interval::new(-0.3, 0.7).unwrap();
        let b = Interval::new(0.2, 0.9).unwrap();
        for round in 0..3 {
            let start = Instant::now();
            for _ in 0..100000 {
                black_box(previous_product(black_box(a), black_box(b)).unwrap());
            }
            let old = start.elapsed();
            let start = Instant::now();
            for _ in 0..100000 {
                black_box(black_box(a).mul(black_box(b)).unwrap());
            }
            eprintln!(
                "round={round} products=100000 old_us={} scalar_us={}",
                old.as_micros(),
                start.elapsed().as_micros()
            );
        }
    }
}

#[cfg(test)]
mod quotient_tests {
    use super::*;
    #[test]
    fn affine_surface_derivative_at_subnormal_parameter_is_enclosed() {
        let source = crate::surface::Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1., 1.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        let tiny = f64::from_bits(4);
        let (bound, _) = crate::normal_alignment::derivative_path_bounds(
            &source,
            [[tiny, tiny], [0., 0.]],
            0,
            32,
        )
        .unwrap();
        let components = bound
            .expect("finite affine derivative at the subnormal chart point")
            .components();
        for (range, exact) in components.into_iter().zip([1., 0., 0.]) {
            assert!(range[0].is_finite() && range[1].is_finite());
            assert!(range[0] <= exact && exact <= range[1]);
        }
    }
    #[test]
    fn finite_subnormal_ratios_survive_reciprocal_overflow() {
        let tiny = f64::from_bits(1);
        let one = Interval::point(tiny).div(Interval::point(tiny)).unwrap();
        assert!(one.contains(1.));
        assert!(one.width() < 1e-14);
        let range = Interval::new(tiny, tiny * 4.)
            .unwrap()
            .div(Interval::new(tiny * 2., tiny * 8.).unwrap())
            .unwrap();
        assert!(range.contains(0.125) && range.contains(2.));
        let negative = Interval::new(-tiny * 4., -tiny)
            .unwrap()
            .div(Interval::new(tiny * 2., tiny * 8.).unwrap())
            .unwrap();
        assert!(negative.contains(-2.) && negative.contains(-0.125));
        let signed = Interval::point(tiny)
            .div_signed(Interval::point(-tiny))
            .unwrap();
        assert!(signed.contains(-1.) && signed.width() < 1e-14);
        assert!(
            Interval::point(0.)
                .div(Interval::point(tiny))
                .unwrap()
                .contains(0.)
        );
        assert!(
            Interval::point(f64::MAX)
                .div(Interval::point(tiny))
                .is_err()
        );
        assert!(
            Interval::point(tiny)
                .div_signed(Interval::new(-tiny, tiny).unwrap())
                .is_err()
        );
    }
}
