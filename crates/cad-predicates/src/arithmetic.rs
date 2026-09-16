//! Independently written from the mathematics in Shewchuk, CMU-CS-96-140R,
//! §§2.3–2.5 (error-free sums/products and insertion into expansions).
//! No upstream implementation was consulted. Operations deliberately use the
//! simpler repeated insertion algorithm and strict bounded exponent checks.
use crate::{Algebra, AuthoredScalar, PredicateContext, Reason, Sign};

#[inline(always)]
fn next_up(value: f64) -> f64 {
    if value == f64::INFINITY {
        return value;
    }
    if value == f64::NEG_INFINITY {
        return -f64::MAX;
    }
    if value == 0. {
        return f64::from_bits(1);
    }
    f64::from_bits(if value > 0. {
        value.to_bits() + 1
    } else {
        value.to_bits() - 1
    })
}
#[inline(always)]
fn next_down(value: f64) -> f64 {
    -next_up(-value)
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Interval {
    pub(crate) lo: f64,
    pub(crate) hi: f64,
}
impl Interval {
    pub(crate) fn point(value: f64) -> Self {
        Self {
            lo: value,
            hi: value,
        }
    }
    fn enclose(lo: f64, hi: f64) -> Self {
        if lo.is_nan() || hi.is_nan() {
            Self {
                lo: f64::NEG_INFINITY,
                hi: f64::INFINITY,
            }
        } else {
            Self {
                lo: next_down(lo),
                hi: next_up(hi),
            }
        }
    }
    fn integer(value: u64, ctx: &mut PredicateContext<'_>) -> Result<Self, Reason> {
        // The two 32-bit chunks are exactly representable. Their sum is
        // enclosed; u64::MAX must never be rounded and called an exact leaf.
        let high = Self::point((value >> 32) as f64 * 4_294_967_296.);
        high.add(&Self::point((value & 0xffff_ffff) as f64), ctx)
    }
    pub(crate) fn authored(
        value: &AuthoredScalar,
        ctx: &mut PredicateContext<'_>,
    ) -> Result<Self, Reason> {
        ctx.charge(1)?;
        match *value {
            AuthoredScalar::Binary64Bits(bits) => Ok(Self::point(f64::from_bits(bits))),
            AuthoredScalar::RationalConstant {
                numerator,
                denominator,
            } => {
                let positive = Self::integer(numerator.unsigned_abs(), ctx)?;
                let numerator = if numerator < 0 {
                    Self {
                        lo: -positive.hi,
                        hi: -positive.lo,
                    }
                } else {
                    positive
                };
                let denominator = Self::integer(denominator, ctx)?;
                ctx.charge(8)?;
                if denominator.lo <= 0. {
                    return Err(Reason::PrecisionExhausted);
                }
                let quotients = [
                    numerator.lo / denominator.lo,
                    numerator.lo / denominator.hi,
                    numerator.hi / denominator.lo,
                    numerator.hi / denominator.hi,
                ];
                Ok(Self::enclose(
                    quotients.iter().copied().fold(f64::INFINITY, f64::min),
                    quotients.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                ))
            }
        }
    }
    pub(crate) fn sign(&self) -> Option<Sign> {
        if self.lo > 0. {
            Some(Sign::Positive)
        } else if self.hi < 0. {
            Some(Sign::Negative)
        } else {
            None
        }
    }
    pub(crate) fn distance_width(&self) -> f64 {
        let low = next_down(self.lo.max(0.).sqrt()).max(0.);
        let high = next_up(self.hi.sqrt());
        next_up(high - low)
    }
    pub(crate) fn width(&self) -> f64 {
        next_up(self.hi - self.lo)
    }
    pub(crate) fn div_nonzero(
        &self,
        denominator: &Self,
        ctx: &mut PredicateContext<'_>,
    ) -> Result<Self, Reason> {
        ctx.charge(8)?;
        if ![self.lo, self.hi, denominator.lo, denominator.hi]
            .iter()
            .all(|v| v.is_finite())
            || (denominator.lo <= 0. && denominator.hi >= 0.)
        {
            return Err(Reason::PrecisionExhausted);
        }
        let values = [
            self.lo / denominator.lo,
            self.lo / denominator.hi,
            self.hi / denominator.lo,
            self.hi / denominator.hi,
        ];
        if values.iter().any(|v| !v.is_finite()) {
            return Err(Reason::PrecisionExhausted);
        }
        let bound = Self::enclose(
            values.into_iter().fold(f64::INFINITY, f64::min),
            values.into_iter().fold(f64::NEG_INFINITY, f64::max),
        );
        if !bound.lo.is_finite() || !bound.hi.is_finite() {
            return Err(Reason::PrecisionExhausted);
        }
        Ok(bound)
    }
}
impl Algebra for Interval {
    fn add(&self, other: &Self, ctx: &mut PredicateContext<'_>) -> Result<Self, Reason> {
        ctx.charge(4)?;
        Ok(Self::enclose(self.lo + other.lo, self.hi + other.hi))
    }
    fn sub(&self, other: &Self, ctx: &mut PredicateContext<'_>) -> Result<Self, Reason> {
        ctx.charge(4)?;
        Ok(Self::enclose(self.lo - other.hi, self.hi - other.lo))
    }
    fn mul(&self, other: &Self, ctx: &mut PredicateContext<'_>) -> Result<Self, Reason> {
        ctx.charge(8)?;
        let candidates = [
            self.lo * other.lo,
            self.lo * other.hi,
            self.hi * other.lo,
            self.hi * other.hi,
        ];
        if candidates.iter().any(|v| v.is_nan()) {
            return Ok(Self {
                lo: f64::NEG_INFINITY,
                hi: f64::INFINITY,
            });
        }
        Ok(Self::enclose(
            candidates.iter().copied().fold(f64::INFINITY, f64::min),
            candidates.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        ))
    }
}

/// A scalar filter with an absolute forward-error envelope. Every propagation
/// operation on nonnegative errors rounds upward; no relative-error assumption
/// is made for a subnormal product or a cancelling sum.
#[derive(Clone)]
pub(crate) struct Approx {
    value: f64,
    error: f64,
}
impl Approx {
    pub(crate) fn leaf(value: f64) -> Self {
        Self { value, error: 0. }
    }
    fn rounding_error(value: f64) -> f64 {
        if !value.is_finite() {
            return f64::INFINITY;
        }
        next_up(value.abs()) - value.abs()
    }
    pub(crate) fn sign(&self) -> Option<Sign> {
        if !self.value.is_finite() || !self.error.is_finite() {
            None
        } else if self.value > self.error {
            Some(Sign::Positive)
        } else if self.value < -self.error {
            Some(Sign::Negative)
        } else {
            None
        }
    }
}
#[inline(always)]
fn up_add(a: f64, b: f64) -> f64 {
    let value = a + b;
    if value.is_nan() {
        f64::INFINITY
    } else {
        next_up(value)
    }
}
#[inline(always)]
fn up_mul(a: f64, b: f64) -> f64 {
    let value = a * b;
    if value.is_nan() {
        f64::INFINITY
    } else {
        next_up(value)
    }
}
impl Algebra for Approx {
    fn add(&self, other: &Self, ctx: &mut PredicateContext<'_>) -> Result<Self, Reason> {
        ctx.charge(6)?;
        let value = self.value + other.value;
        Ok(Self {
            value,
            error: up_add(up_add(self.error, other.error), Self::rounding_error(value)),
        })
    }
    fn sub(&self, other: &Self, ctx: &mut PredicateContext<'_>) -> Result<Self, Reason> {
        self.add(
            &Self {
                value: -other.value,
                error: other.error,
            },
            ctx,
        )
    }
    fn mul(&self, other: &Self, ctx: &mut PredicateContext<'_>) -> Result<Self, Reason> {
        ctx.charge(12)?;
        let value = self.value * other.value;
        let propagated = up_add(
            up_add(
                up_mul(self.value.abs(), other.error),
                up_mul(other.value.abs(), self.error),
            ),
            up_mul(self.error, other.error),
        );
        Ok(Self {
            value,
            error: up_add(propagated, Self::rounding_error(value)),
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Expansion(Vec<f64>);

#[inline(always)]
fn exponent(value: f64) -> i32 {
    let bits = value.abs().to_bits();
    let encoded = ((bits >> 52) & 0x7ff) as i32;
    if encoded != 0 {
        encoded - 1023
    } else {
        -1074 + (63 - (bits & 0x000f_ffff_ffff_ffff).leading_zeros()) as i32
    }
}
#[inline(always)]
fn least_bit_exponent(value: f64) -> i32 {
    let bits = value.abs().to_bits();
    let encoded = ((bits >> 52) & 0x7ff) as i32;
    let mantissa = (bits & 0x000f_ffff_ffff_ffff) | if encoded == 0 { 0 } else { 1 << 52 };
    (if encoded == 0 {
        -1074
    } else {
        encoded - 1023 - 52
    }) + mantissa.trailing_zeros() as i32
}

fn split_sum(left: f64, right: f64, ctx: &mut PredicateContext<'_>) -> Result<(f64, f64), Reason> {
    ctx.charge(8)?;
    let rounded = left + right;
    let accounted_right = rounded - left;
    let accounted_left = rounded - accounted_right;
    let missing_right = right - accounted_right;
    let missing_left = left - accounted_left;
    let residual = missing_left + missing_right;
    if [
        rounded,
        accounted_right,
        accounted_left,
        missing_right,
        missing_left,
        residual,
    ]
    .iter()
    .any(|v| !v.is_finite())
    {
        return Err(Reason::PrecisionExhausted);
    }
    Ok((rounded, residual))
}

fn split_product(
    left: f64,
    right: f64,
    ctx: &mut PredicateContext<'_>,
) -> Result<(f64, f64), Reason> {
    ctx.charge(20)?;
    if left == 0. || right == 0. {
        return Ok((0., 0.));
    }
    let (le, re) = (exponent(left), exponent(right));
    // A 53-bit input has no bit below its leading exponent-52. Splitting
    // gives products with no bit below le+re-104. Keep these strictly normal,
    // and leave ample overflow headroom for the splitter and carry terms.
    if left.is_subnormal()
        || right.is_subnormal()
        || le > 970
        || re > 970
        || !(-900..=900).contains(&(le + re))
    {
        return Err(Reason::PrecisionExhausted);
    }
    let carve = |value: f64| {
        let stretched = value * 134_217_729.;
        let upper = stretched - (stretched - value);
        (upper, value - upper)
    };
    let (left_high, left_low) = carve(left);
    let (right_high, right_low) = carve(right);
    let rounded = left * right;
    let missing_high = rounded - left_high * right_high;
    let missing_left = missing_high - left_low * right_high;
    let missing_right = missing_left - left_high * right_low;
    let residual = left_low * right_low - missing_right;
    if !rounded.is_finite() || !residual.is_finite() {
        return Err(Reason::PrecisionExhausted);
    }
    Ok((rounded, residual))
}

impl Expansion {
    /// Remove a shared integer factor from homogeneous coordinates when their exact
    /// dyadic coefficients fit the checked i128 path. Larger spans retain the
    /// original expansion representation; there is no rounded reconstruction.
    pub(crate) fn reduce_projective<const N: usize>(
        values: &mut [Self; N],
        ctx: &mut PredicateContext<'_>,
    ) -> Result<(), Reason> {
        let mut low = i32::MAX;
        let mut high = i32::MIN;
        for part in values.iter().flat_map(|v| &v.0) {
            ctx.charge(2)?;
            low = low.min(least_bit_exponent(*part));
            high = high.max(exponent(*part));
        }
        if low == i32::MAX || high - low > 120 {
            return Ok(());
        }
        let mut integers = [0i128; N];
        for (value, integer) in values.iter().zip(&mut integers) {
            for part in &value.0 {
                ctx.charge(8)?;
                let bits = part.abs().to_bits();
                let encoded = ((bits >> 52) & 0x7ff) as i32;
                let mantissa =
                    (bits & 0x000f_ffff_ffff_ffff) | if encoded == 0 { 0 } else { 1 << 52 };
                let trailing = mantissa.trailing_zeros();
                let significant = mantissa >> trailing;
                let shift = (if encoded == 0 {
                    -1074
                } else {
                    encoded - 1023 - 52
                }) + trailing as i32
                    - low;
                if shift < 0 || (64 - significant.leading_zeros()) as i32 + shift > 126 {
                    return Ok(());
                }
                let scaled = (significant as i128) << shift;
                let signed = if *part < 0. { -scaled } else { scaled };
                let Some(sum) = integer.checked_add(signed) else {
                    return Ok(());
                };
                *integer = sum;
            }
        }
        let mut common = 0u128;
        for integer in integers {
            let mut next = integer.unsigned_abs();
            while next != 0 {
                ctx.charge(1)?;
                (common, next) = (next, common % next);
            }
        }
        if common <= 1 {
            return Ok(());
        }
        let Ok(divisor) = i128::try_from(common) else {
            return Ok(());
        };
        let mut reduced = std::array::from_fn(|_| Self::scalar(0.));
        for (out, integer) in reduced.iter_mut().zip(integers) {
            let integer = integer / divisor;
            let magnitude = integer.unsigned_abs();
            for shift in [0, 32, 64, 96] {
                ctx.charge(2)?;
                let chunk = ((magnitude >> shift) & 0xffff_ffff) as f64;
                let part = chunk * f64::from_bits(((1023 + shift) as u64) << 52);
                out.insert(if integer < 0 { -part } else { part }, ctx)?;
            }
        }
        // Each old value equals its integer * the SAME positive 2^low.
        // Division by the SAME positive gcd therefore preserves homogeneous data.
        *values = reduced;
        Ok(())
    }
    pub(crate) fn term_count(&self) -> usize {
        self.0.len()
    }
    pub(crate) fn enclosure(&self, ctx: &mut PredicateContext<'_>) -> Result<Interval, Reason> {
        let mut bound = Interval::point(0.);
        for &part in &self.0 {
            bound = bound.add(&Interval::point(part), ctx)?;
        }
        Ok(bound)
    }
    pub(crate) fn scalar(value: f64) -> Self {
        Self(if value == 0. { Vec::new() } else { vec![value] })
    }
    pub(crate) fn is_one(&self) -> bool {
        self.0 == [1.]
    }
    fn insert(&mut self, value: f64, ctx: &mut PredicateContext<'_>) -> Result<(), Reason> {
        ctx.charge(1)?;
        if value == 0. {
            return Ok(());
        }
        let mut carry = value;
        let mut output = Vec::with_capacity(self.0.len().saturating_add(1));
        for &part in &self.0 {
            let (rounded, residual) = split_sum(carry, part, ctx)?;
            if residual != 0. {
                output.push(residual);
            }
            carry = rounded;
        }
        if carry != 0. {
            output.push(carry);
        }
        ctx.terms(output.len())?;
        self.0 = output;
        Ok(())
    }
    pub(crate) fn integer(value: u64, ctx: &mut PredicateContext<'_>) -> Result<Self, Reason> {
        let mut result = Self::scalar((value & 0xffff_ffff) as f64);
        result.insert((value >> 32) as f64 * 4_294_967_296., ctx)?;
        Ok(result)
    }
    pub(crate) fn negated(&self) -> Self {
        Self(self.0.iter().map(|v| -v).collect())
    }
    pub(crate) fn sign(&self) -> Sign {
        match self.0.last() {
            None => Sign::Zero,
            Some(v) if *v > 0. => Sign::Positive,
            _ => Sign::Negative,
        }
    }
    pub(crate) fn normalize_common(
        values: &mut [Self],
        ctx: &mut PredicateContext<'_>,
    ) -> Result<(), Reason> {
        let maximum = values.iter().flat_map(|v| &v.0).map(|v| exponent(*v)).max();
        let Some(maximum) = maximum else {
            return Ok(());
        };
        let shift = -maximum;
        for part in values.iter_mut().flat_map(|v| &mut v.0) {
            ctx.charge(1)?;
            if least_bit_exponent(*part) + shift < -1074 || exponent(*part) + shift > 1023 {
                return Err(Reason::PrecisionExhausted);
            }
            let mut remaining = shift;
            while remaining != 0 {
                ctx.charge(1)?;
                let step = remaining.clamp(-512, 512);
                *part *= f64::from_bits(((step + 1023) as u64) << 52);
                remaining -= step;
            }
            if !part.is_finite() || *part == 0. {
                return Err(Reason::PrecisionExhausted);
            }
        }
        Ok(())
    }
}
impl Algebra for Expansion {
    fn add(&self, other: &Self, ctx: &mut PredicateContext<'_>) -> Result<Self, Reason> {
        ctx.charge(self.0.len() as u64)?;
        let mut result = self.clone();
        for &value in &other.0 {
            result.insert(value, ctx)?;
        }
        ctx.terms(result.0.len())?;
        Ok(result)
    }
    fn sub(&self, other: &Self, ctx: &mut PredicateContext<'_>) -> Result<Self, Reason> {
        self.add(&other.negated(), ctx)
    }
    fn mul(&self, other: &Self, ctx: &mut PredicateContext<'_>) -> Result<Self, Reason> {
        ctx.charge(1)?;
        let mut result = Self::scalar(0.);
        for &left in &self.0 {
            for &right in &other.0 {
                let (rounded, residual) = split_product(left, right, ctx)?;
                result.insert(residual, ctx)?;
                result.insert(rounded, ctx)?;
            }
        }
        Ok(result)
    }
}
