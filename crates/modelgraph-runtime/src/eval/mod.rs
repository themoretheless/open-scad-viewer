//! Bounded expression execution. ASTs stay borrowed and immutable; list values
//! and captured scopes share storage instead of cloning expression subtrees.
use crate::range::resolve_interval;
use crate::units::{self, Dimension, Numeric};
use crate::{Error, Result};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use value_codec::{Value as Json, json};

pub type Scope<'a> = Rc<HashMap<String, Value<'a>>>;

include!("../eval_query.rs");

include!("../eval_match.rs");

mod value;
mod evaluator;
mod array;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod guarded_tests;
#[cfg(test)]
mod memo_tests;
pub use value::*;
use array::*;
