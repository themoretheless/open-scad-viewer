//! Recursive-descent parser: a faithful port of the `Parser` class in
//! `src/services/openscadCompiler.ts` for both language profiles, including
//! node-count/depth limits and exact diagnostic messages and positions.
use crate::ast::*;
use crate::lexer::{TT, Token};
use crate::{
    LanguageProfile, MAX_AST_NODES, MAX_EXPRESSION_DEPTH, MAX_STATEMENT_DEPTH, ParseError,
};

type PResult<T> = Result<T, ParseError>;

mod parser;
mod parser_2;
mod record_insert;
pub use parser::*;
pub use parser_2::*;
use record_insert::*;
