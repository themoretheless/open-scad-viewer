# OpenSCAD Viewer

A client-side OpenSCAD workspace built with Vue, WebGPU, and its own Rust
geometry kernel. The browser is
a fast, independent subset viewer: supported operations produce real geometry,
while unsupported OpenSCAD syntax returns a line/column error instead of a
misleading preview. The optional local MCP server also exposes the
repository-owned `openscad/stable-2021.01` engine while it progresses through a
full language/geometry qualification gate. A separately installed upstream
OpenSCAD runtime is available only as a differential oracle.

## Rust geometry libraries

Two independent domain libraries live in the Cargo workspace under `crates/`:

- `nurbs-core`: rational curves and surfaces, analytic derivatives, knot/degree edits, iso-curves, extrusion, revolution and lofts.
- `polygon-core`: owned triangle meshes, polygonal UV meshing, boundary loops, topology inspection, affine transforms, fixed-vector thickening and STL output. It accepts plain imported meshes and arbitrary parametric samplers, without depending on NURBS.

`geometry-bridge` adapts the two libraries and exposes a shared WASM transport. NURBS surfaces become derived meshes with UV samples; mesh boundary loops become exact degree-one NURBS curves that can construct new surfaces. This does not reconstruct smooth NURBS surfaces from arbitrary meshes. The polygon library implements bounded numerical BSP union, intersection and difference for closed oriented meshes, also exposed as `mesh_boolean` in ModelGraph. A full NURBS B-rep modeler is not implemented. Both ModelGraph and the legacy OpenSCAD route now use the repository-owned Rust CAD kernel; no external CAD runtime or fallback is loaded.

The ModelGraph Text frontend and canonical graph compiler also run in Rust.
The legacy OpenSCAD evaluator and host/renderer adapters remain TypeScript.
The mesh engine is the workspace CAD kernel (`own-rust-cad-v1`) with an exact
WASM SHA-256 fingerprint. A foreign Manifold comparison bench, if needed, lives
only in `tools/manifold-bench` and is not a product dependency.

The CAD implementation includes planar arrangements, offsets, ear-clipped caps,
extrusion/revolution, hulls, bounded BSP booleans, slicing/projection and convex
decomposition for Minkowski sums. It uses floating-point predicates and explicit
budgets; it is not an exact-arithmetic geometry kernel. The UI, serialization,
WASM bindings and build tooling still have dependencies (see `THIRD_PARTY_NOTICES.md`).

Qualification evidence: `docs/qualification/own-rust-cad-v1.json`. Re-run
`node scripts/record-own-cad-evidence.mjs` after rebuilding and validating a changed
Rust artifact. Builds never silently regenerate qualification evidence.
Archived Manifold snapshots are retained separately and do not qualify the new
kernel. Browser/MCP parity remains a separate, pending qualification.

