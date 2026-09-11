# Engineering debt register

This is the living, prioritized debt register for OpenSCAD Viewer. It replaces
the historical hundreds-of-items wishlist, which mixed completed work, product
ideas and obsolete prototype findings. Architecture and invariants live in
[architecture.md](../architecture.md). Evidence also comes from the
[seven-role rewrite review](review-of-main-rewrite.md) and the research
under [`docs/research`](research).

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
| Done | Real CSG and geometry metrics | Handle-only kernel evaluator and geometry tests in [`openscadParser.ts`](../src/services/openscadParser.ts) and [`openscadParser.test.ts`](tests/openscadParser.test.ts). |
| Done | Panel-review geometry correctness fixes | Polyhedron winding/merge, EvenOdd polygon fill, positive/positioned extrusion and positioned revolution failures have regressions. |
| Done | Input/resource hardening | Source/AST/depth/shape/triangle limits plus evaluation-step, evaluated-value, `concat`/`str`, extrusion-slice and pre-decode share-hash caps. |
| Done | Customizer and STL boundary fixes | Structural literal offsets prevent source corruption; binary STL headers truncate encoded UTF-8 to 80 bytes. |
| Done | Statement assertion foundation | `assert(condition, message)` uses OpenSCAD truthiness, strict argument binding, transparent child evaluation and positioned fail-fast diagnostics across the Worker boundary; expression-form assert remains unsupported. |
| Done | Protocol-v3-only geometry builds | Runtime-validated messages and transfer payloads, required preview-reduction metadata, warm same-revision preview→full, latest-only publication, explicit stale/cancel states and tested hard preemption in [`buildCoordinator.ts`](../src/services/buildCoordinator.ts). |
| Done | Stable document/build identity | Versioned snapshots in [`workspaceDocument.ts`](../src/services/workspaceDocument.ts) and revision/job envelopes. |
| Done | Stable operation/evaluated-entity identity | Structural `SourceOperationId`, dynamic `SceneEntityId`, exact spans and ambiguity-safe replacement matching. |
| Done | Neutral mesh/build contract seam | [`src/core/mesh.ts`](../src/core/mesh.ts) and [`src/core/build.ts`](../src/core/build.ts) own contracts and deduplicated transfer discovery; a test keeps parser value imports Worker-only. |
| Done | Tested replacement-scene continuity | [`scenePublication.ts`](../src/services/scenePublication.ts) fail-closes ambiguity and preserves visibility/selection/isolation only when identity is safe. |
| Done | Two-level accelerated picking | Scene AABB hierarchy plus lazy exact traversal of per-mesh triangle BVHs. |
| Done | Renderer lifecycle foundation | Typed lifecycle events, one bounded frame retry, coalesced device loss with one follow-up, false-ready prevention, and App-driven scene/camera/history rehydration with focused tests. |
| Done | One declarative command inventory | [`commandRegistry.ts`](../src/services/commandRegistry.ts) drives palette metadata and scope-aware shortcuts. |
| Done | Repeatable quality gate | Typecheck, Vitest and production build run through `npm run check` in CI. |
| Done | One-build preview promotion | Auto builds run preview first and request full only when the versioned reduction policy reports a real quality change; a pure policy and byte-level equivalence corpus cover promotion and stale targets. A process-local monotonic build generation prevents restored documents from aliasing persistence revisions. |
| Done | Versioned independent language decision | `openscad-viewer-subset@1`, compatibility/contributor policy and third-party notices explicitly exclude bundled official OpenSCAD runtime code. The project VFS is host/network-isolated, Unicode-canonical, traversal-safe and globally budgeted; `include`/`use` remain coded unsupported features until a future contract version defines semantics. |

## Active register

### R1 — Make cancellation cooperative inside compilation

- **Priority/status:** P0 / In progress
- **Evidence:** `BuildCoordinator` keeps same-revision preview/full work on a
  warm Worker, coalesces identical active requests and bounds pending work by
  revision/quality. The evaluator yields between top-level statements, yields
  mid-iteration inside `for` / `intersection_for` at any nesting (module /
  `if` / `let` bodies), yields between nested statements, polls immediately
  before Manifold boolean/hull/difference, forces a macrotask checkpoint
  before extraction, polls cancellation through chunked publication work, and
  reports required parse/bind/initialize/evaluate/analyze timings. A tokenized
  watchdog prevents stale timers from cancelling newer work. Synchronous
  Manifold, BVH and topology WASM calls still cannot observe messages, so
  Worker replacement remains the final cancellation boundary for those phases.
