//! Printer job transport. Not a slicer and not G-code generation.
//!
//! Default surface is protocol orchestration over a pluggable [`Transport`].
//! Bambu Lab LAN (FTPS upload + MQTT control) is the first backend. Live TLS
//! sockets are out of this crate until a `network` adapter is wired; tests use
//! [`mock::MockTransport`].
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]

pub mod bambu;
pub mod job;
pub mod mock;
pub mod transport;

pub use bambu::{BambuLanClient, BambuLanConfig, BambuPrintOptions};
pub use job::{ArtifactKind, JobStatus, PrintArtifact, PrintJob, PrinterId};
pub use mock::MockTransport;
pub use transport::{MqttMessage, Transport};

pub use math_core::{Error, Result};

pub const MAX_ARTIFACT_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_REMOTE_NAME_BYTES: usize = 255;

pub(crate) fn invalid(code: &'static str, message: &str) -> Error {
    Error::new(code, message)
}
