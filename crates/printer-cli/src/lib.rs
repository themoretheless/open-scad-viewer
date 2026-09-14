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

pub use artifact::{infer_artifact_kind, load_artifact, PrintArtifactBytes};
pub use connect::{
    control, discover, send_bytes, send_path, ConnectionArgs, ControlAction, VendorKind,
};
pub use serve::{run_serve, ServeOptions};
