# Cancellable group edits — 2026-10-01

Create empty group and move selection to group use the existing scene worker and asynchronous history commit. They read the validated UI snapshot rather than synchronously materializing history. Selected locked/hidden objects remain excluded from move. The worker clones its input and modifies metadata only. Geometry, IDs and instance matrices are preserved.

Both buttons disable while pending; the command panel announces group work and Esc. Changing active group cancels the request, as do the existing document, selection, visibility and numeric-input invalidations. A newly created group becomes active only after a successful commit.

Real postMessage test passed: linked body metadata changes only, empty group creation, ungrouping, immutable source, duplicate and empty-name refusal. CPU Chromium passed on 1000 linked instances: creation, held-worker termination on Esc, cancelled JSON exact, moved JSON exact, full Undo/Redo and return to original source and links. Typecheck, build and 137 artifact checks passed; 7,155,068 asset bytes.

One browser sample, automation and durable save included: group creation 2137.53 ms, group move 2251.20 ms, maximum corresponding RAF gaps 508.4 and 542.3 ms. No claim of responsive large-scene editing or overall speed improvement. Extended worker/UI regression passed: 321 tests across four files. Group controls, group exports and Undo/Redo were additionally qualified through Tab/Shift+Tab and Enter; the baseline scene import and initial history setup still use the ordinary browser harness. P0–P3 remains open.

First keyboard harness run failed while trying to close the export menu after group-before.json downloaded: forward Tab traversal hit its 1000-step harness bound. No keyboard qualification is claimed. The revised harness chooses Tab or Shift+Tab from DOM order and allows traversal through the virtual list. Its retry is live; whether this fully resolves the failure remains unverified.

Keyboard retry completed successfully. Esc and changing active group both terminate the held worker and preserve the exact document. Create/move, group exports and Undo/Redo use keyboard activation. Source and linked geometry, IDs and placement remain exact. The first failure was the forward-only bounded focus harness; bidirectional traversal resolved it without a product focus change. Evidence: keyboard-controls.json and keyboard-browser-1000.json.