- **Risk:** rapid edits of heavy models repeatedly discard initialized WASM and
  completed intermediate work.
- **Acceptance remaining:** cooperative/async kernel, BVH and topology phases
  that yield *inside* WASM; real-browser tests for App↔Worker supersession,
  watchdog recovery, crash and disposal.

### R3 — Split compiler and kernel phases

- **Priority/status:** P0 / In progress
- **Evidence:** lexer/parser, scope evaluation, tessellation, provenance,
  topology and BVH construction previously shared one module and called
  WASM types directly. The pure [`openscadCompiler.ts`](../src/services/openscadCompiler.ts)
  now owns tokenization, parsing and stable operation identity and returns a
  deeply frozen, structured-clone-safe operation IR without importing
  the official kernel. [`openscadBinder.ts`](../src/services/openscadBinder.ts) binds that
  IR to builtin/user modules and functions, records positioned unresolved-name
  diagnostics and content-addresses compile+bind by source digest plus
  language profile. [`geometryKernel.ts`](../src/services/geometryKernel.ts)
  defines the lifecycle port and opaque solid/section handles;
  [`manifoldGeometryKernel.ts`](../src/services/manifoldGeometryKernel.ts)
  exclusively owns WASM bootstrap, retry, handle-table and GC-session
  disposal. [`openscadParser.ts`](../src/services/openscadParser.ts) and
  [`svgGeometry.ts`](../src/services/svgGeometry.ts) evaluate through opaque
  handle-only kernel ops; tessellation uses `analyzeSolid`. Boolean/hull
  and primitive constructors normalize kernel failures to source positions.
  Compiler phase timings report `bindMs` separately from `parseMs` on the
  protocol-v6 Worker wire. The public facade and serialized lifetime remain
  compatible and parity-covered.
- **Risk:** language, kernel and inspection changes invalidate the entire
  pipeline and main-thread modules depend on a Worker implementation detail.
- **Acceptance remaining:** yield and position every deferred kernel error;
  subtree-level IR cache beyond the whole-source compile/bind digest.

### R4 — Establish one owner for scene and viewport state

- **Priority/status:** P0 / In progress
- **Evidence:** App owns the CPU scene and mirrors renderer-owned selection,
  isolation, visibility, projection and camera-related state. The canonical
  CPU-side meshes/visibility/selection/isolation/hover/measurement/section
  snapshot now lives in [`sceneController.ts`](../src/services/sceneController.ts);
  [`viewportController.ts`](../src/services/viewportController.ts) owns camera,
  face-preset projection, history availability and recovery-revision fencing.
  Vue exposes computed projections and renderer callbacks are typed intents
  into those owners. Replacement-scene publication commits atomically through
  the scene controller, hidden selections and hovers fail closed, and camera
  face presets apply orientation plus projection as one history action.
  Renderer GPU resources remain a deliberate disposable projection of semantic
  state.
- **Risk:** every rebuild/recovery manually reconstructs interaction state and
  new tools can create synchronization bugs.
- **Acceptance remaining:** renderer consumes explicit state deltas and emits
  generation-tagged intents; App becomes composition rather than remaining
  workflow glue around the two controllers.

### R5 — Separate scene entities from geometry artifacts

- **Priority/status:** P1 / In progress
- **Evidence:** stable entity IDs exist, but `MeshData` eagerly combines render
  vertices, topology, provenance, semantic edges and BVH. A versioned neutral
  [`GeometryScene`](../src/core/scene.ts) contract now separates content-addressed
  vertex/index assets from entity identity, transform, material and explicitly
  staged inspection artifacts. Zero-copy legacy adapters preserve current wire
  behavior, shared tessellation transfers once, and validators reject duplicate
  entities/assets, dangling references, invalid indices and unsafe partial
  backing-buffer views.
