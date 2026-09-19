# Consume owned B-rep request fields

The geometry dispatcher owns its `Value` request, but the shared `field()`
helper clones each selected subtree before deserializing it. The earlier
owned-model decoder removed a second copy inside `Model::from_value`; it did
not eliminate this adapter-level copy.

Inspection and the five display/tessellation routes now use `take_field()` to
remove and deserialize the single-use `model` field. Other request fields are
still available for options. Missing fields retain the old Null fallback and
the same `GEOMETRY_INVALID_INPUT` error prefix. Model decoding, topology
validation, identity checks, certification and tessellation budgets are
unchanged. No borrowed caller data or retained handle is consumed.

The helper is deliberately separate from `field()`: converting all borrowed
reads into destructive reads would require auditing reuse and error ordering
throughout the dispatcher. This change does not remove JS-to-WASM encoding,
the request value-tree allocation, nested decoding allocations or copies in
unrelated operations. It is not zero-copy transport.

## Reproduction

`npm run bench:brep-dispatch` builds the release native example. It creates
12-, 32- and 60-tooth herringbone gears (20-degree helix, 3 mm bore), then
measures `brep_nurbs_inspect` with three warmups and nine samples. Preparation
of each owned request is outside timing. Decoding, inspection, response
encoding and destruction of the consumed request are inside. Equality checks
and SHA-256 are outside timing. Every measured result must equal the reference
report; records include input/output hashes and all samples.

Both baseline and modified native builds use the newly pinned
`nightly-2026-09-16` toolchain (rustc 1.100.0-nightly, commit
`215a8af4bb4c106cccf6d6535f84eaae91818265`, LLVM 23.1.1), on macOS arm64 / Apple
M4 Max. The older browser artifact predates that pin,
so browser runs are correctness checks, not an isolated performance comparison
for this patch. Do not run the timed example alongside builds or tests.

Baseline: `tmp/performance/brep-dispatch-before.jsonl`. The first modified run,
`brep-dispatch-after-contended.jsonl`, overlapped another worktree's Binaryen
build and is excluded from speedup claims. Subsequent independent runs in
`brep-dispatch-after.jsonl` and `brep-dispatch-after-repeat.jsonl` were performed
without our tests/builds; process checks before and after found no matching
background geometry builds. All three fixtures retain input/output hashes.

| Teeth | Before median | After median | Repeat median |
| --- | ---: | ---: | ---: |
| 12 | 31.82 ms | 29.44 ms | 30.15 ms |
| 32 | 85.49 ms | 78.55 ms | 79.23 ms |
| 60 | 107.84 ms | 100.90 ms | 99.85 ms |

Measured reduction is approximately 5-8% for this native dispatcher workload.
This is not an end-to-end browser improvement, a tessellation speedup claim,
or a heap/RSS measurement. The two modified runs also expose ordinary local
timing variation; retain the raw samples when comparing future changes.

## Contract coverage

The new integration tests compare inspection, tessellation, polygon conversion
and certified tessellation with direct typed core calls. They also exercise
display buffers/area, missing options, absent/non-object/malformed models and
invalid topology across all six routes. Existing bridge unit tests provide
broader coverage. These are not new geometry qualification claims.

The native checks passed: 249 bridge unit tests and four new integration tests,
including explicit refusal of triangle budgets outside 12..20000. UI and MCP
TypeScript checks passed. The normal Vite build passed, but `verify-dist` still
rejects 5,773,112 bytes against the unchanged 5,600,000-byte limit.

Full regression against the rebuilt WASM: 3,228 passed and nine failed across
334 files. All failures remain the historical engine-manifest and G0/G1
fingerprint checks, not geometry assertions. Log:
`tmp/performance/owned-brep-full-tests.log`. Historical evidence was not rewritten.

Rebuilt geometry WASM: 7,670,726 bytes, SHA-256
`f37b48906375cad0155d2696bb529505b528217798048f8440011ef493bd0a8c`.
Its size difference from the preceding build must not be attributed solely to
the field-ownership change because the compiler and optimizer setup also changed.

The production Chrome check passed source build, group replacement, cancellation
preserving scene/editor, recovery and all three mechanical generators (including
invalid-input recovery). All three source/geometry hashes match the preceding
`owned-json-browser-repeat` build, and there were no page errors. Evidence:
`tmp/performance/owned-brep-browser/report.json`. A background build was active;
this run establishes correctness only, not browser latency improvement.

## Open verification work

- Delivery-size and historical qualification gates remain open.
- The preceding pushed commit's STEP workflow
  [35464231173](https://github.com/themoretheless/open-scad-viewer/actions/runs/35464231173)
  failed at `browser-workbench-indexeddb`. The parent only printed the failed
  step name/status, while its child log was written to a runner-local file and
  no diagnostic artifact was available. The subsequent
  [runner investigation](step-v10-runner-2026-09-19.md) fixes diagnostic loss
  and provisioning. The Solid File menu now exposes STEP import and original
  export; the production browser scenario passes locally. GitHub verification
  remains separate from that local result.
