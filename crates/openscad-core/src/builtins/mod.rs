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
