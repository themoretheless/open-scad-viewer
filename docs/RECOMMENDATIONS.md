# Recommendations & Roadmap

> Updated 2026-11-13: removed 23 resolved items (5 open remain). The donor-era
> `App.vue`, `openscadParser.ts` and `webgpuRenderer.ts` this plan targeted are
> gone. The shell is now `src/App.svt` (~1.8k lines) with 38 `.svt` components in
> `src/components/`, 19 in `src/features/` and 13 composables in
> `src/composables/`; the language runs
> in `crates/openscad-core` (`lexer.rs`, `parser/`, typed `ast.rs`, `eval/`)
> inside `src/workers/geometry.worker.ts`; WGSL is generated from
> `crates/raster-core/shaders` with a named-field `Scene` struct and a frame-pass
> encoder in `src/render/framePassEncoder/`; storage access goes through
> `src/core/safeStorage.ts` (one deliberate fail-loud read remains in
> `src/kernel/solidDraftStore.ts`); `document.write` no longer exists; true CSG and
> Minkowski are kernel operations. The live debt register is
> [recommendation.md](./recommendation.md).

Actionable plan derived from a 10-perspective architecture audit (Frontend, Performance, Parser, GPU, UX, A11y, i18n, DevOps, Test, Security).

This is the **execution plan**. It is kept in sync with:
- [README.md](../README.md) — what the app does (user-facing)
- [architecture.md](../architecture.md) — how it's built
- [ISSUES.md](./ISSUES.md) — the remaining defect catalog (domain→phase mapping at its end)

Status legend: `[ ]` todo · `[~]` in progress · `[x]` done

---

## Phase 1 — Finish decomposing the app shell

Goal: `src/App.svt` becomes a thin shell that composes children.

- [~] **1.1** i18n: dictionaries already live in `src/i18n/appStrings.ts` (ru/en); `t()` in `App.svt` is still a plain key lookup with no interpolation/pluralization
- [x] **1.4** Shared `Modal` and `SidePanel` (`src/components/`, behavior in `src/composables/useModalDialog.ts`): `role="dialog"`, `aria-modal`, title label, focus moved in, Tab/Shift+Tab trap, Escape, focus restore, background scroll lock. The export dialog (`ExportDialog.svt`), `KeyboardShortcuts.svt`, `ExampleGallery.svt`, `CommandPalette.svt` and `FunctionReference.svt` use `Modal`; `SidePanel` has no consumer yet (candidates in P7: `ScanPlanePanel`, DirectModeler operation cards). `MechanicalGenerator.svt` keeps the native `<dialog>` with `showModal()`. Covered by `tests/modalDialogUi.test.ts`

**Exit criteria:** `App.svt` < 1,000 lines; build green; no behavior regressions. Also tracked as R4 in [recommendation.md](./recommendation.md). Status: `App.svt` is 894 lines after moving workspace modes, source history, palette commands and dispatch, shell actions, geometry analysis and customizer state into `src/composables/`, and the group source panel, export dialog, sweep evidence, performance panel and parameter suggestions into `src/components/`.

---

## Phase 3 — Build & test infrastructure

- [ ] **3.5** Add **ESLint** + **Prettier** configs for TypeScript *(still nothing to run; `tsconfig.json` also still lacks `noUnusedLocals`/`noUnusedParameters`, see [ISSUES.md](./ISSUES.md) E1–E8)*

**Exit criteria:** `npm run lint` exists and runs in CI.

---

## Phase 4 — Accessibility

- [~] **4.4** A11y: `aria-live` status regions exist; a shared focus trap and `aria-modal` are not applied to every dialog (`ScanPlanePanel.svt` and the DirectModeler subtract card declare `role="dialog"` without `aria-modal`)
- [ ] **4.5** A11y: non-color cues for errors / diff / CSG (icons, hatch patterns)

**Exit criteria:** passes basic axe a11y checks.

---

## Suggested execution order

1. **Phase 3.5** (lint config) — cheapest, and gives the remaining refactors an automated signal.
2. **Phase 1** (finish shell decomposition) — together with R4 in [recommendation.md](./recommendation.md).
3. **Phase 4.4–4.5** (accessibility).
