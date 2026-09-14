# printer-core

LAN printer job transport. This crate does **not** slice meshes, emit G-code, or
talk to Bambu cloud. It orchestrates upload + print commands over a pluggable
[`Transport`](crate::Transport).

## Bambu Lab LAN

Observed LAN surface (not a vendor API contract):

- MQTT over TLS `:8883`, user `bblp`, password = access code
- Implicit FTPS `:990`, same credentials
- Topics `device/{serial}/request` and `device/{serial}/report`
- Control writes need **LAN Mode + Developer Mode**
- Preferred job: upload `.gcode.3mf` to FTPS root, then `print.project_file`
  with `url = ftp:///<file>` and `param = Metadata/plate_1.gcode`

Wire TLS/MQTT/FTPS sockets are not in this crate yet. Provide a `Transport`
implementation (tests use [`MockTransport`](crate::MockTransport)). Package
preview G-code with `gcode-core::{emit_3mf, package_gcode_3mf}` before submit.

```rust
use printer_core::{
    ArtifactKind, BambuLanClient, BambuLanConfig, MockTransport, PrintArtifact, PrintJob,
};

# fn main() -> printer_core::Result<()> {
let config = BambuLanConfig::new("192.168.1.50", "12345678", "01P00A000000000");
let mut client = BambuLanClient::new(config, MockTransport::default())?;
let job = PrintJob {
    printer: client.printer_id(),
    artifact: PrintArtifact {
        kind: ArtifactKind::Gcode3mf,
        bytes: b"PK..".to_vec(),
        file_name: "part.gcode.3mf".into(),
    },
    plate_gcode_path: "Metadata/plate_1.gcode".into(),
};
client.submit_job(&job)?;
# Ok(())
# }
```

Limits: 64 MiB artifact, basename-only remote names, no printer I/O claims
beyond the transport you inject.
