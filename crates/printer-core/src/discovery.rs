//! LAN printer discovery (mockable; live scanners behind `network`).

use crate::{Result, invalid};
use std::collections::VecDeque;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredPrinter {
    pub vendor: String,
    pub display_name: Option<String>,
    pub host: String,
    pub port: Option<u16>,
    pub serial: Option<String>,
    pub model: Option<String>,
    pub extras: Vec<(String, String)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveryVendor {
    Bambu,
    Snapmaker,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveryOptions {
    pub timeout: Duration,
    pub vendors: Vec<DiscoveryVendor>,
}

impl Default for DiscoveryOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(3),
            vendors: vec![DiscoveryVendor::Bambu, DiscoveryVendor::Snapmaker],
        }
    }
}

pub trait PrinterDiscovery {
    fn discover(&mut self, opts: &DiscoveryOptions) -> Result<Vec<DiscoveredPrinter>>;
}

#[derive(Clone, Debug, Default)]
pub struct MockDiscovery {
    pub results: VecDeque<Vec<DiscoveredPrinter>>,
}

impl MockDiscovery {
    pub fn push(&mut self, batch: Vec<DiscoveredPrinter>) {
        self.results.push_back(batch);
    }
}

impl PrinterDiscovery for MockDiscovery {
    fn discover(&mut self, _opts: &DiscoveryOptions) -> Result<Vec<DiscoveredPrinter>> {
        self.results
            .pop_front()
            .ok_or_else(|| invalid("PRINTER_DISCOVERY", "Mock discovery has no queued results"))
    }
}

/// Fan-out discovery across several scanners.
pub struct CompositeDiscovery {
    pub scanners: Vec<Box<dyn PrinterDiscovery + Send>>,
}

impl PrinterDiscovery for CompositeDiscovery {
    fn discover(&mut self, opts: &DiscoveryOptions) -> Result<Vec<DiscoveredPrinter>> {
        let mut out = Vec::new();
        for scanner in &mut self.scanners {
            out.extend(scanner.discover(opts)?);
        }
        Ok(out)
    }
}

#[cfg(feature = "network")]
pub mod live {
    use super::{DiscoveredPrinter, DiscoveryOptions, DiscoveryVendor, PrinterDiscovery};
    use crate::{Result, invalid};
    use std::collections::BTreeMap;
    use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
    use std::time::{Duration, Instant};

    const SSDP_ADDR: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
    const SSDP_PORT: u16 = 1900;
    const BAMBU_SSDP_ALT: u16 = 2021;
    const SNAPMAKER_PORT: u16 = 19_999;

    pub struct BambuSsdpDiscovery;

    impl PrinterDiscovery for BambuSsdpDiscovery {
        fn discover(&mut self, opts: &DiscoveryOptions) -> Result<Vec<DiscoveredPrinter>> {
            if !opts.vendors.contains(&DiscoveryVendor::Bambu) {
                return Ok(Vec::new());
            }
            let sock = bind_udp()?;
            sock.set_broadcast(true)
                .map_err(|e| invalid("PRINTER_DISCOVERY", &e.to_string()))?;
            let payload = concat!(
                "M-SEARCH * HTTP/1.1\r\n",
                "HOST: 239.255.255.250:1900\r\n",
                "MAN: \"ssdp:discover\"\r\n",
                "MX: 2\r\n",
                "ST: urn:bambulab-com:device:3dprinter:1\r\n",
                "\r\n"
            );
            let _ = sock.send_to(payload.as_bytes(), (SSDP_ADDR, SSDP_PORT));
            let _ = sock.send_to(payload.as_bytes(), (SSDP_ADDR, BAMBU_SSDP_ALT));
            // Also probe common Bambu notify port used on some firmwares.
            let _ = sock.send_to(payload.as_bytes(), (SSDP_ADDR, 1990));
            collect_responses(&sock, opts.timeout, parse_bambu_ssdp)
        }
    }

    pub struct SnapmakerUdpDiscovery;

