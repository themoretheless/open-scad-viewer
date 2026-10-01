# Exact polyline point authoring, 2026-10-02

Polyline now exposes exact X/Y coordinates in the active plane using the existing quantity parser, including units and numeric expression support. Add point modifies the draft only. Existing Close contour / Finish line commands commit the result; Esc abandons the draft. Consecutive/repeated coordinates are disabled, and invalid quantities show field errors and block adding. A new tool start resets numeric inputs and validity. Mouse point placement remains available.

The Enter handler now preserves native buttons while drawing a polyline; otherwise Add point was swallowed by the global finish-line shortcut. Keyboard Enter outside native controls still finishes the line.

Actual Chrome Canary 157 / Apple Metal mouse and sequential keyboard scenarios pass with GPU active and strict console/page error gates. Keyboard uses 1275 Tab presses and typed quantities, no synthetic pointer placement or direct app state writes. The fixture is imported empty; four exact points [0,0],[20,0],[20,10],[0,10] create a closed 20 x 10 mm contour. Invalid text, repeated coordinates, unchanged draft document, canceled draft, exact Undo/Redo and reload are verified. Keyboard draft screenshot was visually inspected. All 295 DirectModeler UI tests, focused acceptance and typecheck pass. Vite/dist pass: total assets 7,206,519 bytes, within measured-growth budgets.

This supplies full keyboard point entry for polyline, not every drawing tool or general NURBS profile preparation. Active-plane coordinates follow the same sketch commit path; a rotated workplane is not separately browser-qualified here. Full P0–P3 remains open.
