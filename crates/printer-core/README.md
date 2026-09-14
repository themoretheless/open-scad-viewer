# printer-core

Multi-vendor **LAN** printer job transport. This crate does **not** slice meshes,
emit G-code, or talk to vendor clouds. Hosts implement [`PrinterBackend`].

| Backend | Artifact | Wire |
| --- | --- | --- |
| Bambu Lab | `.gcode.3mf` | FTPS `:990` + MQTT/TLS `:8883` |
| Moonraker | `.gcode` | HTTP multipart upload + print start |
| OctoPrint | `.gcode` | HTTP `/api/files/local` + `/api/job` |
| PrusaLink | `.gcode` | HTTP `PUT /api/v1/files/{storage}/{path}` |
| Creality | `.gcode` | Moonraker-compatible HTTP (often `:7125`) |
| Snapmaker | `.gcode` | HTTP `:8080` `/api/v1/connect|upload|start_print` |

## Shared API

`PrinterBackend`: `submit_job` → `SubmitOutcome`, plus `pause` / `resume` /
`stop` / `status`. Package G-code with `gcode-core` before Bambu submit.

## Bambu Lab LAN

Requires **LAN Mode + Developer Mode**. Upload to FTPS root, then
`print.project_file` with `url = ftp:///<file>`, `param = Metadata/plate_1.gcode`,
and uppercase MD5 of the uploaded bytes. With `verify_start` (default), the
client polls `push_status` for prepare/running or `print_error`.

```rust
use printer_core::{
    ArtifactKind, BambuLanBackend, BambuLanConfig, MockTransport, PrintArtifact, PrintJob,
    PrinterBackend,
};

# fn main() -> printer_core::Result<()> {
let config = BambuLanConfig::new("192.168.1.50", "12345678", "01P00A000000000");
let mut client = BambuLanBackend::new(config, MockTransport::default())?;
let job = PrintJob {
    printer: client.id(),
    artifact: PrintArtifact {
        kind: ArtifactKind::Gcode3mf,
        bytes: b"PK..".to_vec(),
        file_name: "part.gcode.3mf".into(),
    },
    plate_gcode_path: "Metadata/plate_1.gcode".into(),
    verify_start: false,
};
client.submit_job(&job)?;
# Ok(())
# }
```

## Moonraker / OctoPrint / Creality / PrusaLink

```rust
use printer_core::{
    CrealityConfig, MockHttpTransport, MoonrakerBackend, MoonrakerConfig, PrinterBackend,
    PrusaLinkConfig,
};

# fn main() -> printer_core::Result<()> {
let moon = MoonrakerBackend::new(
    MoonrakerConfig::new("http://192.168.1.20:7125"),
    MockHttpTransport::default(),
)?;
let _ = moon.id();
let _ = CrealityConfig::new("http://192.168.1.50:7125");
let _ = PrusaLinkConfig::new("http://192.168.1.40", "api-key");
# Ok(())
# }
```

Creality LAN here means **Moonraker on the printer** (rooted K1/K2 / Helper
Script). Stock Creality proprietary upload+websocket UI is not implemented.

PrusaLink uses API-key auth and `Print-After-Upload: ?1` on PUT upload.

## Live sockets (`network` feature)

```toml
printer-core = { path = "../printer-core", features = ["network"] }
```

- `BambuLanBackend::connect_lan` / `connect_lan_discover`
- `MoonrakerBackend::connect` / `OctoPrintBackend::connect`
- `PrusaLinkBackend::connect` / `CrealityBackend::connect`
- `SnapmakerBackend::connect`

TLS for Bambu accepts the printer's self-signed X.509 v1 certificate. Access
codes, API keys, and Snapmaker tokens are scrubbed from error messages.

## Discovery

Default builds expose [`PrinterDiscovery`] + [`MockDiscovery`]. With `network`:

- [`BambuSsdpDiscovery`] — SSDP M-SEARCH / notify parse (host, serial, model)
- [`SnapmakerUdpDiscovery`] — Luban-style UDP broadcast probe
- [`default_live_discovery`] — composite of both

Discover fills config; it does **not** auto-connect. Moonraker / OctoPrint /
Prusa / Creality stay **manual URL** (no mDNS in this crate). CLI/UI entry:
[`printer-cli`](../printer-cli).

## Benchmarks (`rbench`)

```sh
cd crates
cargo build --release -p gcode-core --example bench_print_export
cargo build --release -p printer-core --example bench_printer_lan --features network
cd ..
cargo rbench run --program crates/target/release/examples/bench_print_export --protocol \
  --repetitions 6 -o .rbench/print-export -- --profile quick --json
cargo rbench run --program crates/target/release/examples/bench_printer_lan --protocol \
  --repetitions 6 -o .rbench/printer-lan -- --profile quick --json
cargo rbench report .rbench --title "Print pipeline" -o .rbench/print-pipeline.html
cargo rbench serve .rbench --port 8787
```

## Limits / non-goals

64 MiB artifact, basename-only remote names, no cloud, no AMS UI, no camera,
no mDNS for HTTP hosts, no browser/WASM FTPS/MQTT, no Prusa Digest auth, no stock
Creality websocket print start, no Snapmaker on-screen pairing UI.
