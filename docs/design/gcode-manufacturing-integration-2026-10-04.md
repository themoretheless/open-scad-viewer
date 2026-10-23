# G-code and laser integration in the redesigned Solid workspace

The scene panel now has a Manufacturing entry. It passes the selected body's current triangle mesh to the existing G-code worker and laser section adapter. Selection, document changes and restoration invalidate results. Exporting uses the mesh representation; this does not establish an exact B-rep slicing guarantee.

## Firmware state

The additive `analyze_firmware` API requires an explicit firmware flavor. Inspection enables it only for a recognized declared flavor. It reports source lines, active tool, per-tool targets and standby targets, bed/chamber targets, wait policy and retract offset. A heater selector does not change the motion tool. Unknown targets and initial retract state remain unknown. G10/G11 do not change logical E or the preview's positive extrusion accounting.

Marlin M207 S supplies the retraction length; repeated retract commands are idempotent. RepRapFirmware G10 P/S/R sets tool temperatures and is distinct from parameter-free retraction. M109/M190/M191 distinguish S from R waits; a wait with no target retains the previous target, including unknown. The analysis does not estimate wait duration or measured temperatures.

Limits: 4 MiB input, 1024-byte lines, 100000 lines, 256 sparse tools and 8192 state events. Unknown firmware extensions, M208/M209, non-Celsius temperature state, swap retract and unsupported G10 forms are reported as unverified. Physical recovery surplus, firmware macros and actual machine motion are not emulated. UI limits displayed events to the first 200.

Primary command semantics: [Marlin M109](https://marlinfw.org/docs/gcode/M109.html), [Marlin command index](https://marlinfw.org/meta/gcode/). This is a requested-state analysis, not printer execution.

## Configured export

The strict print-job 1 API remains available. Configured jobs use a separate header and can select millimeters/inches, absolute/relative XYZ, absolute/relative E, a bounded verbatim startup template, travel Z hop and a Marlin chamber target. The startup template precedes normal machine initialization. Units and coordinate/extrusion modes are then set explicitly; E mode is reasserted after each XYZ mode change. The first unknown-axis positioning move is absolute, with subsequent moves using requested deltas. Coordinates, extrusion and feedrate all use the selected units.

Z hop lifts before a rapid and lowers before extrusion. A zero chamber field disables chamber heating. Positive chamber targets use M141/M191 for Marlin; unsupported targets return an error. Klipper inch output is refused. Native profile validation and source-flavor checks apply before conversion.

Templates are limited to 64 KiB and 1024 bytes per line and reject NUL. They are emitted verbatim, not expanded as macros or independently validated against a controller. Custom work offsets, tool macros and physical template effects remain outside the preview contract. Configured G-code and its 3MF package contain identical program bytes; reopening routes through the tolerant reader.

## Selective branch integration

`gcode-core-fdm` remains preserved. Its old replacement APIs and known modal/extrusion defects were not copied wholesale. Current volumetric/flow support, writer APIs and job/3MF compatibility remain, with the new state analyzer and configured output added separately.

The `laser-cam-foundation` Rust kernel, strict bridge, section planner, Line/Fill UI and host tests are integrated against current source. See [laser-cam.md](laser-cam.md) for the bounded offline GRBL scope. Frame output remains laser-off. No controller connection or printer command is performed by qualification.

## Reproduction

- Native: `cargo test --offline --locked --manifest-path crates/Cargo.toml -p gcode-core -p gcode-optimize -p slicer-core -p laser-core`
- Real WASM: `vitest run tests/gcodeConfiguredJob.test.ts tests/laserCamHostApi.test.ts tests/meshToolpathHostApi.test.ts`
- UI/worker: `vitest run tests/gcodePanel.test.ts tests/gcodePreviewWorker.test.ts tests/gcodePreviewTransport.test.ts tests/gcodePreviewWorkerTransport.test.ts`
- Browser: `node scripts/check-gcode-manufacturing-browser.mjs <output>` and repeat with `--keyboard`. The browser checker reads downloaded G-code with a separate coordinate-state implementation, verifies deposited points against the source box bounds, reopens the export and checks frame/job GRBL output.

Final artifact hashes and observed results belong in qualification evidence after the final build. This document alone does not claim those final checks passed.
