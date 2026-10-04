# Expanded sweep/miter publication audit

Status: local frontend, UI and independent STEP matrices passed on the final Rust-owned artifact. Full Rust workspace passed; publication/fresh CI remain pending. The goal is not complete.

## Artifact and ownership

Geometry WASM `d18ff8c6a310a587bda1268d92a8b708fd101346ef2497ed95d516a666f67892`, 10,584,966 bytes, is recorded by own-Rust v36 and bound by G0 v45 / G1 v62. Source/artifact drift audit passed. Only the observed local macOS/aarch64 artifact is recorded; no foreign-host identity is inferred. Historical records remain immutable and no G1 clean-run work is imported.

Native kernels own sweep/miter construction, outward wall/cap error bounds, retained decomposition, geometric predicates and Solid admission. `nurbs-core::sweep_seam_set` owns complete seam-set budgets and certification. `brep-core::miter_seams` owns retained profile/station extraction, G2-to-G1 fallback and shared chart budgets. `nurbs-core::sweep_section_correction` owns basis premises, correction work and whole-family displacement bounds. Rust bridge owns retained cap evidence and cap-pair scope/report orchestration over native predicates. The corresponding TypeScript services are transport/report adapters.

## Requirement matrix

| Requirement | Current evidence | Remaining gate |
| --- | --- | --- |
| Affine, authored/moving frames, guides and combined miter through native/WASM/Rush/viewport/Solid | Full frontend passed; 39 sources through both browser widths; 46 independent STEP cases | Fresh CI on the published source |
| Complete boundary bound: walls, frames, miter, decomposition, filled caps, correction | Native outward composition and exact retained-domain premises; packaged correction/boundary/budget tests; UI admission and refusal matrix | Fresh CI |
| Regularity, intersections, holes, nesting and shell orientation | Actual retained B-rep proof gates; explicit unsupported/budget refusals; independent OCCT geometry/topology/material/volume checks | Fresh CI; arbitrary unsupported geometry remains outside positive claims |
| Applicable G1/G2, moving frames, multispan and closed seams; sharp C0 | Native whole-set audits; independent G1 fallback; full frontend and UI proof presentation; reconstructed and closed STEP fixtures | Fresh CI |
| Independent STEP frame/guide/combined matrix | 36 baseline + 10 reconstructed smooth-station cases passed with exact artifact provenance | Complete |
| Wide/narrow UI: Solid, refusal, cancellation, source changes and recovery | 78/78 cases; 896 assertions; zero page errors; widths 1440 and 600; selected wide/narrow Solid screenshots visually inspected | Complete for this stated finite UI matrix |
| Delivery | Draft PR 30; local artifact and source bindings; all active unrelated work preserved | Commit/push; PR refresh; fresh CI; verified RAG archive |

## Verification

- Stable full frontend: 4388 passed, 9 excluded; 449 suites passed, 4 excluded.
- Fingerprint/executor tests: 23 passed. Drift audit passed against own-Rust v36.
- Both application and MCP TypeScript configurations passed.
- Production dist: 136 verified artifacts; 7,657,290 asset bytes and 13,881,358 raw WASM bytes. Geometry JS chunk 3,540,694 bytes, measured limit 3,545,000 bytes; unchanged total asset budget.
- Independent OCCT: all 36 baseline and 10 reconstructed smooth-station cases passed. Checks cover retained control nets and rational bases, world-edge/pcurve ownership and full domains, topology, opposite coedge uses, shell orientation, material-side probes and independent generator volumes. Surface/jet samples supplement exact shared-basis coefficient checks.
- Browser: `ui-d18/matrix.json` binds the final geometry hash, source hashes and production index. All 78 cases passed with 896 assertions and no page errors.
- Optimizer contract: 8 passed. Provider readiness passed at normal and four-way concurrency. GPU/CPU dependency contracts and CI Clippy targets passed.
- Native focused and broader geometry runs passed. The final full workspace passed: 3187 tests, zero failed, four excluded. Terminal logs and hashes are recorded in `local-verification.json`.

## Compatibility regressions resolved

The ported loft had incorrectly rejected varying rational weights. Corresponding degrees/control layouts remain required; weights may differ by station. Existing loft integration tests now pass 9/9. Two STEP assertions had assumed normalized [0,1] cap parameters; they now reflect around actual authored-domain endpoints without loosening geometry tolerances. Canonical quarter-turn conics preserve exact represented G2 profile joins. Native jet reports preserve wholeSeam/normalizedParameters/method. Shared JSON schema is generated from the typed schema. Earlier failed and superseded artifact reports remain diagnostic archives and do not qualify the current source.

## Claim boundaries

Exhausted or unresolved geometric proofs refuse certification. A boundary Hausdorff bound alone does not prove embedding or volume validity; actual body admission checks those independently. Finite fixtures do not establish universal arbitrary-mode guarantees. Held-worker-dispatch cancellation proves lifecycle cleanup and stale-result suppression, not interruption latency inside a synchronous native kernel. Headless CPU browser checks do not qualify GPU rendering. G0/G1 fingerprints are evidence bindings with zero imported clean-run work, not production cutover approval.

## Lossless proof packaging

1690 raw proof files are preserved in ten deterministic gzip/tar archives with archive and per-file SHA-256 hashes in `artifact-archives/index.json`. Each archived file was read back and matched against the original bytes. Compact UI and OpenCascade reports remain directly readable. Raw directories remain unchanged locally and are ignored by Git; restore their complete contents using the archive README.

## CI provenance correction after 124eea3b

Initial published CI passed Rust (3187 tests, four ignored), OpenSCAD MCP and STEP v8/v9/V10. Both Node matrices rejected the unrecorded Ubuntu build; Node 22 reported 4382 passed and six failed. Five failures were the exact artifact identity gate and one was the expected historical `.gitattributes` drift after committing raw terminal logs. Own-Rust v37 records the independently downloaded Linux bytes in addition to unchanged local d18, bound by G0 v46 / G1 v63. All 35 focused tests passed with both actual WASM binaries; local d18 was restored and verified. Historical records remain immutable. Fresh full CI remains required.
