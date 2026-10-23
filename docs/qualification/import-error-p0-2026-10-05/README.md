# P0: invalid Solid import — 2026-10-05

Import failures now present localized mesh/document/material/sketch/workplane and JSON syntax guidance. Unrecognized diagnostics retain their original detail with an import-specific prefix. This does not claim all command errors are localized.

Current Chromium mouse and keyboard scenarios import a malformed mesh into an existing rotated cuboid document, check the Russian message and exact document preservation, then execute a variable-radius fillet, Undo/Redo and reload. Both scenarios passed without page errors. File assignment uses the browser file-input API; subsequent controls use mouse or keyboard navigation as reported.

319 focused tests, type checking and the final Vite build passed. General P0 command execution and Retry coverage remain open.

The final implementation loads localized import guidance only on import failure to preserve the existing CAD bundle budget. Type checking, Vite build and distribution budgets passed after this change. Browser reports are repeated on that final build.
