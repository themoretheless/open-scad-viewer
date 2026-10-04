# Current UI part acceptance — 2026-10-05

Mouse acceptance passed for bracket, flange and enclosure on the published WASM.
Each part executes 20 edits, cancels 7 previews, checks all 20 Undo and Redo states against exported documents, and checks document equality after reload. Face selection uses viewport clicks and verifies the selected face and body through the UI, including renderers without SVG face polygons.

Fresh final documents were exported through the STEP menu, imported and exported again. OpenCascade independently passed all six exports, including bounds, volume and solid validity checks against the existing numeric references.

This qualifies the three fixture chains. General curved fillets, general B-rep diagnostics and the complete P0 command matrix remain open. Keyboard repetition is tracked separately; no claim of completion is made here.

The current source inventory enumerates all 95 commands: 87 literal call sites and eight primitive commands expanded from the literal primitiveKinds declaration. Every execution gate starts as unverified; fixture acceptance is not used to imply full command coverage. The inventory includes source SHA-256 and separate mouse, keyboard, error, Retry, cancel, late-result, history, recovery, multi-tab and latency gates.
