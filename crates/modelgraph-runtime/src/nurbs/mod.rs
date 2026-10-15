//! Numeric own-geometry graph preparation. Kernel execution remains separate.
use crate::units::{self, ANGLE, LENGTH, Numeric, SCALAR};
use crate::{Error, Result};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};
use value_codec::{Value as J, json};

mod compile;
mod compile_text;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod cartesian_loft_tests;
pub use compile::*;
pub use compile_text::*;
