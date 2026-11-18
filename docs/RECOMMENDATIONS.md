# Recommendations & Roadmap

> Updated 2026-11-13: removed 23 resolved items (5 remain listed; on 2026-11-15 items 1.1 and 3.5 were marked done, leaving 4.5 open). The donor-era
> `App.vue`, `openscadParser.ts` and `webgpuRenderer.ts` this plan targeted are
> gone. The shell is now `src/App.svt` (918 lines) with 38 `.svt` components in
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

- [x] **1.1** i18n: dictionaries live in `src/i18n/appStrings.ts` (ru/en); `t()` in `App.svt` delegates to `translate()`, which substitutes `{name}` parameters and picks plural forms by `count` (`key_<category>` with `key_other` fallback)
- [x] **1.4** Shared `Modal` and `SidePanel` (`src/components/`, behavior in `src/composables/useModalDialog.ts`): `role="dialog"`, `aria-modal`, title label, focus moved in, Tab/Shift+Tab trap, Escape, focus restore, background scroll lock. The export dialog (`ExportDialog.svt`), `KeyboardShortcuts.svt`, `ExampleGallery.svt`, `CommandPalette.svt` and `FunctionReference.svt` use `Modal`; `SidePanel` has no consumer yet (candidates in P7: `ScanPlanePanel`, DirectModeler operation cards). `MechanicalGenerator.svt` keeps the native `<dialog>` with `showModal()`. Covered by `tests/modalDialogUi.test.ts`

**Exit criteria:** `App.svt` < 1,000 lines; build green; no behavior regressions. Also tracked as R4 in [recommendation.md](./recommendation.md). Status: `App.svt` is 918 lines after moving workspace modes, source history, palette commands and dispatch, shell actions, geometry analysis and customizer state into `src/composables/`, and the group source panel, export dialog, sweep evidence, performance panel and parameter suggestions into `src/components/`.

---

## Phase 3 — Build & test infrastructure

- [x] **3.5** **ESLint** + **Prettier** configs for TypeScript: `eslint.config.mjs` and `.prettierrc.json` exist, `npm run lint` (`scripts/audit-lint.mjs`, ratchet baseline) runs in CI, and `tsconfig.json` has `noUnusedLocals`/`noUnusedParameters`; the remaining strictness work is ROADMAP P2

**Exit criteria:** `npm run lint` exists and runs in CI.

---

## Phase 4 — Accessibility

- [x] **4.4** A11y: `aria-live` status regions; true modals (export, palette, shortcuts, gallery, reference) share `Modal`/`SidePanel` with focus trap and `aria-modal`; the non-modal `ScanPlanePanel` and DirectModeler subtract card are labelled `role="dialog"` with `aria-modal="false"`. The 3D canvas is focusable with a focus ring, a name, a key description and arrow/Shift+arrow/+/- orbit, pan and zoom; the ViewCube is named as fixed presets with `aria-pressed` and a live current-view status
- [~] **4.5** A11y: non-color cues: invalid fields get `aria-invalid` plus a dashed border, hovered sketch paths a dashed stroke, pressed buttons a heavier outline, the active subtract operand a text marker, the active ViewCube preset a heavier outline; save/build errors already carry icons or text. Remaining: hatch patterns for diff/CSG rendering in the 3D viewport

**Exit criteria:** passes basic axe a11y checks. `axe-core` is not a dependency yet; until browser gates (ROADMAP P5) exist, `tests/accessibilityUi.test.ts` checks roles, names and ARIA state of the touched components.

---

## Suggested execution order

1. **Phase 3.5** (lint config) — done; the ratchet now gives the remaining refactors an automated signal.
2. **Phase 1** (finish shell decomposition) — together with R4 in [recommendation.md](./recommendation.md).
3. **Phase 4.4–4.5** (accessibility).
