//! Host-independent implementations of the stable OpenSCAD 2021.01 built-in
//! function inventory — a 1:1 port of `src/services/openScadBuiltinFunctions.ts`,
//! including diagnostic texts, degree-exact trigonometry, the mt19937 seeded
//! `rands` stream and the `str`/`chr`/`ord` string semantics.
//!
//! Argument access is call-by-name like the reference runtime: the host passes
//! an accessor closure and each handler reads argument positions exactly when
//! the TS getter-based deferred arguments would be read.
use crate::ParseError;
use crate::ast::{Expr, ExpressionArgument, ModuleParam};
use crate::lexer::TT;
use crate::value::{
    Value, compare_values, deep_equal, format_number, js_number_to_string, range_item_count,
};

/// Failure channel of a built-in call. `Fail` is an OpenSCAD value-level
/// diagnostic (`name() message`) which the stable profile degrades to a
/// warning + undef; `Eval` is a positioned evaluation failure from argument
/// evaluation or budget accounting that always propagates.
#[derive(Debug)]
pub enum BuiltinError {
    Fail(String),
    Eval(crate::value::EvalFailure),
}

impl From<crate::value::EvalFailure> for BuiltinError {
    fn from(error: crate::value::EvalFailure) -> Self {
        Self::Eval(error)
    }
}

type BResult<T> = Result<T, BuiltinError>;

fn fail<T>(name: &str, message: impl Into<String>) -> BResult<T> {
    Err(BuiltinError::Fail(format!("{name}() {}", message.into())))
}

/// Host effects available to the built-ins (TS `OpenScadBuiltinFunctionContext`).
pub struct BuiltinContext<'h, 'a> {
    pub warn: &'h mut dyn FnMut(String),
    pub register_array:
        &'h mut dyn FnMut(Vec<Value<'a>>, &str) -> crate::value::EvalResult<Vec<Value<'a>>>,
    pub register_string: &'h mut dyn FnMut(String, &str) -> crate::value::EvalResult<String>,
    pub random: &'h mut dyn FnMut() -> f64,
    pub parent_module: &'h dyn Fn(usize) -> Option<String>,
}

impl<'h, 'a> BuiltinContext<'h, 'a> {
    fn warn(&mut self, name: &str, message: impl Into<String>) {
        (self.warn)(format!("{name}() {}", message.into()));
    }
    fn register_array(&mut self, name: &str, items: Vec<Value<'a>>) -> BResult<Value<'a>> {
        Ok(Value::vector((self.register_array)(
            items,
            &format!("{name}() result"),
        )?))
    }
    fn register_string(&mut self, name: &str, value: String) -> BResult<Value<'a>> {
        Ok(Value::string((self.register_string)(
            value,
            &format!("{name}() result"),
        )?))
    }
}

type ArgReader<'r, 'a> = &'r mut dyn FnMut(usize) -> BResult<Value<'a>>;

fn expect_count(name: &str, argc: usize, counts: &[usize]) -> BResult<()> {
    if counts.contains(&argc) {
        return Ok(());
    }
    let expected = if counts.len() == 1 {
        counts[0].to_string()
    } else {
        counts
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(" or ")
    };
    let plural = if counts.iter().all(|&c| c == 1) {
        ""
    } else {
        "s"
    };
    fail(name, format!("expects {expected} argument{plural}"))
}

fn number_value(name: &str, value: &Value, argument: usize) -> BResult<f64> {
    match value.as_number() {
        Some(v) => Ok(v),
        None => fail(name, format!("argument {} must be a number", argument + 1)),
    }
}

fn finite_number_value(name: &str, value: &Value, argument: usize) -> BResult<f64> {
    let number = number_value(name, value, argument)?;
    if !number.is_finite() {
        return fail(name, format!("argument {} must be finite", argument + 1));
    }
    Ok(number)
}

fn array_value<'v, 'a>(
    name: &str,
    value: &'v Value<'a>,
    argument: usize,
) -> BResult<&'v [Value<'a>]> {
    match value {
        Value::Vector(items) => Ok(items),
        _ => fail(name, format!("argument {} must be a vector", argument + 1)),
    }
}

fn unary_number<'a>(
    name: &'static str,
    operation: fn(f64) -> f64,
) -> impl Fn(usize, ArgReader<'_, 'a>, &mut BuiltinContext<'_, 'a>) -> BResult<Value<'a>> {
    move |argc, arg, _ctx| {
        expect_count(name, argc, &[1])?;
        Ok(Value::Number(operation(number_value(name, &arg(0)?, 0)?)))
    }
}

