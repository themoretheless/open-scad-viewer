use crate::bambu::config::{BambuLanConfig, BambuPrintOptions};
use crate::bambu::messages::{
    pause_payload, project_file_payload, pushall_payload, report_topic, request_topic,
    resume_payload, stop_payload,
};
use crate::job::{ArtifactKind, JobStatus, PrintJob, PrinterId};
use crate::transport::{MqttMessage, Transport};
use crate::{invalid, Result};

/// Orchestrates Bambu LAN upload + MQTT print commands over a [`Transport`].
pub struct BambuLanClient<T> {
    pub config: BambuLanConfig,
    pub options: BambuPrintOptions,
    pub transport: T,
    sequence: u64,
}

impl<T: Transport> BambuLanClient<T> {
    pub fn new(config: BambuLanConfig, transport: T) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            options: BambuPrintOptions::default(),
            transport,
            sequence: 1,
        })
    }

    pub fn with_options(mut self, options: BambuPrintOptions) -> Self {
        self.options = options;
        self
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

    /// Upload the artifact to FTPS root, then publish `print.project_file`.
    ///
    /// Requires LAN Mode + Developer Mode on the printer for control writes.
    /// Does not open sockets itself.
    pub fn submit_job(&mut self, job: &PrintJob) -> Result<()> {
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
        self.transport
            .upload(&job.artifact.file_name, &job.artifact.bytes)?;
        let sequence = self.next_sequence();
        let payload = project_file_payload(
            &sequence,
            &job.artifact.file_name,
            &job.plate_gcode_path,
            &self.options,
        )?;
        self.transport.publish(&MqttMessage {
            topic: request_topic(&self.config.serial),
            payload,
        })
    }

    pub fn pause(&mut self) -> Result<()> {
        self.command(pause_payload)
    }

    pub fn resume(&mut self) -> Result<()> {
        self.command(resume_payload)
    }

    pub fn stop(&mut self) -> Result<()> {
        self.command(stop_payload)
    }

    pub fn push_status(&mut self) -> Result<JobStatus> {
        let sequence = self.next_sequence();
        let raw = self.transport.request_report(
            &request_topic(&self.config.serial),
            &report_topic(&self.config.serial),
            &pushall_payload(&sequence),
        )?;
        Ok(parse_status_report(&raw))
    }

    fn command(&mut self, build: fn(&str) -> String) -> Result<()> {
        let sequence = self.next_sequence();
        self.transport.publish(&MqttMessage {
            topic: request_topic(&self.config.serial),
            payload: build(&sequence),
        })
    }
}

fn parse_status_report(raw: &str) -> JobStatus {
    JobStatus {
        gcode_state: find_string_field(raw, "gcode_state").unwrap_or_default(),
        percent: find_number_field(raw, "mc_percent").and_then(|v| {
            if (0.0..=100.0).contains(&v) {
                Some(v as u8)
            } else {
                None
            }
        }),
        layer: find_number_field(raw, "layer_num").map(|v| v as u32),
        raw: raw.to_owned(),
    }
}

fn find_string_field(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let start = json.find(&needle)? + needle.len();
    let rest = json[start..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}

fn find_number_field(json: &str, key: &str) -> Option<f64> {
    let needle = format!("\"{key}\"");
    let start = json.find(&needle)? + needle.len();
    let rest = json[start..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+'))
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}
