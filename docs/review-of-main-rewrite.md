# Review of the CAD-workspace rewrite (7-perspective panel)

A structured review of the Plasticity-inspired rewrite (`f64b444` + `d9bca2a`) by a seven-role panel: architect, developer, test engineer, security critic, performance critic, system designer, UI/UX designer. Every cited finding was verified against source at review time.

**Verdict: right direction, right foundation.** Real Manifold CSG in a worker, components instead of a monolith (App.vue 1.6k lines vs the feature branch's 13.3k), WebGPU [0,1] depth convention, BVH picking with depth-cycling, error positions, no XSS sink classes. The flaws are missing seams and a set of concrete bugs — not a wrong architecture.

## Fixed immediately after this review (same branch)

| Fix | Where |
|-----|-------|
| `polyhedron()` winding reversed to Manifold's CCW + `mesh.merge()` before `ofMesh` — spec-correct polyhedra previously built inside-out or were rejected | `openscadParser.ts` |
| Customizer `valueStart` computed structurally — `indexOf(value)` could match inside the variable name (`x1 = 1;` → corrupted to `x2 = 1;`) | `scadCustomizer.ts` |
| `CrossSection.ofPolygons(…, 'EvenOdd')` — default Positive fill rule silently emptied clockwise-wound `polygon()` outlines | `openscadParser.ts` |
| DoS caps: evaluation-step budget (nested no-geometry loops), `concat`/`str` value-size cap, `linear_extrude` slices cap (was passed unbounded into the WASM kernel), share-URL hash size cap before decoding | `openscadParser.ts`, `App.vue` |
| Worker id NaN-guard: validate the request id and always respond; the `Math.max(latestRequest, id)` tracker was NaN-poisonable (one malformed message muted all future responses) | `geometry.worker.ts` |
| `linear_extrude`/`rotate_extrude` kernel calls wrapped into positioned parse errors; height validated positive | `openscadParser.ts` |
| STL header truncated by bytes after UTF-8 encoding (80 multibyte chars = up to 240 bytes → RangeError) | `meshExport.ts` |

Regression tests added for each (83 total, all green).

## Remaining top problems (by priority)

1. **Cancellation-by-worker-murder.** `App.vue` terminates the worker whenever a build is in flight; the Manifold WASM (541 kB) re-downloads/compiles on every restart. Heavy models can never finish while the user types. → Persistent worker + ready handshake + depth-1 overwrite queue + cooperative abort checkpoints in the evaluator.
2. **Every edit runs the full pipeline twice** (preview at 180 ms + unconditional full at 780 ms). → Skip the full pass when the preview reported no `$fn` clamping.
3. **Face-hover overlays are O(scene) with GPU buffer destroy/create per pointer frame.** → Precompute faceId→triangles in the worker; reuse persistent buffers via `writeBuffer`.
4. **Manifold not behind an interface** — evaluator calls WASM directly; untestable without WASM, kernel unswappable, no `exact | approximate` tagging for a degraded mode. → `GeometryKernel` interface.
5. **Mesh/protocol types owned by the parser** — renderer/export/inspection all import from it. → Neutral `core/mesh.ts`.
6. **App.vue re-growing into a god component**: inline 2-locale i18n (de/zh lost vs the feature branch's 4-locale module), three parallel registries (palette/shortcuts/i18n), raw `localStorage` that swallows quota errors (the feature branch's `SafeStorage` solved this class), a 70-line untested build-result reconciliation. → Extract composables; port SafeStorage + i18n; one declarative command registry.
7. **State double-ownership**: renderer privately owns selection/visibility and mirrors to Vue via callbacks while App re-injects after builds. → One owner per field; renderer consumes state, emits intents.
8. **Dead multi-select affordance**: outliner emits `{additive, range}`, App drops them. Ctrl-click doing nothing is worse than no affordance.
9. **ViewCube doesn't rotate with the camera** (permanently claims ISO), face views don't snap to ortho, Back/Left/Bottom badges are sub-24px.
10. **Touch camera is single-pointer** — no pinch zoom; mobile regression vs the feature branch.
11. **Storage-key collisions across branches** (`scad-code`/`scad-lang`/`scad-theme` have different schemas; stored `de` silently becomes `ru`). → Schema-version key + one-shot migration (incl. `scad-tabs` → new document model).
12. **Feature losses vs the feature branch**: multi-tab editor, undo snapshots, themes, find/replace, examples gallery, PWA/offline, de/zh, export dialog, keyboard cheatsheet. Port by user value, one feature per PR, against the parity checklist.

## Branch convergence

Git-merge is meaningless (both sides rewrote the same files). **This rewrite is the structural base** (worker + CSG + components can't be retrofitted into a 13.3k-line component; features are portable, architecture is not). Directed porting order: (1) stabilize seams (types/kernel/protocol) → (2) port leaf infrastructure verbatim (SafeStorage, i18n×4, exporters, limits) → (3) port the test suite → (4) merge parser hardening; map the 41 generators onto the kernel → (5) port UI features by value with a parity checklist → (6) storage migration shim → (7) freeze then retire the feature branch.

## Keep these (genuinely good)

Depth-cycle clicking with the visible "2/4" HUD; three-way provenance highlighting (code ↔ outliner ↔ viewport) — the app's killer feature; disabled-with-reason command palette with MRU-as-tiebreak ranking; selection-mode pills with layered Escape; "showing previous result" honesty badge with export gating; camera history.
