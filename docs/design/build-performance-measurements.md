# Build measurements: first optimization step

The collapsed **Build measurements / Замеры сборки** panel below the editor
reports the latest successful scene publication. JSON export retains the last
60 publications of the current page session. It contains no source, file names,
geometry, user agent, or device identity and is downloaded only on request.
Reports are not persisted or sent to a server.

## Boundaries

- Compiler phase timings and Worker duration come from the existing validated
  Worker protocol. They are durations, not cross-realm absolute timestamps.
- Host elapsed time starts on entry to a non-coalesced build request and ends
  at the coordinator's validated publication callback. It includes source
  hashing, queueing, Worker startup, execution, transport and validation.
  It must not be described as isolated transfer time.
- Publication CPU time includes reconciliation, renderer scene ingestion and
  application of scene/inspection state. It excludes deferred Vue layout and
  GPU execution.
- A renderer publication token is consumed immediately after its first
  successful `GPUQueue.submit`. Hidden/undrawable scenes, failed submissions
  and scenes replaced before drawing do not acquire a submission measurement.
  This is neither GPU completion nor physical display presentation.
- Edit latency starts when the source watcher advances the build generation.
  It includes auto-build delay and is absent for a revision without an observed
  edit. Preview/full jobs keep separate samples. An equivalent preview may be
  exportable at full quality while its sample still says `preview`, the actual
  requested build quality.
- Transfer bytes count unique mesh backing buffers. Vertex/index upload bytes
  and allocation counts cover only those two GPU buffer classes, excluding
  uniforms, edges and overlays. Reused entity counts describe entity uses, not
  the number of unique geometry assets.
- Counters count non-coalesced requests, superseded jobs, successful Worker
  creations and hard supersession restarts. Disposal is not a restart. Worker
  creation after an application-level error is reflected in Worker starts.

These durations overlap; do not sum them. A missing boundary is `null` in the
report and a dash in the panel. Snapshots are detached copies. Reporting does
not change build scheduling, preview promotion, or persistence contracts.

## Validation and next step

Unit coverage exercises main/Worker timing separation, request coalescing,
supersession counters, history limits, stale frame rejection, failed/hidden
frame submission and retained geometry upload counts. The panel was also
inspected with a live WebGPU build and a comment-only edit.

For broader tuning, collect comparable reports for a small parametric part, expensive CSG,
repeated entities and a large supported mesh. Compare cold startup separately
from warm edits. The first adaptive scheduling pass is documented in [auto-build-and-presets.md](auto-build-and-presets.md). Use broader observations to tune it;
do not adopt wall-clock CI thresholds or claim speedups from a single run.
