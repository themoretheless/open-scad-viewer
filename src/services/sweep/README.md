# Loft, sweep and miter integration

Rust owns geometry, constructor orchestration, refinement, section corrections,
complete-boundary certificate composition, source ownership, Solid admission,
and validation of claims displayed in the viewport.

- `crates/nurbs-core`: rational geometry and scoped geometric predicates.
- `crates/brep-core`: retained topology and boundary audits.
- `crates/geometry-bridge/src/sweep_pipeline.rs`: same-request construction and
  certificate composition, native stream snapshots and bounded refinement,
  private corrected sections, opaque owner capabilities, placement and smoothing.
- `crates/geometry-bridge/src/sweep_viewport.rs`: validation of display evidence.
- `crates/modelgraph-runtime/src/profiles.rs`: canonical convex graph loft geometry.
  Unreferenced TypeScript copies were removed.

The TypeScript files in this directory carry types and adapt native calls.
Async generators schedule browser events and observe cancellation between kernel
calls; they do not choose refinement levels or assemble geometry/certificates.
The native stream registry retains the source snapshot and owns refinement state.
An abandoned generator releases its native stream in `finally`.

`geometry/brep.ts` remains the public facade; former flat paths are compatibility
re-exports. A transport WeakMap holds opaque Rust owner identifiers. The Rust
registry checks the source geometry and boundary certificate against the privately
owned values before placement or reconstruction. Copies without a capability,
modified geometry, and stale certificates are rejected. Finalization releases
unused native owners; explicit stream release handles cancellation.

Rust certificate entry points are grouped under
`sweeps::certificates`, `progressive_miter::law_certificates`, and
`progressive_miter::boundary_certificates`. Existing module paths remain supported.
These namespaces preserve each certificate's scope and result types.

## Qualification selection

`docs/design/sweep-qualification-catalog.json` owns the selected browser sources,
STEP filenames, smooth reconstruction modes, and native/WASM/Rush suite selection.
It is a finite coverage inventory, not a geometric proof or a cross-target parity
claim. Native and WASM suites cover families; they do not each execute every UI case.

Run `node scripts/run-sweep-qualification.mjs list` to inspect selection, or replace
`list` with `native`, `wasm`, `rush`, `browser`, `step`, or `step-smooth`.
Browser and STEP targets accept the existing export/output arguments. STEP targets
export fixtures; `step-smooth` also fills the volume reference using the existing
independent polynomial-generator oracle. OCCT checks still run through
`scripts/verify-sweep-step-occt.py` against the generated manifest. The STEP exporters
reject missing, extra, or duplicate catalog fixtures. Their construction recipes,
volume calculations, and geometric assertions stay outside the selection catalog.
Browser provenance includes both compatibility facades and implementation sources.
Historical qualification records remain unchanged.

`surface_scaled_sweep` now builds fixed-orientation rational Bernstein products
with independently normalized path and positive scalar law domains. It preserves
profile parameterization and supports compatible multispan cells within degree 25
and 32 controls per axis. Discontinuous cell seams are refused. Ordinary binary64
rounding, regularity, injectivity and solid topology are not certified.

`surface_profile_sweep` now uses the native progressive RMF transport for one
2..32-section level and a fourfold sampled comparison. It returns no surface when
the sampled budget fails. Closed paths require matching endpoint scale and retain
an explicit C0 seam. `continuousBound` remains false. The earlier migration record
is historical; scalar sweep qualification is recorded separately.