fn binary_number<'a>(
    name: &'static str,
    operation: fn(f64, f64) -> f64,
) -> impl Fn(usize, ArgReader<'_, 'a>, &mut BuiltinContext<'_, 'a>) -> BResult<Value<'a>> {
    move |argc, arg, _ctx| {
        expect_count(name, argc, &[2])?;
        // The 2021.01 built-ins read both expressions before validating either.
        let left = arg(0)?;
        let right = arg(1)?;
        Ok(Value::Number(operation(
            number_value(name, &left, 0)?,
            number_value(name, &right, 1)?,
        )))
    }
}

// Degree-exact trigonometry: normalize once, solve a first-quadrant pair and
// restore signs by quadrant so common angles stay exact after full rotations.
const DEG_TO_RAD: f64 = 0.017_453_292_519_943_295;
const RAD_TO_DEG: f64 = 57.295_779_513_082_32;
const SQRT_THREE_QUARTERS: f64 = 0.866_025_403_784_438_6;
const SQRT_ONE_THIRD: f64 = 0.577_350_269_189_625_7;
const TRIG_HUGE_VALUE: f64 = 360.0 * 4_503_599_627_370_496.0; // 360 * 2^52

fn reduce_degrees(value: f64, period: f64) -> Option<(f64, f64)> {
    if !(value < TRIG_HUGE_VALUE && value > -TRIG_HUGE_VALUE) {
        return None;
    }
    if value >= 0.0 && value < period {
        return Some((value, 0.0));
    }
    let cycles = (value / period).floor();
    Some((value - cycles * period, cycles))
}

fn first_quadrant_components(angle: f64) -> (f64, f64) {
    if angle == 30.0 {
        return (0.5, SQRT_THREE_QUARTERS);
    }
    if angle == 45.0 {
        return (
            std::f64::consts::FRAC_1_SQRT_2,
            std::f64::consts::FRAC_1_SQRT_2,
        );
    }
    if angle == 60.0 {
        return (SQRT_THREE_QUARTERS, 0.5);
    }
    if angle < 45.0 {
        let radians = angle * DEG_TO_RAD;
        return (radians.sin(), radians.cos());
    }
    let complement = (90.0 - angle) * DEG_TO_RAD;
    (complement.cos(), complement.sin())
}

fn unit_circle_components(value: f64) -> Option<(f64, f64)> {
    let (angle, _) = reduce_degrees(value, 360.0)?;
    if angle == 0.0 {
        return Some((angle, 1.0));
    }
    if angle == 90.0 {
        return Some((1.0, 0.0));
    }
    if angle == 180.0 {
        return Some((-0.0, -1.0));
    }
    if angle == 270.0 {
        return Some((-1.0, -0.0));
    }
    let quadrant = (angle / 90.0).floor();
    let (sin, cos) = first_quadrant_components(angle - quadrant * 90.0);
    Some(match quadrant as i32 {
        0 => (sin, cos),
        1 => (cos, -sin),
        2 => (-sin, -cos),
        _ => (-cos, sin),
    })
}

fn sin_degrees(value: f64) -> f64 {
    unit_circle_components(value).map_or(f64::NAN, |(s, _)| s)
}
fn cos_degrees(value: f64) -> f64 {
    unit_circle_components(value).map_or(f64::NAN, |(_, c)| c)
}

fn tan_degrees(value: f64) -> f64 {
    let Some((angle, cycles)) = reduce_degrees(value, 180.0) else {
        return f64::NAN;
    };
    if angle == 0.0 {
        return if (cycles as i64) % 2 == 0 { 0.0 } else { -0.0 };
    }
    if angle == 90.0 {
        return if (cycles as i64) % 2 == 0 {
            f64::INFINITY
        } else {
            f64::NEG_INFINITY
        };
    }
    let oppose = angle > 90.0;
    let acute = if oppose { 180.0 - angle } else { angle };
    let magnitude = if acute == 30.0 {
        SQRT_ONE_THIRD
    } else if acute == 45.0 {
        1.0
    } else if acute == 60.0 {
        3.0_f64.sqrt()
    } else {
        (acute * DEG_TO_RAD).tan()
    };
    if oppose { -magnitude } else { magnitude }
}

/// C++ std::round semantics: halfway cases round away from zero.
fn round_away_from_zero(value: f64) -> f64 {
    if !value.is_finite() || value == 0.0 {
        return value;
    }
    value.signum() * (value.abs() + 0.5).floor()
}