Build prerequisites (in addition to Node.js):

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-opt   # or: brew install binaryen
npm run build:geometry
npm run test:geometry
```

`wasm-opt` is required: every kernel is size-optimized after cargo, and the qualification evidence
records the exact bytes it produces. The build fails with an explicit message when it is missing.

The standard npm dev/build/test/typecheck/mcp commands build the WASM bridge automatically. Direct `tsx` or `vitest` invocation requires `npm run build:geometry` first. Generated binaries are ignored. CAD topology and geometry operations live in repository-owned Rust crates; SVG rendering and lossless compression use the dependencies listed in [third-party notices](THIRD_PARTY_NOTICES.md). Two kernels ship: the geometry kernel, and a language kernel with the OpenSCAD and ModelGraph frontends that loads only when source is compiled. The geometry WASM boundary uses the MGV1 binary protocol and direct exports. See [the library contract](crates/README.md) for native and host APIs. Rust is pinned in `rust-toolchain.toml` (`nightly-2026-09-10`, rustc 1.100).

## Authored B-rep modeling

The CAD workbench also provides [G-code export and layer preview](docs/design/gcode-preview.md)
from a selected scene body, with configurable toolpaths, worker cancellation,
file validation and filament/time statistics. The versioned preview dialect
preserves model coordinates. Print jobs can target Marlin, Klipper or
RepRapFirmware command sets, and G-code from other slicers (PrusaSlicer,
Orca/Bambu Studio, Cura, …) opens in a tolerant preview mode that reports the
detected generator and firmware flavor.

Solid provides exact rational cylinders, apex cones, conical frustums, tubes,
spheres and tori. Exact profile rotation supports signed partial turns and
profiles touching the axis. New sketch
extrusions retain B-rep topology, including exact circular walls. Display
retessellation preserves the authored surfaces and face identities. Planar
Push/Pull, Shell and Split preserve B-rep bodies, as does viewport dragging.
B-rep properties integrate NURBS surfaces for area, volume, centroid and inertia
independently of the display mesh.
Code → Solid retains native B-rep snapshots, rational weights and scene placement,
so imported authored bodies keep surface properties and retessellation controls.

B-rep booleans support rational line/circle prismatic profiles with holes,
parallel extrusion axes, different heights, blind pockets, sealed cavities,
empty results and XOR. Stepped results survive serialization and further
operations after rotation; unsupported general curved
intersections remain explicit. Planar booleans support concave outlines and face
holes, including rotated operands and chained operations. Analytic fillets remain
unsupported; the explicit faceted-cylinder option retains planar Boolean use.
The same constructors, supported booleans, chamfers, and faceted fillets are available
through ModelGraph Text and the NURBS MCP tools. See the precise
[operation envelope](crates/brep-core/README.md) and the full
[completion audit](docs/design/brep-completion-status.md).
Try the [curved Boolean example](examples/brep/curved-boolean.mg)
in the viewer's Code mode, or choose **B-rep pocket enclosure** in the example
gallery for a raised boss and blind pocket with a 2 mm floor.

The internal SemanticProgram B-rep backend executes native primitives and
supported operations with immutable snapshots and checked resource ownership.
Its scene adapter preserves multiple outputs, source identities and colors,
and separates display settings from authored geometry. The permanent
`openscad-viewer/brep-1` provider remains unavailable pending its runtime and
qualification integration.
An [isolated diagnostic runner](docs/design/brep-semantic-diagnostic.md) executes
the supported OpenSCAD B-rep subset in a disposable browser or Node worker with
hard cancellation, bounded transport and checked snapshot identities.
The separate [native predicate candidate](crates/cad-predicates/README.md) provides
bounded exact signs for orientation and squared-distance comparisons. It is not
yet connected to geometric construction or the shared WASM runtime.

## Highlights

- Own Rust `union()`, `difference()`, `intersection()`, and `hull()`.
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
- Built-in function reference from the editor toolbar, command palette, or F1:
  searchable RU/EN descriptions, signatures, parameters, and copyable examples
  for OpenSCAD and ModelGraph Text. It opens in the current document's language;
  select a function name in the editor and press F1 to look it up.
- Binary STL and OBJ export with object transforms baked into the result, plus
  ASCII STL, 3MF, PLY, OFF and AMF from the export selector.
- Mesh import and format conversion: STL (ASCII/binary), OBJ, PLY (ASCII/binary),
  OFF, AMF and 3MF open as objects in the Mesh workbench or bodies in the Solid
  workbench, and any of them converts to any export format from the toolbar
  **Convert…** button, a drag-and-drop, the `mesh_convert` MCP tool or
  `node --import tsx scripts/convert-mesh.ts input.stl output.3mf`. See
  [mesh import and conversion](docs/design/mesh-import-convert.md).
- Optional local MCP server with typed OpenSCAD tools/resources and a
  persistent DuckDB catalog for models, revisions, builds, and bounded exports.
- Opt-in upstream OpenSCAD oracle with multi-file/binary project bundles,
  bounded STL/OFF/WRL/3MF/CSG/DXF/SVG reference exports, and a
  fresh permission-model subprocess with a project-local MEMFS for every
  request. This is a host-file boundary, not a complete OS sandbox.
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

### Optional upstream OpenSCAD oracle

Normal product execution does not require upstream OpenSCAD. To run
differential qualification, inspect upstream diagnostics, or produce a
reference export, install the pinned official runtime once:

```bash
npm run setup:openscad
npm run verify:openscad-runtime
npm run status:openscad-runtime
```

The setup command explicitly downloads the official OpenSCAD 2026.09.01 Node
WebAssembly snapshot, verifies archive SHA-256
`82054dfb4911686de0ee3ea36771dbf81f3d014c3460c8ea069ab4f933f6d888`, and
creates a deterministic NODERAWFS-disabled patched copy under the gitignored
`.open-scad-runtime/` directory, whose installed SHA-256 is also pinned and
verified independently of the writable cache manifest. The same explicit setup
downloads the pinned Basic Regular font and its SIL Open Font License 1.1 text,
verifies both
SHA-256 digests, and records them in the runtime manifest so `text()` has a
deterministic default font. The GPL runtime, font, and license are not fetched
by `npm install` or committed or bundled into the web app; exact identities and
redistribution notes are in [OFFICIAL_RUNTIME_NOTICES.md](docs/OFFICIAL_RUNTIME_NOTICES.md).
MCP refuses official execution if the manifest, runtime, font, license, patch,
or Node permission-model precondition cannot be verified.

Run the upstream-oracle qualification suite (38 functions, 35 modules,
multi-file/binary projects, `text()`, all seven advertised export formats with
artifact-integrity checks, resource reads, and an actual stdio MCP process)
with:

```bash
npm run test:official
```

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
- `openscad_independent_check` — development qualification surface for the
  repository-owned `openscad/stable-2021.01` frontend and geometry engine. Its
  result attests `upstream_runtime_used: false` and deliberately keeps
  `complete_language_claim: false` until the full language/file/geometry gate
  passes. Inline and bounded VFS projects accept an optional animation `time`
  in the stable `$t` range `0..1`;
- `openscad_independent_export` — full-quality STL/OBJ export through that same
  repository-owned stable engine. Exact bytes are retained in a bounded,
  content-addressed, session-local cache and returned as an MCP resource link;
  no upstream runtime is invoked and no legacy DuckDB build/artifact provenance
  is written;
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
- `mesh_convert` — stateless conversion of an uploaded STL/OBJ/PLY/OFF/AMF/3MF
  file to STL (ASCII/binary), 3MF, OBJ, PLY, OFF or AMF, returned as an embedded
  resource with source statistics;
- `openscad_official_status` — availability of the optional upstream oracle,
  exact runtime/archive digests,
  pinned default-font/license digests, export formats, experimental-feature
  policy, declared isolation/residual-risk flags, setup guidance, and the
  canonical stable-language summary;
- `openscad_official_check` — differential/reference evaluation by upstream
  OpenSCAD with
  ordered `ECHO`/warning/error logs. It accepts inline source plus bounded
  relative text or base64 binary project files, so `include`, `use`, `import`,
  `surface`, and related file-backed semantics work without host paths. The
  result names the exact stable-language contract separately from the execution
  runtime;
- `openscad_official_export` — upstream-oracle reference export to a bounded,
  content-addressed, session-local MCP resource. Official artifacts are kept out
  of the legacy DuckDB engine-attestation schema rather than being mislabeled as
  independent Rust builds; its result also carries the stable-language
  contract summary;
- `openscad_build_history` — recent DuckDB-backed build results;
- `openscad_catalog_stats` — model/revision/build/artifact counts, stored bytes,
  build outcomes, and current retention limits without exposing SQL.

Saved sources (including immutable revisions), build summaries, artifacts, and
bundled examples are also available under `openscad://models/...`,
`openscad://builds/...`, `openscad://artifacts/...`, and
`openscad://examples/...` resources. Official exports use the separate
session-local `openscad://official-artifacts/{sha256}` template. Independent
stable-engine exports likewise use their own session-local
`openscad://independent-artifacts/{sha256}` template rather than the legacy
DuckDB resource namespace. The immutable
`openscad://language/openscad-2021.01` resource publishes the complete canonical
registry, including syntax/operators, the 38 + 35 built-ins, smoke metadata,
file semantics, and the separately recorded compatibility tail. Arbitrary SQL
is deliberately not exposed. DuckDB external access and extension
autoload/install are disabled in the server process.

