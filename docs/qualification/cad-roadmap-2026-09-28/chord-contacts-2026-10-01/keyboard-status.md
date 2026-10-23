# Keyboard lifecycle qualification

The browser scenario passed on the existing 7071a049 WASM distribution: source selection and command activation, quantity entry, EvenOdd mode selection, Enter apply, Escape cancel, keyboard undo/redo, keyboard JSON export activation, twenty consecutive constructions, full history reversal/replay, reload, delayed worker delivery after cancellation and document replacement, and actionable smooth-corner error recovery. No page errors occurred. Preview screenshot was inspected.

Evidence: browser-keyboard-retry/contract.json and exported document snapshots. The earlier browser-keyboard run timed out because keyboard press did not wait for the disabled export button during reload restoration. The harness now waits for the control to become enabled, matching ordinary button activation readiness.

Controls are focused by the qualification harness. Full Tab traversal is not qualified. This run does not prove the newly compiled contact/cancellation WASM; its optimizer was still running.

A separate browser-tab run passed the same twenty-construction lifecycle with controls reached through real Tab presses (25,794 total presses). Fixture import remains setup. This proves reachability for this scenario, not keyboard efficiency or the full P0 command matrix.
