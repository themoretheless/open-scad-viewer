//! Native CLI / loopback companion for `printer-core` LAN transport.


pub mod artifact;
pub mod connect;
pub mod serve;

pub use artifact::{PrintArtifactBytes, infer_artifact_kind, load_artifact};
pub use connect::{
    ConnectionArgs, ControlAction, VendorKind, control, discover, send_bytes, send_path,
};
pub use serve::{ServeOptions, run_serve};
