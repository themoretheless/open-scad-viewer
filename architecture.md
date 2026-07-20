# Architecture

How the OpenSCAD workspace is actually built today (post-rewrite, with the panel-review fixes applied). The previous version of this document described the pre-rewrite prototype ("CSG is visual only", plain textarea pipeline) and was dangerously stale — see [docs/review-of-main-rewrite.md](docs/review-of-main-rewrite.md) for the review that flagged it.

## High-level shape

Client-only Vue 3 + TypeScript + Vite SPA. Heavy geometry work runs in a dedicated **Web Worker** around the **Manifold** WASM kernel; the main thread owns the UI, the WebGPU viewport, and picking.

```
main.ts
└─ App.vue (~1.6k lines — orchestrator: i18n table, command registry,
   worker lifecycle, storage, file IO, panel wiring)
   ├─ components/   CommandPalette · CustomizerPanel · InspectPanel
   │                SceneOutliner · ViewCube    (props-down / emits-up,
   │                shared DTOs in cadPanels.types.ts)
   ├─ services (main thread)
   │   webgpuRenderer.ts (~1.9k lines) — pipelines, camera, input,
   │       BVH picking, selection/hover overlays, section plane
   │   math3d.ts — mat4/vec3, WebGPU [0,1] depth conventions
   │   meshBvh.ts — transferable triangle BVH build + raycast
   │   meshTopology.ts — boundary/crease/non-manifold edge extraction
   │   meshInspection.ts — provenance, bounds, measurement helpers
   │   meshSelectionOverlay.ts — face/edge highlight geometry
   │   selectionCycling.ts — bounded front-to-back click cycling
   │   commandSearch.ts — tiered fuzzy ranking (RU/EN aliases, MRU)
   │   cameraHistory.ts · scadCustomizer.ts · meshExport.ts (STL/OBJ)
   ├─ services/geometryWorkerProtocol.ts — typed request/response, id-correlated
   └─ workers/geometry.worker.ts
        └─ openscadParser.ts (~1.2k lines): lexer → recursive-descent parser
           (AST with source positions) → evaluator → Manifold CSG →
           tessellation + BVH + semantic edges → transferable MeshData
```

Key properties:

- **Real CSG.** `union`/`difference`/`intersection`/`hull` are true boolean ops via manifold-3d. Exports match the preview. Unsupported syntax fails with a line/column error rather than rendering a misleading approximation.
- **Worker isolation.** The parser/kernel is only value-imported by the worker; the main thread imports types. Meshes, edges, provenance, and BVH buffers cross the boundary as transferables.
- **Evaluation budgets** (all enforced in the worker): source length 250k, AST nodes 25k, eval depth 128, range items 10k, shapes 1k, triangles 750k, `$fn` ≤ 256, evaluation steps ≤ 1M (bounds nested no-geometry loops), `concat`/`str` values ≤ 1M elements, extrude slices ≤ 512.
- **Two-stage builds.** Debounced preview (reduced quality) then full build; stale responses are discarded by request id on the main thread. Export is gated until a full build matches the editor ("showing previous result" badge).
- **Render-on-demand** WebGPU viewport: invalidation-driven redraw, BVH-accelerated hover/pick with rAF coalescing, point/face/body selection modes, depth-cycled clicking, section plane, semantic edges/x-ray, ViewCube + camera history.
- **Provenance.** Each mesh carries source ranges, enabling the three-way highlight (editor caret ↔ outliner rows ↔ viewport geometry).

## Known limitations / active technical debt

Prioritized detail in [docs/review-of-main-rewrite.md](docs/review-of-main-rewrite.md) and [recommendation.md](recommendation.md):

1. Build cancellation restarts the worker, re-instantiating the Manifold WASM (~541 kB) — heavy models thrash while typing. Needs a persistent worker + cooperative abort.
2. Every edit runs preview + unconditional full build even when identical.
3. Face-hover overlays scan all triangles and recreate GPU buffers per pointer frame.
4. Manifold is called directly by the evaluator — no `GeometryKernel` interface, no degraded mode, untestable without WASM.
5. Mesh/protocol types are owned by the parser file; renderer/export/inspection import from it (inverted dependency).
6. App.vue is re-accumulating concerns (inline 2-locale i18n, three parallel command/shortcut/i18n registries, raw localStorage that swallows quota errors).
7. Selection/visibility state is double-owned by renderer and App with callback mirroring.
8. Outliner multi-select is emitted but dropped; ViewCube is static (doesn't rotate with the camera, no ortho snap); touch input is single-pointer (no pinch).
9. Feature gaps vs the frozen feature branch (`claude/top-issues-architecture-sync-00p2q9`): multi-tab editing, undo snapshots, themes, find/replace, examples gallery, PWA/offline, de/zh locales, 3MF export, STL import. Ported per the convergence plan in the review doc.

## Convergence with the feature branch

This rewrite is the structural base. The feature branch is the donor for features, storage hardening (SafeStorage), i18n (ru/en/de/zh), exporters, and its test suite; porting is directed (never a git merge — both lines rewrote the same files), tracked in the review doc's checklist, with a storage-key migration shim (`scad-tabs` → current document model) before cutover.
