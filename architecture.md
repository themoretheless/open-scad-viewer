# Architecture

This document describes the architecture that exists in the repository after
the Manifold rewrite, the [seven-role review](docs/review-of-main-rewrite.md),
and the build/scene foundation pass. It is not a feature wishlist. Prioritized
debt and acceptance criteria live in [recommendation.md](recommendation.md).

## Product boundary

OpenSCAD Viewer is a client-only Vue 3 + TypeScript application. Geometry
compilation and rendering stay in the browser; there is no application backend.

The project implements a **strict, independent OpenSCAD subset**. It does not
embed the official OpenSCAD compiler. Supported constructs produce real
geometry through the Apache-2.0 [`manifold-3d`](https://github.com/elalish/manifold)
package. Unsupported syntax must fail with a source diagnostic rather than
produce an approximate or misleading model.

This boundary is intentional:

- full OpenSCAD compatibility would be better served by the official runtime,
  subject to its GPL-2.0+ distribution requirements;
- subset additions must not silently diverge from documented semantics;
- WebGPU is currently the only renderer backend;
- the current workspace contains one `.scad` document and persists locally.

The supported language and limits are documented in
[README.md](README.md#supported-subset-and-limits).

## System map

```text
Vue workspace / file actions / textarea
        │
        ├─ commandRegistry.ts: palette metadata + scoped keyboard routing
        ▼
WorkspaceDocumentSnapshot
(documentId, source, monotonically increasing geometry revision)
        │
        ▼
BuildCoordinator ───── protocol-v2 ordering, cancellation and Worker lifetime
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

## Current components

| Component | Current responsibility |
| --- | --- |
| [`src/App.vue`](src/App.vue) | Workspace composition, file/share/export actions, build publication, renderer recovery, Vue view state and orchestration. |
| [`src/services/workspaceDocument.ts`](src/services/workspaceDocument.ts) | Versioned, validated single-document persistence and source-derived revisions, including migration from the legacy source key. |
| [`src/services/buildCoordinator.ts`](src/services/buildCoordinator.ts) | Protocol-v2 job ordering, latest-result publication, preview/full policy, cancellation, hard preemption and Worker disposal. |
| [`src/services/geometryWorkerProtocol.ts`](src/services/geometryWorkerProtocol.ts) | Versioned and runtime-validated request/event contract: revision, job, quality, phase, progress and terminal state. |
| [`src/services/commandRegistry.ts`](src/services/commandRegistry.ts) | One typed inventory for palette metadata and deterministic, scope-aware keyboard routing. |
| [`src/workers/geometry.worker.ts`](src/workers/geometry.worker.ts) | Isolates compilation, reports phase/checkpoint events, rejects stale work and transfers geometry buffers. |
| [`src/services/openscadParser.ts`](src/services/openscadParser.ts) | Strict lexer/parser/evaluator, identity assignment, Manifold calls, mesh conversion, provenance and resource budgets. |
| [`src/services/meshBvh.ts`](src/services/meshBvh.ts) | Compact transferable per-mesh triangle BVH and exact ray intersection. |
| [`src/services/sceneAabbIndex.ts`](src/services/sceneAabbIndex.ts) | Deterministic scene-level AABB hierarchy that rejects whole bodies before triangle traversal. |
| [`src/services/meshTopology.ts`](src/services/meshTopology.ts) | Boundary/crease/non-manifold edge extraction and topology diagnostics. |
| [`src/services/meshInspection.ts`](src/services/meshInspection.ts) | Identity-aware replacement matching, provenance lookup, bounds, normals and measurements. |
| [`src/services/webgpuRenderer.ts`](src/services/webgpuRenderer.ts) | WebGPU lifecycle, pipelines/resources, camera/input, visibility, picking, overlays, measurements and sections. |
| [`src/components`](src/components) | Command palette, ViewCube, Scene Outliner, Inspect and Customizer presentation components. |

The parser, renderer and App remain broad modules. This table describes real
boundaries, not an assertion that the separation is finished.

## Data contracts and identity

### Workspace revision

`WorkspaceDocumentSnapshot` is the durable document boundary. `documentId`
persists across edits. `revision` increases only when geometry source changes;
filename-only metadata updates persist without invalidating an identical
in-flight build. The revision, not the source string, is the asynchronous build
identity.

### Build identity and publication

Protocol v2 is the only App/Worker path. Every request/event contains a protocol
version, document revision and job ID; build events also carry quality and a
typed phase. Runtime guards validate both envelopes and transferred mesh shape.
The Worker reports accepted, started, progress, succeeded, failed, cancelled
or stale outcomes. `BuildCoordinator` exposes corresponding immutable state,
including an explicit `stale` state.

Only the latest job of each quality tier for the current revision is
publishable. A preview may publish while a full build for the same revision is
pending, but it can never downgrade a published full result. Preview→full for
one revision shares the warm Worker. When a newer revision supersedes running
synchronous work, the coordinator requests cancellation and replaces the
Worker after a configurable grace period if no real checkpoint is reached.
App performs a final revision check before publishing. Export is allowed only
from a full build of the current source.

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

`MeshData` currently packages interleaved positions/normals, triangle and
semantic-edge indices, transform/color, face IDs, compact provenance, topology
diagnostics and a triangle BVH. The Worker transfers the underlying
`ArrayBuffer` objects. Published results are immutable snapshots; the Worker
must not retain transferred buffers.

## Geometry pipeline and safety

The handwritten parser stores token-derived source spans and evaluates the
document into Manifold solids/cross-sections. `union`, `difference`,
`intersection` and `hull` are real kernel operations. Successful inspection
metrics and full-quality exports are derived from the same geometry.

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

## Rendering and interaction

`WebGPURenderer` owns WebGPU resources and reports typed initialization,
unavailability, readiness, device-loss, frame-error and destruction states. A
transient frame failure gets one bounded retry. On device loss, App reinitializes
the renderer and rehydrates the latest CPU scene, visibility, identity-mapped
selection/isolation, camera, measurement and section state. Teardown is
idempotent and invalidates pending initialization and hover work.

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
without rebuilding geometry.

## State ownership

Current ownership is explicit because its last overlap is active debt:

- the workspace snapshot owns source, filename and geometry revision;
- `BuildCoordinator` owns Worker lifetime and in-flight job correlation;
- App owns the published CPU scene and user-facing build/error state;
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

## Current limitations

- This is not full OpenSCAD. `include`/`use`, user functions, imports, text,
  surfaces, Minkowski and advanced Customizer behavior are unsupported.
- Parser, scope/evaluation logic, direct Manifold calls, tessellation and
  artifact construction remain concentrated in one full-rebuild module; there
  is no `GeometryKernel` interface or content-addressed subtree cache.
- Superseding a running synchronous kernel call discards the Worker and warm
  WASM state. Each edit still schedules preview and unconditional full builds.
- `MeshData` makes the parser a type hub and eagerly bundles render, inspection
  and acceleration artifacts.
- Scene publication recreates all GPU mesh resources. Geometry instancing,
  retained entity deltas and shared GPU assets are absent.
- Face/source overlay creation still scans mesh data and allocates GPU buffers;
  style changes update every object uniform, and bounds are rescanned.
- `App.vue` and `webgpuRenderer.ts` retain broad responsibilities and mirror
  selection/visibility state; build-result reconciliation remains in App.
- Persistence is versioned but single-document/localStorage based. Quota/write
  failures are not yet surfaced to the user.
- The editor remains a textarea. Outliner additive/range selection is emitted
  but not implemented; ViewCube does not follow the live camera; touch input is
  single-pointer.
- Transparency is object-sorted alpha blending rather than OIT; WebGPU has no
  fallback backend.
- CI has no real-browser GPU/accessibility smoke, visual regression,
  fuzz/conformance corpus or performance budget.

## Phased target architecture

This is an extraction plan, not a big-bang rewrite.

### Phase 1 — compiler and state seams

- separate parser, binding/diagnostics, immutable operation IR, kernel,
  tessellation and analysis contracts;
- move mesh/protocol types to a neutral core and keep parser/kernel value
  imports Worker-only;
- add cooperative evaluation checkpoints and phase timing while retaining hard
  preemption as a watchdog;
- extract build-result reconciliation and establish one scene/viewport owner.

### Phase 2 — retained scene and renderer services

- publish entity-keyed scene deltas and share geometry/GPU assets;
- separate device lifecycle, camera/input, picking and overlay composition from
  drawing;
- preserve TLAS/per-mesh BVH traversal while allowing incremental refit;
- remove verified overlay, bounds, uniform-update and visibility hot paths;
- add instancing, culling and explicit transparency quality tiers.

### Phase 3 — workspace and editor

- replace the textarea with a syntax-aware editor and structured diagnostics;
- add quota-aware IndexedDB persistence, recovery snapshots and a versioned
  virtual filesystem;
- decide official-runtime compatibility versus a versioned independent language
  before defining multi-file `include`/`use` semantics;
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
