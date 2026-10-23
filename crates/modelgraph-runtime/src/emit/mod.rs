//! Canonical graph indexing, bounded evaluation, and geometry lowering.
use crate::eval::{Evaluator, Scope, Value as RuntimeValue, numeric_value, sequence};
use crate::units::{ANGLE, Dimension, LENGTH, SCALAR};
use crate::{Error, Result};
use crate::{assembly, profiles, sketch};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use value_codec::{Value, json};

/// Re-exported from `mechanical-core`, which owns the generated-source number spelling.
pub use mechanical_core::append_number;

mod number;
mod emitter;
mod compile;
#[cfg(test)]
mod match_tests;
#[cfg(test)]
mod dynamic_vector_tests;
#[cfg(test)]
mod assertion_tests;
pub use number::*;
pub use compile::*;
