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


use crate::degree_math::{sin_degrees,cos_degrees,tan_degrees,asin_degrees,acos_degrees,atan_degrees,atan2_degrees,round_away_from_zero};

mod builtin_error;
mod character_string;
mod is_builtin_function_name;
pub use builtin_error::*;
use character_string::*;
pub use is_builtin_function_name::*;

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