fn asin_degrees(value: f64) -> f64 {
    let degrees = value.asin() * RAD_TO_DEG;
    let whole = round_away_from_zero(degrees);
    if sin_degrees(whole) == value {
        whole
    } else {
        degrees
    }
}
fn acos_degrees(value: f64) -> f64 {
    let degrees = value.acos() * RAD_TO_DEG;
    let whole = round_away_from_zero(degrees);
    if cos_degrees(whole) == value {
        whole
    } else {
        degrees
    }
}
fn atan_degrees(value: f64) -> f64 {
    let degrees = value.atan() * RAD_TO_DEG;
    let whole = round_away_from_zero(degrees);
    if tan_degrees(whole) == value {
        whole
    } else {
        degrees
    }
}
fn atan2_degrees(y: f64, x: f64) -> f64 {
    let degrees = y.atan2(x) * RAD_TO_DEG;
    let whole = round_away_from_zero(degrees);
    if (degrees - whole).abs() < 3e-14 {
        whole
    } else {
        degrees
    }
}

fn quoted_expression_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        out.push_str(match ch {
            '\t' => "\\t",
            '\n' => "\\n",
            '\r' => "\\r",
            '"' => "\\\"",
            '\\' => "\\\\",
            _ => {
                out.push(ch);
                continue;
            }
        });
    }
    out.push('"');
    out
}

fn binary_token(operator: TT) -> &'static str {
    match operator {
        TT::Plus => "+",
        TT::Minus => "-",
        TT::Star => "*",
        TT::Slash => "/",
        TT::Percent => "%",
        TT::Caret => "^",
        TT::Lt => "<",
        TT::Gt => ">",
        TT::LtEq => "<=",
        TT::GtEq => ">=",
        TT::EqEq => "==",
        TT::NotEq => "!=",
        TT::And => "&&",
        TT::Or => "||",
        other => other.name(),
    }
}

