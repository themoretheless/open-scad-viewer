//! OpenSCAD value evaluation for both repository language profiles.
//! Default evaluation accounts shape names and dimensions for diagnostics.
//! Opt-in recording emits a typed solid program for supported operations;
//! independent geometry kernels execute that program through the host adapter.
mod expressions;
mod comprehensions;
mod function_calls;
mod recording;
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


#[cfg(test)]
mod profile_recording_tests;

#[cfg(test)]
mod profile_matrix_tests;

#[cfg(test)]
mod extrusion_recording_tests;

#[cfg(test)]
mod polygon_recording_tests;

mod quality;
mod evaluator_2;
mod evaluate_program;
pub use quality::*;
use evaluator_2::*;
pub use evaluate_program::*;

mod evaluator_2_impl_1;
mod evaluator_2_impl_2;
mod evaluator_2_impl_3;
mod evaluator_2_impl_4;

mod eval_node_cases_1;
mod eval_node_cases_2;
mod eval_node_cases_3;
mod eval_node_cases_4;
