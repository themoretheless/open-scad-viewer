# Architecture

This document describes the architecture that exists in the repository after
the Manifold rewrite, the [seven-role review](docs/review-of-main-rewrite.md),
and the build/scene foundation pass. It is not a feature wishlist. Prioritized
debt and acceptance criteria live in [recommendation.md](recommendation.md).

## Product boundary

The OpenSCAD Viewer product remains a client-only Vue 3 + TypeScript
application: geometry compilation and rendering stay in the browser and there
is no application backend. The repository also ships an optional local Node
process that exposes headless compiler operations over MCP stdio and persists
its own catalog in DuckDB. The MCP process is a sidecar/tooling surface, not a
runtime dependency of the browser app.

The browser and original MCP geometry lane implement a **strict, independent
OpenSCAD subset**, while a versioned repository-owned full-profile lane is being
qualified against `openscad/stable-2021.01`. Neither embeds or delegates to the
official compiler. Supported constructs produce real geometry through the Apache-2.0
[`manifold-3d`](https://github.com/elalish/manifold) package; unsupported syntax
must fail with a source diagnostic rather than produce an approximate result.
The MCP sidecar also has a separate, opt-in official-runtime lane used only as
a differential oracle and reference exporter. Its GPL runtime is explicitly
downloaded into a local gitignored cache and never enters the browser graph,
independent evaluator, or production router.

This boundary is intentional:

- official OpenSCAD execution is served by an explicit oracle MCP boundary,
  subject to its GPL-2.0+ requirements, and is not the product engine;
- subset additions must not silently diverge from documented semantics;
- official execution must never be mislabeled as an independent Manifold/B-rep
  build or used as an implicit fallback;
- WebGPU is currently the only renderer backend;
- the current workspace contains one `.scad` document and persists locally.

The supported language and limits are documented in
[README.md](README.md#browserindependent-subset-and-full-mcp-profile).

## System map

```text
Vue workspace / file actions / textarea
        │
        ├─ commandRegistry.ts: palette metadata + scoped keyboard routing
        ▼
WorkspaceDocumentSnapshot
(documentId, source, geometry revision, persistence mutation)
        │
        ├─ IndexedDB active head (validated compare-and-swap commits)
        └─ per-writer localStorage recovery journals + causal IDB bases
        │
        ▼
BuildCoordinator ───── protocol-v5 ordering, cancellation and Worker lifetime
        │
        ▼
geometry.worker.ts
        │
        ├─ lexer + recursive-descent parser + evaluator
        ├─ Manifold WASM primitives, booleans and tessellation
        ├─ stable operation/entity identity + triangle provenance
        ├─ semantic-edge/topology analysis
        └─ per-mesh triangle BVH
        │ transferable typed arrays
        ▼
Published MeshData[] + diagnostics + metrics
        │
        ├─ App scene/inspection view model
        └─ WebGPURenderer
              ├─ GPU resources and invalidation-driven drawing
              ├─ camera/input and interaction overlays
              ├─ scene AABB hierarchy (TLAS)
              └─ per-mesh BVH exact picking
```

The optional headless path shares pure compiler/inspection/export services but
does not enter the Vue or WebGPU graph:

```text
MCP client
    │ stdio
    ▼
BoundedTransport ── request admission + serialized bounded writes
    │
    ▼
createOpenScadMcpServer
    ├─ Independent stable-2021.01 evaluator ── check / full STL / OBJ
    │     └─ bounded project VFS + session-local SHA-256 artifact cache
    ├─ HeadlessGeometryService ── inspect / STL / OBJ
    │     │
    │     └─ DirectGeometrySupervisor ── bounded FIFO + deadline/cancel watchdog
    │              │ one disposable Node Worker per job
    │              └─ directGeometry.worker.ts ── parser + Manifold provider
    ├─ OfficialOpenScadRuntimeSupervisor ── differential oracle / reference exports
    │     │ one permission-model subprocess per job
    │     └─ officialOpenScadRuntimeRunner.mjs
    │           ├─ verified patched snapshot + pinned Basic font
    │           └─ bounded project-relative MEMFS only
    ├─ typed tools + openscad:// resources
    └─ DuckDbModelStore
          ├─ versioned models
          ├─ build results
          └─ bounded STL/OBJ artifacts
```

## Current components

| Component | Current responsibility |
| --- | --- |
| [`src/App.vue`](src/App.vue) | Workspace composition, file/share/export actions, explicit recovery-conflict choice, build publication, renderer recovery, Vue view state and orchestration. |
| [`src/core/mesh.ts`](src/core/mesh.ts) / [`src/core/build.ts`](src/core/build.ts) | Renderer-neutral mesh, identity, provenance, transfer and quality contracts; no parser/kernel dependency. |
| [`src/services/workspaceDocument.ts`](src/services/workspaceDocument.ts) | Versioned, validated single-document contract with separate geometry revision and monotonic persistence mutation, including legacy migration. |
| [`src/services/workspaceIndexedDb.ts`](src/services/workspaceIndexedDb.ts) | Browser-only active-head repository with runtime validation, compare-and-swap writes, upgrade handling and bounded open failure. |
| [`src/services/workspacePersistence.ts`](src/services/workspacePersistence.ts) | Pre-mount hydration, ordered autosaves, causal IDB/per-writer-journal reconciliation, CAS conflict handling, and crash journal lifecycle. |
| [`src/services/workspaceShare.ts`](src/services/workspaceShare.ts) | Bounded URL-safe codec for shared source imports. |
| [`src/services/buildCoordinator.ts`](src/services/buildCoordinator.ts) | Protocol-v5 job ordering, exact-source/route/span re-attestation, post-clone provenance freezing, latest-result publication, preview/full policy, cancellation, hard preemption and Worker disposal. |
| [`src/services/geometryWorkerProtocol.ts`](src/services/geometryWorkerProtocol.ts) | Versioned and runtime-validated request/event contract: revision, monotonic job, exact source digest, quality, phase, bounded sorted provenance, progress and terminal state. |
| [`src/services/commandRegistry.ts`](src/services/commandRegistry.ts) | One typed inventory for palette metadata and deterministic, scope-aware keyboard routing. |
| [`src/workers/geometry.worker.ts`](src/workers/geometry.worker.ts) | Isolates compilation, validates routing before queue state, rejects replay/stale work, warms only the selected provider, reports checkpoints and transfers geometry buffers. |
| [`src/services/openscadParser.ts`](src/services/openscadParser.ts) | Strict lexer/parser/evaluator, identity assignment, Manifold calls, mesh conversion, provenance and resource budgets. |
| [`src/services/meshBvh.ts`](src/services/meshBvh.ts) | Compact transferable per-mesh triangle BVH and exact ray intersection. |
| [`src/services/sceneAabbIndex.ts`](src/services/sceneAabbIndex.ts) | Deterministic scene-level AABB hierarchy that rejects whole bodies before triangle traversal. |
| [`src/services/meshTopology.ts`](src/services/meshTopology.ts) | Boundary/crease/non-manifold edge extraction and topology diagnostics. |
| [`src/services/meshInspection.ts`](src/services/meshInspection.ts) | Identity-aware replacement matching, provenance lookup, bounds, normals and measurements. |
| [`src/services/scenePublication.ts`](src/services/scenePublication.ts) | Pure, identity-safe visibility/selection/isolation/measurement continuity plan for replacement scenes. |
| [`src/services/cameraHistory.ts`](src/services/cameraHistory.ts) | Validated, bounded camera navigation history with immutable snapshot/restore for renderer replacement. |
| [`src/services/rendererRecoveryGate.ts`](src/services/rendererRecoveryGate.ts) | Pure device-loss policy that coalesces concurrent loss signals, permits one follow-up attempt and prevents false-ready publication. |
| [`src/services/webgpuRenderer.ts`](src/services/webgpuRenderer.ts) | WebGPU lifecycle, pipelines/resources, camera/input, visibility, picking, overlays, measurements and sections. |
| [`src/components`](src/components) | Command palette, ViewCube, Scene Outliner, Inspect and Customizer presentation components. |
| [`src/mcp/geometryService.ts`](src/mcp/geometryService.ts) | Headless summary, validated Customizer replacement, size-bounded STL/OBJ export, and a process-wide bounded pending-job gate. Production injects the direct Worker runtime; deterministic unit tests may retain the in-process engine. |
| [`src/mcp/directGeometryProtocol.ts`](src/mcp/directGeometryProtocol.ts) / [`src/mcp/directGeometry.worker.ts`](src/mcp/directGeometry.worker.ts) | Exact, source-hash-correlated Worker envelopes and the disposable production realm that invokes build/capability work on the direct default geometry engine. The child has no DuckDB, MCP transport, filesystem, network or subprocess dependency. |
| [`src/mcp/directGeometrySupervisor.ts`](src/mcp/directGeometrySupervisor.ts) | One-at-a-time FIFO admission, queue-inclusive deadline, startup/cancel/join watchdogs, hard Worker termination, join-before-settlement and permanent quarantine after an unjoined child. |
| [`src/mcp/officialOpenScadRuntimeProtocol.ts`](src/mcp/officialOpenScadRuntimeProtocol.ts) / [`src/mcp/officialOpenScadRuntimeRunner.mjs`](src/mcp/officialOpenScadRuntimeRunner.mjs) | Exact bounded project/result envelopes and the dependency-free one-shot child that mounts source/assets in Emscripten MEMFS and calls the verified official runtime. |
| [`src/mcp/officialOpenScadRuntimeService.ts`](src/mcp/officialOpenScadRuntimeService.ts) | Runtime/font/license-manifest verification, one-at-a-time subprocess admission, Node host-file permission boundary, deadline/cancel/kill/join lifecycle, explicit network/WASM-memory residuals, and stable-language check/export API. |
| [`src/mcp/independentOpenScadExecution.ts`](src/mcp/independentOpenScadExecution.ts) / [`src/mcp/independentArtifactCache.ts`](src/mcp/independentArtifactCache.ts) | Shared bounded execution path for independent check/full-quality export and a per-session byte-SHA-256 STL/OBJ resource cache that deliberately bypasses legacy DuckDB provenance. |
| [`src/core/openScad2021Contract.ts`](src/core/openScad2021Contract.ts) | Frozen stable OpenSCAD 2021.01 inventory, exact source revision, functions/modules, language surface, file semantics, compatibility tail, smoke programs, and the separately named execution snapshot. |
| [`src/mcp/boundedTransport.ts`](src/mcp/boundedTransport.ts) | Process-wide MCP request admission tied to actual handler settlement, bounded modern subscriptions, inbound notification coalescing/filtering, fail-fast busy responses, serialized stdio writes, and bounded outbound backpressure. |
| [`src/mcp/duckdbModelStore.ts`](src/mcp/duckdbModelStore.ts) | Node-only, parameterized DuckDB repository with schema-v5 source-attestation classes, versioned diagnostics, immutable source revisions, retention quotas, catalog statistics, and owner-only POSIX file permissions; external access and extension loading are disabled. |
| [`src/mcp/createServer.ts`](src/mcp/createServer.ts) / [`src/mcp/server.ts`](src/mcp/server.ts) | Dual-era typed MCP tools/resources/prompts, reproducible revision selectors, cache hints, guided instructions, and the local stdio/bootstrap lifecycle. |
| [`src/mcp/errorContract.ts`](src/mcp/errorContract.ts) / [`src/mcp/publicError.ts`](src/mcp/publicError.ts) | Frozen public MCP error taxonomy/retry policy, versioned persisted diagnostics, correlation IDs, and redaction of source excerpts/internal failures before persistence or wire output. |

The parser, renderer and App remain broad modules. This table describes real
boundaries, not an assertion that the separation is finished.

## Data contracts and identity

### Workspace revision

`WorkspaceDocumentSnapshot` is the durable document boundary. `documentId`
persists across edits. `revision` increases only when geometry source changes;
filename-only metadata updates persist without invalidating an identical
in-flight build. `mutation` increases for every source or metadata change and
orders edits within one causal branch without depending on wall-clock
monotonicity. Recovery replay additionally requires its recorded base to match
the current IndexedDB head. The geometry revision, not the source string, is
the asynchronous build identity.

### Build identity and publication

Protocol v5 is the only App/Worker path. Every request/event contains a protocol
version, document revision, job ID and SHA-256 of the exact UTF-8 source;
build events also carry quality and a typed phase. Runtime guards validate the
digest on both sides, reject same-route replay, bound all transferred text and
mesh data, require non-empty sorted/non-overlapping provenance runs, and
validate mesh and failed-diagnostic spans against the attested source. The
Coordinator re-freezes execution provenance after structured clone and before
calling publication hooks. Worker job IDs use a bounded monotonic high-water
tombstone, so an active or completed ID cannot be replayed.
The Worker reports accepted, started, progress, succeeded, failed, cancelled
or stale outcomes. Successful events require `reduced`: a preview with
`reduced=false` is byte-identical to full quality, so App promotes it to full
and cancels the redundant scheduled build. `BuildCoordinator` exposes
corresponding immutable state, including an explicit `stale` state.

Only the latest job of each quality tier for the current revision is
publishable. A preview may publish while a full build for the same revision is
pending, but it can never downgrade a published full result. Preview→full for
one revision shares the warm Worker. Worker import itself does not warm
Manifold; warm/build happens only after the source-selected provider passes
admission. When a newer revision supersedes running
synchronous work, the coordinator requests cancellation and replaces the
Worker after a configurable grace period if no real checkpoint is reached.
App performs a final revision check before publishing. Export is allowed only
from an error-free full build of the current source; a retained last-known-good
mesh after failure is visual context, not a current exportable result.

Engine selection follows the accepted source-only contract in
[`ADR 0002`](docs/adr/0002-permanent-geometry-engine-routing.md). The frozen
negative/precedence oracle is
[`geometry-routing-contract-v1.json`](docs/qualification/geometry-routing-contract-v1.json).

### Source and scene identity

Identity has three layers:

- `SourceOperationId` identifies a static geometry call by structural AST path.
  It survives whitespace/comments, preview/full quality, argument-value edits
  and changes to differently named siblings.
- `SceneEntityId` identifies an evaluated instance by combining its call path
  with dynamic frames such as loop iterator value and duplicate occurrence.
- `MeshSourceReference` maps triangle provenance to an exact exclusive source
  span and carries both identities.

Same-name sibling occurrence is positional. Across a changed source snapshot,
a group containing multiple such siblings is considered ambiguous and UI state
is not guessed for it. Preview→full builds of the same snapshot can reuse the
IDs safely. Newly compiled meshes populate identity fields; optional fields at
the type boundary exist only for migration compatibility.

Mesh replacement uses unique entity identity first. Legacy provenance is used
only when unique on both sides; scene-array order is never an identity.

### Geometry artifact

The neutral `core/mesh.ts` contract defines `MeshData`, identity/provenance,
BVH/topology shapes and deduplicated transfer-buffer discovery. `MeshData`
currently packages interleaved positions/normals, triangle and
semantic-edge indices, transform/color, face IDs, compact provenance, topology
diagnostics and a triangle BVH. The Worker transfers the underlying
`ArrayBuffer` objects. Published results are immutable snapshots; the Worker
must not retain transferred buffers.

## Geometry pipeline and safety

The handwritten parser stores token-derived source spans and evaluates the
document into Manifold solids/cross-sections. `union`, `difference`,
`intersection` and `hull` are real kernel operations. Successful inspection
metrics and full-quality exports are derived from the same geometry.

Statement-form `assert(condition, message)` is a fail-fast language boundary.
Arguments are bound and evaluated in the current module/loop scope; a passing
assertion transparently evaluates its child geometry, while a failed assertion
publishes one positioned `failed` terminal per job and never exposes partial
geometry. Cancellation or supersession wins a race with that failure, so a job
still has only one terminal outcome. Expression-form assertions remain
explicitly outside the supported subset. Cooperative yields currently occur
between top-level statements: a large child block guarded by one passing
assertion remains synchronous until that statement ends, with hard Worker
replacement retained as the watchdog boundary.

The post-rewrite review fixed several important compatibility and safety bugs:

- OpenSCAD polyhedron winding is converted to Manifold convention and duplicate
  coordinates are merged before `ofMesh`;
- polygons use the EvenOdd fill rule, so valid clockwise outlines do not vanish;
- extrusion/revolution failures become positioned diagnostics and non-positive
  extrusion height is rejected;
- Customizer replacement computes literal offsets structurally;
- binary STL headers are truncated to 80 UTF-8 **bytes**, not characters.

Worker-side budgets cover source length, AST and expression depth, evaluation
depth, evaluated-value allocation, range expansion, object/triangle count,
`$fn`, total evaluation steps, `concat`/`str` result size and extrusion slices.
The share URL is size-checked before decoding. A limit violation is a diagnostic,
not partial geometry.

Manifold initialization and compilation are asynchronous from the main thread,
but the kernel section is synchronous within its Worker. Cancellation can be
observed at Worker checkpoints; an already-running kernel call still requires
hard Worker replacement.

### Upstream oracle execution boundary

The upstream oracle is intentionally not another provider in the frozen or
independent production geometry router. An explicit setup command downloads the pinned official
WebAssembly artifact, verifies the archive digest before extraction, replaces
the Node raw-filesystem overlay with injected MEMFS, and records the patched
runtime digest in a strict colocated manifest. The same command downloads and
verifies the pinned OFL Basic Regular font plus its license text. The cache is
local and ignored by Git.

MCP accepts source plus a bounded list of project-relative text/base64 files.
The parent and child independently validate canonical paths, duplicate names,
digests, aggregate bytes, defines, experimental feature names, and the exact
wire envelope. Every operation then starts a fresh Node subprocess with the
permission model enabled and host-file read access limited to the child runner,
verified runtime, and verified font; host-file writes are denied. The child
mounts only `/project` in MEMFS, injects the Basic font through a minimal
Fontconfig configuration, and resolves SCAD file paths inside that virtual
filesystem rather than on the host.

The supervisor admits one official job at a time and rejects concurrent
official work as busy; it has no internal job queue. It bounds both child
streams and kills/joins the process on cancellation, timeout, malformed
protocol, or oversized output. The admission slot is released only after that
child joins. Official diagnostics and `ECHO` messages are returned as bounded
ordered logs. Successful exports enter a content-addressed per-session cache
and are exposed as MCP resources. They do not enter DuckDB because the current
persisted descriptor can attest only the independent router. The runner exposes
no network API to SCAD, but Node's permission model does not enforce a network
sandbox. Its V8 old-space setting also does not cap WebAssembly linear memory;
OS-level network isolation and hostile-memory containment are residual risks
outside this boundary.

The stable compatibility source of truth is the versioned OpenSCAD 2021.01
contract. The repository-owned engine implements that target; the optional
oracle uses the named official 2026.09.01 snapshot. Experimental snapshot
features are off by default. The versioned registry names the compatibility
target and its relationship to the oracle snapshot. Its immutable
`openscad://language/openscad-2021.01` resource publishes the full contract;
capabilities, status, checks, and exports carry the derived stable-language
summary. MCP capability resources also expose runtime/patch/font/license
digests, declared isolation flags and fail-closed setup state; they do not
derive the stable inventory from the newer runtime.

## Rendering and interaction

`WebGPURenderer` owns WebGPU resources and reports typed initialization,
unavailability, readiness, device-loss, frame-error and destruction states. A
transient frame failure gets one bounded retry. On device loss, App coalesces
overlapping loss signals, permits at most one follow-up recovery and never
publishes `ready` from a device already reported lost. Recovery rehydrates the
latest CPU scene, visibility, identity-mapped selection/isolation, current
camera, Previous View history, measurement and section state. Renderer-local
surface hits are deliberately cleared because their triangle ownership cannot
survive resource replacement. Teardown is idempotent and invalidates pending
initialization and hover work.

Rendering is requested on state changes rather than running continuously.
Opaque/transparent bodies, grid, semantic edges, selection/source overlays,
measurements and the virtual section plane use dedicated passes. Transparent
bodies are sorted back-to-front by camera depth; intersecting transparent
triangles remain order-dependent.

Picking uses two CPU acceleration levels:

1. the scene AABB hierarchy returns body bounds front-to-back;
2. body triangle BVHs are opened lazily until no unopened bound can beat the
   nearest exact hit required by the current hit budget;
3. a bounded nearest-first merge supports deterministic depth cycling.

Visibility changes invalidate the scene hierarchy. Section clipping is applied
while candidates are consumed, allowing traversal behind clipped surfaces
without rebuilding geometry. Replacement visibility is committed as one batch,
so bounds, TLAS invalidation, overlays and render scheduling update once.

## State ownership

Current ownership is explicit because its last overlap is active debt:

- the workspace snapshot owns source, filename and geometry revision;
- the MCP DuckDB catalog independently owns only models explicitly saved
  through MCP; it does not mirror or mutate the browser's IndexedDB or recovery
  journal;
- `BuildCoordinator` owns Worker lifetime and in-flight job correlation;
- App owns the published CPU scene and user-facing build/error state;
- `scenePublication.ts` computes replacement continuity without owning mutable
  UI or renderer state;
- the renderer owns GPU resources, camera mechanics and transient hit/overlay
  resources;
- App mirrors some renderer interaction state for panels and commands.

New features must not introduce another writable copy of selection, visibility,
projection or camera state. The target is one typed scene/viewport state with
commands flowing inward and renderer intents/events flowing outward.

## Architectural invariants

1. **No stale publication.** Older work never replaces a newer document, and
   preview never downgrades full output for the same revision.
2. **No approximate language fallback.** Unsupported input fails explicitly.
3. **Real geometry.** Booleans, metrics and authoritative exports use the same
   Manifold result.
4. **Identity before order.** UI continuity uses entity/operation identity;
   array position is only a render index.
5. **Bounded work.** User-controlled parsing, evaluation, geometry, overlays,
   sharing and picking have explicit limits.
6. **Immutable publication.** Worker output is transferred once and treated as
   a snapshot.
7. **Lifecycle visibility.** Worker/GPU failures are surfaced and owned
   resources/listeners are released on teardown.
8. **Full-build exports.** Preview tessellation is never silently exported as
   authoritative geometry.
9. **Browser/server import separation.** Native DuckDB and MCP packages stay
   outside the Vite graph; only the Node-side MCP boundary imports them.
10. **No arbitrary database execution.** MCP exposes typed repository methods,
    not user-supplied DuckDB SQL; external access and extension loading remain
    disabled.
11. **No profile laundering.** Official execution and artifacts are never
    labeled as independent Manifold/B-rep results, persisted under their engine
    attestations, or used as an automatic fallback.
12. **Official files are virtual.** Official MCP requests can resolve only the
    bounded project bundle mounted in MEMFS, never ambient host paths.
13. **Bounded MCP persistence and wire output.** Source history, build history,
    artifact count/bytes, diagnostic strings, analysis detail and stdio-sized
    resource payloads have explicit limits; pruning is transactional. Inbound
    notifications are reduced to lifecycle events for active work before the
    SDK queue.
14. **Reproducible MCP reads.** Check, compare, analyze, customize and export can
    select an immutable saved revision; read-only check/compare operations never
    add build history, while mutation remains guarded by expected revision.
15. **Machine-actionable MCP failure.** Expected tool failures expose stable
    codes and recovery details; new persisted failures/cancellations require
    diagnostic contract v1 with an attested retry class, while pre-contract
    rows are explicitly `legacy-unattested`. Unexpected errors expose only a
    correlation ID.
16. **Pinned MCP lifecycle seam.** The server SDK stays exact-pinned while
    admission settlement wraps its request registry, including handlers added
    by the modern stdio host after factory creation; wire regressions gate an
    SDK upgrade.
17. **MCP provider realm disposal.** Production stdio never calls the direct
    geometry provider in its event-loop realm. Every job owns one Worker;
    terminal publication and next-job admission wait for its join. Mutable host
    isolation/limits are advertised separately from immutable engine manifests.

## Current limitations

- The frozen browser/legacy route is not full OpenSCAD. The separate
  repository-owned stable-profile route now has user functions, all 38 stable
  value functions, bounded `include`/`use`, and most stable modules, but it is
  still under qualification: expression/scoping semantics and file-backed
  `import` and `text` remain release blockers. Bounded DAT/PNG `surface()` is
  implemented through the project VFS. The upstream oracle
  can diagnose differences but is never a product fallback.
- Parser, scope/evaluation logic, direct Manifold calls, tessellation and
  artifact construction remain concentrated in one full-rebuild module; there
  is no `GeometryKernel` interface or content-addressed subtree cache.
- Superseding a running synchronous kernel call discards the Worker and warm
  WASM state. Each edit still schedules preview and unconditional full builds.
- `MeshData` no longer makes the parser a type hub, but still eagerly bundles
  render, inspection and acceleration artifacts.
- Scene publication recreates all GPU mesh resources. Geometry instancing,
  retained entity deltas and shared GPU assets are absent.
- Face/source overlay creation still scans mesh data and allocates GPU buffers;
  style changes update every object uniform, and bounds are rescanned.
- `App.vue` and `webgpuRenderer.ts` retain broad responsibilities and mirror
  selection/visibility state. Replacement continuity is now a pure service,
  but App still applies the resulting state to both owners.
- Browser persistence is versioned and IndexedDB-first, with a synchronous
  localStorage recovery journal, but remains single-document and independent
  from the optional MCP/DuckDB catalog.
- MCP production compilation now runs behind a disposable Worker watchdog, so
  a synchronous Manifold call cannot block stdio and deadline/cancel can hard
  terminate its realm. Worker threads still share the host process and the
  current contract has no OS-enforced memory ceiling for WASM/native
  allocation; subprocess/cgroup/job-object containment remains residual risk.
- Official-runtime jobs use fresh subprocesses and a permission-limited host
  filesystem, but there is no enforced network sandbox or WebAssembly
  linear-memory ceiling. The one-shot process, bounded wire data, deadline and
  hard kill reduce exposure without claiming OS-level hostile-code containment.
- The editor remains a textarea and scene selection remains single-object.
  Dead additive/range affordances are removed; the ViewCube compass follows the
  live camera, face presets snap to orthographic, and two-pointer pinch plus
  midpoint pan are supported. The dock and Outliner now expose tab/tree ARIA
  contracts, while keyboard camera motion and browser/AT verification remain.
- Transparency is explicitly reported as approximate object-sorted alpha rather
  than OIT. WebGPU has no raster fallback, but its failure no longer removes the
  workspace: CPU/WASM builds, editing, persistence and export remain available
  in a typed headless-geometry tier while the viewport offers a retry.
- CI has exact upstream-oracle stable-inventory conformance and growing
  independent-profile gates, but no real-browser
  GPU/accessibility smoke, visual regression, differential/fuzz corpus, or
  performance budget.

## Phased target architecture

This is an extraction plan, not a big-bang rewrite.

### Phase 1 — compiler and state seams

- separate parser, binding/diagnostics, immutable operation IR, kernel,
  tessellation and analysis contracts;
- keep neutral mesh/build contracts and the parser/kernel value import boundary
  enforced while compilation phases are split;
- add cooperative evaluation checkpoints and phase timing while retaining hard
  preemption as a watchdog;
- build on the extracted reconciliation plan to establish one scene/viewport
  owner.

### Phase 2 — retained scene and renderer services

- publish entity-keyed scene deltas and share geometry/GPU assets;
- separate device lifecycle, camera/input, picking and overlay composition from
  drawing;
- preserve TLAS/per-mesh BVH traversal while allowing incremental refit;
- remove verified overlay, bounds, uniform-update and visibility hot paths;
- add instancing, culling and explicit transparency quality tiers.

### Phase 3 — workspace and editor

- replace the textarea with a syntax-aware editor and structured diagnostics;
- evolve the active-head IndexedDB store and recovery journal into a versioned
  virtual filesystem with browsable recovery/history snapshots;
- preserve the explicit oracle boundary while evolving the independent project
  VFS into the browser workspace filesystem;
- port selected feature-branch capabilities through documented adapters, never
  by merging its competing architecture wholesale.

### Phase 4 — product quality gates

- add browser Worker/WebGPU lifecycle and accessibility integration tests;
- add deterministic visual fixtures, language conformance/fuzz inputs and
  performance budgets;
- measure rebuild phases, transfer size, GPU allocation, overlays and picking;
- require architecture/subset docs to change with behavior.

## Further reading

- [Seven-role review and branch-convergence plan](docs/review-of-main-rewrite.md)
- [Repository benchmark](docs/research/top-100-repositories.md)
- [Second independent repository benchmark](docs/research/top-100-repositories-second.md)
- [Plasticity interaction patterns](docs/research/plasticity-patterns.md)
- [Geometry-pipeline literature review](docs/research/geometry-pipeline-literature.md)
- [CAD/HCI literature review](docs/research/cad-hci-literature.md)
- [Ten-agent MCP synthesis](docs/research/mcp-ten-agent-review.md)