fn format_expression_arguments(args: &[ExpressionArgument]) -> String {
    args.iter()
        .map(|argument| match &argument.name {
            None => format_expression(&argument.value),
            Some(name) => format!("{name} = {}", format_expression(&argument.value)),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_params(params: &[ModuleParam]) -> String {
    params
        .iter()
        .map(|parameter| match &parameter.default_value {
            None => parameter.name.clone(),
            Some(default) => format!("{} = {}", parameter.name, format_expression(default)),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Canonical source-shaped expression rendering used by `str(function-value)`.
pub fn format_expression(expression: &Expr) -> String {
    match expression {
        Expr::Literal { value, .. } => match value {
            crate::ast::LiteralValue::String(s) => quoted_expression_string(s),
            crate::ast::LiteralValue::Undef => "undef".to_string(),
            crate::ast::LiteralValue::Bool(v) => if *v { "true" } else { "false" }.to_string(),
            crate::ast::LiteralValue::Number(v) => format_number(*v),
        },
        Expr::Identifier { name, .. } => name.clone(),
        Expr::Vector { items, .. } => format!(
            "[{}]",
            items
                .iter()
                .map(format_expression)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::Range {
            start, step, end, ..
        } => format!(
            "[{} : {}{}]",
            format_expression(start),
            match step {
                None => String::new(),
                Some(step) => format!("{} : ", format_expression(step)),
            },
            format_expression(end)
        ),
        Expr::Unary { op, value, .. } => format!(
            "{}{}",
            match op {
                TT::Not => "!",
                TT::Minus => "-",
                _ => "+",
            },
            format_expression(value)
        ),
        Expr::Binary {
            op, left, right, ..
        } => format!(
            "({} {} {})",
            format_expression(left),
            binary_token(*op),
            format_expression(right)
        ),
        Expr::Ternary { test, yes, no, .. } => format!(
            "({} ? {} : {})",
            format_expression(test),
            format_expression(yes),
            format_expression(no)
        ),
        Expr::Function { params, body, .. } => {
            format!(
                "function({}) {}",
                format_params(params),
                format_expression(body)
            )
        }
        Expr::Call { callee, args, .. } => {
            let rendered = format_expression(callee);
            let callee_text = if matches!(**callee, Expr::Function { .. }) {
                format!("({rendered})")
            } else {
                rendered
            };
            format!("{callee_text}({})", format_expression_arguments(args))
        }
        Expr::Index { value, index, .. } => {
            format!("{}[{}]", format_expression(value), format_expression(index))
        }
        Expr::Member { value, name, .. } => format!("{}.{}", format_expression(value), name),
        Expr::Let { args, body, .. } => {
            format!(
                "let({}) {}",
                format_expression_arguments(args),
                format_expression(body)
            )
        }
        Expr::Assert { args, body, .. } => format!(
            "assert({}){}",
            format_expression_arguments(args),
            match body {
                None => String::new(),
                Some(body) => format!(" {}", format_expression(body)),
            }
        ),
        Expr::Echo { args, body, .. } => format!(
            "echo({}){}",
            format_expression_arguments(args),
            match body {
                None => String::new(),
                Some(body) => format!(" {}", format_expression(body)),
            }
        ),
        Expr::LcFor { args, body, .. } => format!(
            "for({}) ({})",
            format_expression_arguments(args),
            format_expression(body)
        ),
        Expr::LcForC {
            init,
            condition,
            update,
            body,
            ..
        } => format!(
            "for({}; {}; {}) ({})",
            format_expression_arguments(init),
            format_expression(condition),
            format_expression_arguments(update),
            format_expression(body)
        ),
        Expr::LcIf {
            condition, yes, no, ..
        } => format!(
            "if({}) ({}){}",
            format_expression(condition),
            format_expression(yes),
            match no {
                None => String::new(),
                Some(no) => format!(" else ({})", format_expression(no)),
            }
        ),
        Expr::LcLet { args, body, .. } => format!(
            "let({}) ({})",
            format_expression_arguments(args),
            format_expression(body)
        ),
        Expr::LcEach { value, .. } => format!("each ({})", format_expression(value)),
    }
}

fn format_function(params: &[ModuleParam], body: &Expr) -> String {
    format!(
        "function({}) {}",
        format_params(params),
        format_expression(body)
    )
}

/// `formatValue` of the TS built-ins: bare strings at the top level, quoted
/// when nested, six-significant-digit numbers.
pub fn format_builtin_value(value: &Value, nested: bool) -> String {
    match value {
        Value::Undef => "undef".to_string(),
        Value::Bool(v) => if *v { "true" } else { "false" }.to_string(),
        Value::Number(v) => format_number(*v),
        Value::Str(s) => {
            if nested {
                format!("\"{s}\"")
            } else {
                s.to_string()
            }
        }
        Value::Vector(items) => format!(
            "[{}]",
            items
                .iter()
                .map(|item| format_builtin_value(item, true))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Range { start, step, end } => format!(
            "[{} : {} : {}]",
            format_number(*start),
            format_number(*step),
            format_number(*end)
        ),
        Value::Function(f) => format_function(f.params, f.body),
    }
}

const MAX_GENERATED_ITEMS: usize = 1_000_000;
const MAX_CHR_RANGE_ITEMS: f64 = 10_000.0;
const HALF_MAX_DOUBLE: f64 = f64::MAX / 2.0;

/// OpenSCAD 2021.01's 31-bit floating-point hash, used to seed mt19937.
fn hash_floating_point(value: f64) -> i32 {
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

struct Mt19937 {
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
struct SeededRandom(Mt19937);
impl SeededRandom {
    fn next(&mut self) -> f64 {
        let low = self.0.next() as f64;
        let high = self.0.next() as f64;
        (low + high * 4_294_967_296.0) / 18_446_744_073_709_551_616.0
    }
}

fn rands<'a>(
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

fn len<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    _ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    expect_count("len", argc, &[1])?;
    match &arg(0)? {
        Value::Str(deref!(s)) => Ok(Value::Number(s.chars().count() as f64)),
        Value::Vector(deref!(items)) => Ok(Value::Number(items.len() as f64)),
        _ => fail("len", "argument 1 must be a string or vector"),
    }
}

fn log<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    _ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    expect_count("log", argc, &[1, 2])?;
    if argc == 1 {
        return Ok(Value::Number(number_value("log", &arg(0)?, 0)?.log10()));
    }
    let base = number_value("log", &arg(0)?, 0)?;
    let value = number_value("log", &arg(1)?, 1)?;
    Ok(Value::Number(value.ln() / base.ln()))
}

fn str_<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    let mut out = String::new();
    for index in 0..argc {
        out.push_str(&format_builtin_value(&arg(index)?, false));
    }
    ctx.register_string("str", out)
}

fn character_string(value: &Value, ctx: &mut BuiltinContext) -> String {
    match value {
        Value::Number(v) => {
            let code_point = v.trunc();
            if !(code_point > 0.0)
                || code_point > 0x10ffff as f64
                || (code_point >= 0xd800 as f64 && code_point <= 0xdfff as f64)
            {
                return String::new();
            }
            char::from_u32(code_point as u32).map_or_else(String::new, |c| c.to_string())
        }
        Value::Vector(items) => items
            .iter()
            .map(|item| character_string(item, ctx))
            .collect(),
        Value::Range { start, step, end } => {
            let count = range_item_count(*start, *step, *end);
            if count >= MAX_CHR_RANGE_ITEMS {
                ctx.warn(
                    "chr",
                    format!(
                        "range exceeds the {}-item character limit",
                        crate::value::locale(MAX_CHR_RANGE_ITEMS as usize)
                    ),
                );
                return String::new();
            }
            // RangeType::numValues() reports one item when all three fields
            // are equal, but its 2021.01 iterator is empty for zero steps.
            if *step == 0.0 {
                return String::new();
            }
            let mut result = String::new();
            for index in 0..count as usize {
                result.push_str(&character_string(
                    &Value::Number(start + step * index as f64),
                    ctx,
                ));
            }
            result
        }
        _ => String::new(),
    }
}

fn chr<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    let mut out = String::new();
    for index in 0..argc {
        out.push_str(&character_string(&arg(index)?, ctx));
    }
    ctx.register_string("chr", out)
}

fn ord<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    _ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    if argc == 0 {
        return Ok(Value::Undef);
    }
    if argc != 1 {
        return fail("ord", "expects 1 argument");
    }
    let value = arg(0)?;
    let Value::Str(deref!(s)) = &value else {
        return fail("ord", "argument 1 must be a string");
    };
    // Rust strings are well-formed UTF-8 by construction, so the TS
    // "argument 1 must be valid Unicode" check cannot fire here.
    match s.chars().next() {
        None => Ok(Value::Undef),
        Some(first) => Ok(Value::Number(first as u32 as f64)),
    }
}

fn concat<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    let mut result: Vec<Value<'a>> = Vec::new();
    for index in 0..argc {
        let value = arg(index)?;
        let addition = match &value {
            Value::Vector(items) => items.len(),
            _ => 1,
        };
        if result.len() + addition > MAX_GENERATED_ITEMS {
            return fail(
                "concat",
                format!(
                    "result exceeds {} items",
                    crate::value::locale(MAX_GENERATED_ITEMS)
                ),
            );
        }
        match value {
            Value::Vector(items) => result.extend(items.iter().cloned()),
            other => result.push(other),
        }
    }
    ctx.register_array("concat", result)
}

fn lookup<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    _ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    expect_count("lookup", argc, &[2])?;
    let position_value = arg(0)?;
    let Some(position) = position_value.as_number().filter(|v| v.is_finite()) else {
        // The reference diagnostic formats a second evaluation of the first
        // expression and never reads the table on this path.
        let _ = arg(0)?;
        return if position_value.as_number().is_some() {
            fail("lookup", "argument 1 must be finite")
        } else {
            fail("lookup", "argument 1 must be a number")
        };
    };
    let table = arg(1)?;
    let Value::Vector(rows) = &table else {
        return Ok(Value::Undef);
    };
    if rows.is_empty() {
        return Ok(Value::Undef);
    }
    let pair = |row: &Value| -> Option<(f64, f64)> {
        let row = row.as_vector()?;
        if row.len() != 2 {
            return None;
        }
        Some((row[0].as_number()?, row[1].as_number()?))
    };
    let Some(first) = pair(&rows[0]) else {
        return Ok(Value::Undef);
    };
    let (mut low_position, mut low_value) = first;
    let (mut high_position, mut high_value) = first;
    for row in rows.iter().skip(1) {
        let Some((candidate_position, candidate_value)) = pair(row) else {
            continue;
        };
        if candidate_position <= position
            && (candidate_position > low_position || low_position > position)
        {
            low_position = candidate_position;
            low_value = candidate_value;
        }
        if candidate_position >= position
            && (candidate_position < high_position || high_position < position)
        {
            high_position = candidate_position;
            high_value = candidate_value;
        }
    }
    if position <= low_position {
        return Ok(Value::Number(high_value));
    }
    if position >= high_position {
        return Ok(Value::Number(low_value));
    }
    let fraction = (position - low_position) / (high_position - low_position);
    Ok(Value::Number(
        high_value * fraction + low_value * (1.0 - fraction),
    ))
}

fn unsigned_search_parameter(value: &Value) -> usize {
    match value.as_number() {
        Some(v) if v.is_finite() => {
            let truncated = v.trunc();
            if truncated < 0.0 {
                // JS `>>> 0` wraps modulo 2^32.
                (truncated as i64).rem_euclid(0x1_0000_0000) as usize
            } else {
                (truncated as u64 % 0x1_0000_0000) as usize
            }
        }
        _ => 0,
    }
}

fn return_matches<'a>(
    ctx: &mut BuiltinContext<'_, 'a>,
    matches: Vec<Value<'a>>,
    maximum: usize,
) -> BResult<Value<'a>> {
    let selected = if maximum == 0 {
        matches
    } else {
        matches.into_iter().take(maximum).collect()
    };
    ctx.register_array("search", selected)
}

fn search_needles<'a>(
    ctx: &mut BuiltinContext<'_, 'a>,
    needles: &[Value<'a>],
    table: &[Value<'a>],
    maximum: usize,
    column: usize,
    include_empty_first_match: bool,
) -> BResult<Value<'a>> {
    let mut output: Vec<Value<'a>> = Vec::new();
    for needle in needles {
        let mut matches: Vec<Value<'a>> = Vec::new();
        for (index, row) in table.iter().enumerate() {
            let hit = (column == 0 && deep_equal(needle, row))
                || match row {
                    Value::Vector(items) => {
                        column < items.len() && deep_equal(needle, &items[column])
                    }
                    _ => false,
                };
            if hit {
                matches.push(Value::Number(index as f64));
            }
            if maximum != 0 && matches.len() >= maximum {
                break;
            }
        }
        if maximum == 1 {
            if let Some(first) = matches.into_iter().next() {
                output.push(first);
            } else if include_empty_first_match {
                output.push(ctx.register_array("search", Vec::new())?);
            }
        } else {
            output.push(return_matches(ctx, matches, maximum)?);
        }
    }
    ctx.register_array("search", output)
}

fn search_string_rows<'a>(
    ctx: &mut BuiltinContext<'_, 'a>,
    characters: Vec<char>,
    rows: &[Value<'a>],
    maximum: usize,
    column: usize,
) -> BResult<Value<'a>> {
    let mut output: Vec<Value<'a>> = Vec::new();
    for character in characters {
        let mut matches: Vec<Value<'a>> = Vec::new();
        for (index, row) in rows.iter().enumerate() {
            let Some(items) = row.as_vector() else {
                ctx.warn(
                    "search",
                    format!("table row {index} does not contain column {column}"),
                );
                return ctx.register_array("search", Vec::new());
            };
            if items.len() <= column {
                ctx.warn(
                    "search",
                    format!("table row {index} does not contain column {column}"),
                );
                return ctx.register_array("search", Vec::new());
            }
            let candidate = format_builtin_value(&items[column], false).chars().next();
            if candidate == Some(character) {
                matches.push(Value::Number(index as f64));
            }
            if maximum != 0 && matches.len() >= maximum {
                break;
            }
        }
        if matches.is_empty() {
            ctx.warn("search", format!("term {character} was not found"));
        }
        if maximum == 1 {
            if let Some(first) = matches.into_iter().next() {
                output.push(first);
            }
        } else {
            output.push(return_matches(ctx, matches, maximum)?);
        }
    }
    ctx.register_array("search", output)
}

