//! Transport adapter for the native progressive profile body owner.
use crate::*;

pub(super) fn build(v: Value) -> Result<Value> {
    crate::dispatch::nurbs::handle_progressive_profile_body(&v)
}
