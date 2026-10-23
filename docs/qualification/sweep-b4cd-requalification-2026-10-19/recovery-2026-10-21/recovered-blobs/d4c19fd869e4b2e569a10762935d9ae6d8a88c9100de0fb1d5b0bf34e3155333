//! Live Bambu LAN sockets: implicit FTPS :990 and MQTT/TLS :8883.
//!
//! Connect-per-op. Credentials never appear in error messages.

use std::io::Cursor;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use rumqttc::{
    AsyncClient, Event, EventLoop, MqttOptions, Packet, QoS, TlsConfiguration,
    Transport as MqttTransport,
};
use suppaftp::{RustlsConnector, RustlsFtpStream};

use crate::bambu::config::BambuLanConfig;
use crate::bambu::tls::lan_client_config;
use crate::job::admit_remote_name;
use crate::scrub::scrub_secrets;
use crate::transport::{MqttMessage, Transport};
use crate::{MAX_ARTIFACT_BYTES, Result, invalid};

const MQTT_USER: &str = "bblp";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);

/// FTPS upload + MQTT publish/report over the printer's LAN TLS endpoints.
#[derive(Clone, Debug)]
pub struct BambuLanTransport {
    pub config: BambuLanConfig,
    pub timeout: Duration,
}

impl BambuLanTransport {
    pub fn new(config: BambuLanConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            timeout: DEFAULT_TIMEOUT,
        })
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// TLS-handshake the MQTT port and read the peer certificate CN as serial.
    pub fn fetch_serial_from_tls(config: &BambuLanConfig) -> Result<String> {
        use rustls::pki_types::ServerName;
        use std::net::TcpStream;
        use std::sync::Arc;

        config.validate_for_tls_serial()?;
        let tls =
            lan_client_config().map_err(|e| network_err_scrubbed(config, "PRINTER_TLS", &e))?;
        let addr = format!("{}:{}", config.host, config.mqtt_port);
        let mut sock = TcpStream::connect(addr)
            .map_err(|e| network_err_scrubbed(config, "PRINTER_TLS", &e))?;
        let _ = sock.set_read_timeout(Some(Duration::from_secs(10)));
        let _ = sock.set_write_timeout(Some(Duration::from_secs(10)));
        let server_name = ServerName::try_from(config.host.clone()).map_err(|_| {
            invalid(
                "PRINTER_HOST",
                "Bambu LAN host is not a valid TLS server name",
            )
        })?;
        let mut conn = rustls::ClientConnection::new(Arc::clone(&tls), server_name)
            .map_err(|e| network_err_scrubbed(config, "PRINTER_TLS", &e))?;
        while conn.is_handshaking() {
            if conn.wants_write() {
                conn.write_tls(&mut sock)
                    .map_err(|e| network_err_scrubbed(config, "PRINTER_TLS", &e))?;
            }
            if conn.is_handshaking() && conn.wants_read() {
                conn.read_tls(&mut sock)
                    .map_err(|e| network_err_scrubbed(config, "PRINTER_TLS", &e))?;
                conn.process_new_packets()
                    .map_err(|e| network_err_scrubbed(config, "PRINTER_TLS", &e))?;
            }
            if !conn.is_handshaking() {
                break;
            }
            // Keep draining until handshake completes or socket stalls.
            if !conn.wants_read() && !conn.wants_write() {
                break;
            }
        }
        let certs = conn.peer_certificates().ok_or_else(|| {
            invalid(
                "PRINTER_TLS",
                "No peer certificate after Bambu TLS handshake",
            )
        })?;
        let der = certs
            .first()
            .ok_or_else(|| invalid("PRINTER_TLS", "Empty peer certificate chain from Bambu"))?;
        crate::bambu::tls::serial_from_certificate_der(der.as_ref()).ok_or_else(|| {
            invalid(
                "PRINTER_SERIAL",
                "Could not read printer serial from TLS certificate CN",
            )
        })
    }

    fn ftps_upload(&self, remote_name: &str, bytes: &[u8]) -> Result<()> {
        admit_remote_name(remote_name)?;
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err(invalid(
                "PRINTER_ARTIFACT_LIMIT",
                "Print artifact exceeds 64 MiB",
            ));
        }
        let tls = lan_client_config()
            .map_err(|e| network_err_scrubbed(&self.config, "PRINTER_TLS", &e))?;
        let mut ftp = RustlsFtpStream::connect_secure_implicit(
            (self.config.host.as_str(), self.config.ftps_port),
            RustlsConnector::from(tls),
            &self.config.host,
        )
        .map_err(|e| network_err_scrubbed(&self.config, "PRINTER_FTPS", &e))?;
        ftp.login(MQTT_USER, self.config.mqtt_password())
            .map_err(|e| network_err_scrubbed(&self.config, "PRINTER_FTPS", &e))?;
        let mut reader = Cursor::new(bytes);
        let result = ftp
            .put_file(remote_name, &mut reader)
            .map_err(|e| network_err_scrubbed(&self.config, "PRINTER_FTPS", &e));
        let _ = ftp.quit();
        result.map(|_| ())
    }

    fn mqtt_publish(&self, topic: &str, payload: &str) -> Result<()> {
        if topic.is_empty() || payload.is_empty() {
            return Err(invalid(
                "PRINTER_MQTT",
                "MQTT topic and payload must be non-empty",
            ));
        }
        run_async(self.timeout, async {
            let (client, mut eventloop) = mqtt_connect(&self.config).await?;
            wait_connack(&mut eventloop, self.timeout).await?;
            client
                .publish(topic, QoS::AtLeastOnce, false, payload)
                .await
                .map_err(|e| network_err_scrubbed(&self.config, "PRINTER_MQTT", &e))?;
            // Drain until PUBACK or timeout so the publish is on the wire.
            let deadline = tokio::time::Instant::now() + self.timeout;
            loop {
                let ev = match tokio::time::timeout_at(deadline, eventloop.poll()).await {
                    Ok(Ok(ev)) => ev,
                    Ok(Err(e)) => {
                        return Err(network_err_scrubbed(&self.config, "PRINTER_MQTT", &e));
                    }
                    Err(_) => break,
                };
                if matches!(ev, Event::Incoming(Packet::PubAck(_))) {
                    break;
                }
            }
            drop(client);
            Ok(())
        })
    }

    fn mqtt_request_report(
        &self,
        request_topic: &str,
        report_topic: &str,
        payload: &str,
    ) -> Result<String> {
        if request_topic.is_empty() || report_topic.is_empty() || payload.is_empty() {
            return Err(invalid(
                "PRINTER_MQTT",
                "MQTT topics and payload must be non-empty",
            ));
        }
        run_async(self.timeout, async {
            let (client, mut eventloop) = mqtt_connect(&self.config).await?;
            wait_connack(&mut eventloop, self.timeout).await?;
            client
                .subscribe(report_topic, QoS::AtMostOnce)
                .await
                .map_err(|e| network_err_scrubbed(&self.config, "PRINTER_MQTT", &e))?;
            wait_suback(&mut eventloop, self.timeout).await?;
            client
                .publish(request_topic, QoS::AtMostOnce, false, payload)
                .await
                .map_err(|e| network_err_scrubbed(&self.config, "PRINTER_MQTT", &e))?;
            let deadline = tokio::time::Instant::now() + self.timeout;
            loop {
                let ev = match tokio::time::timeout_at(deadline, eventloop.poll()).await {
                    Ok(Ok(ev)) => ev,
                    Ok(Err(e)) => {
                        return Err(network_err_scrubbed(&self.config, "PRINTER_MQTT", &e));
                    }
                    Err(_) => {
                        return Err(invalid(
                            "PRINTER_REPORT",
                            "Timed out waiting for printer MQTT report",
                        ));
                    }
                };
                if let Event::Incoming(Packet::Publish(p)) = ev {
                    let text = String::from_utf8_lossy(&p.payload).into_owned();
                    if text.contains("push_status") || text.contains("gcode_state") {
                        drop(client);
                        return Ok(text);
                    }
                }
            }
        })
    }
}

