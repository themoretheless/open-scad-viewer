# Solid Cold-Start STEP Race

## Evidence

[CI run 35494336311](https://github.com/themoretheless/open-scad-viewer/actions/runs/35494336311)
on `31b830cc` failed in `browser-workbench-indexeddb`: the saved scene before
STEP import was null. The browser had no page errors; the later imported
assembly was visible. The script dereferenced `before.bodies`, obscuring the
missing initial Box. This is separate from the nine historical qualification
binding failures in Vitest. Logs and downloaded diagnostics are retained in
`/private/tmp/osv-step-v10-ci-35494336311`.

The ordinary local production-browser run on Chrome 156 passed, but explicitly
holding the large WASM WebCrypto digest reproduced an enabled Box button
while the geometry kernel was unavailable. Native primitive creation is
synchronous; the main-thread bridge refuses calls before asynchronous warm-up.
Its handled UI error was subsequently cleared by STEP import, and the initial
scene was never committed. The controlled reproduction establishes the race;
the original CI artifact alone does not prove its timing or exact first error.

## Changes

- Primitive buttons and matching command-palette entries wait for kernel
  readiness. Warm-up failures reach the existing error area instead of being
  silently swallowed.
- The asynchronous STEP service awaits the shared warm-up before import or
  reading/validating the retained model for export. It snapshots the scene
  before yielding, preserving the input against concurrent caller mutation.
- The browser scenario asserts that the initial Box exists before import,
  captures handled alerts on failure, and supports a deterministic digest gate.
  The V10 qualification runner enables this gate. After reload, export starts
  while warm-up is held and completes after release.

The delayed reload also exposed an independent export race: IndexedDB model
validation called the native kernel before readiness. The async-service fix
handles this without sleeps or increasing timeouts. No synchronous geometry
contract, binary artifact, archived qualification evidence or runtime identity
was changed.

## Verification

- Before the UI fix, the delayed probe failed its disabled-Box assertion.
- Before the service fix, delayed reload/export timed out, with the handled
  error requiring `warmGeometryKernel()` captured in diagnostics.
- After both fixes, the full production-browser scenario passes: existing Box
  preserved, three STEP occurrences imported, original bytes downloaded,
  invalid import atomic, reload/export preserved, and download reimported in a
  fresh context. Desktop/mobile screenshots are in
  `/private/tmp/osv-step-v10-delayed-fixed`.
- 40 focused UI/STEP tests and four qualification-runner tests pass. New
  service tests cover pending warm-up, input mutation, warm-up failure/retry
  and absence of premature IndexedDB/native access.
- Vue typecheck, Vite build and `verify-dist` pass (92 artifacts, 15,714,928
  total bytes). No latency improvement is claimed.
- MCP typecheck passes. Full Vitest: 3408 passed, nine existing qualification
  binding failures across four suites. Ordinary and repeated delayed browser
  runs pass; the final delayed report is in `/private/tmp/osv-step-v10-delayed-final`.

This is local production-browser evidence, not a successful rerun of the
complete remote STEP qualification or resolution of runtime-manifest drift.

## Remote Follow-Up

[Run 35495407612](https://github.com/themoretheless/open-scad-viewer/actions/runs/35495407612)
subsequently completed successfully on exact commit
`be0f0a36b2c84475ee9be97a2994dd36126c5bc6`. Its downloaded browser report
confirms `delayedKernel: true`, all roundtrip/preservation assertions, three
occurrences and one definition under Linux Chrome 151.0.7922.34. Diagnostics
are retained in `/private/tmp/osv-step-v10-ci-be0f0a36`.
This establishes the remote STEP V10 result for that commit, not qualification
of later artifacts or resolution of the separate runtime-manifest mismatch.
The same commit's Node 20 CI job still failed the nine known archive-binding
tests (3403 passed, five skipped).
