use crate::backend::{PrinterBackend, SubmitOutcome};
use crate::bambu::config::{BambuLanConfig, BambuPrintOptions};
use crate::bambu::messages::{
    artifact_md5, pause_payload, project_file_payload, pushall_payload, report_topic, request_topic,
    resume_payload, stop_payload,
};
use crate::bambu::status::parse_bambu_report;
use crate::job::{ArtifactKind, JobStatus, PrintJob, PrinterId};
use crate::scrub::scrub_secrets;
use crate::transport::{MqttMessage, Transport};
use crate::{invalid, Result};

/// Bambu Lab LAN backend over a pluggable FTPS/MQTT [`Transport`].
pub struct BambuLanBackend<T> {
    pub config: BambuLanConfig,
    pub options: BambuPrintOptions,
    pub transport: T,
    sequence: u64,
    /// When false, `project_file` sends `md5:""` (legacy skip).
    pub send_md5: bool,
}

/// Alias kept for earlier call sites.
pub type BambuLanClient<T> = BambuLanBackend<T>;

impl<T: Transport> BambuLanBackend<T> {
    pub fn new(config: BambuLanConfig, transport: T) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            options: BambuPrintOptions::default(),
            transport,
            sequence: 1,
            send_md5: true,
        })
    }

    pub fn with_options(mut self, options: BambuPrintOptions) -> Self {
        self.options = options;
        self
    }

    pub fn without_md5(mut self) -> Self {
        self.send_md5 = false;
        self
    }

    /// Connect with the live FTPS/MQTT transport (`network` feature).
    #[cfg(feature = "network")]
    pub fn connect_lan(config: BambuLanConfig) -> Result<BambuLanBackend<crate::bambu::BambuLanTransport>> {
        let transport = crate::bambu::BambuLanTransport::new(config.clone())?;
        BambuLanBackend::new(config, transport)
    }

    /// Like [`connect_lan`], but if `serial` is empty, read it from the printer TLS cert CN.
    #[cfg(feature = "network")]
    pub fn connect_lan_discover(
        mut config: BambuLanConfig,
    ) -> Result<BambuLanBackend<crate::bambu::BambuLanTransport>> {
        config.validate_for_tls_serial()?;
        if config.serial.trim().is_empty() {
            let serial = crate::bambu::BambuLanTransport::fetch_serial_from_tls(&config)?;
            config.serial = serial;
        }
        Self::connect_lan(config)
    }

    pub fn printer_id(&self) -> PrinterId {
        PrinterId {
            vendor: "bambu".into(),
            serial: self.config.serial.clone(),
        }
    }

    fn next_sequence(&mut self) -> String {
        let id = self.sequence;
        self.sequence = self.sequence.saturating_add(1);
        id.to_string()
    }

    fn wrap_err(&self, err: crate::Error) -> crate::Error {
        crate::Error::new(
            err.code,
            scrub_secrets(&err.message, &[self.config.access_code.as_str()]),
        )
    }

    fn command(&mut self, build: fn(&str) -> String) -> Result<()> {
        let sequence = self.next_sequence();
        self.transport
            .publish(&MqttMessage {
                topic: request_topic(&self.config.serial),
                payload: build(&sequence),
            })
            .map_err(|e| self.wrap_err(e))
    }

    fn verify_start(&mut self) -> Result<Option<String>> {
        // Poll up to a few reports for prepare/running or print_error.
        for _ in 0..4 {
            let sequence = self.next_sequence();
            let raw = self
                .transport
                .request_report(
                    &request_topic(&self.config.serial),
                    &report_topic(&self.config.serial),
                    &pushall_payload(&sequence),
                )
                .map_err(|e| self.wrap_err(e))?;
            let report = parse_bambu_report(&raw);
            if report.has_print_error() {
                return Err(invalid(
                    "PRINTER_START_FAILED",
                    &format!(
                        "Bambu print_error={} gcode_state={}",
                        report.print_error.unwrap_or(0),
                        report.gcode_state
                    ),
                ));
            }
            if report.is_start_like() {
                return Ok(Some(report.gcode_state));
            }
        }
        Err(invalid(
            "PRINTER_START_FAILED",
            "Bambu did not enter prepare/running after project_file",
        ))
    }
}

impl<T: Transport> PrinterBackend for BambuLanBackend<T> {
    fn id(&self) -> PrinterId {
        self.printer_id()
    }

    fn submit_job(&mut self, job: &PrintJob) -> Result<SubmitOutcome> {
        job.validate()?;
        if job.printer.serial != self.config.serial {
            return Err(invalid(
                "PRINTER_SERIAL",
                "Job serial does not match the Bambu LAN client serial",
            ));
        }
        match job.artifact.kind {
            ArtifactKind::Gcode3mf => {}
            ArtifactKind::Gcode => {
                return Err(invalid(
                    "PRINTER_ARTIFACT_KIND",
                    "Bambu LAN submit_job expects a .gcode.3mf artifact; package G-code first",
                ));
            }
        }
        if job.plate_gcode_path.is_empty() {
            return Err(invalid(
                "PRINTER_PLATE_PATH",
                "Bambu submit_job requires plate_gcode_path",
            ));
        }
        self.transport
            .upload(&job.artifact.file_name, &job.artifact.bytes)
            .map_err(|e| self.wrap_err(e))?;
        let sequence = self.next_sequence();
        let md5 = if self.send_md5 {
            artifact_md5(&job.artifact.bytes)
        } else {
            String::new()
        };
        let payload = project_file_payload(
            &sequence,
            &job.artifact.file_name,
            &job.plate_gcode_path,
            &self.options,
            &md5,
        )?;
        self.transport
            .publish(&MqttMessage {
                topic: request_topic(&self.config.serial),
                payload,
            })
            .map_err(|e| self.wrap_err(e))?;
        let mut outcome = SubmitOutcome {
            remote_name: job.artifact.file_name.clone(),
            verified: false,
            gcode_state: None,
        };
        if job.verify_start {
            outcome.gcode_state = self.verify_start()?;
            outcome.verified = true;
        }
        Ok(outcome)
    }

    fn pause(&mut self) -> Result<()> {
        self.command(pause_payload)
    }

    fn resume(&mut self) -> Result<()> {
        self.command(resume_payload)
    }

    fn stop(&mut self) -> Result<()> {
        self.command(stop_payload)
    }

    fn status(&mut self) -> Result<JobStatus> {
        let sequence = self.next_sequence();
        let raw = self
            .transport
            .request_report(
                &request_topic(&self.config.serial),
                &report_topic(&self.config.serial),
                &pushall_payload(&sequence),
            )
            .map_err(|e| self.wrap_err(e))?;
        Ok(parse_bambu_report(&raw).to_status())
    }
}

/// Convenience methods mirroring the trait (keep older call style).
impl<T: Transport> BambuLanBackend<T> {
    pub fn push_status(&mut self) -> Result<JobStatus> {
        self.status()
    }
}
