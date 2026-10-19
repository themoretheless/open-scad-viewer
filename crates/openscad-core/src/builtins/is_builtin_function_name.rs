use super::*;

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
pub(super) fn _assert_parse_error_used(_: &ParseError) {}