The word `official` in these tool names means “the upstream OpenSCAD
implementation”. These tools are intentionally separate from the
repository-owned engine and are never called by `openscad_independent_check` or
`openscad_independent_export`.

`openscad://capabilities` describes the independent engine/host contracts,
the canonical official-language summary and registry URI, official-runtime
status and limits, protocol versions, and the deliberate IndexedDB/DuckDB
boundary. `openscad://official-runtime` gives the language summary and provider
status directly. Both resources derive the summary from
[`src/core/openScad2021Contract.ts`](src/core/openScad2021Contract.ts); the
stable target is not silently inferred from the newer execution snapshot.
It reports the mutable stdio host/watchdog contract separately from immutable
engine manifests, so moving an unchanged provider behind a Worker does not
rewrite its engine identity or digest. `openscad://parity` uses the same
separation when comparing browser and MCP targets.
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
response accumulation. Production stdio geometry runs in one disposable Node
Worker per job: the queue-inclusive deadline is 30 seconds, startup is bounded
to five seconds, cancellation gets a 25 ms cooperative grace period, and the
Worker is then terminated. A result settles and the next FIFO job starts only
after the previous Worker joins; a one-second join failure permanently
quarantines the supervisor and unrefs the stuck realm so it cannot pin process
shutdown. This closes the event-loop blocking/cancellation
P1. It is not a subprocess sandbox: Worker threads share the MCP process and no
OS-enforced memory limit contains WASM/native allocation, so hostile-memory
isolation remains residual work.

