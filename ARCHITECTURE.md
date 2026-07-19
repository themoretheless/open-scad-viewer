# Architecture

This document describes how **OpenSCAD Viewer** is structured today, the data flow, and the planned modular refactor.

---

## Overview

```
┌─────────────────────────────────────────────────────────────┐
│                          App.vue                             │
│  (UI, state, editor logic, i18n, orchestration)              │
│                                                              │
│   code (string)                                              │
│        │                                                     │
│        ▼                                                     │
│  parseOpenSCADWithAST() ──► MeshData[] + AST + echos        │
│        │                          │                          │
│        │                          ▼                          │
│        │                   WebGPURenderer.setMeshes()        │
│        ▼                          │                          │
│   Object tree / stats             ▼                          │
│                            GPU render loop ──► <canvas>      │
└─────────────────────────────────────────────────────────────┘
```

The pipeline is: **source code → parse → evaluate to meshes → upload to GPU → render**.

---

## Core Modules

### `services/openscadParser.ts`
A self-contained OpenSCAD-subset compiler. Stages:

1. **Tokenizer** — `tokenize(src)` produces a flat `Token[]` (numbers, strings, identifiers, operators, comments stripped).
2. **Parser** — a recursive-descent `Parser` class builds an AST of `ASTNode { type, name, args, children }`. Includes a full **expression sub-parser** with operator precedence (ternary → logical → comparison → additive → multiplicative → unary → primary) and function calls.
3. **Evaluator** — `evalNodes` / `evalNode` walk the AST, threading a context of `vars`, `modules`, `echos`, `annotations`, caller `children`, and profiling state. Produces `MeshData[]`.
4. **Mesh generators** — `makeCube`, `makeSphere`, `makeCylinder`, … (~35 generators) return interleaved vertex/normal buffers + index arrays.
5. **Geometry operations** — `convexHull3D` (incremental hull), `earClip` (polygon triangulation), `extrudeMesh` / `rotateExtrudeMesh`, CSG handling for `difference`/`intersection` (visual approximation: subtracted bodies rendered translucent).

**Public API:**
- `parseOpenSCAD(src): MeshData[]`
- `parseOpenSCADWithAST(src, resolveFile?): { meshes, ast, echos, profileEntries }`

**`MeshData`:**
```ts
interface MeshData {
  vertices: Float32Array  // interleaved [x,y,z, nx,ny,nz, ...]
  indices: Uint32Array
  color: [number, number, number, number]
  transform: Mat4         // row-major 4x4
}
```

### `services/webgpuRenderer.ts`
A `WebGPURenderer` class owning the WebGPU device, pipelines, and the render loop.

- **Pipelines:** mesh (opaque), mesh (transparent), line (grid/wireframe/edges), outline, sky.
- **Shaders:** WGSL embedded as template strings (`MESH_WGSL`, `LINE_WGSL`, `OUTLINE_WGSL`, `SKY_WGSL`).
- **Scene uniform:** a single uniform buffer holding view-projection, eye, light, ambient, clip plane, fog, section box, and feature flags (flat/SSAO/Gooch/toon shading packed into padding fields).
- **Camera:** orbit + fly modes, animation (`animateTo` with easing), inertia, projection (perspective/ortho), FOV.
- **Render-on-demand:** a dirty flag + `requestRender()` avoids a continuous 60 fps loop when idle; auto-rotate/animation re-enable continuous frames.
- **Lifecycle:** `init()`, `setMeshes()`, `resize()` (via `ResizeObserver`), `destroy()`, device-lost recovery.

### `services/math3d.ts`
Minimal row-major `Mat4` (`Float32Array`) and `Vec3` helpers: `identity`, `multiply`, `translate`, `rotateX/Y/Z`, `scale`, `perspective`, `ortho`, `lookAt`, `transpose`, `invert`.

### Export / Import
- `stlExport.ts` — binary STL writer (applies per-mesh transforms).
- `objExport.ts` — Wavefront OBJ writer.
- `threemfExport.ts` — 3MF (ZIP of XML) writer.
- `zipExport.ts` — minimal uncompressed ZIP builder (used by 3MF, turntable, all-tabs export).
- `stlImport.ts` — binary STL reader → `MeshData[]`.

### Extracted modules (July 2026 decomposition pass)

| Module | Lines | Responsibility |
|--------|-------|----------------|
| `src/i18n/index.ts` | ~1.6k | All four locale dictionaries (ru/en/de/zh), the singleton `lang` ref, `t()`, `toggleLang` |
| `src/config/index.ts` | ~150 | `EDITOR_THEMES`, `BUILT_IN_PRESETS`, `SHORTCUT_PRESETS` + `matchesBinding` and their types |
| `src/parser/geometry.ts` | ~2.6k | All 41 pure mesh generators (makeCube…makeTeardrop), `earClip`, extrusion, `convexHull3D`, bitmap font — zero parser/evaluator state, Worker-ready |
| `src/parser/limits.ts` | — | Shared `MAX_FN` DoS bound |
| `src/renderer/shaders.ts` | ~140 | The four WGSL sources; the Scene uniform struct is defined once and interpolated into each shader (drift-proof, locked by `tests/shaders.test.ts`) |
| `src/services/safeStorage.ts` | ~100 | The single localStorage access point: validated reads (int/float/enum/bool/JSON), quota-safe writes reporting failures via an injectable handler (App.vue shows a throttled toast) |

