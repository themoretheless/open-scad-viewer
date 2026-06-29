# Recommendations & Roadmap

Actionable plan derived from a 10-perspective architecture audit (Frontend, Performance, Parser, GPU, UX, A11y, i18n, DevOps, Test, Security).

This is the **execution plan**. It is kept in sync with:
- [README.md](./README.md) — what the app does (user-facing)
- [ARCHITECTURE.md](./ARCHITECTURE.md) — how it's built + the target structure (the 4 phases below mirror its "Planned Refactor")
- [ISSUES.md](./ISSUES.md) — the 500-item defect catalog these phases resolve (domain→phase mapping at its end)
- [TOP-50-ISSUES.md](./TOP-50-ISSUES.md) — severity-ranked shortlist with a concrete fix-sequencing order

Status legend: `[ ]` todo · `[~]` in progress · `[x]` done

---

## Current State (baseline)

| Metric | Value |
|--------|-------|
| `App.vue` | ~14.7k lines (script + template + style in one file) |
| `openscadParser.ts` | ~4.7k lines (tokenizer + parser + evaluator + geometry) |
| `webgpuRenderer.ts` | ~2.4k lines |
| Components / composables | 0 / 0 |
| Tests / CI | 0 / none |
| Bundle | single chunk, >500 KB JS |
| Features | ~250 |

**Top systemic issues:** god-component, no state layer, inline i18n (~1.6k lines), zero tests, no code splitting, parser uses module-global state, committed `.js`/`.js.map` artifacts.

---

## Phase 1 — Decompose `App.vue`  *(highest impact)*

Goal: reduce `App.vue` from ~14.7k to a thin shell that composes children.

- [ ] **1.1** Extract i18n dictionaries (`App.vue` script) → `src/locales/{ru,en,de,zh}.json` + a `useI18n()` composable with `t()` and interpolation/pluralization
- [ ] **1.2** Extract composables from the 262 refs / 330 functions:
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
- [ ] **3.2** Remove already-committed `.js`/`.js.map` artifacts from `src/` (git rm --cached)
- [ ] **3.3** Add **Vitest**; first suite = parser unit + snapshot tests (tokenizer, expressions, primitives → triangle/vertex counts)
- [ ] **3.4** Configure Vite `manualChunks`: isolate parser chunk; lazy-import export modules; split locales
- [ ] **3.5** Add **ESLint** + **Prettier** configs
- [ ] **3.6** Add `.github/workflows/ci.yml`: `vue-tsc` typecheck + build + tests
- [ ] **3.7** Add `npm run lint`, `npm run test` scripts

**Exit criteria:** CI green on PR; bundle split into ≥3 chunks; parser test coverage > 50%.

---

## Phase 4 — Renderer & accessibility & security

- [ ] **4.1** Extract WGSL shaders to `src/renderer/shaders/*.wgsl` (Vite `?raw` import)
- [ ] **4.2** Replace `_pad0`/`_pad1` flag smuggling with a typed `FeatureFlags` uniform struct
- [ ] **4.3** Introduce a `RenderPass` abstraction (mesh / line / outline / sky / reflection each a class) instead of one monolithic `render()`
- [ ] **4.4** A11y: focus trap in every modal (`useFocusTrap()`), `role="dialog"`+`aria-modal`, `aria-live` status region
- [ ] **4.5** A11y: non-color cues for errors / diff / CSG (icons, hatch patterns)
- [ ] **4.6** Security: `SafeStorage` wrapper validating/allow-listing all `localStorage` reads
- [ ] **4.7** Security: remove `document.write` (print path) → DOM/print-stylesheet approach; cap share-URL payload size

**Exit criteria:** shaders external; passes basic axe a11y checks; no `document.write`.

---

## Correctness backlog (geometry — tracked separately)

These are documented limitations in [ARCHITECTURE.md](./ARCHITECTURE.md#known-limitations):

- [ ] True boolean CSG for `difference` / `intersection` (currently translucent visual approximation)
- [ ] Real `minkowski` (currently pass-through)
- [ ] Robust `linear_extrude` / `rotate_extrude` / `offset` / `projection`
- [ ] Real `fillet` / `chamfer` operations (currently pass-through with echo)

---

## Suggested execution order

1. **Phase 3.2 + 3.3** (cheap, unblocks confidence) — clean artifacts, add Vitest + parser tests **first** so the refactor is safe.
2. **Phase 2** (split parser) — now covered by tests.
3. **Phase 1** (decompose `App.vue`) — the big win.
4. **Phase 3.4–3.6** (chunks, lint, CI).
5. **Phase 4** (renderer/a11y/security).

> Rationale: write tests **before** moving code, so each extraction is verified, not hoped.
