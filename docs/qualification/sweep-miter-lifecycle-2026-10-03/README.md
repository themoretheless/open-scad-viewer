# Retained miter UI lifecycle qualification — 2026-10-03

The isolated built snapshot passed **34 UI scenarios (17 source modes × 1440/600 px) and 302 assertions**. The matrix checks build success, held-dispatch build cancellation, held-dispatch Solid cancellation, source replacement, publication, CPU SVG visibility, reload restoration, invalid-path refusal, and bounded-work refusal where applicable. Cancellation proves termination before dispatch; it does not measure interruption latency during native execution.

`miter-hollow-body.r` is an expected unproved spatial Solid refusal, not a successful Solid publication. Its empty project also survives reload and subsequent invalid-path refusal. The corrected spatial variant succeeds. An existing body survives invalid-path and resource-budget refusals unchanged. Positive cases compare restored native BRep values directly, rather than just body counts. Browser qualification used Chromium 151.0.7922.34 with CPU SVG fallback; WebGPU hardware remains unqualified.

## Product fixes

- A synchronous source/file/group-source watcher cancels any superseded exact Solid request.
- The source button has its own toolbar column; narrow layout reserves its position explicitly and clips the filename within available space.
- A ResizeObserver measures the actual toolbar height. Solid and the source drawer begin below that height on both layouts; observers disconnect on unmount.

`ui-matrix.json` and `source-dist-manifest.json` identify the tested source snapshot and every distribution file. The sample wide/narrow screenshots show the corrected combined frame/affine mode. Complete download and screenshot artifacts remain in `/tmp/sweep-ui-delivered`; the checked-in matrix records BRep geometry hashes. The source snapshot is `/private/tmp/open-scad-viewer-sweep-step-2026-10-03`.

## Independent STEP verification

The `step/` directory preserves freshly exported requests, STEP files and OpenCascade report for **22 cases**. OCP 8.0.1.0 passed topology, geometry/control net, edge ownership/orientation and analytic volume checks. Fixture import is not a general continuous sweep error, global containment or smoothness proof. STEP was freshly re-exported from the delivered WASM snapshot with the 1000000 per-face correspondence budget. `step-runtime-manifest.json` identifies the runtime and exporter/source hashes.

## Remaining proof scope

The broad sweep goal is still open. Authored frames and orientation guides are mutually exclusive. Full source-to-filled-boundary `continuousBound`, all-mode global embedding, moving-frame and closed/multispan smoothness, and a wider independent STEP matrix still require separate work. The new UI matrix qualifies these finite retained-miter cases; it does not promote those geometry flags or cover every progressive surface mode.

Loft PR #28 was merged as `939f87e41b56de0c5e5b16af804a399ae374cf24`; the broader sweep changes in this working tree are not yet published.