- **Risk:** every build transfers and retains every artifact, repeated geometry
  cannot be instanced, and inspection work cannot be deferred.
- **Acceptance remaining:** migrate Worker protocol, renderer, exporters and
  MCP compilation to the normalized scene contract; key selection/picking by
  entity rather than array index; retain/refcount GPU assets; implement the
  artifact request/coalescing provider and preview/full asset reuse policy.

### R6 — Retain GPU resources and remove verified interaction hot paths

- **Priority/status:** P1 / In progress
- **Evidence:** `setMeshes()` uploads a complete replacement and destroys old
  buffers unless a verified `geometryAssetId` matches. The renderer now retains
  and shares exact VB/IB payloads across replacement publications/instances,
  rolls back failed entity staging without touching live geometry, deduplicates
  destruction, and has fake-GPU allocation regressions. Style writes were
  already dirty-tracked and overlay slots capacity-retained; growth now swaps
  only after successful allocation/upload. Ordinary selection changes no longer
  rebuild source overlays unless isolation changed effective visibility.
- **Risk:** edit and hover latency scale with the full scene and cause avoidable
  allocation/queue pressure.
- **Acceptance remaining:** explicit refcounted/LRU asset cache and scene deltas;
  share lazy edge buffers; worker-cached bounds; typed-array overlay staging;
  capacity-retained measurement buffer; incremental TLAS rebuild/refit and
  counter/browser benchmarks for 1,000 entities and deep picking.

### R7 — Decompose the renderer without changing interaction behavior

- **Priority/status:** P1 / In progress
- **Evidence:** [`cameraHistory.ts`](../src/services/cameraHistory.ts) and
  [`rendererRecoveryGate.ts`](../src/services/rendererRecoveryGate.ts) now provide
  adapter-free tested contracts for navigation history and bounded recovery.
  Neutral public types now live in
  [`rendererContracts.ts`](../src/services/rendererContracts.ts), canonical view
  orientation/projection in [`viewportModel.ts`](../src/services/viewportModel.ts),
  and orbit/pan/pinch/wheel math in
  [`cameraGestures.ts`](../src/services/cameraGestures.ts). App and ViewCube no
  longer depend on the WebGPU monolith for model/types, while compatibility
  re-exports preserve callers. The main lifecycle workflow remains in App, while
  [`webgpuRenderer.ts`](../src/services/webgpuRenderer.ts) still owns device
  lifecycle, pipelines/resources, camera/input, picking, visibility, selection,
  measurements, sections and overlays.
- **Risk:** recovery, interaction and draw changes share broad mutable state and
  are difficult to test independently.
- **Acceptance remaining:** device/surface manager, retained render world,
  camera history controller, DOM input adapter,
  picker and overlay composer have narrow contracts and one resource owner;
  lifecycle/camera/picker tests run without a real adapter; existing Plasticity-
  inspired controls and invalidation-driven redraw remain stable.

### R9 — Build reliable storage and a structured editor/workspace

- **Priority/status:** P1 / In progress; multi-file portion blocked on R8
- **Evidence:** persistence validates/version-tags one localStorage snapshot,
  migrates `scad-code`, journals causal per-tab recovery and commits CAS-guarded
  IndexedDB snapshots. Conflict UI can export the open draft before choosing a
  winner. Worker errors retain stable diagnostic codes and positioned ranges;
  clickable diagnostics focus/select the exact textarea token. Customizer edits
  use stale-safe reversible source-splice transactions and `setRangeText`,
  preserving native undo rather than replacing the whole control value.
- **Risk:** a failed write is visible but recovery is not actionable; customizer
  replacements disrupt native undo; source↔geometry navigation remains
  caret-based.
- **Acceptance remaining:** full conflict review/diff and bounded losing-copy
  backups; typed sequential migration outcomes including unsupported-newer;
  syntax highlighting/completion and multi-file tabs after VFS semantics ship;
  multi-diagnostic decorations and bounded explicit undo/redo history.

### R10 — Close verified interaction and accessibility gaps

