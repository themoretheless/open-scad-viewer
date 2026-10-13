use super::*;

pub(super) const MAX_GENERATED_ITEMS: usize = 1_000_000;
pub(super) const MAX_CHR_RANGE_ITEMS: f64 = 10_000.0;
pub(super) const HALF_MAX_DOUBLE: f64 = f64::MAX / 2.0;

/// OpenSCAD 2021.01's 31-bit floating-point hash, used to seed mt19937.
pub(super) fn hash_floating_point(value: f64) -> i32 {
    if !value.is_finite() {
        return if value.is_nan() {
            0
        } else if value > 0.0 {
            314159
        } else {
            -314159
        };
    }
    if value == 0.0 {
        return 0;
    }
    let mut exponent = value.abs().log2().floor() as i64 + 1;
    let divisor = 2f64.powi(exponent as i32);
    let mut mantissa = if divisor.is_finite() && divisor != 0.0 {
        value / divisor
    } else {
        value / 2f64.powi((exponent - 1) as i32) / 2.0
    };
    let mut sign: i128 = 1;
    if mantissa < 0.0 {
        sign = -1;
        mantissa = -mantissa;
    }
    let modulus: i128 = (1 << 31) - 1;
    let mut hash: i128 = 0;
    while mantissa != 0.0 {
        hash = ((hash << 28) & modulus) | (hash >> 3);
        mantissa *= 268_435_456.0;
        exponent -= 28;
        let integral = mantissa.trunc();
        mantissa -= integral;
        hash += integral as i128;
        if hash >= modulus {
            hash -= modulus;
        }
    }
    let exponent = if exponent >= 0 {
        exponent % 31
    } else {
        30 - ((-1 - exponent) % 31)
    };
    hash = ((hash << exponent) & modulus) | (hash >> (31 - exponent));
    (hash * sign) as i32
}

pub(super) struct Mt19937 {
    state: [u32; 624],
    index: usize,
}

impl Mt19937 {
    fn new(seed: u32) -> Self {
        let mut state = [0u32; 624];
        state[0] = seed;
        for index in 1..624 {
            let previous = state[index - 1] ^ (state[index - 1] >> 30);
            state[index] = 1_812_433_253u32
                .wrapping_mul(previous)
                .wrapping_add(index as u32);
        }
        Self { state, index: 624 }
    }
    fn next(&mut self) -> u32 {
        if self.index >= 624 {
            self.twist();
        }
        let mut value = self.state[self.index];
        self.index += 1;
        value ^= value >> 11;
        value ^= (value << 7) & 0x9d2c5680;
        value ^= (value << 15) & 0xefc60000;
        value ^= value >> 18;
        value
    }
    fn twist(&mut self) {
        for index in 0..624 {
            let combined =
                (self.state[index] & 0x80000000) | (self.state[(index + 1) % 624] & 0x7fffffff);
            self.state[index] = self.state[(index + 397) % 624]
                ^ (combined >> 1)
                ^ (if combined & 1 == 0 { 0 } else { 0x9908b0df });
        }
        self.index = 0;
    }
}

/// libc++/libstdc++ generate_canonical<double, 53> over mt19937.
pub(super) struct SeededRandom(Mt19937);
impl SeededRandom {
    fn next(&mut self) -> f64 {
        let low = self.0.next() as f64;
        let high = self.0.next() as f64;
        (low + high * 4_294_967_296.0) / 18_446_744_073_709_551_616.0
    }
}

pub(super) fn rands<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    expect_count("rands", argc, &[3, 4])?;
    let minimum_value = arg(0)?;
    let Some(minimum_value) = minimum_value.as_number() else {
        return Ok(Value::Undef);
    };
    let mut minimum = minimum_value;
    if !minimum.is_finite() {
        ctx.warn(
            "rands",
            "range minimum is non-finite; using a bounded minimum",
        );
        minimum = -HALF_MAX_DOUBLE;
    }
    let maximum_value = arg(1)?;
    let Some(maximum_value) = maximum_value.as_number() else {
        return Ok(Value::Undef);
    };
    let mut maximum = maximum_value;
    if !maximum.is_finite() {
        ctx.warn(
            "rands",
            "range maximum is non-finite; using a bounded maximum",
        );
        maximum = HALF_MAX_DOUBLE;
    }
    if maximum < minimum {
        std::mem::swap(&mut minimum, &mut maximum);
    }
    let count_value = arg(2)?;
    let Some(count_value) = count_value.as_number() else {
        return Ok(Value::Undef);
    };
    let mut requested_count = count_value.abs();
    if !requested_count.is_finite() {
        ctx.warn("rands", "result count is non-finite; using one result");
        requested_count = 1.0;
    }
    let count = requested_count.trunc();
    if count < 0.0 || count > 9_007_199_254_740_991.0 || count > MAX_GENERATED_ITEMS as f64 {
        return fail(
            "rands",
            format!(
                "result exceeds {} items",
                crate::value::locale(MAX_GENERATED_ITEMS)
            ),
        );
    }
    let mut seeded = if argc == 4 {
        let seed = arg(3)?;
        let Some(seed) = seed.as_number() else {
            return Ok(Value::Undef);
        };
        Some(SeededRandom(Mt19937::new(hash_floating_point(seed) as u32)))
    } else {
        None
    };
    let mut result = Vec::new();
    for _ in 0..count as usize {
        if minimum == maximum {
            result.push(Value::Number(minimum));
            continue;
        }
        let unit = match &mut seeded {
            Some(generator) => generator.next(),
            None => (ctx.random)(),
        };
        if !unit.is_finite() || unit < 0.0 || unit >= 1.0 {
            return fail(
                "rands",
                "random source must return a finite value in [0, 1)",
            );
        }
        result.push(Value::Number(minimum + (maximum - minimum) * unit));
    }
    ctx.register_array("rands", result)
}
