//! Native CLI / loopback companion for `printer-core` LAN transport.

#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]

pub mod artifact;
pub mod connect;
pub mod serve;

pub use artifact::{PrintArtifactBytes, infer_artifact_kind, load_artifact};
pub use connect::{
    ConnectionArgs, ControlAction, VendorKind, control, discover, send_bytes, send_path,
};
pub use serve::{ServeOptions, run_serve};
