# Numeric circle authoring — 2026-10-02

Exact center X/Y and radius inputs use the existing quantity parser. Preview is derived from the analytic circle without committing the document. Create commits one sketch in the active workplane. Invalid inputs disable creation and hide the preview; Escape cancels.

## Verified locally

- Focused UI tests: 2 passed (numeric circle and polyline).
- Full DirectModeler UI suite: 296 passed, 66.15 seconds.
- TypeScript: `npx vue-tsc --noEmit` passed.
- Production build: `npx vite build` passed.
- Distribution verification: 140 artifacts; 7,208,418 asset bytes, 11,669,725 raw WASM bytes.
- DirectModeler chunk: 390,417 bytes, +1,899 bytes.
- Test checks center [20,-5], analytic radius 6, unit conversion, uncommitted preview, Undo/Redo, invalid radius and Escape.

## Remaining qualification

No real browser mouse/keyboard, reload or rotated-workplane qualification for this new panel in this run. General command matrix and P1–P3 remain open. This change does not qualify arbitrary fillets, general B-rep diagnostics or surfaces.
