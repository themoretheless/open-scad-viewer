use crate::job::admit_remote_name;
use crate::transport::{MqttMessage, Transport};
use crate::{invalid, Result, MAX_ARTIFACT_BYTES};
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Debug, Default)]
pub struct MockTransport {
    pub files: BTreeMap<String, Vec<u8>>,
    pub published: Vec<MqttMessage>,
    /// Single fallback report when `report_queue` is empty.
    pub next_report: String,
    /// FIFO reports consumed by `request_report`.
    pub report_queue: VecDeque<String>,
}

impl MockTransport {
    pub fn push_report(&mut self, report: impl Into<String>) {
        self.report_queue.push_back(report.into());
    }
}

impl Transport for MockTransport {
    fn upload(&mut self, remote_name: &str, bytes: &[u8]) -> Result<()> {
        admit_remote_name(remote_name)?;
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err(invalid(
                "PRINTER_ARTIFACT_LIMIT",
                "Print artifact exceeds 64 MiB",
            ));
        }
        self.files.insert(remote_name.to_owned(), bytes.to_vec());
        Ok(())
    }

    fn publish(&mut self, message: &MqttMessage) -> Result<()> {
        if message.topic.is_empty() || message.payload.is_empty() {
            return Err(invalid(
                "PRINTER_MQTT",
                "MQTT topic and payload must be non-empty",
            ));
        }
        self.published.push(message.clone());
        Ok(())
    }

    fn request_report(
        &mut self,
        request_topic: &str,
        _report_topic: &str,
        payload: &str,
    ) -> Result<String> {
        self.publish(&MqttMessage {
            topic: request_topic.to_owned(),
            payload: payload.to_owned(),
        })?;
        if let Some(report) = self.report_queue.pop_front() {
            return Ok(report);
        }
        if self.next_report.is_empty() {
            return Err(invalid(
                "PRINTER_REPORT",
                "Mock transport has no queued report",
            ));
        }
        Ok(self.next_report.clone())
    }
}
