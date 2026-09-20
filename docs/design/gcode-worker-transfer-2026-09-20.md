# Transferable G-code worker results

The [roundtrip baseline](gcode-worker-roundtrip-2026-09-20.md) showed a reproducible
large-document cost after the packed WASM boundary improvement. This change
transfers an owned Float64Array instead of structured-cloning the move objects.

## Contract and ownership

- Requests opt into `responseFormat: 'f64-moves-v1'`. Requests without it retain
  their object response; unknown formats fail before warming the kernel.
- Seven doubles encode x/y/z, extrusion, feedrate, layer index and extrusion flag.
  The row decoder is shared with the packed WASM boundary. Double precision and
  signed zero are preserved; flags, layer indices and finite values are checked.
- The receiving client admits only full-span, non-shared Float64Array buffers
  within the existing 100,000-move bound, then restores ordinary move objects.
  Malformed responses discard the worker and allow retry. Errors, IDs, deadlines,
  cancellation, idle cleanup and public document metadata remain unchanged.
- Only a newly allocated output buffer is transferred. The scene's mesh buffers
  are still cloned on input, never detached. Parse, slice and print-job exports
  use the same response path; the 3MF attachment is preserved.

This is not a zero-allocation or end-to-end zero-copy pipeline: the worker still
materializes native rows into objects, packs the output buffer, and the client
reconstructs objects. Eliminating the intermediate worker objects is a separate
experiment, not a prerequisite for this measured transport improvement.

## Measurements

Production baseline source: `abe272ab`. Node v22.23.2, macOS arm64. The unchanged
geometry WASM SHA-256 is
`909b94a4b895db447a184bcc6cfb544a226b51589b94b124424da6670eb0b8ca`.

Run `node --import tsx benchmarks/gcode-worker-roundtrip.mts` for the panel client
and actual worker entrypoint through Node worker_threads. Set
`GCODE_WORKER_TRANSPORT=legacy` for a same-source, same-WASM cloning control.
Four fresh-process campaigns ran in transfer/legacy/legacy/transfer order, with
10 warmups and 31 alternating direct/worker pairs per fixture. No tests or builds
ran alongside measurements. Full initial results and cross-campaign result
hashes matched; per-sample move count, time and final move were checked.

| Extruding moves | Legacy A/B p50, ms | Transfer A/B p50, ms |
| --- | --- | --- |
| 100 | 0.1679 / 0.1653 | 0.1377 / 0.1385 |
| 10,000 | 8.7625 / 8.7669 | 4.6118 / 4.6664 |
| 80,000 | 75.4112 / 75.8398 | 38.9976 / 39.4970 |

The large case is about 48% faster in this Node client roundtrip. There is one
additional positioning move; its numeric output buffer is 4,480,056 bytes.

For the browser control, build with `node_modules/.bin/vite build`, then run
`GCODE_WORKER_BENCH=1 node scripts/check-geometry-worker-startup-browser.mjs gcode`.
Set `CHROMIUM_EXECUTABLE` when using an installed browser. Two fresh Chrome
156.0.8063.3 runs used the production worker and 31 alternating legacy/transfer
pairs after 10 warmups. The 80,000-move p50 was **55.5 / 55.8 ms legacy** versus
**32.6 / 32.4 ms transfer**. p95 was 63.7 / 61.6 versus 37.3 / 37.2 ms.
This includes independent object reconstruction, not the production panel client
or rendering. The probe verifies actual sender-buffer detachment, complete
legacy/transfer result equality, verified asynchronous startup, reuse, and
cancellation without late responses. SVG startup/cancellation also passed.

## Validation and limits

55 focused tests passed. Vue, MCP and standalone benchmark/test typechecks passed.
Full Vitest: 3,416 passed, nine known qualification artifact-binding failures in
four files; the suite is not globally green and archives were not rewritten.
Production build and dist verification passed: 92 artifacts, 15,720,446 bytes,
up 2,113 bytes from the preceding build; WASM bytes are unchanged.
MainModelingTools is 100,285 bytes and now has an explicit 102,000-byte budget.
The general 100,000-byte named-budget threshold remains unchanged.