impl Transport for BambuLanTransport {
    fn upload(&mut self, remote_name: &str, bytes: &[u8]) -> Result<()> {
        self.ftps_upload(remote_name, bytes)
    }

    fn publish(&mut self, message: &MqttMessage) -> Result<()> {
        self.mqtt_publish(&message.topic, &message.payload)
    }

    fn request_report(
        &mut self,
        request_topic: &str,
        report_topic: &str,
        payload: &str,
    ) -> Result<String> {
        self.mqtt_request_report(request_topic, report_topic, payload)
    }
}

async fn mqtt_connect(config: &BambuLanConfig) -> Result<(AsyncClient, EventLoop)> {
    let mut opts = MqttOptions::new(unique_client_id(), &config.host, config.mqtt_port);
    opts.set_credentials(MQTT_USER, config.mqtt_password());
    opts.set_keep_alive(Duration::from_secs(30));
    let tls = lan_client_config().map_err(|e| network_err("PRINTER_TLS", &e))?;
    opts.set_transport(MqttTransport::Tls(TlsConfiguration::Rustls(tls)));
    Ok(AsyncClient::new(opts, 16))
}

async fn wait_connack(eventloop: &mut EventLoop, timeout: Duration) -> Result<()> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let ev = match tokio::time::timeout_at(deadline, eventloop.poll()).await {
            Ok(Ok(ev)) => ev,
            Ok(Err(e)) => return Err(network_err("PRINTER_MQTT", &e)),
            Err(_) => {
                return Err(invalid(
                    "PRINTER_MQTT",
                    "Timed out waiting for MQTT CONNACK",
                ));
            }
        };
        if matches!(ev, Event::Incoming(Packet::ConnAck(_))) {
            return Ok(());
        }
    }
}

