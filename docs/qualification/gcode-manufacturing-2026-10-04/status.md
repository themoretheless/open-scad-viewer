# G-code manufacturing qualification

Local qualified scope: configured export and explicit firmware-state analysis, additive compatibility with existing G-code APIs, integration of laser CAM, and the new Solid scene-panel entry.

## Observed checks

- Native gcode-core / gcode-optimize / slicer-core / laser-core: 123 passed.
- Native geometry-bridge G-code integration: 7 passed; laser bridge cases: 2 passed.
- Real current WASM G-code / laser / existing mesh toolpaths / worker boundary: 25 passed.
- UI and worker protocol: 41 passed.
- CAD regression suite: 915 passed across 74 files.
- Build contracts: 9 passed; Vue/TypeScript checking passed.
- Production distribution: 150 artifacts; 7,889,516 asset bytes and 14,249,285 raw WASM bytes.
- Browser mouse and keyboard activation: configured export, independent coordinate check on 2550 printed moves, reopen, laser-off frame and GRBL job all passed. Browser errors: zero. The companion health endpoint was an explicit offline fixture; no printer/controller execution was tested or requested.

The browser coordinate reader is independent of the Rust parser and compares deposited positions with the 10 × 20 × 30 mm source box bounds. This is a bounded software check, not a firmware or printable-job certificate. See the integration design document for template, thermal, recovery and B-rep slicing limits.

## Delivery and existing CI

The previous main commit 88ed3e25 has a successful STEP workflow but failed CI: run 37220475483 reports an analytic B-rep LOD-32 tessellation triangle-budget error and existing JS qualification/fingerprint and UI failures. Those failures preceded this G-code work. The local scoped checks above do not establish that the complete CI pipeline is green. Publication is deferred; no branch, stash or active checkout was removed.

Reference CI: https://github.com/themoretheless/open-scad-viewer/actions/runs/37220475483
