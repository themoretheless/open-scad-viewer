use crate::Result;
use crate::backend::{PrinterBackend, SubmitOutcome};
use crate::creality::CrealityConfig;
use crate::http::HttpTransport;
use crate::job::{JobStatus, PrintJob, PrinterId};
use crate::moonraker::MoonrakerBackend;

/// Creality LAN host speaking Moonraker (rooted Klipper Creality printers).
pub struct CrealityBackend<T> {
    inner: MoonrakerBackend<T>,
}

impl<T: HttpTransport> CrealityBackend<T> {
    pub fn new(config: CrealityConfig, transport: T) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            inner: MoonrakerBackend::new(config.inner, transport)?,
        })
    }

    pub fn moonraker(&self) -> &MoonrakerBackend<T> {
        &self.inner
    }

    pub fn moonraker_mut(&mut self) -> &mut MoonrakerBackend<T> {
        &mut self.inner
    }
}

#[cfg(feature = "network")]
impl CrealityBackend<crate::http::live::UreqHttpTransport> {
    pub fn connect(config: CrealityConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            inner: MoonrakerBackend::<crate::http::live::UreqHttpTransport>::connect(config.inner)?,
        })
    }
}

impl<T: HttpTransport> PrinterBackend for CrealityBackend<T> {
    fn id(&self) -> PrinterId {
        PrinterId {
            vendor: "creality".into(),
            serial: self.inner.config.printer_name.clone(),
        }
    }

    fn submit_job(&mut self, job: &PrintJob) -> Result<SubmitOutcome> {
        self.inner.submit_job(job)
    }

    fn pause(&mut self) -> Result<()> {
        self.inner.pause()
    }

    fn resume(&mut self) -> Result<()> {
        self.inner.resume()
    }

    fn stop(&mut self) -> Result<()> {
        self.inner.stop()
    }

    fn status(&mut self) -> Result<JobStatus> {
        self.inner.status()
    }
}