Official-runtime jobs do not use that Worker boundary. They run one-at-a-time
in fresh subprocesses with Node permissions, no inherited application
environment (only `NODE_NO_WARNINGS` is supplied), NODERAWFS disabled, and an
in-memory virtual project. Host-file read permission is limited to the runner,
verified patched runtime, and verified Basic font; host-file writes are denied.
The supervisor bounds the project, serialized child request, arguments, logs,
result, and wall time; it admits one official job and rejects concurrent
official work as busy
instead of maintaining an internal queue. Cancellation or deadline kills and
joins the child before the slot becomes available again. SCAD code receives no
host path and the runner exposes no network API, but Node's permission model
does not enforce a network sandbox. The V8 old-space setting also does not cap
WebAssembly linear memory, so OS-level network and hostile-memory containment
remain explicit residual risks. There is no automatic fallback to the subset.

## GPU geometry computation (experimental)

Select a part from a current full build, open **Inspect**, and choose
**Compute surface area on GPU**. A WebGPU compute shader calculates triangle
surface area with the object's transform, with CPU verification and an explicit
CPU fallback. Cancellation and scene/source changes invalidate the analysis.
This opt-in pilot does not accelerate or replace Rust construction/booleans.
The native Rust kernels have their own opt-in placements: `Acceleration::Gpu`
(wgpu — Vulkan/DX12/Metal) and `Acceleration::Cuda` (CUDA driver API on
NVIDIA, PTX kernels, no toolkit needed to build) for SDF grid sampling, the
lattice field and photogrammetry matching/depth sweep, each falling back to
the CPU reference. See
[native GPU and CUDA acceleration](docs/design/native-gpu-cuda.md) and the
[compute boundaries and live smoke checks](docs/design/geometry-compute-pilot.md).

## Automatic builds and parameter presets

