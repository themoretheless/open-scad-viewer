//! Exact selected scalar roots. Intervals enclose a root; their midpoints
//! never replace the owned polynomial and unique-root selection.
use super::*;
use num_traits::{Signed, ToPrimitive};
mod polynomial;
use polynomial::*;
#[derive(Clone, Copy, Debug)]
pub struct SsAlgebraicClockLimits {
    pub max_degree: usize,
    pub max_operations: usize,
    pub max_coefficient_bits: u64,
}
impl Default for SsAlgebraicClockLimits {
    fn default() -> Self {
        Self {
            max_degree: 64,
            max_operations: 100000,
            max_coefficient_bits: 32768,
        }
    }
}
#[derive(Clone, Debug)]
pub struct QualifiedSsAlgebraicClock {
    polynomial: Vec<Q>,
    interval: [Q; 2],
    chain: Vec<Vec<Q>>,
    operations: usize,
    limits: SsAlgebraicClockLimits,
}
#[derive(Clone, Debug)]
pub struct SsAlgebraicClockEnclosure {
    pub bounds: [Q; 2],
    pub operations: usize,
}
impl QualifiedSsAlgebraicClock {
    pub fn qualify(
        mut polynomial: Vec<Q>,
        interval: [Q; 2],
        limits: SsAlgebraicClockLimits,
    ) -> Result<Self> {
        let mut w = Work::new(limits)?;
        check(
            !polynomial.is_empty() && polynomial.len() <= limits.max_degree + 1,
            "Algebraic clock polynomial degree allowance exceeded",
        )?;
        for c in &polynomial {
            w.input(c.clone())?;
        }
        for x in &interval {
            w.input(x.clone())?;
        }
        trim(&mut polynomial);
        check(
            polynomial.len() >= 2 && interval[0] <= interval[1],
            "Algebraic clock needs a nonconstant polynomial and ordered bounds",
        )?;
        let chain = sturm(&polynomial, &mut w)?;
        let a = evaluate(&polynomial, &interval[0], &mut w)?;
        let b = evaluate(&polynomial, &interval[1], &mut w)?;
        if interval[0] == interval[1] {
            check(a.is_zero(), "Point clock interval is not an exact root")?;
        } else {
            check(
                !a.is_zero() && !b.is_zero(),
                "Nonpoint clock isolator requires nonroot endpoints",
            )?;
            check(
                count(&chain, &interval, &mut w)? == 1,
                "Algebraic clock interval does not contain exactly one distinct root",
            )?;
        }
        Ok(Self {
            polynomial,
            interval,
            chain,
            operations: w.used,
            limits,
        })
    }
    pub fn from_binary(
        polynomial: &[f64],
        interval: [f64; 2],
        limits: SsAlgebraicClockLimits,
    ) -> Result<Self> {
        let q = |x| {
            Q::from_float(x)
                .ok_or_else(|| crate::input("Nonfinite algebraic clock coefficient or bound"))
        };
        Self::qualify(
            polynomial.iter().map(|x| q(*x)).collect::<Result<_>>()?,
            [q(interval[0])?, q(interval[1])?],
            limits,
        )
    }
    pub fn polynomial(&self) -> &[Q] {
        &self.polynomial
    }
    pub fn isolating_interval(&self) -> &[Q; 2] {
        &self.interval
    }
    pub fn operations(&self) -> usize {
        self.operations
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = Self::qualify(self.polynomial.clone(), self.interval.clone(), self.limits)?;
        check(fresh.chain == self.chain, "Exact clock Sturm proof changed")
    }
    /// Exact equality to the selected root, with counted rational work.
    /// A zero of the polynomial outside the owned isolator is a different root.
    pub fn equals_rational(&self, value: &Q, limits: SsAlgebraicClockLimits) -> Result<(bool, usize)> {
        let mut w = Work::new(limits)?;
        check(self.polynomial.len() <= limits.max_degree + 1,
            "Algebraic clock equality degree allowance exceeded")?;
        w.input(value.clone())?;
        for x in self.polynomial.iter().chain(self.interval.iter()) { w.keep(x.clone())?; }
        let equal = self.interval[0] <= *value && *value <= self.interval[1]
            && evaluate(&self.polynomial, value, &mut w)?.is_zero();
        Ok((equal, w.used))
    }
    pub fn enclose(
        &self,
        bisections: usize,
        limits: SsAlgebraicClockLimits,
    ) -> Result<SsAlgebraicClockEnclosure> {
        check(
            bisections <= 1024 && self.polynomial.len() <= limits.max_degree.saturating_add(1),
            "Algebraic clock refinement allowance exceeded",
        )?;
        let mut w = Work::new(limits)?;
        for p in &self.chain {
            for c in p {
                w.keep(c.clone())?;
            }
        }
        let mut d = self.interval.clone();
        for x in &d {
            w.keep(x.clone())?;
        }
        for _ in 0..bisections {
            if d[0] == d[1] {
                break;
            }
            let sum = w.keep(&d[0] + &d[1])?;
            let mid = w.keep(sum / Q::from_integer(2.into()))?;
            if evaluate(&self.polynomial, &mid, &mut w)?.is_zero() {
                d = [mid.clone(), mid];
                break;
            }
            let left = [d[0].clone(), mid.clone()];
            match count(&self.chain, &left, &mut w)? {
                1 => d = left,
                0 => d[0] = mid,
                _ => {
                    return Err(crate::input(
                        "Clock refinement contradicts unique-root ownership",
                    ))
                }
            }
        }
        Ok(SsAlgebraicClockEnclosure {
            bounds: d,
            operations: w.used,
        })
    }
}
pub(super) fn float_bounds(bounds: &[Q; 2]) -> Result<[f64; 2]> {
    let convert = |q: &Q, lower: bool| -> Result<f64> {
        let mut x = q
            .to_f64()
            .filter(|x| x.is_finite())
            .ok_or_else(|| crate::input("Algebraic clock float enclosure overflow"))?;
        for _ in 0..8 {
            let value = Q::from_float(x)
                .ok_or_else(|| crate::input("Algebraic clock float enclosure overflow"))?;
            if (lower && value <= *q) || (!lower && *q <= value) {
                return Ok(x);
            }
            x = if lower { x.next_down() } else { x.next_up() };
        }
        Err(crate::input(
            "Algebraic clock outward conversion remains unproved",
        ))
    };
    Ok([convert(&bounds[0], true)?, convert(&bounds[1], false)?])
}