async fn wait_suback(eventloop: &mut EventLoop, timeout: Duration) -> Result<()> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let ev = match tokio::time::timeout_at(deadline, eventloop.poll()).await {
            Ok(Ok(ev)) => ev,
            Ok(Err(e)) => return Err(network_err("PRINTER_MQTT", &e)),
            Err(_) => {
                return Err(invalid("PRINTER_MQTT", "Timed out waiting for MQTT SUBACK"));
            }
        };
        if matches!(ev, Event::Incoming(Packet::SubAck(_))) {
            return Ok(());
        }
    }
}

fn unique_client_id() -> String {
    static N: AtomicU64 = AtomicU64::new(0);
    format!(
        "printer-core-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    )
}

fn run_async<F, T>(timeout: Duration, fut: F) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| network_err("PRINTER_RUNTIME", &e))?;
    rt.block_on(async {
        match tokio::time::timeout(timeout.saturating_mul(2), fut).await {
            Ok(result) => result,
            Err(_) => Err(invalid(
                "PRINTER_MQTT",
                "LAN MQTT/FTPS operation exceeded overall timeout",
            )),
        }
    })
}

fn network_err(code: &'static str, err: &impl std::fmt::Display) -> crate::Error {
    invalid(code, &format!("{err}"))
}

fn network_err_scrubbed(
    config: &BambuLanConfig,
    code: &'static str,
    err: &impl std::fmt::Display,
) -> crate::Error {
    invalid(
        code,
        &scrub_secrets(&format!("{err}"), &[config.access_code.as_str()]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_client_ids_differ() {
        assert_ne!(unique_client_id(), unique_client_id());
    }

    #[test]
    fn transport_validates_config() {
        let bad = BambuLanConfig::new("", "12345678", "S1");
        assert_eq!(
            BambuLanTransport::new(bad).unwrap_err().code,
            "PRINTER_HOST"
        );
    }
}
