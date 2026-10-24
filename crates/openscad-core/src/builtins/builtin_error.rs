use super::*;

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

pub(super) type BResult<T> = Result<T, BuiltinError>;
pub(super) type RegisterArray<'h, 'a> =
    &'h mut dyn FnMut(Vec<Value<'a>>, &str) -> crate::value::EvalResult<Vec<Value<'a>>>;

pub(super) fn fail<T>(name: &str, message: impl Into<String>) -> BResult<T> {
    Err(BuiltinError::Fail(format!("{name}() {}", message.into())))
}

/// Host effects available to the built-ins (TS `OpenScadBuiltinFunctionContext`).
pub struct BuiltinContext<'h, 'a> {
    pub warn: &'h mut dyn FnMut(String),
    pub register_array: RegisterArray<'h, 'a>,
    pub register_string: &'h mut dyn FnMut(String, &str) -> crate::value::EvalResult<String>,
    pub random: &'h mut dyn FnMut() -> f64,
    pub parent_module: &'h dyn Fn(usize) -> Option<String>,
}

impl<'h, 'a> BuiltinContext<'h, 'a> {
    pub(super) fn warn(&mut self, name: &str, message: impl Into<String>) {
        (self.warn)(format!("{name}() {}", message.into()));
    }
    pub(super) fn register_array(&mut self, name: &str, items: Vec<Value<'a>>) -> BResult<Value<'a>> {
        Ok(Value::vector((self.register_array)(
            items,
            &format!("{name}() result"),
        )?))
    }
    pub(super) fn register_string(&mut self, name: &str, value: String) -> BResult<Value<'a>> {
        Ok(Value::string((self.register_string)(
            value,
            &format!("{name}() result"),
        )?))
    }
}

pub(super) type ArgReader<'r, 'a> = &'r mut dyn FnMut(usize) -> BResult<Value<'a>>;

pub(super) fn expect_count(name: &str, argc: usize, counts: &[usize]) -> BResult<()> {
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

pub(super) fn number_value(name: &str, value: &Value, argument: usize) -> BResult<f64> {
    match value.as_number() {
        Some(v) => Ok(v),
        None => fail(name, format!("argument {} must be a number", argument + 1)),
    }
}

pub(super) fn finite_number_value(name: &str, value: &Value, argument: usize) -> BResult<f64> {
    let number = number_value(name, value, argument)?;
    if !number.is_finite() {
        return fail(name, format!("argument {} must be finite", argument + 1));
    }
    Ok(number)
}

pub(super) fn array_value<'v, 'a>(
    name: &str,
    value: &'v Value<'a>,
    argument: usize,
) -> BResult<&'v [Value<'a>]> {
    match value {
        Value::Vector(items) => Ok(items),
        _ => fail(name, format!("argument {} must be a vector", argument + 1)),
    }
}

pub(super) fn unary_number<'a>(
    name: &'static str,
    operation: fn(f64) -> f64,
) -> impl Fn(usize, ArgReader<'_, 'a>, &mut BuiltinContext<'_, 'a>) -> BResult<Value<'a>> {
    move |argc, arg, _ctx| {
        expect_count(name, argc, &[1])?;
        Ok(Value::Number(operation(number_value(name, &arg(0)?, 0)?)))
    }
}

pub(super) fn binary_number<'a>(
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


pub(super) fn quoted_expression_string(value: &str) -> String {
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

pub(super) fn binary_token(operator: TT) -> &'static str {
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

pub(super) fn format_expression_arguments(args: &[ExpressionArgument]) -> String {
    args.iter()
        .map(|argument| match &argument.name {
            None => format_expression(&argument.value),
            Some(name) => format!("{name} = {}", format_expression(&argument.value)),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn format_params(params: &[ModuleParam]) -> String {
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

pub(super) fn format_function(params: &[ModuleParam], body: &Expr) -> String {
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
    pub(super) state: [u32; 624],
    pub(super) index: usize,
}

impl Mt19937 {
    pub(super) fn new(seed: u32) -> Self {
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
    pub(super) fn next(&mut self) -> u32 {
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
    pub(super) fn twist(&mut self) {
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
pub(super) struct SeededRandom(pub(super) Mt19937);
impl SeededRandom {
    pub(super) fn next(&mut self) -> f64 {
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

pub(super) fn len<'a>(
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

pub(super) fn log<'a>(
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

pub(super) fn str_<'a>(
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
