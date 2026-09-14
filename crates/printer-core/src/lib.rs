//! Multi-vendor LAN printer job transport. Not a slicer and not G-code generation.
//!
//! Public surface is [`PrinterBackend`] (Bambu / Moonraker / OctoPrint / PrusaLink /
//! Creality / Snapmaker). Enable the `network` feature for live sockets; default builds stay
//! offline with mocks.
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]

pub mod backend;
pub mod bambu;
pub mod creality;
pub mod hash;
pub mod http;
pub mod job;
pub mod mock;
pub mod moonraker;
pub mod octoprint;
pub mod prusa;
pub mod scrub;
pub mod snapmaker;
pub mod transport;

pub use backend::{PrinterBackend, SubmitOutcome};
pub use bambu::{BambuLanBackend, BambuLanClient, BambuLanConfig, BambuPrintOptions};
#[cfg(feature = "network")]
pub use bambu::{lan_client_config, BambuLanTransport};
pub use creality::{CrealityBackend, CrealityConfig};
pub use http::{HttpRequest, HttpResponse, HttpTransport, MockHttpTransport};
pub use job::{
    ArtifactKind, JobState, JobStatus, PrintArtifact, PrintJob, PrinterId, map_vendor_state,
};
pub use mock::MockTransport;
pub use moonraker::{MoonrakerBackend, MoonrakerConfig};
pub use octoprint::{OctoPrintBackend, OctoPrintConfig};
pub use prusa::{PrusaLinkBackend, PrusaLinkConfig};
pub use scrub::scrub_secrets;
pub use snapmaker::{SnapmakerBackend, SnapmakerConfig};
pub use transport::{MqttMessage, Transport};

pub use math_core::{Error, Result};

pub const MAX_ARTIFACT_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_REMOTE_NAME_BYTES: usize = 255;

pub(crate) fn invalid(code: &'static str, message: &str) -> Error {
    Error::new(code, message)
}
