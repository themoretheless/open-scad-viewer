# Numeric circle authoring — 2026-10-02

Exact center X/Y and radius inputs use the existing quantity parser. Preview is derived from the analytic circle without committing the document. Create commits one sketch in the active workplane. Invalid inputs disable creation and hide the preview; Escape cancels.

## Verified locally

- Focused UI tests: 2 passed (numeric circle and polyline).
- Full DirectModeler UI suite: 296 passed, 96.74 seconds after Escape fix.
- TypeScript: `npx vue-tsc --noEmit` passed.
- Production build: `npx vite build` passed.
- Distribution verification: 140 artifacts; 7,208,435 asset bytes, 11,669,725 raw WASM bytes.
- DirectModeler chunk: 390,434 bytes, +1,916 bytes.
- Test checks center [20,-5], analytic radius 6, unit conversion, uncommitted preview, Undo/Redo, invalid radius and Escape.

## Browser qualification after Escape fix

Native Chrome Canary 157.0.8081.0, WebGPU active, four scenarios pass: mouse/XY, keyboard/XY, mouse/rotated, keyboard/rotated. Keyboard uses actual Tab, Enter and text input (1,156 / 1,285 Tab events). The custom plane has origin [5,10,15], u [0,1,0], v [0,0,1]. Exported analytic circle and plane match exactly.

Each run checks radius `bad`, 0, -1 and 1,000,001, invalid center, blocked create and hidden preview, an uncommitted valid preview, exact creation, Undo/Redo, Escape clearing the tool and preview, and exact reload. No page/console errors. Actual keyboard draft screenshot inspected.

The first browser run found that Escape cancelled gesture state but left the drawing tool active, so the new derived preview remained visible. `cancelCommand` now returns to Select; regression test checks a valid preview disappears. Initial failure files are retained as diagnostic evidence.

## Remaining qualification

This fixture selects an imported rotated sketch plane; selecting a body face and creating in the 3D pane remains a separate scenario. General command matrix and P1–P3 remain open. This change does not qualify arbitrary fillets, general B-rep diagnostics or surfaces.

## Follow-up

The final shared circle/arc implementation was additionally qualified on a body face through actual mouse and keyboard input. See ../arc-coordinates-2026-10-02/status.md and its circle-face-mouse / circle-face-keyboard reports for pinned final source/build identity, supportBodyId and Y=-10 plane checks.
