# Recommendations & Roadmap

Actionable plan derived from a 10-perspective architecture audit (Frontend, Performance, Parser, GPU, UX, A11y, i18n, DevOps, Test, Security).

This is the **execution plan**. It is kept in sync with:
- [README.md](../README.md) — what the app does (user-facing)
- [architecture.md](../architecture.md) — how it's built + the target structure (the 4 phases below mirror its "Planned Refactor")
- [ISSUES.md](./ISSUES.md) — the 500-item defect catalog these phases resolve (domain→phase mapping at its end)
- [TOP-50-ISSUES.md](./TOP-50-ISSUES.md) — severity-ranked shortlist with a concrete fix-sequencing order (now on **audit Pass 2** — the numbered items referenced throughout this file are Pass 2's list, not the original)

Status legend: `[ ]` todo · `[~]` in progress · `[x]` done

---

## Current State (baseline, refreshed after audit Pass 2)

| Metric | Value |
|--------|-------|
| `App.vue` | ~15.0k lines (script + template + style in one file; script portion alone is ~8.9k lines) |
| `openscadParser.ts` | ~4.8k lines (tokenizer + parser + evaluator + geometry) |
| `webgpuRenderer.ts` | ~2.5k lines |
| Components / composables | 0 / 0 |
| Tests / CI | 26+ Vitest tests / GitHub Actions (typecheck + test + build) — see Phase 3 |
| Bundle | code-split via `manualChunks` (parser/renderer/exporters/vue) |
| Features | ~250+ |

**Top systemic issues (still open, see [TOP-50-ISSUES.md](./TOP-50-ISSUES.md) for the current ranked list):** god-component, no state layer, inline i18n (~1.6k lines), ~98 unguarded direct `localStorage` call sites (several with no try/catch at all — actively causing silent data loss, not just a style nit), parser still uses module-global state, `render()`/`renderScaled()` diverged render paths.

---

## Phase 1 — Decompose `App.vue`  *(highest impact)*

Goal: reduce `App.vue` from ~15.0k to a thin shell that composes children.

- [ ] **1.1** Extract i18n dictionaries (`App.vue` script) → `src/locales/{ru,en,de,zh}.json` + a `useI18n()` composable with `t()` and interpolation/pluralization
- [ ] **1.2** Extract composables from the ~237 refs/computeds / ~333 functions (re-counted in audit Pass 2):
  - [ ] `useEditor.ts` — tabs, undo/redo, folding, find/replace, cursor
  - [ ] `useViewport.ts` — camera, render modes, wireframe/grid/shading toggles
  - [ ] `usePreferences.ts` — all localStorage-backed settings
  - [ ] `useExport.ts` — STL/OBJ/3MF/PNG/ZIP (lazy-loaded)
  - [ ] `useToast.ts`, `useSession.ts`, `useKeyboard.ts`
- [ ] **1.3** Extract components:
  - [ ] `EditorPanel.vue`, `TabBar.vue`, `Toolbar.vue`, `ConsolePanel.vue`, `Minimap.vue`
  - [ ] `ViewportCanvas.vue`, `OrientationGizmo.vue`, `AxisLabels.vue`
  - [ ] `modals/`: `CommandPalette.vue`, `PreferencesModal.vue`, `WelcomeModal.vue`, `ExampleGallery.vue`, `ScadReference.vue`
  - [ ] panels: `ObjectTree.vue`, `StatisticsPanel.vue`, `ProfilePanel.vue`
- [ ] **1.4** Introduce a shared `SidePanel.vue` + `Modal.vue` to standardize open/close + focus behavior

**Exit criteria:** `App.vue` < 1,000 lines; build green; no behavior regressions.

---

## Phase 2 — Split the parser

Goal: separate parsing / evaluation / geometry; make it Web-Worker-ready.

- [ ] **2.1** Create `src/parser/`: `tokenizer.ts`, `ast.ts`, `parser.ts`, `evaluator.ts`, `context.ts`
- [ ] **2.2** Create `src/parser/primitives/` (basic, advanced, shapes, solids, organic, patterns, text) and `src/parser/operations/` (csg, extrude)
- [ ] **2.3** Keep `services/openscadParser.ts` as a thin re-export so no callers break
- [ ] **2.4** Replace module-global state (`cIdx`, `_resolveFile`, `_profiling`, `_source`) with an explicit `EvalContext` object threaded through `evalNodes`
- [ ] **2.5** Replace the untyped `args` bag with a discriminated-union typed AST (`CubeNode`, `SphereNode`, …)
- [ ] **2.6** Move parsing off the main thread into a Web Worker

**Exit criteria:** parser modules each < 800 lines; no global mutable state; build green.

---

## Phase 3 — Build & test infrastructure

- [x] **3.1** `.gitignore` excludes `src/**/*.js` and `*.js.map`
- [x] **3.2** No `.js`/`.js.map` tracked in git; `tsconfig` `noEmit:true` stops vue-tsc emitting new ones
- [x] **3.3** Added **Vitest** + 26 parser/STL tests + `test`/`test:watch`/`typecheck` scripts
- [x] **3.4** Vite `manualChunks` isolates parser/renderer/exporters/vue chunks *(lazy-import + locale split still TODO)*
- [ ] **3.5** Add **ESLint** + **Prettier** configs *(still nothing to run — `tsconfig.json` also still lacks `noUnusedLocals`/`noUnusedParameters`, see [TOP-50-ISSUES.md](./TOP-50-ISSUES.md) #49)*
- [x] **3.6** Add `.github/workflows/ci.yml`: `vue-tsc` typecheck + build + tests *(done — runs on every push/PR)*
- [x] **3.7** Add `npm run test` script *(done; `npm run lint` still not added — blocked on 3.5)*

**Exit criteria:** CI green on PR ✅; bundle split into ≥3 chunks ✅; parser test coverage > 50% *(26+ tests, not yet measured against source LOC)*.

---

## Phase 4 — Renderer & accessibility & security

- [ ] **4.1** Extract WGSL shaders to `src/renderer/shaders/*.wgsl` (Vite `?raw` import)
- [ ] **4.2** Replace `_pad0`/`_pad1` flag smuggling with a typed `FeatureFlags` uniform struct
- [ ] **4.3** Introduce a `RenderPass` abstraction (mesh / line / outline / sky / reflection each a class) instead of one monolithic `render()`
- [ ] **4.4** A11y: focus trap in every modal (`useFocusTrap()`), `role="dialog"`+`aria-modal`, `aria-live` status region
- [ ] **4.5** A11y: non-color cues for errors / diff / CSG (icons, hatch patterns)
- [ ] **4.6** Security/reliability: `SafeStorage` wrapper validating/allow-listing all `localStorage` reads **and retrying writes on quota errors** — audit Pass 2 found this is no longer a hypothetical: `flushSaveTabs`, `saveUserPresetsToStorage`, `saveRecentFiles`, `saveBookmarksToStorage`, and `saveSessionBackup` (the crash-recovery net itself) all currently swallow quota errors and lose data silently. See [TOP-50-ISSUES.md](./TOP-50-ISSUES.md) criticals #1, #6–#10, #26. **Promote this ahead of 4.1–4.5** — it is the highest-value item in Phase 4 now.
- [ ] **4.7** Security: remove `document.write` (print path) → DOM/print-stylesheet approach; cap share-URL payload size

**Exit criteria:** shaders external; passes basic axe a11y checks; no `document.write`; no un-guarded `localStorage` write in the codebase.

---

## Correctness backlog (geometry — tracked separately)

These are documented limitations in [architecture.md](./architecture.md#known-limitations):

- [ ] True boolean CSG for `difference` / `intersection` (currently translucent visual approximation)
- [ ] Real `minkowski` (currently pass-through)
- [ ] Robust `linear_extrude` / `rotate_extrude` / `offset` / `projection`
- [ ] Real `fillet` / `chamfer` operations (currently pass-through with echo)

---

## Suggested execution order

1. ~~Phase 3.2 + 3.3~~ (done — Vitest + parser tests + CI landed in Pass 1).
2. **Phase 4.6 first, ahead of everything else** — audit Pass 2 found active data-loss bugs (not just missing validation) in exactly the code 4.6 targets. This is now the single highest-value next step: wrap every `localStorage.setItem` in one retry-on-quota helper. See [TOP-50-ISSUES.md](./TOP-50-ISSUES.md) criticals #1, #6–#10.
3. **Phase 2** (split parser) — now covered by tests; also fixes the `torus`/`donut`/`gear` hangs and the `convexHull3D` horizon bug found in Pass 2 (TOP-50 #4, #5, #11, #16) as part of the same file's rework.
4. **Phase 1** (decompose `App.vue`) — the big win; also where the three-watchers-on-`code`, stale-closure history bug, and shortcut-preset crash (TOP-50 #2, #3, #24) get fixed structurally rather than patched in place.
5. **Phase 3.4–3.5** (lint config — 3.6/3.7 CI+test are now done).
6. **Phase 4.1–4.5, 4.7** (renderer/a11y/security, remaining).

> Rationale: write tests **before** moving code, so each extraction is verified, not hoped. Pass 2 changed the ordering here: it found the storage-quota bugs are live data-loss, not theoretical, so they now jump the queue ahead of the parser/App.vue refactors.
