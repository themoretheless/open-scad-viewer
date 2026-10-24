use crate::value::json;
use value_codec::Value as J;
type R<T> = crate::Result<T>;

include!("../parser_match.rs");

#[cfg(test)]
mod statement_check_tests;

mod statement;
mod parser;
mod number;
pub use statement::*;
use parser::*;
pub use number::*;

mod parser_impl_1;
mod parser_impl_2;
