//! OpenSCAD runtime values and value semantics: a faithful port of
//! `src/services/openScadValueSemantics.ts` plus the value helpers embedded in
//! `src/services/openscadParser.ts` (truthiness, deep equality, comparison,
//! unary/binary/index/member operators, ranges, and the OpenSCAD/JS number
//! formatting used by echo/assert/str).
use crate::ParseError;
use crate::ast::{Expr, FunctionNode, ModuleNode, ModuleParam, Statement};
use crate::lexer::TT;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

mod shape_descriptor;
mod binary;
pub use shape_descriptor::*;
pub use binary::*;
