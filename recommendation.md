# Engineering debt register

This is the living, prioritized debt register for OpenSCAD Viewer. It replaces
the historical hundreds-of-items wishlist, which mixed completed work, product
ideas and obsolete prototype findings. Architecture and invariants live in
[architecture.md](architecture.md). Evidence also comes from the
[seven-role rewrite review](docs/review-of-main-rewrite.md) and the research
under [`docs/research`](docs/research).

## Status vocabulary

- **Done**: implemented with focused regression coverage.
- **In progress**: a usable foundation exists; listed acceptance remains.
- **Open**: no complete implementation exists.
- **Decision**: product/licensing direction must precede implementation.

P0 constrains correctness or further architecture, P1 constrains scalability or
core product work, and P2 is important hardening/UX.

## Completed foundation

Reopen these only for a demonstrated regression.

| Status | Capability | Evidence |
| --- | --- | --- |
| Done | Real CSG and geometry metrics | Manifold-backed evaluator and geometry tests in [`openscadParser.ts`](src/services/openscadParser.ts) and [`openscadParser.test.ts`](tests/openscadParser.test.ts). |
| Done | Panel-review geometry correctness fixes | Polyhedron winding/merge, EvenOdd polygon fill, positive/positioned extrusion and positioned revolution failures have regressions. |
| Done | Input/resource hardening | Source/AST/depth/shape/triangle limits plus evaluation-step, evaluated-value, `concat`/`str`, extrusion-slice and pre-decode share-hash caps. |
| Done | Customizer and STL boundary fixes | Structural literal offsets prevent source corruption; binary STL headers truncate encoded UTF-8 to 80 bytes. |
| Done | Protocol-v2-only geometry builds | Runtime-validated messages and transfer payloads, warm same-revision preview→full, latest-only publication, explicit stale/cancel states and tested hard preemption in [`buildCoordinator.ts`](src/services/buildCoordinator.ts). |
| Done | Stable document/build identity | Versioned snapshots in [`workspaceDocument.ts`](src/services/workspaceDocument.ts) and revision/job envelopes. |
| Done | Stable operation/evaluated-entity identity | Structural `SourceOperationId`, dynamic `SceneEntityId`, exact spans and ambiguity-safe replacement matching. |
| Done | Neutral mesh/build contract seam | [`src/core/mesh.ts`](src/core/mesh.ts) and [`src/core/build.ts`](src/core/build.ts) own contracts and deduplicated transfer discovery; a test keeps parser value imports Worker-only. |
| Done | Tested replacement-scene continuity | [`scenePublication.ts`](src/services/scenePublication.ts) fail-closes ambiguity and preserves visibility/selection/isolation only when identity is safe. |
| Done | Two-level accelerated picking | Scene AABB hierarchy plus lazy exact traversal of per-mesh triangle BVHs. |
| Done | Renderer lifecycle foundation | Typed lifecycle events, one bounded frame retry, coalesced device loss with one follow-up, false-ready prevention, and App-driven scene/camera/history rehydration with focused tests. |
| Done | One declarative command inventory | [`commandRegistry.ts`](src/services/commandRegistry.ts) drives palette metadata and scope-aware shortcuts. |
| Done | Repeatable quality gate | Typecheck, Vitest and production build run through `npm run check` in CI. |

## Active register

### R1 — Make cancellation cooperative inside compilation

- **Priority/status:** P0 / In progress
- **Evidence:** `BuildCoordinator` keeps same-revision preview/full work on a
  warm Worker and safely supersedes revisions. A synchronous Manifold call
  cannot observe messages, so the configured grace still ends in Worker
  replacement when no checkpoint is reached.
- **Risk:** rapid edits of heavy models repeatedly discard initialized WASM and
  completed intermediate work.
- **Acceptance remaining:** cancellation checkpoints between evaluator/kernel/
  analysis phases; depth-one latest-work queue; phase timings; hard replacement
  only as watchdog; real-browser tests for App↔Worker publication, supersession,
  crash and disposal.

### R2 — Avoid unconditional duplicate builds

- **Priority/status:** P0 / Open
- **Evidence:** each edit schedules a reduced preview and an unconditional full
  pass even when preview quality did not alter tessellation.
