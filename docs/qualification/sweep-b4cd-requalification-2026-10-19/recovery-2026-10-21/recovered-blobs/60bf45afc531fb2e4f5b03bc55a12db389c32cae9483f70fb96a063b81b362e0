//! Printer LAN microbenchmarks via rbench (release + `network`).
//!
//! ```sh
//! cargo run --release -p printer-core --example bench_printer_lan --features network -- --profile quick
//! ```
use printer_core::{
    DiscoveryOptions, MockDiscovery, PrinterDiscovery, parse_bambu_ssdp, parse_snapmaker_udp,
    scrub_secrets,
};
use rbench::{DropPolicy, Suite};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

const SSDP: &str = concat!(
    "HTTP/1.1 200 OK\r\n",
    "USN: 01P00A000000001\r\n",
    "DevModel.bambu.com: N1\r\n",
    "Location: http://192.168.1.50:80/\r\n",
    "\r\n"
);
const SNAP: &str = r#"{"name":"My Artisan","model":"Artisan","id":"SM123"}"#;

fn main() -> rbench::Result<()> {
    let mut suite = Suite::new("printer-lan");
    let from_bambu = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 1, 50), 1900));
    let from_snap = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 8), 19_999));

    suite
        .bench_with_input(
            "discovery/parse_bambu_ssdp",
            || SSDP.to_owned(),
            |msg| {
                parse_bambu_ssdp(msg.as_bytes(), from_bambu)
                    .map(|p| p.host.len())
                    .unwrap_or(0)
            },
            DropPolicy::InsideTiming,
        )
        .parameter("bytes", SSDP.len());
    suite
        .bench_with_input(
            "discovery/parse_snapmaker_udp",
            || SNAP.to_owned(),
            |msg| {
                parse_snapmaker_udp(msg.as_bytes(), from_snap)
                    .map(|p| p.host.len())
                    .unwrap_or(0)
            },
            DropPolicy::InsideTiming,
        )
        .parameter("bytes", SNAP.len());
    suite
        .bench_with_input(
            "scrub/secrets",
            || {
                (
                    "upload failed access=SECRETCODE apikey=KEY123TOKEN".to_owned(),
                    vec!["SECRETCODE".to_owned(), "KEY123TOKEN".to_owned()],
                )
            },
            |(msg, secrets)| {
                let refs: Vec<&str> = secrets.iter().map(String::as_str).collect();
                scrub_secrets(msg, &refs).len()
            },
            DropPolicy::InsideTiming,
        )
        .parameter("secrets", 2);
    suite
        .bench_with_input(
            "discovery/mock_batch",
            || {
                let mut mock = MockDiscovery::default();
                mock.push(vec![printer_core::DiscoveredPrinter {
                    vendor: "bambu".into(),
                    display_name: None,
                    host: "192.168.1.1".into(),
                    port: None,
                    serial: Some("S1".into()),
                    model: None,
                    extras: Vec::new(),
                }]);
                mock
            },
            |mock| mock.discover(&DiscoveryOptions::default()).unwrap().len(),
            DropPolicy::InsideTiming,
        )
        .parameter("printers", 1);
    suite.main()
}
