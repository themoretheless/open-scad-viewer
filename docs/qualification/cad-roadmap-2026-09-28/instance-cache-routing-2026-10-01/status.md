# Instance worker cache routing

Creation and relative instance transforms now receive the existing worker-local exact batch cache. Cache hits own fresh geometry trees; authoritative parsing remains unchanged. The regression compares cached and uncached placement, checks cache reuse and verifies input geometry remains independent.

34 tests across scene cache, instance operations and real postMessage boundary passed. Vue typecheck passed. The first test run exposed an incorrect cache variable introduced during editing; fixed before the passing retry.

Build session 97058 is live, compiling geometry-wasm through npm prebuild. Do not restart or run a browser against partially rebuilt dist. Log: /private/tmp/cad-instance-cache-routing-build.log. Browser placement check was added behind --instance-placement to check source preservation, +7 mm linked placement and exact Undo; it has not run yet. No speed improvement has been established. Full roadmap remains open.

Build 97058 completed successfully: 137 artifacts, 7,155,719 asset bytes; geometry WASM SHA256 remains b2ebc7ddc537db72d15b0449123d506cfa75a15cd47e788b07c38a3f6146eb06. Expanded real-worker parity includes instance creation and placement; 34 tests passed.

First browser run failed because its new assertion expected linked mesh in compact JSON. Compact export intentionally stores only source and link matrix. The harness now compares the complete placement matrix against +7 mm and retains source/identity/Undo assertions. Geometry parity is separately proved by actual worker tests. Retry 99948 is active; no passing browser claim yet.

Retry 99948 completed with exit 1: placement/source checks passed, but immediate export after Undo still reflected placement. The scenario now waits for history-restore hidden and durable-save status before comparing. Final retry 37264 is live.

Final retry 37264 completed with exit 0: instance creation, exact +7 mm matrix, unchanged source, exact Undo after restoration, held-worker Esc cancellation, late-result rejection, transform preview/apply and Undo/Redo passed. Nine downloads. Mouse scenario; keyboard-only and large-scene speed qualification remain open. See browser-result.json.

Keyboard acceptance completed with exit 0 in sessions 20892 and 21413. Commands, numerical placement, Apply, Esc, exports and Undo/Redo use Tab/Enter; initial fixture upload is automated. The second run includes invalid-input and accessible field-error checks. Trial click only checks actionability and sends no pointer input. See keyboard-result.json and keyboard-errors-result.json. These results qualify the named scenario, not the full P0 tool matrix.

Full test:cad-roadmap gate passed at 2a1c04b5 before the subsequent retained-revolve work: 785 tests across 65 files, no skips/failures, duration 51.49 s. See full-roadmap-tests.txt.