    impl PrinterDiscovery for SnapmakerUdpDiscovery {
        fn discover(&mut self, opts: &DiscoveryOptions) -> Result<Vec<DiscoveredPrinter>> {
            if !opts.vendors.contains(&DiscoveryVendor::Snapmaker) {
                return Ok(Vec::new());
            }
            let sock = bind_udp()?;
            sock.set_broadcast(true)
                .map_err(|e| invalid("PRINTER_DISCOVERY", &e.to_string()))?;
            // Luban-style probe: empty / "discover" datagrams on the Snapmaker UDP port.
            let targets = [
                SocketAddr::from((Ipv4Addr::BROADCAST, SNAPMAKER_PORT)),
                SocketAddr::from((Ipv4Addr::new(255, 255, 255, 255), SNAPMAKER_PORT)),
            ];
            for addr in targets {
                let _ = sock.send_to(b"discover", addr);
                let _ = sock.send_to(b"", addr);
            }
            collect_responses(&sock, opts.timeout, parse_snapmaker_udp)
        }
    }

    pub fn default_live_discovery() -> super::CompositeDiscovery {
        super::CompositeDiscovery {
            scanners: vec![
                Box::new(BambuSsdpDiscovery),
                Box::new(SnapmakerUdpDiscovery),
            ],
        }
    }

