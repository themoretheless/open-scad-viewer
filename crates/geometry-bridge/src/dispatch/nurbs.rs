//! Bridge operations of the `nurbs` family.
use crate::*;

mod booleans_and_features;
mod lofts_and_sweeps;
mod primitives_and_transforms;

pub(crate) fn route(v: Value) -> Routed {
    let op = v["op"].as_str().unwrap_or("");
    if primitives_and_transforms::OPS.contains(&op) {
        let op = op.to_owned();
        return Routed::Handled(primitives_and_transforms::handle(&op, v));
    }
    if lofts_and_sweeps::OPS.contains(&op) {
        let op = op.to_owned();
        return Routed::Handled(lofts_and_sweeps::handle(&op, v));
    }
    if booleans_and_features::OPS.contains(&op) {
        let op = op.to_owned();
        return Routed::Handled(booleans_and_features::handle(&op, v));
    }
    Routed::Unhandled(v)
}
