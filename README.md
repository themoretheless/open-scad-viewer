# OpenSCAD Viewer

A client-side OpenSCAD workspace built with Vue, WebGPU, and the
[Manifold](https://github.com/elalish/manifold) geometry kernel. It is designed
as a fast, independent subset viewer: supported operations produce
real geometry, while unsupported OpenSCAD syntax returns a line/column error
instead of a misleading preview.

## Highlights

- Real manifold `union()`, `difference()`, `intersection()`, and `hull()`.
- Variables, expressions, ranges, `for`, `if`, `let`, user modules,
  `children()`, and fail-fast OpenSCAD statement-form
  `assert(condition, message)`.
- 3D primitives, 2D shapes, `linear_extrude()`, `rotate_extrude()`,
  `projection()`, transforms, colors, and polyhedra.
- Versioned preview → full geometry compilation in a warm dedicated Worker,
  with latest-result publication, hard preemption of superseded synchronous
  work, source/AST/tessellation budgets, and transferable geometry artifacts.
- Stable source-operation and evaluated-entity identities preserve selection,
  isolation, and visibility across preview/full builds and safe source edits.
- Z-up WebGPU viewport with scene-AABB + triangle-BVH preselection/picking, point/face/
  body selection modes, focus/isolate/hide, shaded/semantic-edge/x-ray display,
  bounded front-to-back click cycling, an interactive view cube, fit/reset,
  Previous View history, standard views, grid control, perspective/orthographic
  projection, and event-driven redraw.
- Plasticity-inspired fuzzy/contextual command palette with RU/EN aliases and
  MRU, Scene Outliner, Inspect workspace, bounded source ↔ CSG cross-highlighting,
  two-point measurements, virtual section-plane clipping, and viewport-scoped shortcuts.
- Lightweight OpenSCAD Customizer controls for top-level literal variables,
  including `// [min:step:max]` sliders and choice lists.
- Binary STL and OBJ export with object transforms baked into the result.
- Optional local MCP server with typed OpenSCAD tools/resources and a
  persistent DuckDB catalog for models, revisions, builds, and bounded exports.
- Open/save/drag-and-drop `.scad` files, shareable source links, and a validated
  IndexedDB workspace with ordered autosaves plus a synchronous crash-recovery
  journal per live tab, each carrying its exact causal IDB base. UI preferences
  remain in `localStorage`; cross-tab conflicts require an explicit IndexedDB/local-draft
  choice instead of last-writer-wins. RU/EN UI, light/dark themes, and a
  resizable workspace are included.
- Strict diagnostics and tests for booleans, transforms, modules, loops,
  extrusion, projection math, and all bundled examples.

## Run locally

Requirements: Node.js 20.19+ and a browser with WebGPU.

```bash
npm ci
npm run dev
```

Quality gate:

```bash
npm run check
```

Production preview:

```bash
npm run build
npm run preview
```

## MCP and DuckDB

The optional Node-side MCP server exposes the same strict OpenSCAD compiler
without adding a backend to the browser app. It uses stdio and stores its
catalog in DuckDB:

```bash
npm run --silent mcp
# or choose the database explicitly
npm run --silent mcp -- --db /absolute/path/.open-scad-viewer.duckdb
# ephemeral test session
npm run --silent mcp -- --memory
```

`--silent` keeps npm output away from MCP's stdout JSON-RPC channel. The
database defaults to `.open-scad-viewer.duckdb` in the server working directory;
`OPENSCAD_VIEWER_DUCKDB` provides another default path.

Example MCP client configuration (replace the repository path):

```json
{
  "mcpServers": {
    "open-scad-viewer": {
      "command": "npm",
      "args": [
        "--prefix",
        "/absolute/path/to/open-scad-viewer",
        "run",
        "--silent",
        "mcp",
        "--",
        "--db",
        "/absolute/path/to/open-scad-viewer/.open-scad-viewer.duckdb"
      ]
    }
  }
}
```

Available tools:

- `openscad_save_model`, `openscad_get_model`, `openscad_list_models`, and
  `openscad_list_model_revisions` —
  model catalog with source-sensitive revisions and optimistic revision checks.
  A revision increases only when source changes; name-only updates keep the same
  revision, and `expected_revision` guards that source revision;
- `openscad_check` — read-only preview/full validation that does not add a build
  record; saved inputs can be pinned to an immutable `revision`;
- `openscad_compare` — read-only metric, dimension, and topology deltas between
  two inline sources or saved revisions (not an exact geometric boolean diff);
- `openscad_analyze` — full/preview compilation with metrics, bounds, topology,
  source provenance, and Customizer metadata; results are recorded as builds;
- `openscad_customize` — validated parameter replacement without an implicit
  database write;
- `openscad_customize_model` — one-call Customizer update and model save guarded
  by `expected_revision`;
- `openscad_export` — bounded full-quality STL/OBJ export returned as an MCP
  resource link and persisted in DuckDB;
- `openscad_build_history` — recent DuckDB-backed build results;
- `openscad_catalog_stats` — model/revision/build/artifact counts, stored bytes,
  build outcomes, and current retention limits without exposing SQL.

Saved sources (including immutable revisions), build summaries, artifacts, and
bundled examples are also available under `openscad://models/...`, `openscad://builds/...`,
`openscad://artifacts/...`, and `openscad://examples/...` resources. Arbitrary
SQL is deliberately not exposed. DuckDB external access and extension
autoload/install are disabled in the server process.

`openscad://capabilities` describes the supported OpenSCAD subset, protocol
versions, active wire/work limits, and the deliberate IndexedDB/DuckDB boundary.
The server also advertises `openscad_review_model` and
`openscad_customize_workflow` prompts plus concise usage instructions. It serves
both MCP `2026-07-28` (`server/discover`) and legacy clients; modern discovery,
tools, prompts, and immutable resources carry conservative cache hints.
The MCP server SDK is pinned exactly to `2.0.0`: request lifecycle accounting
uses its current outer-dispatch registry seam, including handlers installed by
the modern stdio host after server creation, and exact wire tests guard that
integration before any SDK upgrade.
Expected failures use stable machine-readable codes such as
`model_not_found`, `revision_conflict`, `source_syntax_error`,
`artifact_too_large`, `quota_exceeded`, and `server_busy`. Unexpected failures
are redacted and include a correlation ID.

The catalog applies logical retention: at most 500 models, 256 source revisions
per model / 5,000 overall, and 64 MiB of saved revision source; the latest 500
unreferenced builds; and at most 100 artifacts / 64 MiB of artifact data. One
artifact is capped at 6 MiB (4 MiB by default) so
its base64 MCP resource remains within the stdio frame budget. Old artifacts
and unreferenced builds are pruned transactionally; DuckDB may retain allocated
pages for reuse, so long-running installations should still monitor the file.
On POSIX systems the database and WAL are hardened to owner-only permissions.
Use `--memory` for fully disposable sessions.

The stdio host admits at most eight concurrent MCP requests, serializes writes,
keeps cancelled work admitted until its actual handler settles, permits at most
eight modern subscriptions, and keeps separate bounded queues for normal output
and small overload replies. String JSON-RPC request IDs are capped at 128
characters so overload responses cannot amplify an attacker-sized ID.
Inbound notifications are coalesced to the single initialization event and one
cancellation per active request or subscription; unrelated notifications and
client-originated responses are dropped before they can accumulate in the SDK.
Within it, at most eight headless geometry requests may be active or queued;
additional calls fail fast with retry guidance. These limits bound request and
response accumulation, but a single synchronous Manifold kernel call still
cannot be preempted. A worker-thread watchdog remains the next hard-isolation
step for hostile workloads.

## Controls

- Left drag: orbit.
- Right drag or Shift + left drag: pan.
- Mouse wheel: zoom.
- Hover: preselect the nearest surface through the mesh BVH.
- Click without dragging: select the exact surface and its source operation;
  repeat at the same screen point to cycle front-to-back through up to 32 targets.
- Move the editor caret into a geometry call, or hover/focus its Outliner source
  row, to highlight that operation's surviving triangles in the viewport.
- `1`, `3`, `4`: point, face, and body selection modes.
- `Shift + M`: cycle selection modes (`Tab` keeps normal keyboard focus traversal).
- `F` or `/`: focus the selection, or fit the model when nothing is selected.
- `.`: isolate/unisolate the selected object.
- `H`: hide the selected object; use the Outliner to show it again.
- `Esc`: clear selection.
- `E`, `X`, `G`: toggle mesh edges, x-ray, and grid.
- `5`: toggle perspective/orthographic projection.
- `[`: restore the previous camera view (camera actions, drag gestures, and
  coalesced wheel-zoom bursts share a bounded history).
- Numpad `0`, `1`, `3`, `7`: isometric, front, right, and top views.
- `Ctrl/⌘ + =`: start/cancel a two-point measurement.
- `Shift + F`: flip an active section plane.
- `Ctrl/⌘ + K`: open the command palette.
- `Ctrl/⌘ + Shift + B`: toggle the Scene/Inspect/Parameters dock.
- `Ctrl/⌘ + Enter`: render.

## Supported subset and limits

This project does **not** bundle the official OpenSCAD compiler. The official
WASM runtime is the best route to full OpenSCAD compatibility, but bundling it
introduces GPL-2.0+ licensing requirements. This viewer instead uses the
Apache-2.0 `manifold-3d` package and implements a strict language subset.

Currently unsupported features include `include`/`use`, user functions,
expression-form `assert(condition) value`, `import()`, `surface()`, `text()`,
Minkowski operations, and advanced OpenSCAD Customizer annotations. These fail
explicitly. Complexity is bounded to protect the browser: source length, AST
size, parse/evaluation depth, evaluated-value allocation, range size, object
count, `$fn`, and final triangle count all have limits.

## Architecture

- `src/core/mesh.ts` and `src/core/build.ts`: renderer-neutral geometry,
  identity, provenance, transfer, and build-quality contracts.
- `src/services/openscadParser.ts`: lexer, expression/statement parser,
  evaluator, Manifold geometry conversion, diagnostics, and budgets.
- `src/services/buildCoordinator.ts`: protocol-v3 jobs, preview/full ordering,
  stale-result rejection, cancellation, and Worker replacement.
- `src/services/workspaceDocument.ts`: validated, migratable single-document
  snapshot contract with separate monotonic geometry and persistence revisions.
- `src/services/workspaceIndexedDb.ts`: browser-only active-workspace repository
  with runtime validation, compare-and-swap commits, and bounded open failure.
- `src/services/workspacePersistence.ts`: pre-mount hydration, legacy migration,
  ordered autosaves, IndexedDB fallback, CAS conflict detection, and atomic
  per-writer synchronous recovery envelopes.
- `src/services/workspaceShare.ts`: bounded URL-safe source-link codec.
- `src/workers/geometry.worker.ts`: asynchronous compilation and transferable
  protocol-v3 geometry results, including the required preview-reduction flag.
- `src/services/webgpuRenderer.ts`: WebGPU resource lifecycle, lighting,
  camera, BVH picking/preselection, measurement and section overlays, grid,
  input, resize, and event-driven rendering.
- `src/services/meshBvh.ts`: compact transferable triangle BVH and raycast.
- `src/services/sceneAabbIndex.ts`: scene-level AABB hierarchy for rejecting
  whole bodies before exact triangle traversal.
- `src/services/meshTopology.ts`: boundary/crease/non-manifold edge extraction.
- `src/services/meshInspection.ts`: provenance, bounds, hit, and measurement
  helpers.
- `src/services/scenePublication.ts`: pure identity-safe reconciliation of
  visibility, selection, isolation, and measurement continuity between builds.
- `src/services/cameraHistory.ts` and `src/services/rendererRecoveryGate.ts`:
  testable navigation-history snapshots and bounded device-loss recovery policy.
- `src/services/commandRegistry.ts`: one typed inventory for command-palette
  metadata and scope-aware keyboard routing.
- `src/components/`: command palette, ViewCube, Scene Outliner, Inspect, and
  Customizer panels.
- `src/mcp/`: optional stdio MCP host, headless analysis/export facade, and the
  Node-only DuckDB repository. It is excluded from the browser TypeScript/Vite
  graph and checked separately by `tsconfig.mcp.json`.
- `src/App.vue`: workspace UI, file actions, settings, stale/error state, and
  viewer controls.
- `tests/`: mathematical and geometry golden tests plus protocol, contract-
  boundary, publication, visibility, camera, and recovery unit tests.

The implementation priorities came from two timestamped, non-overlapping
comparisons covering 200 OpenSCAD, CAD, mesh, and 3D-viewer repositories. See
[the benchmark](docs/research/top-100-repositories.md).
The follow-up UI pass is documented in
[Plasticity interaction patterns](docs/research/plasticity-patterns.md).
The second independent benchmark adds
[100 non-overlapping repositories](docs/research/top-100-repositories-second.md),
while the [geometry literature review](docs/research/geometry-pipeline-literature.md),
[CAD/HCI review](docs/research/cad-hci-literature.md), and
[prioritized roadmap](docs/research/second-pass-recommendations.md) document the
next evidence-backed implementation candidates. A third pass adds
[10 ideas with distinct user outcomes](docs/research/ten-new-ideas-third-pass.md)
for model contracts, dimensional linting, printability, profiling, multi-view
inspection, tolerance analysis, reproducible exports, assembly interference, automatic
failure reduction, and parameter galleries. A subsequent
[ten-agent adversarial review](docs/research/ten-agent-idea-review.md) separates
the next safe slices from ideas that need stronger mathematical or architectural
prerequisites. The MCP sidecar has its own
[ten-agent protocol/product/security synthesis](docs/research/mcp-ten-agent-review.md).

## Docs

- [architecture.md](architecture.md) — the current module map, worker/kernel design, and active technical debt.
- [recommendation.md](recommendation.md) — the live prioritized backlog (P0 correctness → P3 process).
- [docs/review-of-main-rewrite.md](docs/review-of-main-rewrite.md) — the 7-role panel review of this rewrite: what was fixed immediately, what remains, and the convergence plan with the feature branch (`claude/top-issues-architecture-sync-00p2q9`).
- [docs/research/mcp-ten-agent-review.md](docs/research/mcp-ten-agent-review.md) — ten MCP reviews, implemented decisions, rejected scope, and the ranked next stage.