Automatic builds adapt to recent build cost: text changes wait 60–450 ms,
while range drags coalesce updates without repeatedly interrupting in-flight
geometry. Release flushes the latest value and completes full quality when
needed. Manual Render remains immediate.

The Parameters dock can save up to 20 named parameter sets, apply them and undo
an application. Sets survive reload in the local workspace and are bound to
the same source template. They are not included in SCAD downloads or source
links. Existing workspaces migrate automatically. See
[the scheduling and preset contracts](docs/design/auto-build-and-presets.md).

## Build measurements

The collapsible **Build measurements / Замеры сборки** panel below the editor
shows compiler phases, request-to-result and scene-publication time, the first
GPU frame submission, transferred buffer bytes and vertex/index GPU reuse.
Download a source-free JSON report of the last 60 successful publications for
comparisons. Frame submission is not the moment the image appears on screen;
missing observations remain blank. See the
[measurement boundaries](docs/design/build-performance-measurements.md).

For reproducible bottleneck investigations, run `npm run bench:cpu` and
`npm run bench:gpu` separately. The CPU suite includes cold/warm compiler phases,
mesh analysis, validation, inspection and export; `--profiles build,stl` adds
separate CPU and allocation profiles. The browser suite uses the real WebGPU
renderer, records adapter identity, GPU timestamps when available, resource
counts, scene replacement and idle/animation behavior. See the
[benchmark instructions and results](docs/design/performance-benchmarks.md).

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

The **Section / Срез** toolbar button opens the scanning-plane panel. Enable
the cut, choose X/Y/Z, move it in millimeters, or press **Scan / Сканировать**
for an automatic back-and-forth pass. You can reverse the visible side or
return the plane to the center. Closing the panel pauses the scan; disable
the cut to restore the full view. Sections affect rendering and picking only:
source geometry and exported files remain complete.

Close zoom keeps the camera in front of the model in both projection modes.
Magnification continues without accidentally slicing geometry; only the
explicit section controls cut the displayed model.

## Independent language profiles and oracle boundary

This project does **not** bundle or silently invoke the official OpenSCAD
compiler. `openscad-viewer-subset@1` remains the frozen browser/legacy MCP
contract. `openscad/stable-2021.01` is a separate repository-owned engine
profile: it already executes the canonical 38 value functions, user functions,
bounded `include`/`use` projects, all seven 2021.01 import formats (with NEF3
reported as unavailable without CGAL), DAT/PNG `surface()`, shaped `text()`,
stable lexical/dynamic scope, and the canonical 35 modules. It remains a
development surface while exact module semantics and the deprecated
compatibility tail finish qualification. Unsupported behavior fails explicitly
and the MCP result keeps `complete_language_claim: false`.

The opt-in upstream 2026.09.01 snapshot is exposed only by
`openscad_official_*`. It is useful for differential tests and reference
artifacts, but a successful upstream run never counts as implementation of the
independent profile. Deprecated aliases and snapshot-only experiments remain
outside the stable 38 + 35 gate. External libraries are bounded project inputs,
not silently searched on the host.

All paths are bounded. Independent limits cover source/project length, AST size,
parse/evaluation depth, evaluated-value allocation, range size, object count,
`$fn`, and final triangle count. Official MCP limits cover the aggregate virtual
project, files, arguments, logs, output, one-at-a-time fail-fast admission,
subprocess lifetime, and session-local artifact cache.

## Architecture

- `src/core/mesh.ts` and `src/core/build.ts`: renderer-neutral geometry,
  identity, provenance, transfer, and build-quality contracts.
- `src/services/openscadParser.ts`: lexer, expression/statement parser,
  legacy evaluator, Rust geometry calls, diagnostics, and budgets.
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
- `src/mcp/officialOpenScadRuntime*.ts`: verified official-runtime project
  protocol, MEMFS child, fail-closed subprocess supervisor, and MCP-facing
  execution service. `scripts/official-openscad-runtime.mjs` is the explicit
  checksum-verifying installer; it is never part of the browser graph.
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