fn search<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    if argc < 2 {
        return fail("search", "expects at least 2 arguments");
    }
    let needle = arg(0)?;
    let table = arg(1)?;
    let maximum = if argc > 2 {
        unsigned_search_parameter(&arg(2)?)
    } else {
        1
    };
    let column = if argc > 3 {
        unsigned_search_parameter(&arg(3)?)
    } else {
        0
    };

    if let (Value::Str(deref!(needle)), Value::Str(deref!(table))) = (&needle, &table) {
        let table_characters: Vec<char> = table.chars().collect();
        let mut output: Vec<Value<'a>> = Vec::new();
        for character in needle.chars() {
            let mut matches: Vec<Value<'a>> = Vec::new();
            for (index, candidate) in table_characters.iter().enumerate() {
                if *candidate == character {
                    matches.push(Value::Number(index as f64));
                }
                if maximum != 0 && matches.len() >= maximum {
                    break;
                }
            }
            if maximum == 1 {
                if let Some(first) = matches.into_iter().next() {
                    output.push(first);
                }
            } else {
                output.push(return_matches(ctx, matches, maximum)?);
            }
        }
        return ctx.register_array("search", output);
    }

    let rows: Vec<Value<'a>> = match &table {
        Value::Vector(items) => items.as_ref().clone(),
        _ => Vec::new(),
    };
    match &needle {
        Value::Number(_) => {
            let mut matches: Vec<Value<'a>> = Vec::new();
            for (index, row) in rows.iter().enumerate() {
                let hit = (column == 0 && deep_equal(&needle, row))
                    || match row {
                        Value::Vector(items) => {
                            column < items.len() && deep_equal(&needle, &items[column])
                        }
                        _ => false,
                    };
                if hit {
                    matches.push(Value::Number(index as f64));
                }
                if maximum != 0 && matches.len() >= maximum {
                    break;
                }
            }
            ctx.register_array("search", matches)
        }
        Value::Str(s) => search_string_rows(ctx, s.chars().collect(), &rows, maximum, column),
        Value::Vector(items) => search_needles(ctx, &items.clone(), &rows, maximum, column, true),
        _ => Ok(Value::Undef),
    }
}

