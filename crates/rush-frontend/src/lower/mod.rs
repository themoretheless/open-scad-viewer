use crate::ordered_map::OrderedMap as Map;
use crate::parser::{Statement, number};
use crate::value::json;
use std::{collections::BTreeSet as Set, rc::Rc};
use value_codec::Value as J;
type R<T> = crate::Result<T>;
type Env = Map<String, V>;
type Types = Map<String, J>;

use signatures::signature;

mod signatures;

include!("../lower_query.rs");

include!("../lower_match.rs");

include!("../lower_traits.rs");

mod v;
mod compiler;
mod compile;
use v::*;
use compiler::*;
pub use compile::*;

mod compiler_impl_1;
mod compiler_impl_2;
mod compiler_impl_3;
mod compiler_impl_4;
mod compiler_impl_5;