- **Risk:** parse/evaluation/kernel/transfer work can run twice for no visible or
  export difference.
- **Acceptance:** preview reports whether quality clamping changed geometry;
  skip or reuse the full pass when equivalent; content/revision/quality cache
  rules are explicit and covered by timing/correctness tests.

### R3 — Split compiler and kernel phases

- **Priority/status:** P0 / In progress
- **Evidence:** lexer/parser, scope evaluation, direct Manifold calls,
  tessellation, provenance, topology and BVH construction remain in
  [`openscadParser.ts`](src/services/openscadParser.ts). Neutral mesh/build
  contracts and transfer discovery now live under [`src/core`](src/core), and
  an import-boundary test prevents main-thread parser value imports.
- **Risk:** language, kernel and inspection changes invalidate the entire
  pipeline and main-thread modules depend on a Worker implementation detail.
- **Acceptance:** typed parse→bind/diagnose→immutable operation IR→kernel→
  tessellation→analysis phases; preserve the neutral core contracts;
  `GeometryKernel` interface with positioned error normalization; import guard
  against main-thread parser/kernel value imports; phase tests/timing and
  content-addressed subtree caching.

### R4 — Establish one owner for scene and viewport state

- **Priority/status:** P0 / In progress
- **Evidence:** App owns the CPU scene and mirrors renderer-owned selection,
  isolation, visibility, projection and camera-related state. Replacement-scene
  continuity is now pure and tested in
  [`scenePublication.ts`](src/services/scenePublication.ts), but App still
  applies the plan to both Vue and renderer owners.
- **Risk:** every rebuild/recovery manually reconstructs interaction state and
  new tools can create synchronization bugs.
- **Acceptance:** typed document/build/scene/viewport controllers have explicit
  ownership; renderer consumes state/deltas and emits intents; reconciliation
  is a pure tested service; App is composition rather than workflow logic; no
  mirrored writable state exists without a documented adapter.

### R5 — Separate scene entities from geometry artifacts

- **Priority/status:** P1 / In progress
- **Evidence:** stable entity IDs exist, but `MeshData` eagerly combines render
  vertices, topology, provenance, semantic edges and BVH; transforms are
  generally baked and array position is still a common render/UI index.
- **Risk:** every build transfers and retains every artifact, repeated geometry
  cannot be instanced, and inspection work cannot be deferred.
- **Acceptance:** entity-keyed scene nodes reference immutable geometry assets;
  material/transform/visibility are independent; artifacts are versioned and
  lazily requestable; preview/full preserve entity identity.

### R6 — Retain GPU resources and remove verified interaction hot paths

- **Priority/status:** P1 / In progress
- **Evidence:** `setMeshes()` uploads a complete replacement and destroys old
  buffers; bounds are rescanned; face/source overlays scan mesh data and create
  buffers; style changes rewrite every object uniform. Visibility restoration
  now uses one batched transaction for bounds, TLAS invalidation, overlays and
  render scheduling.
- **Risk:** edit and hover latency scale with the full scene and cause avoidable
  allocation/queue pressure.
- **Acceptance:** entity/content-keyed GPU cache and scene deltas; worker-cached
  bounds and faceId→triangle lookup; persistent bounded overlay buffers; dirty
  uniform writes; retain the batch visibility API; incremental TLAS rebuild/refit;
  benchmarks for all paths.

### R7 — Decompose the renderer without changing interaction behavior

- **Priority/status:** P1 / In progress
- **Evidence:** [`cameraHistory.ts`](src/services/cameraHistory.ts) and
  [`rendererRecoveryGate.ts`](src/services/rendererRecoveryGate.ts) now provide
  adapter-free tested contracts for navigation history and bounded recovery.
  The main lifecycle workflow remains in App, while
  [`webgpuRenderer.ts`](src/services/webgpuRenderer.ts) still owns device
  lifecycle, pipelines/resources, camera/input, picking, visibility, selection,
  measurements, sections and overlays.
- **Risk:** recovery, interaction and draw changes share broad mutable state and
  are difficult to test independently.
