# Current WASM: three-part history and independent STEP acceptance

Kernel SHA256: `7525335b37e56aa8548e38b12206c175ac2e0a74a441fb6d5b478269498090b9`.
Source archive: `cad-parts-current-2026-10-08`.

The current production parser and history replayed six archived geometry chains
(mouse/keyboard bracket, flange, enclosure). Each chain passed twenty commits,
twenty Undo and Redo steps, seven cancelled asynchronous loads with rejection
of their late results, and exact final serialization/reparse. Current native
STEP export/import/re-export retained topology identity sets. Twelve STEP
files were generated in this new output directory without replacing old runs.

Independent OpenCascade gauges passed all 32 specified wall, bore and radius
measurements across all twelve exports. Evidence: `transport-report.json`,
`dimension-report.json` and `manifest.json`.

Logs: `/tmp/cad-parts-global-root-history-20261010.log`,
`/tmp/cad-parts-global-root-gauges-20261010.log`,
`/tmp/cad-parts-global-root-occt-20261010.log`.

Scope: archived geometry state replay on current source/binary plus independent
STEP measurements. This does not re-execute current mouse/keyboard commands,
prove physical crash recovery, general fillets, or complete wall coverage.
Independent solid validity/volume/bounds verification is recorded separately in
`occt-report.json` once it completes.

Independent OpenCascade shape verification completed: all twelve STEP files pass solid validity, bounding-box and volume checks (`occt-report.json`).
