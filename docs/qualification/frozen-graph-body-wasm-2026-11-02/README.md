# Frozen graph body: actual WASM and worker replay

Added `brep_frozen_graph_body_restore` to the common native/WASM B-rep router and `frozenGraphBodyRestore` to the MainSolid worker protocol. Restore reconstructs a `FrozenGraphVolume`, re-proves its native volume/global embedding/G2-and-curvature prerequisites, and exports its canonical exact artifact. The carrier is explicitly `native-exact-graph-body`; it never converts rational vertices, UV/root cuts or coefficients into a floating-point B-rep.

The TypeScript result guard compares the complete exact geometry and original identity links against an immutable request expectation, including non-dyadic cuts. It binds numerical resource limits and report counts to the request. Radius, thickness and application admission remain false. No UI/document commit is introduced.

## Actual evidence

19 selected tests pass: 7 native router/Runout tests, 8 transport tests and 4 separate-worker tests (including the existing graph-preview worker regression). Full vue-tsc passes. The standalone verifier exercises the real packaged WASM and actual handler, with original-input preservation, exact geometry/ID equality, corrupted-ID refusal and zero-work refusal.

The included control artifact restores to one nine-face body with 27 edges, 20 vertices and no open edges. All 36 distinct face pairs are re-proven. The two additional vertices/edges preserve root partitions at 1/3 and 2/3. A real worker test cancels the old realm, rejects an artifact with changed source identity and accepts an unmodified Retry.

The measured direct helper time includes cold module work; `workerMs` in the verifier is an in-place handler call after warmup, not separate-realm transport latency. The separate-worker tests prove behavior, not a complete latency distribution. The small warm plane-evaluation benchmark is a noisy snapshot comparison only: no general speedup or all-command performance qualification is claimed.

## Bounded delivery

The initial optimized module was 16,922,027 bytes and failed the existing 16 MiB decompression cap. Convergent/GUFA/merge passes and codec/topology-only size profiles did not solve the bound; none of those unsuccessful changes was retained. The retained geometry-WASM build defaults to opt-level z while preserving explicit hot-package settings, location stripping and native profiles. Final WASM is 16,438,042 bytes, below the unchanged 16,777,216-byte cap. No operation was removed and no decompression limit was raised.

The pre-existing native Binaryen override was used with version 116, matching the installed JS Binaryen. Its original-profile single-pass output was byte-identical to the JS optimizer output. Normal build commands remain `node scripts/build-geometry-kernels.mjs`; an existing absolute `OSV_WASM_OPT_NATIVE` path is optional and version-checked by the build helper.

## Open scope

This gate covers the nine-face supported graph fixture. The preceding 27-face native replay is not a 27-face WASM qualification. Showing/applying the closed body in the UI, a complete radius law, wall-thickness admission, arbitrary NURBS/mixed-radius nodes, parts/STEP/history and the complete P0 matrix remain open.
