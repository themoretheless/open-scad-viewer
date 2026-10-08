# Mapped retained carrier qualification

The native control is a closed planar tetrahedron with one original edge split at a qualified root: four faces, seven edges and five vertices. Its affine carrier maps cover the original curve; each retained half remains incomplete. The ordered two-fragment chain proves complete coverage on both incident faces.

Native source-family tests: 131 passed. Real WASM worker tests: two passed, including STEP preparation (5 vertices, 7 edges, 4 faces; endpoint error at most 1e-7 mm), archive restoration, metadata Undo/Redo and fresh geometry restoration after reload. Vue type checking and diff checking passed.

These worker results use intermediate WASM SHA-256 `9d9ebf775a57fbf2cfdcd0c2aad87db8212436409e932b1eb33a1a9cca41927b`. It includes the individual-fragment coverage correction, but predates the primary mapped-chain contact extension. Admission already succeeds through another native contact proof. The native controlled regression establishes the chain extension; the worker test establishes transport and restoration, not a before/after admission change. Latest package qualification is pending.

Independent OCCT 8.0.1 accepts the exported control as one valid solid. Volume is 0.16666666666666663 mm³ (absolute error 2.78e-17 mm³); axis extents differ from the unit tetrahedron by approximately 1e-7 mm, within the independent 1e-6 mm acceptance tolerance. The STEP file and hash-bound report are retained here. Reproduce with `scripts/verify-mapped-root-step.py`.

This does not qualify general NURBS fillets, geometric editing history or the three required production parts.

Full real-worker regression suite on the intermediate package: 26/26 passed in 90.70 seconds. This includes mapped root restoration/history/STEP, reference regular seam checks, singular endpoint diagnostics, cancellation and retained-domain wall checks. It preserves explicit unresolved results; it does not establish whole-wall clearance or complete UI acceptance.

## Final package

WASM SHA-256 `d3c14ce4318b38b245e51006387c1acfc9b1295bce5653edf285afad6e8b0caf`, 12,107,173 bytes, includes the mapped-chain contact extension. After packaging completed, the full real-worker suite passed 26/26 in 90.98 seconds. Independent OCCT repeated STEP acceptance successfully. Type checking, package audit and distribution verification passed: 152 artifacts, 8,397,731 asset bytes plus 15,403,565 raw WASM bytes. Existing budgets were retained.

An earlier concurrent worker/build run overlapped publication of generated package files and failed; it is excluded from this qualification. Final evidence comes from runs after packaging completion.

## Retained-domain query reuse (native; package pending)

Each face-gap call now reuses classification of exactly identical original UV rectangles within the same immutable body and tolerance. Cache entries include their query allowance; an unresolved result may be recomputed when a larger allowance becomes available. Outside pruning remains dependent on the original bounded classifier, and uncertainty never creates a gap certificate. Cache lifetime is one call and storage is bounded by at most two entries per examined product cell.

The control regression previously performed 16 domain queries for eight product cells; after reuse it performs fewer than 16, while the impossible 30.01 mm clearance for a 30 mm cuboid remains refused. Source-family tests pass. Annular runtime savings and package qualification remain pending.

Native annular replay at identical limits: 9,473 domain queries and 574,598 endpoint queries, versus 18,261 and 957,614 in the preceding WASM report. Native elapsed time was 56.77 seconds; this is not a before/after timing comparison across runtimes. The same 16 retained pairs qualify and 45 distinct-face obligations remain unresolved. Counters and original request are archived; updated WASM replay is pending.

## Normal refinement experiment

Increasing regular and factored product refinement from 128 to 1,024 cells per pair (100,000 total cells each) at otherwise identical limits produces no new or lost proofs: 45 pairs remain unresolved. The native run used 70,630 regular product cells, 5,625 factored product cells and 19,130 normal spans in 60.61 seconds. This excludes that budget increase alone as a solution for this control. A combined retained product criterion (normal exclusion or separation for each covered product) remains a candidate requiring a native certificate and tests; it is not implemented by this experiment.

## Native combined retained wall-pair criterion

`source_wall_pair` supplies a distinct private certificate: every original product is covered by normal-direction exclusion, outside-domain exclusion or interval distance above the threshold. It does not supply an ordinary clearance certificate for products excluded by normals. The certificate retains original Body ownership, face indices, minimum threshold and angle.

