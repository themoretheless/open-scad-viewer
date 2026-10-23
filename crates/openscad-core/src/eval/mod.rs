//! OpenSCAD value evaluator (migration stage 2): a faithful port of the
//! evaluator half of `src/services/openscadParser.ts` for both language
//! profiles. Geometry modules contribute accounted shape descriptors (name +
//! dimension) through the same shape-list algebra as the TS evaluator
//! (boolean/extrude/hull combine to one, transforms pass through or union per
//! profile); the real geometry kernels arrive with stage 3 via the shared
//! `geometry-bridge` handle store.
use crate::ast::*;
use crate::builtins::{self, BuiltinContext, BuiltinError};
use crate::lexer::TT;
use crate::value::index as index_value;
use crate::value::*;
use crate::{LanguageProfile, ParseError};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Mirrors `MAX_EVAL_DEPTH` in `openscadParser.ts`.
pub const MAX_EVAL_DEPTH: usize = 128;
/// Mirrors `MAX_EVAL_OPS` in `openscadParser.ts`.
pub const MAX_EVAL_OPS: u64 = 1_000_000;
/// Mirrors `MAX_VALUE_ELEMENTS` in `openscadParser.ts`.
pub const MAX_VALUE_ELEMENTS: usize = 1_000_000;
/// Mirrors `MAX_RANGE_ITEMS` in `openscadParser.ts`.
pub const MAX_RANGE_ITEMS: usize = 10_000;
/// Mirrors `MAX_EVALUATED_VALUE_UNITS` in `openscadParser.ts`.
pub const MAX_EVALUATED_VALUE_UNITS: u64 = 500_000;
/// Mirrors `MAX_SHAPES` in `openscadParser.ts`.
pub const MAX_SHAPES: usize = 1_000;
/// Mirrors `MAX_FN` in `openscadParser.ts`.
pub const MAX_FN: f64 = 256.0;

use crate::{MAX_EXPRESSION_DEPTH, MAX_SOURCE_LENGTH};

mod quality;
mod evaluator;
mod evaluate_program;
pub use quality::*;
pub use evaluate_program::*;

mod expressions;
mod comprehensions;
mod function_calls;
mod recording;
