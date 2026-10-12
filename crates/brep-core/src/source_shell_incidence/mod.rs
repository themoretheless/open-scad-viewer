//! Closed oriented incidence of original surface-composed wires.
//! This is not a certificate of embedded faces, material orientation or volume.
use crate::{
    source_shared_edge::{self, SharedEdge},
    source_world_wire::Wire,
};
use nurbs_core::{Error, Result, curve::Curve};
use std::collections::BTreeMap;

mod address;
mod assemble_with_pole_inputs;
#[cfg(test)]
mod tests;
pub use address::*;
pub use assemble_with_pole_inputs::*;
