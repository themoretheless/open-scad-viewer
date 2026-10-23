//! Operations no local router owns go to the NURBS kernel.
use crate::*;

pub(crate) fn dispatch(v: Value) -> Result<Value> {
    match v["op"].as_str().unwrap_or("") {
        _ => Ok(nurbs_core::dispatch(v)?),
    }
}
