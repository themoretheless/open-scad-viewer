# Manifold library changelog

This changelog covers the workspace-internal `manifold-core` and
`manifold-ops` crates, their manifold ABI in `geometry-bridge`/`geometry-wasm`,
and the TypeScript wrappers in `src/services/geometry/manifoldAnalysis.ts`.
Entries are ordered newest first.

The crates currently use Cargo version `0.1.0` and workspace `publish = false`.
The baseline below describes the checked-in contracts; it does not claim a
crates.io publication or reconstruct release history that is not present in
the repository.

## 0.1.0 — current workspace baseline

### Added — `manifold-core`

- Indexed triangle-mesh validation through `validate_indexed_mesh`, with
  structured `IndexedMeshError` variants for incomplete xyz/triangle buffers,
  non-finite coordinates, and out-of-range indices.
- Full topology diagnostics through `check`/`ManifoldReport`, plus the strict
  `is_manifold` predicate. Reports cover boundary and non-manifold edges,
  orientation conflicts, repeated-corner degenerate triangles, disconnected
  vertex fans, isolated vertices, and connected-component counts.
- Vertex-link validation through `validate_vertex_links` and
  `VertexLinkDefect`.
- Conservative and Full repair through `repair`, `repair_with_mode`,
  `RepairMode`, `RepairOutcome`, and `RepairReport`. Repair reports include
  change counters and a residual `ManifoldReport`; returning a repair outcome
  does not imply strict manifoldness.
- Whole-mesh and per-component measurements through `metrics`, `MeshMetrics`,
  and `ComponentMetrics`: counts, Euler characteristic, genus where defined,
  signed volume, surface area, and watertight status.
- Native property, defect-corpus, fuzz, and rbench coverage.

### Added — `manifold-ops`

- Union, intersection, and difference through `boolean_manifold` and
  `boolean_manifold_with_options` with `BooleanOp` and `BooleanOptions`.
- Full pre-repair for operands that fail admission, strict output checking,
  and one conservative post-repair pass. A successful `BooleanOutcome` is
  strictly manifold; unrepairable input, kernel failure, or a non-manifold
  result is returned as `BooleanError` instead of silently publishing a bad
  mesh.
- Operand-repair diagnostics, polygon-kernel statistics, output topology
  diagnostics, and mesh measurement/conversion helpers on `BooleanOutcome`.
- Default options `weld_epsilon = 0` and `relative_tolerance = 1e-9`; explicit
  option validation uses `0..=1` for weld epsilon and `1e-12..=1e-5` for
  relative tolerance.
- Differential qualification against the sidecar `manifold-3d` package and
  cancellation/budget tests at the integration boundary.

### Added — manifold ABI and TypeScript

- WASM exports `abi_manifold_check`, `abi_manifold_repair`,
  `abi_manifold_metrics`, and `abi_manifold_boolean`. Their successful packed
  responses decode to analysis-result handles read with `abi_array_field` and
  released exactly once with `abi_array_free`.
- Vertex format tags `0 = f32` and `1 = f64`, repair mode tags
  `0 = Conservative` and `1 = Full`, and boolean tags `0 = union`,
  `1 = intersection`, and `2 = difference`.
- Repaired and boolean output positions are f64 and output indices are u32,
  including when the input vertices are f32. Result-buffer element types and
  slot assignments are defined by `AnalysisBuffers` in
  `crates/geometry-bridge/src/mesh_analysis.rs`.
- Automatic transport validation, resource limits, structured error codes,
  and caller/native handle cleanup. Request allocations remain caller-owned
  and are released with `abi_free` in `finally` paths.
- TypeScript wrappers `checkManifoldInKernel`, `repairManifoldInKernel`,
  `metricsManifoldInKernel`, and `booleanManifoldInKernel`, including their
  typed result interfaces and `MANIFOLD_BOOLEAN_DEFAULT_OPTIONS`.
- Native ABI smoke/fuzz tests, packaged-WASM worker integration tests, and a
  native/WASM f64/f32 comparison matrix.

## Versioning and change-recording policy

### Crate versions

`manifold-core` and `manifold-ops` have independent package versions. A change
to one does not implicitly change the other, but `manifold-ops` must raise its
`manifold-core` dependency requirement when it relies on a newer core API.
Update the package manifest, the corresponding workspace dependency in
`crates/Cargo.toml`, and `crates/Cargo.lock` together through Cargo.

Before 1.0, Cargo treats the leftmost nonzero version component as the
compatibility boundary:

- `0.1.0 → 0.1.1`: compatible fixes or additive functionality;
- `0.1.x → 0.2.0`: a breaking public Rust API change.

Do not silently change an existing function signature, public field, enum
variant, default, tolerance range, result invariant, or documented error
meaning in a compatible release. Record additions, changes, fixes, breaking
changes, and migrations under their own headings in the relevant version
entry.

### ABI compatibility

There is no separate semantic manifold ABI version constant. Packaged artifact
identity is instead verified by SHA-256 and byte length before compilation or
trusted worker bootstrap. That detects different bytes; it does not make an
incompatible function or result layout safe.

Treat all of the following as ABI contract changes:

- an export name or signature, argument order, format/mode/operation tag;
- an `AnalysisBuffers` result variant's slot assignment, element type, flags,
  or per-component record layout;
- packed response/error mapping, resource-limit behavior visible to callers,
  or allocation/handle ownership and lifetime rules;
- TypeScript `KernelExports` declarations and wrapper result contracts that
  correspond to those native changes.

Changing an existing slot, tag, ownership rule, or export signature is
breaking. Do not reuse an old tag or slot for a new meaning. Add a changelog
entry under **Breaking**, **ABI**, and **Migration** as applicable, identify
both crate versions and the affected exports, and explain how callers must
change. A digest-only rebuild without a semantic contract change does not need
a fabricated ABI version bump.

For every ABI change, update the native bridge, `geometry-wasm`, TypeScript
`KernelExports` and wrappers together; rebuild the packaged artifact; and run
`npm run test:manifold`, the native/WASM precision matrix, and the artifact
export/smoke checks. Add or update ABI fuzz coverage for malformed buffers and
handle cleanup.

### Required entry content

For each subsequent version, record only changes actually present in that
version and link to the affected guide when semantics change:

1. public Rust API and behavioral changes, including defaults and residuals;
2. ABI exports, tags, result layouts, lifetimes, and error mappings;
3. TypeScript API and worker/protocol effects;
4. migration steps for breaking changes;
5. qualification evidence, including intentional benchmark-baseline updates.

Use an actual release date only when a release is really made. Do not invent
historical versions, dates, hashes, or compatibility guarantees. Qualification
entries must follow the single production-ready checklist in
`docs/manifold-definition-of-done.md`; unexecuted checks are not evidence.
