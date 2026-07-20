# Recommendations — the live backlog (post-rewrite)

The previous content of this file cataloged defects of the pre-rewrite prototype ("Top 50 / 250 problems / 200 ideas"); the Manifold rewrite made most of it either done (real CSG, expressions/modules/loops, line-column errors, worker, components, tests) or moot. This is the actualized backlog. Sources: the 7-role panel review ([docs/review-of-main-rewrite.md](docs/review-of-main-rewrite.md)), the research passes under [docs/research/](docs/research/), and the feature-branch catalog (`recommendation.md` on `claude/top-issues-architecture-sync-00p2q9`, 500 items — still useful for ported features).

Status legend: `[ ]` open · `[x]` done.

## P0 — correctness & robustness (panel findings)

- [x] polyhedron winding (CW→CCW) + `mesh.merge()`; EvenOdd fill rule for polygon(); customizer valueStart corruption; evaluation-step / value-size / extrude-slices / share-hash caps; worker id NaN-guard; positioned kernel errors for extrude/revolve; STL header byte-truncation. *(fixed after the review)*
- [ ] Persistent worker + cooperative cancellation: stop terminate-as-cancel (Manifold WASM re-instantiates on every restart); ready-handshake, depth-1 overwrite queue, abort checkpoints in the evaluator; watchdog terminate only as last resort.
- [ ] Skip the full build when the preview reported no `$fn` clamping (halves pipeline work per edit).
- [ ] `GeometryKernel` interface: evaluator calls the kernel through it; Manifold primary; results tagged `exact | approximate`; wrap all remaining kernel calls into positioned errors; normalize non-`Error` WASM throws.
- [ ] Move MeshData/protocol types to a neutral `core/mesh.ts` (parser currently the type hub for renderer/export/inspection); colocate a `meshTransferables()` helper; ESLint `no-restricted-imports` guard keeping parser/kernel worker-only.
- [ ] Extract and test the build-result reconciliation (App.vue's ~70-line selection/visibility restore) as `applyBuildResult()`.
- [ ] SafeStorage port from the feature branch: quota-aware writes with a user-visible failure signal (current `writeStorage` swallows quota errors — silent autosave loss), schema-version key, `scad-tabs`/`scad-code` migration shim.

## P1 — performance (verified hot spots)

- [ ] Face-hover overlays: precompute faceId→triangles in the worker; persistent pre-sized GPU buffers via `writeBuffer` (currently O(all triangles) scan + buffer destroy/create per pointer frame).
- [ ] Bounds in the worker, cached on MeshData (kills the per-vertex allocating `measureMeshBounds` main-thread stall and the `inspectMesh` rescans in computeds).
- [ ] Dirty-index uniform writes in `updateMeshStyles` (currently rewrites every mesh's uniform on any hover/selection change); emit hover callback only on target change.
- [ ] Prototype-chained scopes instead of `new Map(env)` per scope/iteration in the evaluator; hoist tokenizer operator tables; single-`Float32Array` vertex interleave when `numProp === 6`.
- [ ] Debounce caret-driven source highlighting; reuse overlay buffers; newline-offset table + binary search for line/column derivation in `sceneRows`.
- [ ] Batch `setMeshesVisibility(bool[])` (visibility restore currently recomputes scene bounds + overlays per mesh).

## P2 — UX & product (designer findings + ports from the feature branch)

- [ ] Wire up or remove outliner multi-select (`{additive, range}` is emitted and silently dropped — Ctrl-click does nothing).
- [ ] ViewCube: rotate with the camera (it permanently claims ISO), ortho snap on face views, ≥24px hit targets for Back/Left/Bottom, camera tweens.
- [ ] Viewport shortcuts: move to the global handler with modifier/input guards (Ctrl+F over canvas currently hijacked; palette advertises keys that only fire with canvas focus); one declarative command registry feeding palette + keymaps + labels (three hand-synced structures today).
- [ ] Touch: two-pointer pinch/pan (input layer is single-pointer — mobile zoom impossible); keyboard camera (arrows orbit, +/- zoom) with `role="application"` on the canvas.
- [ ] Dock: resizable, side-by-side Scene+Inspect (selection currently forces tab ping-pong); replace `/` `.` `×` HUD glyphs with labeled actions; outliner context menu.
- [ ] Editor: line numbers + click-to-line from errors; customizer edits should splice text (wholesale `code.value` replacement destroys native undo); then port find/replace + undo snapshots from the feature branch; CodeMirror 6 as the end state.
- [ ] i18n: one shared dictionary module (port the feature branch's ru/en/de/zh), kill per-component copy tables and "English / Русский" dual strings, browser-language detection, `<html lang>` sync.
- [ ] A11y: tree semantics for the outliner, tablist for dock tabs, un-hide the shortcut legends (currently `aria-hidden` 9px), live result counts in the palette, typographic floor ≥11px, tokenize the three divergent axis-color palettes.
- [ ] Ports by user value: multi-tab editing → themes → examples gallery → export dialog (3MF + options) → STL import (with its header-validation) → PWA/offline → keyboard cheatsheet.

## P3 — process

- [ ] Tests for: worker stale-id gating, Manifold error paths (empty difference, invalid polyhedron), recursion/limit budgets, STL under mirroring (normals+winding bytes), customizer comment/string edge cases, `WebGPURenderer.init()` degradation, CommandPalette component behavior (needs jsdom env).
- [ ] ESLint flat config + lint script + CI step; coverage reporting; import-boundary rules (main thread must not value-import the parser).
- [ ] Feature-parity checklist for retiring the feature branch; freeze it to security-only once the test suite and SafeStorage land here.
