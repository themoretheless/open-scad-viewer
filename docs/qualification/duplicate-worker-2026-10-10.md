# Duplicate command in worker

The modeler's Duplicate command now sends a `sceneEdit: duplicate` request to
the existing worker. Rust still performs the selected geometry transform.
The worker handles identity remapping and resolves copied linked instances,
then the existing authoritative document parser validates the returned result.
The UI commits it through asynchronous history admission and rejects responses
whose generation was cancelled by Escape, a document change or selection change.
Copy identities must be distinct, complete, and unoccupied in the document.

Validation:
- Source plus linked-instance duplication preserves originals and maps the
  copied instance to the copied source; Undo restores the original document.
- Escape discards a delayed duplicate result; a fresh duplicate then succeeds.
- Existing asynchronous transform publication and input invalidation checks pass.
- Missing, repeated, nonexistent and occupied identity requests are rejected.
- Vue TypeScript check and git diff whitespace check pass.

Logs: `/tmp/duplicate-worker-controls-20261010.log` (4 selected UI tests),
`/tmp/duplicate-worker-identities-20261010.log` (1 service test),
`/tmp/duplicate-worker-typecheck-20261010.log`.

UI tests use the worker test adapter. A real-browser duplicate latency comparison
and full command matrix are still required. No latency speedup is claimed here.

Real worker validation also passed via `mainSolidWorkerRealBoundary.test.ts`:
source and instance copies, preserved original buffers, remapped source identity,
refusal of occupied copy identities and successful subsequent request. Log:
`/tmp/duplicate-real-worker-20261010.log` (1 selected test, 3.99 seconds).
This executes the production worker through Node worker_threads and the public
WASM runtime; it does not substitute for a browser latency measurement.

Recovery and registry follow-up: 3 duplicate UI tests pass, including worker
failure followed by retry and Undo. The full pre-follow-up UI run completed
with 326 passes and two stale command-count assertions (expected 96, actual 97).
The additional command is `profile-bridge-check`; the registry now asserts its
presence explicitly. Both RU/EN registry tests plus the three duplicate tests
pass (5 selected tests). Logs: `/tmp/duplicate-worker-recovery-fixed-20261010.log`,
`/tmp/duplicate-worker-ui-full-20261010.log`,
`/tmp/duplicate-and-registry-final-20261010.log`.

Actual browser worker: the isolated CAD draft qualification fixture also sends
the actual saved enclosure through `createSolidPreviewWorker()` using the
production `sceneEdit: duplicate` request. The worker returned the new copy,
the original input serialized identically and the saved revision remained
unchanged. Console errors: none. Cold request including worker startup,
geometry transport and authoritative output parsing: 345.4 ms under concurrent
native work. This is not a measured latency speedup or a complete main-modeler
interaction. Screenshot: `/tmp/cad-browser-worker-duplicate-20261010.png`.
Vue TypeScript check passed after the fixture addition.

Known worker protocol/transport/timeout failures in duplicate and numeric
transform now use the existing localized calculation-failure formatter.
RU/EN protocol-failure tests confirm geometry is unchanged and the raw protocol
message is replaced by actionable localized text. Six selected duplicate and
transform tests pass, along with Vue TypeScript. Logs:
`/tmp/duplicate-worker-locales-fixed-20261010.log`,
`/tmp/duplicate-worker-locales-typecheck-20261010.log`.

Mixed body/NURBS selections now disable Duplicate with an explicit supported
input message, instead of permitting a partial copy or an identity refusal.
A dedicated mixed-selection test passes. Delayed duplication is also tested
against selection changes, in addition to Escape: both discard the stale result
and allow a fresh duplicate followed by Undo. Three selected tests pass; log:
`/tmp/duplicate-selection-cancel-20261010.log`.

Keyboard and active-command follow-up: Ctrl/Meta+D now uses the same input
eligibility as the command menu. An active extrusion blocks both entry points;
Escape cancels it and allows a fresh duplicate. Ten selected tests passed in
`/tmp/duplicate-active-command-20261010.log`. Vue TypeScript also passed for
the current guarded implementation (`/tmp/duplicate-current-typecheck-20261010.log`).
The full current UI suite completed: 337 passed, zero failed, 146.55 seconds.
Log: `/tmp/duplicate-current-ui-full-20261010.log`. This is the component suite;
it does not qualify the complete browser P0 command matrix.

Real-worker late-result follow-up: a successful Duplicate reply is captured
before delivery, then the client is cancelled. Its pending history commit
rejects with `CAD_CANCELLED` and adds no Undo state. A replacement runs in a
new worker; the captured old reply is delivered both during and after it.
Only the replacement identity enters history, with one Undo state, exact
Undo/Redo and serialized reload. Both real Duplicate tests pass (3.89 seconds
test time): `/tmp/duplicate-real-worker-cancel-20261010.log`.
Published kernel SHA during this run:
`7525335b37e56aa8548e38b12206c175ac2e0a74a441fb6d5b478269498090b9`.
This validates worker/history cancellation for the existing box path; it does
not qualify the new native remainder cache or all browser command scenarios.

Actual main-modeler browser acceptance now covers a cube on the isolated
origin `http://127.0.0.1:5295`: create by mouse, Duplicate through the command
palette by mouse, Control+D, mouse Undo, Control+Shift+Z Redo, then two browser
reloads and JSON downloads. The two downloaded documents match byte for byte;
three identities and all three 20 × 20 × 20 mm B-rep vertex bounds persist.
Copies occupy X/Y ranges [0,20] and [10,30], with the original at [-10,10].
Console errors: none. The visible Solid scene rendered throughout. The hidden
OpenSCAD renderer reported WebGPU unavailable, so this is not a WebGPU renderer
qualification. Browser Meta+D, cancellation, error injection and Retry remain
open. The user origin on port 5195 was untouched; the temporary tab and server
were closed after the check.

Evidence: `cad-command-coverage-global-root-2026-10-10/duplicate-browser-report.json`
and `duplicate-browser-recovered.json`; screenshot
`/tmp/duplicate-browser-recovered-20261010.png`.
