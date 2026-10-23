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
type RegisterArray<'h, 'a> =
    &'h mut dyn FnMut(Vec<Value<'a>>, &str) -> crate::value::EvalResult<Vec<Value<'a>>>;

fn fail<T>(name: &str, message: impl Into<String>) -> BResult<T> {
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

mod degrees;
mod format;
mod random;
mod functions;
use degrees::*;
pub use format::{format_builtin_value, format_expression};
use random::*;
use functions::*;

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


/// Pure standalone OpenSCAD `lookup(key, table)` linear interpolation helper over `(key, value)` pairs:
/// clamps to the lowest key's value below the table range, clamps to the highest key's value above
/// the table range, and linearly interpolates between the tightest bounding keys inside the range.
/// Returns `f64::NAN` if `table` is empty or `key` is non-finite.
pub fn lookup_table_interpolate(key: f64, table: &[[f64; 2]]) -> f64 {
    if !key.is_finite() || table.is_empty() {
        return f64::NAN;
    }
    let (mut low_k, mut low_v) = (table[0][0], table[0][1]);
    let (mut high_k, mut high_v) = (table[0][0], table[0][1]);
    for &[ck, cv] in table.iter().skip(1) {
        if !ck.is_finite() || !cv.is_finite() {
            continue;
        }
        if ck <= key && (ck > low_k || low_k > key) {
            low_k = ck;
            low_v = cv;
        }
        if ck >= key && (ck < high_k || high_k < key) {
            high_k = ck;
            high_v = cv;
        }
    }
    if key <= low_k {
        return high_v;
    }
    if key >= high_k {
        return low_v;
    }
    let fraction = (key - low_k) / (high_k - low_k);
    high_v * fraction + low_v * (1.0 - fraction)
}

/// Pure standalone OpenSCAD Euclidean vector norm (`norm(v) = sqrt(sum(v_i^2))`) using
/// overflow-safe hypot scaling.
pub fn vector_norm(v: &[f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let mut max_abs = 0.0_f64;
    for &x in v {
        if x.is_nan() {
            return f64::NAN;
        }
        max_abs = max_abs.max(x.abs());
    }
    if max_abs == 0.0 || !max_abs.is_finite() {
        return max_abs;
    }
    let sum_sq: f64 = v.iter().map(|&x| (x / max_abs).powi(2)).sum();
    max_abs * sum_sq.sqrt()
}

/// Pure standalone 3D vector cross product (`cross(a, b)`).
pub fn vector_cross_3d(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Pure standalone 2D vector cross product (`cross([ax, ay], [bx, by]) = ax * by - ay * bx`).
pub fn vector_cross_2d(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

/// Folds constant-literal binary and unary arithmetic expressions in an OpenSCAD [`Expr`] AST
/// (`+`, `-`, `*`, `/`, `%`, `^` on `LiteralValue::Number` and `!` on `LiteralValue::Bool`)
/// without altering source spans or non-constant subtrees.
pub fn fold_constant_expr(expr: &Expr) -> Expr {
    match expr {
        Expr::Unary { op, value, p } => {
            let folded = fold_constant_expr(value);
            if let Expr::Literal {
                value: ref lit,
                ..
            } = folded
            {
                match (op, lit) {
                    (TT::Minus, crate::ast::LiteralValue::Number(n)) if n.is_finite() => {
                        return Expr::Literal {
                            value: crate::ast::LiteralValue::Number(-n),
                            p: *p,
                        };
                    }
                    (TT::Plus, crate::ast::LiteralValue::Number(n)) if n.is_finite() => {
                        return Expr::Literal {
                            value: crate::ast::LiteralValue::Number(*n),
                            p: *p,
                        };
                    }
                    (TT::Not, crate::ast::LiteralValue::Bool(b)) => {
                        return Expr::Literal {
                            value: crate::ast::LiteralValue::Bool(!b),
                            p: *p,
                        };
                    }
                    _ => {}
                }
            }
            Expr::Unary {
                op: *op,
                value: Box::new(folded),
                p: *p,
            }
        }
        Expr::Binary { op, left, right, p } => {
            let l = fold_constant_expr(left);
            let r = fold_constant_expr(right);
            if let (
                Expr::Literal {
                    value: crate::ast::LiteralValue::Number(a),
                    ..
                },
                Expr::Literal {
                    value: crate::ast::LiteralValue::Number(b),
                    ..
                },
            ) = (&l, &r)
            {
                let folded_val = match op {
                    TT::Plus => Some(a + b),
                    TT::Minus => Some(a - b),
                    TT::Star => Some(a * b),
                    TT::Slash if *b != 0.0 => Some(a / b),
                    TT::Percent if *b != 0.0 => Some(a % b),
                    TT::Caret => Some(a.powf(*b)),
                    _ => None,
                };
                if let Some(res) = folded_val
                    && res.is_finite()
                {
                    return Expr::Literal {
                        value: crate::ast::LiteralValue::Number(res),
                        p: *p,
                    };
                }
            }
            Expr::Binary {
                op: *op,
                left: Box::new(l),
                right: Box::new(r),
                p: *p,
            }
        }
        Expr::Vector { items, p } => Expr::Vector {
            items: items.iter().map(fold_constant_expr).collect(),
            p: *p,
        },
        other => other.clone(),
    }
}
