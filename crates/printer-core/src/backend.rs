//! Vendor-neutral printer LAN backend.

use crate::job::{JobStatus, PrintJob, PrinterId};
use crate::Result;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmitOutcome {
    pub remote_name: String,
    /// True when the backend waited for a start ACK / state transition.
    pub verified: bool,
    pub gcode_state: Option<String>,
}

/// LAN print host: upload + start, pause/resume/stop, status.
pub trait PrinterBackend {
    fn id(&self) -> PrinterId;
    fn submit_job(&mut self, job: &PrintJob) -> Result<SubmitOutcome>;
    fn pause(&mut self) -> Result<()>;
    fn resume(&mut self) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
    fn status(&mut self) -> Result<JobStatus>;
}
