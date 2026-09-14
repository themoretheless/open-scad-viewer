use crate::Result;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MqttMessage {
    pub topic: String,
    pub payload: String,
}

/// Pluggable FTPS + MQTT surface. Live TLS sockets live outside this trait.
pub trait Transport {
    fn upload(&mut self, remote_name: &str, bytes: &[u8]) -> Result<()>;
    fn publish(&mut self, message: &MqttMessage) -> Result<()>;
    fn request_report(&mut self, request_topic: &str, report_topic: &str, payload: &str) -> Result<String>;
}