fn version<'a>(
    _argc: usize,
    _arg: ArgReader<'_, 'a>,
    ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    // The pinned 2021.01 implementation ignores surplus arguments; the
    // official release reports an explicit zero day component.
    ctx.register_array(
        "version",
        vec![
            Value::Number(2021.0),
            Value::Number(1.0),
            Value::Number(0.0),
        ],
    )
}

fn version_num<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    _ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    if argc == 0 {
        return Ok(Value::Number(20210100.0));
    }
    let value = arg(0)?;
    let Some(parts) = value.as_vector() else {
        return Ok(Value::Undef);
    };
    if parts.len() != 2 && parts.len() != 3 {
        return Ok(Value::Undef);
    }
    let (Some(year), Some(month)) = (parts[0].as_number(), parts[1].as_number()) else {
        return Ok(Value::Undef);
    };
    let day = if parts.len() == 3 {
        let Some(day) = parts[2].as_number() else {
            return Ok(Value::Undef);
        };
        day
    } else {
        0.0
    };
    Ok(Value::Number(year * 10_000.0 + month * 100.0 + day))
}

fn norm<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    _ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    expect_count("norm", argc, &[1])?;
    let value = arg(0)?;
    let Some(vector) = value.as_vector() else {
        return Ok(Value::Undef);
    };
    let vector = vector.clone();
    let mut result = 0.0;
    for (index, item) in vector.iter().enumerate() {
        let value = number_value("norm", item, index)?;
        result += value * value;
    }
    Ok(Value::Number(result.sqrt()))
}

