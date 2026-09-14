# Printer LAN transport

Library contract for sending a ready print artifact to a LAN host. Not a slicer
and not G-code generation. Implementation: [`crates/printer-core`](../../crates/printer-core).

## Backends

| Vendor | Artifact | Observed wire |
| --- | --- | --- |
| Bambu Lab | `.gcode.3mf` | Implicit FTPS `:990` + MQTT/TLS `:8883`, user `bblp`, password = access code |
| Moonraker | `.gcode` | `POST /server/files/upload` then `POST /printer/print/start` |
| OctoPrint | `.gcode` | `POST /api/files/local` (`select`+`print`) ; job control via `/api/job` |
| PrusaLink | `.gcode` | `PUT /api/v1/files/{storage}/{path}` with `Print-After-Upload` / `Overwrite`; job via `/api/v1/job/{id}` |
| Creality | `.gcode` | Same as Moonraker (rooted Klipper Creality / Helper Script, often `:7125`) |
| Snapmaker | `.gcode` | `POST /api/v1/connect`, multipart `POST /api/v1/upload`, `POST /api/v1/start_print` (token query, port `8080`) |

These are **observed** integrations, not vendor-guaranteed API contracts.

## Bambu notes

- Control writes need LAN Mode **and** Developer Mode.
- Prefer FTPS **root** upload (not `cache/`) with `url = ftp:///<file>`.
- `project_file.param` = `Metadata/plate_N.gcode`.
- `md5` = uppercase hex of uploaded bytes (empty skips check on some firmware).
- Full status snapshots use `push_status` with `msg == 0`; deltas also use
  `push_status` with `msg == 1`.
- TLS cert is self-signed X.509 v1 (CN = serial); clients skip verification.

## PrusaLink notes

- Auth in this crate: `X-Api-Key` only (HTTP Digest not implemented).
- Preferred upload is PUT octet-stream with RFC8941 booleans `?0` / `?1`.
- Pause/resume/stop need a job id from `/api/v1/job` or status telemetry.

## Creality notes

- Supported path: Moonraker HTTP API (stock rooted / Helper Script K1 family).
- Unsupported: stock Creality web UI `POST /upload` + websocket `opGcodeFile`.

## Snapmaker notes

- Token from Luban `machine.json` (or a previously approved pairing).
- Always `connect` before upload/status/control; HTTP 204 means waiting for
  on-controller confirmation.
- Upload is multipart `file=`; start is a separate `start_print` call.
- UDP discovery / first-time pairing UI are out of scope.

## Shared surface

`PrinterBackend`: `submit_job` → `SubmitOutcome`, plus `pause` / `resume` /
`stop` / `status` (`JobState` + vendor string). Feature `network` enables live
sockets; default builds use mock transports.

## Non-goals

Cloud accounts, AMS mapping UI, camera streams, SSDP/UDP discovery, browser/WASM
send, CLI host, Prusa Digest auth, stock Creality websocket start, Snapmaker
on-screen pairing flow, claiming slicer or firmware parity.