- **Priority/status:** P2 / In progress; verified accessibility foundation shipped
- **Evidence:** additive/range affordances were removed until the scene model can
  own multi-selection. Camera gestures support two-pointer pinch plus midpoint
  pan; face presets atomically snap to orthographic, and the live camera drives
  the ViewCube compass. The dock now implements tablist/tab/tabpanel semantics,
  wrapping arrow/Home/End navigation and close-focus restoration. Outliner mesh
  rows expose tree/treeitem/group structure, single selection, expansion and a
  stable roving tab stop; shortcut help is screen-reader-visible and coarse
  pointer targets expand to 44 px. Splitter pointer, keyboard, persisted and
  responsive bounds share one tested policy, expose a dynamic ARIA maximum and
  stop on lost pointer capture.
- **Risk:** controls advertise behavior they do not provide, and mobile/keyboard
  workflows are incomplete.
- **Acceptance remaining:** identity-based object multi-selection across scene,
  renderer and publication; keyboard orbit/pan/dolly; a truly camera-projected
  cube (or honest preset/compass naming); bounded live-result announcements;
  browser/AT verification and a resizable side-by-side Scene/Inspect workflow.

### R11 — Add conformance, browser and performance gates

- **Priority/status:** P1 / In progress; deterministic PR gates shipped
- **Evidence:** CI now cancels superseded runs and executes a version-pinned,
  team-authored language corpus whose positive/negative feature coverage must
  exactly match the public contract. A fixed-seed 256-case compiler mutation
  gate accepts only clone-safe IR or positioned domain diagnostics. Deterministic
  scale tests exercise a 1,000-body scene index and the zero-copy 750,000-
  triangle transfer path. Worker success payloads receive allocation-free
  mesh/triangle/byte/warning preflight before deep graph validation. Production
  builds verify required HTML/CSS/JS/WASM artifacts and generous size ceilings.
- **Risk:** GPU lifecycle, focus/layout, pathological input and latency
  regressions can pass unit tests.
- **Acceptance remaining:** Playwright interaction/accessibility and real Worker
  smoke after a deterministic no-GPU shell/test-renderer seam; deterministic
  visual fixtures; measured coverage/lint baselines; operation counters for
  protocol validation, BVH/TLAS/overlays and retained GPU uploads; non-blocking
  benchmark telemetry before any wall-clock threshold becomes a PR gate.

### R12 — Define transparency and backend quality tiers

- **Priority/status:** P2 / In progress; honest tiers and degraded shell shipped
- **Evidence:** an immutable UI-facing backend contract distinguishes full CPU
  geometry builds from the interactive WebGPU viewport and labels X-ray as
  approximate object-sorted alpha, not OIT. All normalized alpha values below
  1 now use the transparent pass (removing the old 0.99 quality cliff), and the
  Worker protocol rejects RGBA outside [0,1]. If WebGPU/adapter/context startup
  fails, the editor, persistence, Worker builds and exports remain mounted; only
  the viewport shows the exact failure with a bounded retry action, and a retry
  hydrates the latest CPU scene. The active backend tier is visible in the UI.
- **Risk:** overlapping x-ray geometry can be visually wrong and unsupported
  browsers cannot display the model.
- **Acceptance remaining:** extract a pure frame plan and pipeline descriptor
  tests; add pixel-budgeted weighted blended OIT with transactional attachments
  and sorted-alpha downgrade; browser visual fixtures for self-overlap,
  intersections, opaque occlusion and overlays; keep a WebGL/CPU raster backend
  as a separate product decision.
- **Acceptance:** document a WebGPU-only support tier or add a tested fallback;
  expose the transparency limitation; if correctness is required, implement and
  visually test weighted OIT, depth peeling or another explicit strategy.

### R13 — Converge feature work without merging competing architecture

- **Priority/status:** P2 / In progress; executable parity ledger and safe donor discovery shipped
- **Evidence:** `SafeStorage` has been ported through current contracts. The
  donor branch still contains RU/EN plus partial ZH/DE strings, multi-tab/undo/find,
  exporters/import, examples, PWA/offline and tests, but was built around a
  competing large-component architecture. The review identifies this rewrite
  as the structural base.