- [ModelGraph/1 model prompt](docs/languages/modelgraph-1-prompt.md) — the MCP functional modeling language: pure functions, lexical closures, immutable lists and geometry values, bounded evaluation, and examples. Read `openscad://language/modelgraph-1` from MCP for the same guide and JSON Schema. Regenerate artifacts with `node --import tsx scripts/export-modelgraph-language.mjs`.
- [architecture.md](architecture.md) — the current module map, worker/kernel design, and active technical debt.
- [recommendation.md](docs/recommendation.md) — the live prioritized backlog (P0 correctness → P3 process).
- [docs/review-of-main-rewrite.md](docs/review-of-main-rewrite.md) — the 7-role panel review of this rewrite: what was fixed immediately, what remains, and the convergence plan with the feature branch (`claude/top-issues-architecture-sync-00p2q9`).
- [docs/research/mcp-ten-agent-review.md](docs/research/mcp-ten-agent-review.md) — ten MCP reviews, implemented decisions, rejected scope, and the ranked next stage.

## ModelGraph LLM plugin

[Local client packages and remote HTTP setup](integrations/modelgraph/README.md) cover Claude Desktop, Claude Code, Cursor, VS Code, Codex and an HTTP endpoint for web clients. Generate local configurations with `node scripts/package-modelgraph-plugin.mjs`.

## SVG creation and conversion

Examples: [artwork with text and CSS](examples/svg/static-artwork.svg),
[dimensioned plate with holes](examples/svg/print-profile.svg), and
[rendered silhouette with effects](examples/svg/rendered-silhouette.svg).

Open **SVG ↔ 3D** below the editor to create an example, edit SVG markup or
load a local SVG. **Preview** and **Download SVG** retain the static artwork:
colors, gradients, patterns, clipping paths, masks, filters and embedded raster
images. CSS, `<use>`, symbols, nested viewports and text are resolved into a
portable document. Text is outlined using bundled Noto Sans (Latin, Greek and
Cyrillic), or explicitly loaded outline TTF/OTF/TTC fonts. No system fonts or remote
resources are required. Use your original font when its exact metrics matter.
SVG 2 geometry properties (`x`, `y`, `width`, `height`, `cx`, `cy`, `r`, `rx`,
`ry`, and `d: path(...)`) participate in the CSS cascade, including selectors,
inline styles, `!important` and explicit inheritance through each `use` instance.
Physical units, percentages and `em`/`ex` are resolved in their proper context.
`vector-effect="non-scaling-stroke"` retains the intrinsic physical stroke
width through affine transforms, nested viewports, text and stroke-width markers.
Patterns and masks receive the painted instance's coordinate context. Normalized
artwork freezes these strokes into filled outlines at the document's intrinsic
size, so subsequent CAD import and export do not depend on the consumer's DPI.
Embedded PNG/JPEG/GIF/WebP images are decoded and validated; GIF/WebP are
normalized to a PNG of their first frame with a diagnostic, so preview and
silhouette use the same static image.
Custom font collections are checked face by face, with at most 128 faces per file. SVG, COLR and bitmap glyph
tables are rejected: these embedded renderers bypass the SVG document admission
path. Convert colored glyphs to ordinary SVG paths before import.
SVG processing runs in a dedicated worker with cancellation and a 30-second
deadline. The draft, settings and uploaded fonts are saved in IndexedDB; storage
failure is reported, and a second tab cannot silently overwrite a newer draft.

**Add extrusion to model** appends self-contained SCAD with an editable height
in millimeters to an OpenSCAD document and starts a full build. Editable native
ModelGraph output is available through MCP as described below. The source is checked against the remaining
workspace capacity before insertion. The default **Vector outlines** mode handles fills, holes,
clip paths, markers and strokes with caps, joins and dash patterns. Curves are
flattened at the selected tolerance in millimeters. Paint colors do not affect
extrusion height; opaque gradient-filled paths use their vector outline. **Download
contours** exports the same CAD profile for subsequent editing or cutting.

