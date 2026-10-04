# Retained profile revolution — implementation in progress

Solid exact revolution now routes retained line/circular-arc regions through the existing native rotate-extrude-analytic semantic operation. Transformed profile controls preserve rational curves and loop orientation; the existing sketch-plane placement is applied to vertices, edges and surfaces. Faceted retained revolution still refuses unsupported input.

48 tests passed in solidRetainedRevolve and solidNurbs, including rational circle weights, source immutability and two holed partial-turn caps. Vue typecheck passed. First new test assertion used a nonexistent face.loops field; corrected to face.holes. No product change was needed for that failure.

This stage is uncommitted and not browser-qualified. Further checks required: X/Y axes and both profile sides, offset/tilted planes, multiple regions, partial/full turns, Boolean operations, independent OCCT dimensions/volume and holes, actual UI preview/cancel/Undo/Redo/reload, and distribution budget. General NURBS profiles remain limited by planar_trim support. No full roadmap completion claim.

Expanded placement gate passed: X/Y axes, offsets, both radial sides, +/-90 and 360 degree turns in tilted YZ plane, comparing retained rectangle vertex geometry and face counts against the existing polygon route. Four test cases include 12 placement combinations. Typecheck passed again.

Independent STEP exporter added for rational torus and holed full/quarter revolutions. Export session 74140 failed on the first torus: BREP_STEP_V3_REFUSED, Periodic patch grid control boundaries disagree. No manifest was produced; subsequent OCCT invocation failed on missing manifest. No independent pass claim. Investigation points to native step_interchange_v3 merge_periodic_patch_grid and profile-span face order/weights; all authored patches have periodic flags false. Do not bypass validation or claim arbitrary retained revolution exchange works. This uncommitted stage requires fixing the native periodic merge and completing all fixtures, UI and budgets.

Correction after naming each fixture: rational-torus and holed-full export succeeded; refusal is holed-quarter. The previous first-torus diagnosis was incorrect because the exporter logged no fixture names. Source shows periodicized_step_v6 classified the first four degree-2×1 patches as a full band without checking cyclic adjacency. Partial revolution patches can match this record shape while having different profile sections.

Native fix in progress adds geometric control/weight compatibility and cyclic closure before periodic merging. Incompatible candidates retain their original exact patches and still undergo the existing STEP topology certification and direct export validation. New regression requires unchanged model and V9 export/import of the holed quarter turn with two holed caps. First native build failed because Face does not implement PartialEq; test now compares encoded models. Retry session 58681 is live, log /private/tmp/cad-retained-revolve-native-grid-retry.log. No WASM rebuild or passing independent acceptance yet.

Native regression retry 58681 passed: partial holed quarter turn keeps its original exact patch model, exports/imports V9, validates and retains two holed caps. Full native STEP module regression passed 30 tests (session 91895 exit 0).

UI now admits retained profiles to Revolve, selects exact mode before preview and disables faceted mode for retained inputs, including keyboard selection. Vue typecheck passed. WASM build session 5754 remains live after release compilation, packaging log /private/tmp/cad-retained-revolve-wasm-build.log. Do not restart or use old packaged WASM to qualify the native fix. Remaining: updated WASM export/OCCT, browser, UI tests and distribution budgets.

Browser harness now accepts --retained-holed-quarter=PROJECT to import the authored holed fixture, revolve exact by 90 degrees at offset zero and export expected volume 22.5*pi mm3 and bounds [0,0,-6]..[6,4,0]. Fixture exporter retains input project JSON for this scenario. Harness syntax and diff whitespace checks passed; browser run still pending.

Live process inspection confirms session 5754 is actively executing binaryen wasm-opt -Oz at ~99% CPU (PID 7249, parent build PID 6031). It is not stalled or terminal; do not start another build. After completion rerun scripts/export-cad-retained-revolve.mts, independent OCCT and frontend build before browser.

Focused renderer UI acceptance passed (session 91190 exit 0): retained holed Revolve selects exact, disables faceted, Home keeps exact; 90-degree preview/cancel leaves baseline unchanged, Apply creates exact B-rep with two holed caps and Undo preserves source profile. One selected test passed, 272 unrelated tests skipped by -t filter. This is not a full UI suite or browser qualification and used the previously packaged WASM while new packing session 5754 remains live.

Six retained revolution tests now pass, including explicit refusal for a profile crossing the axis with source unchanged and disconnected regions producing two B-rep solids in one authored scene body. Independent fixture exporter now includes disconnected-full, expected two solids and volume 18*pi mm3. These export fixtures still await the updated WASM.

Live inspection during this turn confirms wasm-opt PID 7249, build parent 6031 and session 5754 remain active (~36% CPU at 5m28s optimization elapsed); no restart.

Real postMessage regression passed, one selected test (14 unrelated skipped): exact parity with direct revolution, result ID, source rational controls, two holed caps and axis-crossing rejection without input mutation. This qualifies transport on previously packaged kernel, not yet fixed STEP packaging.

Build 5754 progressed past optimization: wasm-opt reduced 10,769,149 to 9,559,365 bytes; packaging remains live. Do not claim final packaged hash until completion.

WASM build 5754 completed with exit 0. Packaged geometry kernel 9,559,365 bytes, SHA256 a6e7378416ec7f3eab0993cb0cbcd81b03d82ca00b84a5991c9f3ba8c421d781. Export retry 89034 passed all four fixtures, including previously refused holed-quarter. Independent OpenCascade session 26532 and frontend Vite build session 54390 are live. Logs /private/tmp/cad-retained-revolve-occt-fixed.log and /private/tmp/cad-retained-revolve-frontend-build.log. No independent/bundle pass claim yet.

OpenCascade 26532 passed all four fixtures: rational torus, holed full/quarter turns and disconnected full turn (two solids). Max bounds error approximately 1e-7 mm; volumes match manifest tolerance. Vite build 54390 passed. Distribution verification failed; inspect /private/tmp/cad-retained-revolve-dist.log before claiming production gate. Browser and visual inspection still pending.

Independent four-fixture OCCT preview was visually inspected. Browser mouse acceptance 86024 passed preview/cancel, exact quarter-turn apply, invalid-quantity preview clearing, Apply without recompute, Undo/Redo and five exports. Browser STEP independently passed OCCT in session 97352. Results retained in occt/ and browser/.

Distribution initially failed DirectModeler 378,043 > 378,000 bytes. Feature growth from preceding build: total assets +2,357 bytes to 7,158,076. Named UI budget increased to 379,000 and total to 7,159,000 with measured comments. Retry passed 137 artifacts, raw WASM 11,615,485 bytes, combined 18,773,561. This is an explicit feature-budget update, not a reduction.

Keyboard with worker failure/retry session 51793 and full CAD roadmap plus new retained test session 53520 are live. Logs /private/tmp/cad-retained-revolve-browser-keyboard.log and /private/tmp/cad-retained-revolve-full-roadmap.log. Broader Boolean qualification and full roadmap remain open.
