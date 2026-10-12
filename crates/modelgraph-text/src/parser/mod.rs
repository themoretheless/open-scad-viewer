use crate::value::json;
use value_codec::Value as J;
type R<T> = crate::Result<T>;

include!("../parser_match.rs");

mod statement;
mod parser;
mod number;
#[cfg(test)]
mod statement_check_tests;
pub use statement::*;
use parser::*;
pub use number::*;
