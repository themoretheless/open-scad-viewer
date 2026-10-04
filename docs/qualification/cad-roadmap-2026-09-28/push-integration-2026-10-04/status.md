# CAD integration with the latest local interface — 2026-10-04

The qualified CAD branch was merged with origin/main (792a72d7, including sweep PR #30). The latest local App layout and Solid/Mesh styles were preserved from snapshot 123895b2. The active original checkout and stash were kept untouched.

## Changes

- Combine oblique sweep proofs, quotient/polar/projective face proofs, exact boundary contacts and material-wall diagnostics. Keep both branches’ regression tests.
- Keep the 96-command registry, scene-panel toggle and numeric CAD controls. Give the topbar command button a stable accessible label.
- Create the source viewport renderer when its viewport is visible; destroy it when hidden and reject initialization completed after hiding. Source geometry remains cached.
- Rebuild the geometry WASM from the combined source. Measure the resulting bundle before setting its size limits.
- Use full Chromium for browser qualification: headless_shell/SwiftShader failed an isolated blank WebGPU canvas, while full Chromium/Apple Metal passed it. The sweep browser check accepts completed WebGPU drawing or SVG fallback and captures console errors.

## Verified locally

- Rust libraries: 779 B-rep + 356 bridge + 547 NURBS tests passed; 4 existing ignored gates remain.
- CAD roadmap: 915 tests / 74 files passed. Material/protocol coverage: 54 tests / 6 files passed. Build contracts: 9 tests passed.
- Vue and MCP TypeScript checks passed. Source/scripts/tests whitespace checks passed; archived raw evidence is preserved.
- Fresh WASM: whole-wall and selected-wall checks each passed 7 kernel cases and 7 worker-handler cases.
- Whole-wall browser: 3 models each with mouse and keyboard; cancellation, Retry, explicit curved-wall uncertainty, unchanged document and exact reload passed.
- Source preview: miter-periodic-hollow.r at 1440 and 600 px, 11 checks each, using WebGPU; preview, cancellation, stale dispatch, publication to Solid and exact restoration passed.
- Distribution: 141 artifacts; 7,858,330 asset bytes + 14,213,692 raw WASM bytes = 22,072,022 total bytes.

## Boundaries

These are local qualification results. CI and remote publication are separate steps. This does not complete the global P0 command matrix, general curved fillets, arbitrary curved whole-wall thickness, all sweep modes or the P1–P3 roadmap. The source-preview browser matrix here covers one mode and two viewport sizes.

The original checkout contains active work in another task. Its full snapshot is retained on codex/preserve-main-work-2026-10-04. Older G-code and laser CAM branches remain separate: their merges have additional conflicts and need separate reconciliation. No branch, worktree or stash was deleted.