For masks, filters, patterns, gradients with transparent stops and embedded images, explicitly select **Rendered
silhouette**. This renders the artwork on a transparent bitmap, thresholds its
alpha channel, and traces its boundary including holes. Resolution (128–2048
pixels on the longest edge) and alpha threshold are editable. This is a pixel
approximation clipped to the SVG viewport, not an exact vector reconstruction.

Standalone SVG uses 96 DPI by default; physical units such as `mm` preserve
scale. OpenSCAD project `import("shape.svg", dpi=...)` shares the vector engine
and keeps the existing 72 DPI compatibility convention (explicit CSS `px`
remains 96 pixels/inch). Project TTF/OTF/TTC assets are available to SVG text.

After a current full build, export a silhouette along X/Y/Z or select a planar
face and export it in its own plane at true scale. Export preserves the current
artwork; **Open exported SVG** explicitly loads the result for another extrusion.
Export uses all model geometry,
including hidden bodies; section clipping does not change it. Face export uses
the selected mesh's kernel face identity and verifies planarity. Curved-surface
unwrapping is not implemented. CAD SVG exports crop to contour bounds;
reimport preserves size and holes, not the original world-space origin.

MCP exposes `modelgraph_svg_preview(svg, ...)`, `modelgraph_svg_extrude(svg,
height, ...)` with a full geometry check, and `modelgraph_svg_export(document,
axis?, face?)`. Preview/extrusion accept `dpi`, `tolerance`, `geometryMode`,
`rasterSize`, `alphaThreshold` and optional base64 `fonts`. Extrusion returns
conversion warnings along with the SCAD, contour SVG and measured geometry.
With `includeModelGraph: true`, extrusion also returns a validated editable
ModelGraph document with a height parameter, preserving holes and nested islands.
Both representations undergo full builds. This option uses ModelGraph's own
limits (including 256 points per polygon and 128 nodes) and reports a conversion
error if the profile cannot fit; SCAD remains the default representation.
`face` contains zero-based `meshIndex` and `triangleIndex` from a full build.

The supported profile is static SVG. Scripts, event handlers, animation,
`foreignObject`, external references/stylesheets, CSS font loading and
vector effects other than `none` and `non-scaling-stroke` are rejected with a
diagnostic. CSS math/variables, viewport-relative or unsupported font-relative
lengths, min/max sizing, cascade at-rules and unsupported geometry selectors
require resolved values; they are diagnosed before geometry conversion.
Linked images must
be embedded; local fonts must be supplied explicitly. The input is never
inserted into the application DOM as active markup.

Limits: 4 MiB input SVG, 20000 contour points in the panel/MCP, 500000 points in
the shared import engine, 20000 source triangles for model projection, and
4 MiB normalized or CAD SVG output. The SCAD model also has a 250000-character
source limit; increase curve tolerance or lower silhouette resolution if needed.
Selected-face export counts only that face's triangles. Up to 16 custom fonts,
4 MiB each and 8 MiB total.
Rendered silhouettes additionally bound pixel and filter/layer work. General
project and MCP transport limits still apply.
Reference/text/marker expansion has a separate conservative budget of 500000
geometry units, checked before normalization; unused definitions, overridden
CSS declarations and markers in clipping paths may contribute to that estimate.
Instance-dependent non-scaling paint resources have a separate 500000-unit
budget, checked before cloning resources. CSS selector matching is bounded too.

Run `node scripts/record-svg-evidence.mjs` to rebuild the current Rust sources
and run the native SVG/planar geometry suites plus the SVG cycle,
worker/persistence behavior, MCP and vector tool checks.
It records `docs/qualification/svg-static-cycle-v1.json` only when every check
passes and neither the sources, test inputs, build configuration nor geometry
WASM/Brotli decoder change during qualification. This
component evidence does not replace the application's broader release gates.