### `App.vue`
Still the main Vue component (~13.3k lines after extraction; target < 1k): editor, tabs, viewport overlays, modals, panels, preferences, and orchestration between parser and renderer. Decomposition continues per [RECOMMENDATIONS.md](./RECOMMENDATIONS.md) Phase 1 and [recommendation.md](./recommendation.md) items 6–14.

---

## Data Flow Detail

1. User edits `code` (a tab's reactive string).
2. A debounced watcher calls `doRender()`.
3. `doRender()` optionally pre-processes the code (`$t` animation substitution, fast-preview `$fn` halving), then calls `parseOpenSCADWithAST()`.
4. Result `meshes` are filtered (object visibility, color overrides) and passed to `renderer.setMeshes()`.
5. The renderer uploads vertex/index/uniform buffers and flags a redraw.
6. `echos` and profiling go to the console panel; the AST feeds the object tree; meshes feed the statistics/weight/cost panels.

State persists to `localStorage` (tabs, preferences, theme, bookmarks, snapshots, history, session backup).

---

## Known Limitations

> A full 500-item defect catalog is in [ISSUES.md](./ISSUES.md), with the 50 most severe in [TOP-50-ISSUES.md](./TOP-50-ISSUES.md) (now on its second audit pass). The items below are the geometry-correctness subset.

- **CSG is visual, not boolean.** `difference`/`intersection` do not compute true mesh booleans; subtracted bodies are shown translucent (an in-UI "CSG: visual preview" badge makes this honest to the user). `hull` is a real 3D convex hull — though Pass 2 found its horizon-edge detection has a logic bug that can produce non-convex/self-intersecting hulls (see TOP-50 #16); `minkowski` is a pass-through.
- **`linear_extrude` / `rotate_extrude` / `offset` / `projection`** are approximations; `linear_extrude`'s `twist` is also unbounded (TOP-50 #12), and `torus`/`donut` divide by `r1` with no zero-guard, which hangs the tab (TOP-50 #4, #5).
- **`fillet` / `chamfer`** operations are pass-throughs that log a note.
- Parser still uses module-level mutable state (`cIdx`, `_resolveFile`, `_profiling`, …) — not Web-Worker-ready yet, and re-entrant parses can corrupt each other's state (TOP-50 #22).

---

## Planned Refactor (modularity & loose coupling)

`App.vue` (~15.0k lines), `openscadParser.ts` (~4.8k), and `webgpuRenderer.ts` (~2.5k) are monoliths. The target structure, derived from a 10-perspective architecture audit, is summarized below. The **actionable checklist with priorities and exit criteria** lives in [RECOMMENDATIONS.md](./RECOMMENDATIONS.md) — the phases here map 1:1 to it.

### Phase 1 — Decompose `App.vue`
```
src/
├── locales/{ru,en,de,zh}.json     # extract ~1.6k lines of inline i18n
├── composables/
│   ├── useEditor.ts               # tabs, undo/redo, folding, find/replace
│   ├── useViewport.ts             # camera, render modes
│   ├── usePreferences.ts          # localStorage-backed settings
│   ├── useExport.ts               # STL/OBJ/3MF/PNG/ZIP (lazy-loaded)
│   └── useI18n.ts
└── components/
    ├── EditorPanel.vue, TabBar.vue, Toolbar.vue, Console.vue
    ├── ViewportCanvas.vue, OrientationGizmo.vue
    └── modals/ (CommandPalette, Preferences, Welcome, ...)
```

### Phase 2 — Split the parser
```
src/parser/
├── tokenizer.ts   ast.ts   parser.ts   evaluator.ts   context.ts
├── primitives/    (basic, advanced, shapes, solids, organic, patterns, text)
└── operations/    (csg, extrude)
```
Replace the untyped `args` bag with a **discriminated-union AST**, and replace the 8-parameter `evalNodes` signature with a single `EvalContext` object (no module-global state → Web-Worker-ready).

### Phase 3 — Build & test infra
- Delete committed `.js`/`.js.map` artifacts in `src/` (already gitignored).
- Add **Vitest** with parser snapshot tests.
- Configure Vite `manualChunks` + lazy-import export modules (the bundle currently exceeds 500 KB as one chunk).
- Add ESLint, Prettier, and a CI workflow (`vue-tsc` + build + tests).

### Phase 4 — Renderer & accessibility
- Extract WGSL to `.wgsl` files (Vite `?raw`); replace `_pad` flag smuggling with a typed `FeatureFlags` struct.
- Introduce a `RenderPass` abstraction instead of one monolithic `render()`.
- Add focus trapping, `aria-live` status regions, and a `SafeStorage` wrapper validating `localStorage` reads.

---

## Tech Stack

| Concern | Choice |
|---------|--------|
| Framework | Vue 3 (`<script setup>`) |
| Build | Vite 8 + `vue-tsc` |
| Language | TypeScript 6 |
| Rendering | WebGPU (WGSL shaders) |
| Runtime deps | Vue only |