Original annular pair [4,5], previously unresolved, qualifies in 53 product cells with eight normal-excluded products and 78 geometric spans. The whole-wall aggregator has not consumed this new certificate yet; the global count remains 45. Other selected controls remain unresolved, and pair [16,20] explicitly reports a numeric limit with all remaining products retained and no certificate. Cuboid controls preserve refusal above true wall thickness and distinguish normal exclusion from gap proof. The 131 source-family tests pass, including the annular combined proof and numeric refusal. WASM/worker/UI integration of this new criterion is pending.

## Packaged trim-query cache

WASM `e2177f38aae3f047bd35b68da80d54f31b517bd33d95bba7f3c5800989355d0f` (12,111,309 bytes) includes original-domain query reuse, predating the mixed wall-pair criterion. The full real-worker suite passes 26/26 in 95.89 seconds. The annular counters match native exactly: 9,473 domain queries and 574,598 endpoint queries; 45 pairs remain unproven. Isolated request times are 10.019 and 10.892 seconds without and with the retained pass; this is not a before/after cache timing measurement. Distribution passes at 8,398,403 asset bytes, retaining the established 608-byte geometry and 761-byte aggregate margins.

Mixed-pair certificates are now accepted by the native whole-wall aggregator with original Body pointer, unique unordered face pair, minimum threshold and angle checks. Tests reject a foreign but geometrically identical Body, duplicate reversed pairs, increased requested thickness and a broader requested angle. The original annular pair [4,5] is consumed with reason `source-wall-mixed-product-qualified`; remaining obligations keep the whole-wall certificate absent. The 131 source-family tests pass. Automatic mixed-pass scheduling and bridge/worker/UI transport remain pending.

The native aggregator now supports an optional automatic mixed pass over only unresolved distinct-face pairs. Total and per-pair product, geometric-span, domain and normal-span budgets are separate. It preserves prior proofs, exposes remaining products and retains newly produced private certificates in any completed whole-wall certificate. Cube exhaustion tests verify no certificate on an insufficient threshold/budget, preservation of previous proofs and rejection of zero per-pair normal work. The 131 source-family tests pass. Automatic annular measurement and bridge/worker/UI integration remain pending.

Automatic native annular pass completed: 45 unresolved distinct-face pairs examined, 13 newly certified ([0,4], [0,9], [0,26], [3,17], [4,5], [5,9], [5,14], [5,15], [5,23], [9,10], [10,14], [10,18], [13,25]). It uses 36,393 product cells, 57,498 geometric spans, 23,359 normal spans and 309,509 domain cells. Thirty-two examined distinct-face obligations remain unresolved; whole-wall qualification remains false. The archived report contains per-pair reasons and unresolved product counts. This result is native; packaged transport and visual acceptance are still pending.

## Bridge and UI integration (new package pending)

Optional `wallCoverage.mixedProducts` binds the bounded mixed pass to the original request. Native bridge replay with adaptive self facts confirms all 27 self-face exclusions, 13 new distinct-face proofs and 32 remaining distinct-face obligations. Native report and request are archived. TypeScript validates the complete native response, pair order, counters, budgets, reasons and unresolved products; a retained-gap refusal may be superseded only by a separately validated mixed success.

The new interface requests this pass, displays its additional face-pair count and numerical-limit guidance; existing pending-face highlighting follows the combined main pair report. Tests cover malformed/dropped/rebound reports and original request snapshots. A mocked transport test with the archived native response verifies cancellation, late response rejection and Retry; it is separate from actual WASM execution. Existing UI suite (317 tests), transport suite (14 tests), native bridge dispatch test and Vue type checking pass. A real-worker 45-to-32 regression is added and awaits the new package. Visual browser acceptance remains unverified.

Six further native pair controls ([0,2], [0,6], [2,5], [5,7], [7,10], [10,12]) remain unresolved at 8,192 product cells and 10,000 geometric/domain/normal work limits. This budget increase supplies no additional certificate; per-pair results are archived.