- **Current evidence:** [`docs/feature-parity.json`](feature-parity.json)
  pins the donor ref and records an explicit port/partial/defer decision with
  current-file evidence for every reviewed capability; CI rejects duplicate or
  evidence-free entries. Audit found the donor does not actually contain a
  complete German UI catalog, so parity no longer repeats that claim. A bounded
  whole-array `scad-tabs` reader can recover the selected donor tab ahead of a
  stale `scad-code`; after durable commit it preserves the original multi-tab
  keys and an exact backup because the current single-document schema cannot
  honestly claim lossless migration of every tab.
- **Imported geometry evidence:** a strict binary-STL boundary validates the
  exact little-endian container and a conservative 250,000-facet allocation
  cap before decoding, rejects non-finite positions, ignores advisory file
  normals, preserves winding, filters only exact degenerates and adapts the
  result into content-addressed geometry with BVH, semantic edges, face IDs and
  null-source provenance. Text STL and malformed/trailing payloads fail with
  stable error codes. File-picker publication remains intentionally gated on a
  worker-backed imported-scene state so late source builds cannot overwrite it.
- **Example-gallery evidence:** browser and MCP now consume one validated,
  stable-ID RU/EN catalog. The editor toolbar opens a searchable modal with
  labelled controls, trapped focus, Escape/backdrop close, focus restoration,
  responsive cards and an explicit cancel/download-current/replace decision.
  Replacement advances source and filename in one workspace mutation and one
  build/persistence generation; the donor's incompatible extension examples
  and parallel registries were not copied.
- **Shortcut-help evidence:** the visible RU/EN keyboard guide is generated
  from the same exact bindings and scopes used by event routing, including
  keyboard-only conditional actions and platform-aware modifier formatting.
  It opens from the toolbar, command palette or `?`, never hijacks editable
  fields, traps and restores focus, and suppresses commands behind open modal
  surfaces. Palette-to-help handoff explicitly prevents stale focus restoration.
- **Theme evidence:** a versioned enum selects System, Dark, Light, Nord or
  Solarized from a closed, complete token catalog. Core text, muted text,
  control boundary, focus and primary-button contrast are checked in CI;
  arbitrary CSS values never cross storage. The palette is applied before the
  asynchronous workspace bootstrap, System follows OS changes without changing
  document/build state, native controls receive the resolved color scheme, and
  the WebGPU clear color follows the same validated canvas token. Forced-colors
  and reduced-motion retain explicit fallbacks. Donor custom-theme JSON remains
  excluded until a bounded seed-color schema and full component contrast audit exist.
- **Risk:** a wholesale merge restores resolved coupling and storage-schema
  conflicts; ignoring the donor loses tested user value.
- **Acceptance remaining:** make every donor tab user-addressable through a
  versioned multi-file workspace before retiring its keys; add typed migration
  receipts/outcomes and recovery UI; port approved leaf features one at a time;
  archive the exact donor SHA under an immutable tag, fix the remote default
  branch (currently points at the donor), decide service-worker retirement, and
  retire the donor only after the ledger has no unreviewed decisions.

### Native P0 — topology lineage and edge selection

- **Priority/status:** P0 / In progress
- **Evidence:** [`topologyLineage.ts`](../src/core/topologyLineage.ts) and
  [`polygon-core` lineage](../crates/polygon-core/src/lineage.rs) record
  persist/split/merge of opaque `TopoId`s, transfer or explicitly lose
  selection, and emit a schema-1 durable snapshot. Native edge and
  control-point references are revision-fenced like faces.
- **Acceptance remaining:** attach lineage to Boolean split/merge products;
  renderer edge/control-point picking; persist snapshots beside native
  geometry artifacts.

### Print P2 — mesh sections to G-code preview

- **Priority/status:** P2 / In progress
- **Evidence:** [`polygon-core` print](../crates/polygon-core/src/print.rs)
  turns indexed mesh sections into concentric walls, sparse hatch infill and
  a parse-back G-code preview with filament-length E. Box and annulus
  regressions cover hole exclusion and volume-flow agreement.
- **Acceptance remaining:** browser/WASM dispatch, machine-profile dialects,
  variable width and a UI layer preview. Does not launch a printer.

## Update rule

Any change that materially affects an item must update its status, evidence or
acceptance criteria here. Completed items move to the foundation table.
Speculative product ideas belong in research or an issue tracker, not this file.