    fn bind_udp() -> Result<UdpSocket> {
        let sock = UdpSocket::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)))
            .map_err(|e| invalid("PRINTER_DISCOVERY", &e.to_string()))?;
        sock.set_read_timeout(Some(Duration::from_millis(200)))
            .map_err(|e| invalid("PRINTER_DISCOVERY", &e.to_string()))?;
        Ok(sock)
    }

    fn collect_responses(
        sock: &UdpSocket,
        timeout: Duration,
        parse: fn(&[u8], SocketAddr) -> Option<DiscoveredPrinter>,
    ) -> Result<Vec<DiscoveredPrinter>> {
        let deadline = Instant::now() + timeout;
        let mut found: BTreeMap<String, DiscoveredPrinter> = BTreeMap::new();
        let mut buf = [0u8; 4096];
        while Instant::now() < deadline {
            match sock.recv_from(&mut buf) {
                Ok((n, from)) => {
                    if let Some(printer) = parse(&buf[..n], from) {
                        found
                            .entry(format!("{}:{}", printer.vendor, printer.host))
                            .or_insert(printer);
                    }
                }
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => return Err(invalid("PRINTER_DISCOVERY", &e.to_string())),
            }
        }
        Ok(found.into_values().collect())
    }

    /// Parse a Bambu SSDP notify/response datagram (also used by benches).
    pub fn parse_bambu_ssdp(bytes: &[u8], from: SocketAddr) -> Option<DiscoveredPrinter> {
        let text = std::str::from_utf8(bytes).ok()?;
        let upper = text.to_ascii_uppercase();
        if !(upper.contains("BAMBULAB")
            || upper.contains("3DPRINTER")
            || upper.contains("DEVMODEL.BAMBU.COM")
            || upper.contains("URN:BAMBULAB"))
        {
            return None;
        }
        let headers = parse_headers(text);
        let serial = headers
            .get("usn")
            .and_then(|usn| {
                usn.split(':')
                    .find(|part| part.len() >= 8 && part.chars().all(|c| c.is_ascii_alphanumeric()))
                    .map(str::to_owned)
            })
            .or_else(|| headers.get("devname.bambu.com").cloned());
        let model = headers
            .get("devmodel.bambu.com")
            .cloned()
            .or_else(|| headers.get("server").cloned());
        let host = headers
            .get("location")
            .and_then(|loc| host_from_location(loc))
            .unwrap_or_else(|| from.ip().to_string());
        Some(DiscoveredPrinter {
            vendor: "bambu".into(),
            display_name: headers.get("friendlyname.bambu.com").cloned(),
            host,
            port: None,
            serial,
            model,
            extras: headers.into_iter().collect(),
        })
    }

    /// Parse a Snapmaker UDP discovery reply (also used by benches).
    pub fn parse_snapmaker_udp(bytes: &[u8], from: SocketAddr) -> Option<DiscoveredPrinter> {
        if bytes.is_empty() {
            return None;
        }
        // Responses may be JSON or key=value / plaintext with model hints.
        if let Ok(text) = std::str::from_utf8(bytes) {
            let lower = text.to_ascii_lowercase();
            if !(lower.contains("snapmaker")
                || lower.contains("artisan")
                || lower.contains("\"name\"")
                || lower.contains("model"))
            {
                // Accept any non-empty UDP reply from the Snapmaker probe port as a candidate.
                if from.port() != SNAPMAKER_PORT && !lower.contains("token") {
                    return None;
                }
            }
            let name = jsonish_string(text, "name").or_else(|| jsonish_string(text, "model"));
            let model = jsonish_string(text, "model");
            return Some(DiscoveredPrinter {
                vendor: "snapmaker".into(),
                display_name: name,
                host: from.ip().to_string(),
                port: Some(8080),
                serial: jsonish_string(text, "id").or_else(|| jsonish_string(text, "serial")),
                model,
                extras: vec![("raw".into(), text.chars().take(200).collect())],
            });
        }
        Some(DiscoveredPrinter {
            vendor: "snapmaker".into(),
            display_name: None,
            host: from.ip().to_string(),
            port: Some(8080),
            serial: None,
            model: None,
            extras: Vec::new(),
        })
    }

    fn parse_headers(text: &str) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        for line in text.lines().skip(1) {
            if let Some((k, v)) = line.split_once(':') {
                map.insert(k.trim().to_ascii_lowercase(), v.trim().to_owned());
            }
        }
        map
    }

    fn host_from_location(location: &str) -> Option<String> {
        let rest = location
            .strip_prefix("http://")
            .or_else(|| location.strip_prefix("https://"))?;
        let host = rest.split('/').next()?.split(':').next()?.trim();
        if host.is_empty() {
            None
        } else {
            Some(host.to_owned())
        }
    }

    fn jsonish_string(text: &str, key: &str) -> Option<String> {
        let needle = format!("\"{key}\"");
        let start = text.find(&needle)? + needle.len();
        let rest = text[start..].trim_start().strip_prefix(':')?.trim_start();
        let rest = rest.strip_prefix('"')?;
        let end = rest.find('"')?;
        Some(rest[..end].to_owned())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::net::SocketAddrV4;

        #[test]
        fn parses_bambu_ssdp_notify() {
            let msg = concat!(
                "HTTP/1.1 200 OK\r\n",
                "USN: 01P00A000000001\r\n",
                "DevModel.bambu.com: N1\r\n",
                "Location: http://192.168.1.50:80/\r\n",
                "\r\n"
            );
            let from = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 1, 50), 1900));
            let p = parse_bambu_ssdp(msg.as_bytes(), from).unwrap();
            assert_eq!(p.vendor, "bambu");
            assert_eq!(p.host, "192.168.1.50");
            assert_eq!(p.serial.as_deref(), Some("01P00A000000001"));
            assert_eq!(p.model.as_deref(), Some("N1"));
        }

        #[test]
        fn parses_snapmaker_json() {
            let msg = r#"{"name":"My Artisan","model":"Artisan","id":"SM123"}"#;
            let from = SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::new(10, 0, 0, 8),
                SNAPMAKER_PORT,
            ));
            let p = parse_snapmaker_udp(msg.as_bytes(), from).unwrap();
            assert_eq!(p.vendor, "snapmaker");
            assert_eq!(p.display_name.as_deref(), Some("My Artisan"));
            assert_eq!(p.port, Some(8080));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_discovery_returns_queued() {
        let mut mock = MockDiscovery::default();
        mock.push(vec![DiscoveredPrinter {
            vendor: "bambu".into(),
            display_name: None,
            host: "192.168.1.1".into(),
            port: None,
            serial: Some("S1".into()),
            model: None,
            extras: Vec::new(),
        }]);
        let found = mock.discover(&DiscoveryOptions::default()).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].host, "192.168.1.1");
    }
}
