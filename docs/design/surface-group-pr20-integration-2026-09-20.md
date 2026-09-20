# PR 20 integration reconciliation

Audited source: `37ce2a1d9ca489808ed3e1451314288c6af758fd`, the only
unmerged commit of `themoretheless-rust-surface-grouping`. The fetched remote
and local heads agree. Target before reconciliation: main `32c3cb05`.
The PR originally targeted the old optimization branch, not main.

The useful implementation was already ported in `de97f4fa` and `2bae19e0`,
enabled in the worker, then optimized in `1e8077fa`. Reconciliation retains
that implementation instead of applying the old patch over it. The original
branch commit remains reachable through the merge's second parent.

## Complete Patch Mapping

| Original changed files | Current disposition |
| --- | --- |
| `crates/geometry-bridge/src/mesh.rs` | Grouping algorithm extracted into `mesh_surface_groups.rs`, including exact f32 welding, signed-zero normalization, DSU components and compact IDs. Current implementation additionally handles sparse references and removes redundant edge-order storage. Original export/render hooks are intentionally replaced by opt-in worker publication: the old path groups during export and again during rendering, adds work to nonvisual consumers and makes export inherit the 100k grouping budget. |
| `crates/geometry-bridge/src/abi.rs`, `mesh_analysis.rs`, `crates/geometry-wasm/src/lib.rs` | Surface-group ABI, owned result storage and array access/free are present. The algorithm has a dedicated module rather than increasing `mesh.rs`. |
| `src/services/geometry/kernel.ts`, `geometry/meshAnalysis.ts` | Export declaration and typed-array upload/copy/free boundary are present. WASM results survive memory growth and transfer. |
| `src/core/mesh.ts`, `src/core/scene.ts`, `src/services/geometryWorkerProtocol.ts` | `faceIdsSurfaceGroups` is represented by the existing `faceIdsInferred` metadata; scene conversion preserves it and protocol validation checks its type. Authored CAD face identity remains separate. |
| `src/services/legacyV5Assembler.ts`, `openscadParser.ts`, `modelGraphTextScene.ts` | Unconditional flags on parser output are not copied. Only the worker publisher marks actually computed inferred groups; authoritative CAD faces are bypassed unchanged. This keeps raw/headless output contracts intact. |
| `src/services/meshSurfaceGroups.ts` | `withSelectionSurfaces` bypasses authoritative or explicitly inferred IDs. Unprepared host meshes keep the bounded cached TS fallback rather than forcing synchronous WASM initialization on the UI thread. Rust grouping runs at worker publication. |
| `src/services/webgpuRenderer.ts` | The proposed private renderer flag was not read by any branch code. No unused replacement field is added. `App.vue` consumes the flag before handing grouped face IDs to the renderer. |
| `scripts/bench-surface-groups.mts` | Superseded by `benchmarks/surface-group-boundary.mts` and the real worker publication harness. These measure the upload boundary and complete publication, verify exact results, and distinguish cache hits from misses; the branch's transported-length check alone does not measure publication. |
| `tests/meshSurfaceGroups.test.ts` | Existing cases retained; transported/inferred bypass and zero-filled legacy IDs are explicitly covered. Kernel parity, scene roundtrip, cache ownership and worker tests extend the branch coverage. |
| `docs/design/optimization-audit-2026-09-19.md` | Historical timings are not transplanted as current claims. The measured replacement architecture and its limitations are in `surface-group-boundary-2026-09-20.md`. |

No branch-only production behavior remains unaccounted for. Grouping directly
inside the final retained-solid producer may still be a future optimization,
but must avoid duplicate work and preserve nonvisual/export semantics; it is
not necessary to retain the old branch's implementation.

## Revalidation

- 93 tests passed across `meshSurfaceGroups`, `surfaceGroupsKernel`, `coreScene`,
  `geometryWorkerProtocol`, `geometryWorker`, `buildCoordinator` and
  `webgpuRendererResources` (3.06 seconds).
- Seven native `mesh_surface_groups` tests passed. Existing unrelated Rust
  warnings remain; this is not a clean-Clippy claim.
- Rebuilt the native parity example; 838 deterministic cases matched exact
  TS IDs/errors. Fixture SHA-256:
  `3ddf4513ba1b65ccc34c3f3f53378c11ddde596332ef8860f696be543ec2d46b`.
- Chrome production app smoke requested and received worker-inferred groups,
  completed successfully and displayed four mesh entries. Report:
  `/private/tmp/osv-surface-pr20-smoke/report.json`.
- No production source or WASM bytes change in this reconciliation. The latest
  full suite on `32c3cb05` has 3394 passing tests and nine qualification-binding
  failures. Those failures are neither fixed nor waived by this merge.

The new tests guard the remaining branch regression cases. An explicit
reconciliation merge preserves main's tested tree and records the source
branch as integrated; it must not be described as applying the original
export/render grouping hooks unchanged.