The Codex in-app browser inventory had no tabs; a new tab on the same in-app browser opened `http://127.0.0.1:5195/` successfully. Visible viewport, panels and toolbar startup are captured in `cad-startup.jpg`. This verifies empty-document startup, not mixed-wall execution or its highlight/cancel/Retry UI scenario. The new package remains under optimization.

## Mixed pass packaged and visible in the new interface

WASM SHA-256 `4802e59126264d9af13ff7b0b50ea86acaca5b1c949197b7e270a6d98353937d`, 12,128,661 bytes. The completed real-worker suite passes 28/28 in 112.04 seconds. Package audit and distribution verification pass; measured budgets preserve the previous margins. Actual request/result data are archived in `mixed-wall-packaged-worker.json.gz`.

The normal document import and browser recovery preserve the original source body. In the Codex in-app browser, selecting source edge 0 and entering a 5.99 mm threshold enables whole-wall checking. A prior real request was cancelled; a new request launched with Enter completes. The visible result reports 32 unresolved distinct-face pairs, 13/45 additional mixed proofs and 27/27 self-face exclusions; the scene shows pending regions. `mixed-wall-ui-result.jpg` captures this result. The unconfirmed result exposes a repeat-check action. This does not prove arbitrary failure Retry, geometric Undo/Redo, whole-wall thickness, or absence of self-intersections.

The actual result action "Повторить проверку всей стенки" starts another real worker request and shows busy/cancel. Cancelling it clears the result; a later observation after the usual computation interval retains the cancelled state, selected source edge and responsive 60 FPS scene. This is an observed browser cancellation scenario, complemented by deterministic late-response transport tests. Thickness input accessible names now follow the active UI language; both Russian spinbuttons resolve through browser role/name lookup.

## Bound mixed subdivision by requested UV resolution

Inspection found depth-first products with parameter widths near 1e-163 while coarse queued products remained. The mixed pass now refuses to split a coordinate narrower than its requested UV tolerance. It retains terminal unresolved products; the tolerance never supplies a wall certificate. A coarse-tolerance cuboid regression requires one retained unresolved product and no certificate. On the identical automatic annular replay, the same 13 additional face pairs qualify and 32 remain unproven. Normal spans fall from 23,359 to 13,661 and domain cells from 309,509 to 278,419; product cells remain 36,393. Pair [16,20] now exhausts product work conservatively instead of entering the former numeric-underflow branch. Native report: `mixed-wall-uv-stop-native.json`. This native change is not yet rebuilt into WASM.

## UV stop packaged worker acceptance

WASM `38128357365b225d09e98896d18d18283691278361a9d2fc25a651b555e1993d`, 12,129,091 bytes, contains the UV subdivision stop. After complete packaging, all 28 real-worker tests pass in 115.41 seconds. The mixed report matches native normal/domain counts and every pair identity, proof state, product count and remaining-product count. All 13 additional proofs are preserved; 32 distinct-face obligations remain. Type checking, 14 transport tests, exact package round trip and distribution verification pass (152 artifacts, 8,409,242 asset bytes plus 15,425,483 raw WASM bytes). Existing budgets pass. A browser reload restored the original document and launched this package, but the tab became unavailable to CUA before result inspection; this attempt does not qualify the latest browser result.

## Continuous endpoint chord direction exclusion

The mixed pass now compares original point-difference enclosures against paid original normal enclosures. A continuous squared-sine lower bound above the requested endpoint angle excludes that whole product from the aligned wall criterion. It cannot certify ordinary clearance; overlapping point boxes, missing/zero normal directions and angular/numeric uncertainty supply no exclusion. Unit controls distinguish transverse and aligned chords, overlap, missing normals and invalid coordinates. Normal-alignment tests pass 14 with one existing ignored test; source-family tests pass 131/131.

At identical automatic annular limits, all 13 existing mixed proofs survive and no new pair qualifies; 32 remain unresolved. Product cells decrease from 36,393 to 32,365, while geometric spans increase 56,100 to 60,984, normal spans 13,661 to 16,704 and domain cells 278,419 to 336,621. This is not a measured runtime speedup. Pair [4,5] qualifies with 49 products instead of 53. Selected unresolved pairs retain their work-limit refusals and explicit product counts. See `mixed-wall-chord-native.json`. This extension is native only and is not in the packaged UV-stop WASM.
