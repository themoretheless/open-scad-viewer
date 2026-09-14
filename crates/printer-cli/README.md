# printer-cli

Native CLI and **loopback companion** for [`printer-core`](../printer-core)
LAN submit / discovery. The Vue app must not open FTPS/MQTT/raw printer
sockets; this binary is the bridge (`serve` binds `127.0.0.1` only).

```bash
cargo run -p printer-cli -- discover
cargo run -p printer-cli -- send --vendor moonraker --host http://192.168.1.20:7125 --file part.gcode
cargo run -p printer-cli -- serve --port 17890
```

## Commands

| Command | Behavior |
| --- | --- |
| `discover` | Bambu SSDP + Snapmaker UDP; `--format json\|table` |
| `send` | Artifact path → vendor `submit_job` (kind from extension) |
| `status` / `pause` / `resume` / `stop` | Control a connected printer |
| `serve` | HTTP JSON API on `127.0.0.1` for the Vue panel |

`send` / control flags: `--vendor bambu|moonraker|octoprint|prusa|creality|snapmaker`
plus `--host`, and credentials as required (`--access-code`, `--serial`,
`--api-key`, `--token`).

Artifact inference: `.gcode.3mf` / `.3mf` → Bambu package; `.gcode` / `.gco` →
HTTP hosts.

## Loopback API (`serve`)

Default `http://127.0.0.1:17890`:

- `GET /health` → `{ "ok": true }`
- `GET /discover` → `{ "printers": [...] }`
- `POST /send` → `{ vendor, config, fileName, bytesBase64 }`
- `POST /control` → `{ vendor, config, action }` with `pause|resume|stop|status`

Secrets are scrubbed from error strings. CORS allows the local Vite origin.

## Benchmarks

Print-export + LAN discovery microbenchmarks live under `gcode-core` /
`printer-core` examples (`bench_print_export`, `bench_printer_lan`). Aggregate
HTML: `.rbench/print-pipeline.html`. Live UI: `cargo rbench serve .rbench`.