fn cross<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    expect_count("cross", argc, &[2])?;
    let left_value = arg(0)?;
    let right_value = arg(1)?;
    let left = array_value("cross", &left_value, 0)?;
    let right = array_value("cross", &right_value, 1)?;
    if left.len() != right.len() || (left.len() != 2 && left.len() != 3) {
        return fail("cross", "arguments must be matching 2D or 3D vectors");
    }
    if left.len() == 2 {
        // Historical 2021.01 behaviour coerces non-numeric 2D elements to zero.
        let n = |v: &Value| v.as_number().unwrap_or(0.0);
        return Ok(Value::Number(
            n(&left[0]) * n(&right[1]) - n(&left[1]) * n(&right[0]),
        ));
    }
    let a = [
        finite_number_value("cross", &left[0], 0)?,
        finite_number_value("cross", &left[1], 1)?,
        finite_number_value("cross", &left[2], 2)?,
    ];
    let b = [
        finite_number_value("cross", &right[0], 0)?,
        finite_number_value("cross", &right[1], 1)?,
        finite_number_value("cross", &right[2], 2)?,
    ];
    ctx.register_array(
        "cross",
        vec![
            Value::Number(a[1] * b[2] - a[2] * b[1]),
            Value::Number(a[2] * b[0] - a[0] * b[2]),
            Value::Number(a[0] * b[1] - a[1] * b[0]),
        ],
    )
}

fn parent_module<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    expect_count("parent_module", argc, &[0, 1])?;
    let value = if argc == 0 {
        Value::Number(1.0)
    } else {
        arg(0)?
    };
    let Some(raw_depth) = value.as_number() else {
        return Ok(Value::Undef);
    };
    let depth = if raw_depth.is_nan() {
        0.0
    } else if raw_depth == f64::INFINITY {
        0x7fff_ffff as f64
    } else if raw_depth == f64::NEG_INFINITY {
        -0x8000_0000 as f64
    } else {
        raw_depth.trunc()
    };
    if depth < 0.0 {
        ctx.warn(
            "parent_module",
            format!(
                "negative index {} is not allowed",
                js_number_to_string(depth)
            ),
        );
        return Ok(Value::Undef);
    }
    let result = (ctx.parent_module)(depth as usize);
    match result {
        Some(name) => Ok(Value::string(name)),
        None => {
            ctx.warn(
                "parent_module",
                format!(
                    "index {} is outside the active module stack",
                    js_number_to_string(depth)
                ),
            );
            Ok(Value::Undef)
        }
    }
}

fn min_max<'a>(
    name: &'static str,
    argc: usize,
    arg: ArgReader<'_, 'a>,
    _ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    if argc == 0 {
        return fail(name, "expects at least 1 argument");
    }
    let first = arg(0)?;
    if argc == 1
        && let Value::Vector(values) = &first
    {
        if values.is_empty() {
            return fail(name, "expects at least 1 vector element");
        }
        let mut selected = values[0].clone();
        for value in values.iter().skip(1) {
            let compared = if name == "min" {
                compare_values(value, &selected)
            } else {
                compare_values(&selected, value)
            };
            if compared.is_some_and(|c| c < 0.0) {
                selected = value.clone();
            }
        }
        return Ok(selected);
    }
    let mut selected = number_value(name, &first, 0)?;
    for index in 1..argc {
        let value = number_value(name, &arg(index)?, index)?;
        if (name == "min" && value < selected) || (name == "max" && value > selected) {
            selected = value;
        }
    }
    Ok(Value::Number(selected))
}