- **Acceptance:** device/surface manager, retained render world, camera/input,
  picker and overlay composer have narrow contracts and one resource owner;
  lifecycle/camera/picker tests run without a real adapter; existing Plasticity-
  inspired controls and invalidation-driven redraw remain stable.

### R8 — Decide compatibility before multi-file semantics

- **Priority/status:** P1 / Decision
- **Evidence:** the strict subset excludes `include`, `use`, imports and user
  functions; the workspace is single-document.
- **Risk:** piecemeal project support can look OpenSCAD-compatible while scopes,
  paths and dependency behavior differ.
- **Acceptance:** record official-runtime/GPL compatibility versus a versioned
  independent language; publish a conformance contract; then define virtual
  filesystem, dependency resolution and sandbox rules.

### R9 — Build reliable storage and a structured editor/workspace

- **Priority/status:** P1 / Open; multi-file portion blocked on R8
- **Evidence:** persistence validates/version-tags one localStorage snapshot and
  migrates `scad-code`, but write/quota failures are returned and ignored. The
  editor is a textarea and diagnostics are not decorations.
- **Risk:** autosave can fail silently; customizer replacements disrupt native
  undo; source↔geometry navigation remains caret-based.
- **Acceptance:** quota-aware storage with visible failure/recovery, migrations
  including feature-branch `scad-tabs`, IndexedDB snapshots, syntax-aware editor,
  structured diagnostics/navigation and source-splice edits preserving undo.

### R10 — Close verified interaction and accessibility gaps

- **Priority/status:** P2 / Open
- **Evidence:** Outliner emits additive/range selection that App drops; ViewCube
  is static and does not snap face views to orthographic; camera input tracks one
  pointer; several dock/outliner semantics and small targets remain weak.
- **Risk:** controls advertise behavior they do not provide, and mobile/keyboard
  workflows are incomplete.
- **Acceptance:** implement or remove multi-select affordances; camera-synced
  ViewCube with ortho snap and adequate targets; pinch/pan plus keyboard camera;
  accessible tree/tablist/live-result semantics; resizable side-by-side
  Scene/Inspect workflow.

### R11 — Add conformance, browser and performance gates

- **Priority/status:** P1 / Open
- **Evidence:** CI runs typecheck, unit tests and build, but no real-browser
  Worker/WebGPU flow, visual comparison, parser fuzz corpus, coverage/lint or
  performance budget.
- **Risk:** GPU lifecycle, focus/layout, pathological input and latency
  regressions can pass unit tests.
- **Acceptance:** language conformance fixtures and bounded generated inputs;
  Playwright interaction/accessibility smoke; deterministic visual fixtures;
  lint/import-boundary and meaningful coverage rules; budgets for rapid edits,
  1,000 bodies, 750,000 triangles, transfer, overlays, TLAS query and GPU upload.

### R12 — Define transparency and backend quality tiers

- **Priority/status:** P2 / Decision
- **Evidence:** transparent bodies are sorted back-to-front, which is adequate
  for separated bodies but not intersecting transparent triangles; WebGPU has
  no fallback.
- **Risk:** overlapping x-ray geometry can be visually wrong and unsupported
  browsers cannot display the model.
- **Acceptance:** document a WebGPU-only support tier or add a tested fallback;
  expose the transparency limitation; if correctness is required, implement and
  visually test weighted OIT, depth peeling or another explicit strategy.

### R13 — Converge feature work without merging competing architecture

- **Priority/status:** P2 / Open
- **Evidence:** the frozen feature branch contains SafeStorage, four-locale i18n,
  multi-tab/undo/find, exporters/import, examples, PWA/offline and tests, but was
  built around a competing large-component architecture. The review identifies
  this rewrite as the structural base.
- **Risk:** a wholesale merge restores resolved coupling and storage-schema
  conflicts; ignoring the donor loses tested user value.
- **Acceptance:** maintain a parity checklist; port leaf infrastructure and
  tests through current contracts one feature per change; ship one-shot storage
  migration; freeze and retire the donor branch after parity decisions.

## Update rule

Any change that materially affects an item must update its status, evidence or
acceptance criteria here. Completed items move to the foundation table.
Speculative product ideas belong in research or an issue tracker, not this file.