fn predicate<'a>(
    name: &'static str,
    test: fn(&Value<'a>) -> bool,
) -> impl Fn(usize, ArgReader<'_, 'a>, &mut BuiltinContext<'_, 'a>) -> BResult<Value<'a>> {
    move |argc, arg, _ctx| {
        expect_count(name, argc, &[1])?;
        Ok(Value::Bool(test(&arg(0)?)))
    }
}

pub const BUILTIN_FUNCTION_NAMES: &[&str] = &[
    "abs",
    "sign",
    "rands",
    "min",
    "max",
    "sin",
    "cos",
    "asin",
    "acos",
    "tan",
    "atan",
    "atan2",
    "round",
    "ceil",
    "floor",
    "pow",
    "sqrt",
    "exp",
    "len",
    "log",
    "ln",
    "str",
    "chr",
    "ord",
    "concat",
    "lookup",
    "search",
    "version",
    "version_num",
    "norm",
    "cross",
    "parent_module",
    "is_undef",
    "is_list",
    "is_num",
    "is_bool",
    "is_string",
    "is_function",
];

pub fn is_builtin_function_name(name: &str) -> bool {
    BUILTIN_FUNCTION_NAMES.contains(&name)
}

/// Evaluate one call by name with deferred argument access, or return `None`
/// when the name is not a 2021.01 built-in (user or unknown function, which
/// the host evaluator must resolve).
pub fn evaluate_builtin<'a>(
    name: &str,
    argc: usize,
    arg: ArgReader<'_, 'a>,
    ctx: &mut BuiltinContext<'_, 'a>,
) -> Option<BResult<Value<'a>>> {
    let result = match name {
        "abs" => unary_number("abs", f64::abs)(argc, arg, ctx),
        "sign" => unary_number("sign", |v| {
            if v < 0.0 {
                -1.0
            } else if v > 0.0 {
                1.0
            } else {
                0.0
            }
        })(argc, arg, ctx),
        "rands" => rands(argc, arg, ctx),
        "min" => min_max("min", argc, arg, ctx),
        "max" => min_max("max", argc, arg, ctx),
        "sin" => unary_number("sin", sin_degrees)(argc, arg, ctx),
        "cos" => unary_number("cos", cos_degrees)(argc, arg, ctx),
        "asin" => unary_number("asin", asin_degrees)(argc, arg, ctx),
        "acos" => unary_number("acos", acos_degrees)(argc, arg, ctx),
        "tan" => unary_number("tan", tan_degrees)(argc, arg, ctx),
        "atan" => unary_number("atan", atan_degrees)(argc, arg, ctx),
        "atan2" => binary_number("atan2", atan2_degrees)(argc, arg, ctx),
        "round" => unary_number("round", round_away_from_zero)(argc, arg, ctx),
        "ceil" => unary_number("ceil", f64::ceil)(argc, arg, ctx),
        "floor" => unary_number("floor", f64::floor)(argc, arg, ctx),
        "pow" => binary_number("pow", f64::powf)(argc, arg, ctx),
        "sqrt" => unary_number("sqrt", f64::sqrt)(argc, arg, ctx),
        "exp" => unary_number("exp", f64::exp)(argc, arg, ctx),
        "len" => len(argc, arg, ctx),
        "log" => log(argc, arg, ctx),
        "ln" => unary_number("ln", f64::ln)(argc, arg, ctx),
        "str" => str_(argc, arg, ctx),
        "chr" => chr(argc, arg, ctx),
        "ord" => ord(argc, arg, ctx),
        "concat" => concat(argc, arg, ctx),
        "lookup" => lookup(argc, arg, ctx),
        "search" => search(argc, arg, ctx),
        "version" => version(argc, arg, ctx),
        "version_num" => version_num(argc, arg, ctx),
        "norm" => norm(argc, arg, ctx),
        "cross" => cross(argc, arg, ctx),
        "parent_module" => parent_module(argc, arg, ctx),
        "is_undef" => predicate("is_undef", |v| v.is_undef())(argc, arg, ctx),
        "is_list" => predicate("is_list", |v| matches!(v, Value::Vector(_)))(argc, arg, ctx),
        "is_num" => {
            predicate("is_num", |v| matches!(v, Value::Number(n) if !n.is_nan()))(argc, arg, ctx)
        }
        "is_bool" => predicate("is_bool", |v| matches!(v, Value::Bool(_)))(argc, arg, ctx),
        "is_string" => predicate("is_string", |v| matches!(v, Value::Str(_)))(argc, arg, ctx),
        "is_function" => {
            let r = predicate("is_function", |v| v.is_function());
            r(argc, arg, ctx)
        }
        _ => return None,
    };
    Some(result)
}

/// Keep `ParseError` referenced for downstream diagnostics plumbing.
#[allow(unused)]
fn _assert_parse_error_used(_: &ParseError) {}
