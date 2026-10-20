> **Profile cache ownership audit, 2026-10-10:** An internal-helper regression reproduced reuse across two distinct profile owners with equal UV keys. Keys now include profile identity. Inspection of the public `constrain_original_rails` constructor established that it already rejects distinct profile owners (including equal geometry), so that helper misuse is not reachable through the current public corrected-bridge workspace. Earlier blanket invalidation of aligned-chord reports was too broad; existing reports retain their original bounded scope. Public alternating-side regression and a fresh native/WASM rerun are tracked in `profile-cache-owner-correction-2026-10-10.json`. This hardening does not add support for independent-profile bridges.

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

Endpoint chord input validation now checks both optional normal enclosures before any early success, rejecting a malformed second enclosure even when the first could exclude the product. Expanded controls preserve uncertainty at the exact squared-sine boundary, zero/broad normal cones, reversing endpoints and antiparallel normals, with coordinate/normal scales from 1e-90 to 1e90. Normal tests pass 15 with one existing ignored test.

The current in-app browser tab (different from the earlier unavailable tab) completes the packaged UV-stop whole-wall check at 5.99 mm. Source edge 0 is selected by Enter, and Enter launches the command. The visible result shows 32 unresolved face pairs, 13/45 mixed proofs, 7,241 remaining mixed products and 27/27 self-face exclusions, matching the UV-stop native product total. Pending regions and face buttons are highlighted. `mixed-wall-uv-ui-result.jpg` captures the result. This verifies the UV-stop package before publication of the still-building endpoint-chord package; it does not qualify that newer package.

## Endpoint chord packaged worker acceptance

WASM `34eadf7186ce3561165c624f7ea41192e564827d9559591f7f998f94a8228ceb` (12,131,304 bytes) includes continuous endpoint direction exclusion and the expanded input checks. All 28 real-worker tests pass in 111.97 seconds after packaging. The native and WASM reports match every mixed pair identity, proof state, cell count and remaining-product count, plus aggregate normal/domain work. All 13 mixed proofs survive and 32 distinct-face pairs remain unproven. Package audit and distribution verification pass at 8,411,216 asset bytes with established geometry/aggregate margins preserved. These counters do not establish a runtime speedup or complete wall thickness.

After reload, the current in-app browser completes the endpoint-chord package at 5.99 mm through source-edge and command Enter actions. Its visible main result is 32 unresolved distinct-face pairs, 13/45 additional mixed proofs, 3,207 remaining mixed products and 27/27 self-face exclusions. The mixed-product total matches native; pending faces and regions remain highlighted. Screenshot `mixed-wall-chord-ui-result.jpg` records this actual result. The 7,241-to-3,207 remaining-product change is a coverage/work result, not a runtime or whole-wall certificate.

## Reuse owned factored normals in mixed products

The mixed pass reuses normal-factor certificates already prepared by the whole-wall gate. Factors and product enclosures use the exact same retained-region Surface objects; an initial attempt using equal shell copies was rejected by the source identity guard and excluded from qualification. Additional factor queries charge the existing shared factored query budget. A product uses ordinary normal bounds when fewer than two factor queries remain. A limiting normal still does not establish a regular tangent plane at a pole.

The final annular native replay preserves the same 13 qualified pairs and 32 unresolved pairs. Product cells fall 32,365 to 30,461; geometric spans 60,984 to 58,080; ordinary normal spans 16,704 to 11,647; domain cells 336,621 to 314,744. Remaining products fall 3,207 to 3,128. These are work counters, not a measured runtime speedup. Source-family tests pass 131/131 and normal tests pass 15 with one existing ignored test. Report: `mixed-wall-factor-native.json`. This reuse extension is not in the shipped endpoint-chord WASM yet.

Direct source normal-cache controls now verify zero factored budget, one charged query, a free exact-key cache hit, rejection of equal copied sources and rejection of excessive query budgets. Normal tests pass 15 with one existing ignored test. The repeated annular report includes shared exact-work and query counters and preserves every mixed pair outcome. Both counters remain within the original request budgets. WASM packaging is running; worker expectations now compare these shared counters as well.

## Owned normal factors packaged acceptance

## Original corner cone packaged worker and UI acceptance

WASM `8c435e4f1446b8c21cdb2ed32709c66c3c44238cd95c3b8a49a40a31b91ede47`, 12,138,666 bytes, packages the four-corner integration snapshot. All 28 real worker tests pass; per-pair proof states, cells and unresolved product counts match native, as do normal/domain and shared factor counters. Exact compression round-trip passes. Distribution verification passes at 8,413,811 asset bytes plus 15,435,058 raw WASM bytes, with the existing 608-byte geometry and 761-byte aggregate margins retained over measured sizes. The actual reloaded in-app browser at 5.99 mm shows 28 unresolved pairs, 17/45 additional mixed proofs, 3,124 remaining mixed products and 27/27 self exclusions. Uncertain regions remain highlighted and Retry available; whole-wall thickness remains refused. Archives: `mixed-wall-corner-worker.json.gz`, `mixed-wall-corner-package-audit.json`, `mixed-wall-corner-ui-result.jpg`. Arbitrary certified anchor paths added later are not part of this package snapshot.

### Native positive chord cone predicate

The first remaining product of [0,1] now receives a source-bound polar anchored chord exclusion with projection [-1,-1,-1] and eight charged path/enclosure/normal units. Exact anchor preparation and original factor preparation remain separately recorded. The first product of [0,2] retains refusal for all 26 integer projection proposals. This establishes one product direction proof, not full coverage of [0,1], any skipped products or whole-wall thickness. Archive: `mixed-wall-polar-cone-products-native.json`. Diagnostic preparation is still outside the automatic Body mixed-pass shared budget and therefore must not be treated as production admission.

`qualify_polar_anchored_chords` combines exact source anchors with optional prepared original pole path/normal factors. It checks all source identities and matching pole ends before early refusal. The polar leg uses interval world-axis displacement relative to the exact anchor, plus a known V displacement sign; inverse-coordinate path coverage comes from the original decomposition. Ordinary sources retain derivative-path integration. Every path restriction, point enclosure and endpoint normal query is charged; preparation is separately owned caller work. A collapsed planar control paired with a planar quadrant qualifies with eight charged units, refuses at seven, and rejects a factor from an equal copied Surface. Full self-chord-family tests pass. This function is native only, not yet applied to annular mixed Body products or packaged WASM; the shipped 28-pair status is unchanged.

Polar replay on the first unresolved product of [0,1] and [0,2] prepares the exact original face-0 decomposition (1,538,943 work) and factored normal (18,181,977 work). Each coordinate-path report charges five net restrictions and returns a matrix in (world Y, weighted V), with first column Y≈1 and second X≈5.3033; factored normal is approximately along Z. These columns are not original UV derivatives: positive chord coefficients require world-axis displacement signs relative to the exact anchor, plus the V path sign and original inverse-path coverage. The planar side rejects the polar representation, while face 2 refuses its pole factor. No new pair admission is claimed. `coordinate_path_report` is public so callers can charge its existing restriction count; all seven rational-pole differential tests pass. Detail is only for the first product of each pair; archive `mixed-wall-polar-work-products-native.json`.

Original pcurve-endpoint proposals supply exact arbitrary anchors on the work-limit controls. Of the explicitly capped first 32 products, both source anchor identities qualify for 30 products of [0,1] and 31 of [0,2]; remaining proposals refuse the second anchor. The first common world anchor is exactly (20,0,6), with original UV pairs [(1,0),(20,0)] and [(1,0),(0,1)] respectively. None of the 26 integer projections excludes a chord cone. Anchor coincidence alone therefore closes no wall pair; the remaining path/normal bounds near the collapsed pole require further treatment. `mixed-wall-anchored-work-products-native.json` records exact-work charges and proposal failures. No statements are made about products outside the 32-product detail cap.

`qualify_anchored_chords` connects two independently issued exact source anchor certificates. Their world coordinates must match exactly; path domains use their own original UV coordinates. Endpoint rectangles must stay on a known side of each anchor in both parameters, so coefficients of the derivative cone remain nonnegative. Crossing rectangles conservatively refuse. The corner path implementation now delegates to the same private path gate. All 29 self-chord family tests pass, including interior anchors, unequal anchor rejection and reflected corner regressions; the original annular four-pair regression passes again (15.20 seconds). Arbitrary anchor paths are native only and are not yet admitted by the Body mixed pass. The in-flight WASM build still targets the earlier four-corner integration snapshot.

`qualify_surface_anchor` introduces a private original-source anchor certificate at an arbitrary UV point on a supported Bezier chart. Constant world/pcurves enter the existing exact homogeneous composition identity gate, which independently requires chart inclusion and valid positive weights. Only Equal produces the certificate; Different, unsupported representation, outside-chart points and work exhaustion remain unproved. Direct controls prove an interior planar point and refuse a 1e-15 world displacement and zero work. This is an anchor identity prerequisite, not a paired path or wall certificate, and is separate from the currently building corner WASM package.

Work-limit follow-up replays [0,1] and [0,2] at 1,024 cells without prepared factored normals. They retain 55 and 75 products respectively; world/corner detail is explicitly truncated to the first 32, while all UV products are retained. For [0,1], each of those proposals rejects unequal natural corner anchors. For [0,2], 30 products have exact anchors but none of the 26 integer projections proves exclusion; two reject unequal anchors. This does not qualify the omitted products or prove a defect. A natural-corner-only cone cannot close these obligations; arbitrary exact source boundary anchors and coupled paths require further work. Report: `mixed-wall-work-products-native.json`.

Integrated native mixed-product replay now qualifies [0,7], [2,5], [5,12], [7,10] at the unchanged automatic budgets. Qualified mixed pairs increase 13 to 17; unresolved distinct-face pairs decrease 32 to 28 and unresolved products 3,128 to 3,124. All previous pair proofs remain. Cells, ordinary normal spans, domain cells and shared factor counters remain unchanged; geometric spans increase 58,080 to 59,588 within 100,000. Whole-wall qualification remains false. Corner attempts only run at the original UV resolution stop, charge the existing span budget, and conservatively retain refusals on unmatched anchors, exhausted work or numerical failure. Full source-family tests pass 131/131; an added original annular regression qualifies all four pairs and verifies Body ownership, face identities and threshold. Report: `mixed-wall-corner-integrated-native.json`. New WASM is building; these four added proofs are not yet qualified through the packaged worker.

Independent original-source replay excludes the remaining product of each of [0,7], [2,5], [5,12], [7,10] using exact common corners and proposed integer projections. Total paid spans across proposals are 15, 23, 3 and 11 respectively. No approximate anchor was accepted. `mixed-wall-corner-cone-native.json` records every proposal, refusal and successful exclusion. Reflected affine UV maps and endpoint-order reversal also pass direct source tests (3/3). These are product-direction certificates; complete mixed-pair integration and whole-wall status require the subsequent integrated replay.

`surface_self_chord::qualify_corner_chords` now binds this predicate to two immutable original NURBS sources. Exactly equal three-dimensional control poles at verified nonperiodic clamped corners establish the common anchor. Continuous original derivative hulls cover corner-to-endpoint rectangles; their signed columns generate every nonzero chord as a positive combination. Endpoint normal bounds come from the same original sources and charge the shared span allowance. Private certificate fields retain both source identities, endpoint rectangles and requested angle. Tests qualify opposite planar quadrants, refuse possible cancellation and insufficient normal work, and reject even a 1e-15 displaced anchor. These source-bound predicates are not yet wired into Body mixed products or shipped WASM; the 32-pair annular status is unchanged.

`normal_alignment::positive_chord_cone_excluded` bounds the normal angle of every nonzero positive combination of supplied interval generators. A strict positive projection prevents cancellation; the ratio of normal dot product to that projection bounds the cosine through Cauchy-Schwarz. All generators are validated before early refusal. Zero normal/projection, nonpositive projected generators, possible cancellation and aligned chords supply no exclusion. Two direct tests pass and the complete normal-alignment test filter passes. This is native algebra only: original source derivative paths and exact common-boundary ownership are not yet bound to these generators, so it supplies no new Body certificate or packaged wall proof.

### Increased per-pair work is not an improvement

World-enclosure replay of all four endpoint-resolution products (none truncated) places them near (18.47759065, 7.65366865, 4.75) or its XY reflection. Original rectangle bounds overlap; box-distance lower bounds are zero and upper bounds range from 9.933294e-8 to 1.374485e-7 mm. These are bounds of unresolved boxes, not a wall witness, measured thickness or permission to discard a contact. Source-boundary dependence and chord-direction control near the endpoints require further proof. `mixed-wall-resolution-world-native.json` preserves every UV rectangle, both original coordinate boxes and their box-distance interval. The inspection example limits world-detail output to 32 products and explicitly marks truncation; full unresolved UV products remain preserved.

Independent pair replay isolates the four resolution-limit pairs [0,7], [2,5], [5,12], [7,10]. Each retains exactly one UV product, at chart endpoints, with maximum UV width 7.450580596923828e-9 (below requested 1e-8). Two share corners u=0,v=1 against u=0,v=1; two share u=3,v=1 against u=1,v=1. These products remain unproved, not numerical zeroes or exempt contacts. The inspection example now preserves actual unresolved UV products, enabling endpoint/source-boundary analysis. Archive: `mixed-wall-resolution-products-native.json`. This distinguishes endpoint-resolution obligations from the other work-limited pairs without relaxing production tolerances.

Native replay with 4,096 automatic cells per pair and unchanged global budgets adds no new pair proof. Geometric spans exhaust the global 100,000 allowance; cells rise from 30,461 to 51,431, factored queries from 4,562 to 7,865, and remaining visited products from 3,128 to 5,322. Only 12 proofs are visited, versus 13 at 1,024 cells. Previously proven pairs that are visited retain their proof, but late pairs are omitted when global work runs out. The baseline refusals comprise 28 work-limit and four resolution-limit results. Keep production limits unchanged; deeper subdivision alone does not resolve the remaining obligations. Report: `mixed-wall-factor-4096-native.json`. The inspection example accepts an optional automatic cell budget for reproducing this comparison.

Published WASM SHA256 `a3c2cf9037041319f1b3c1fa4cfbc2e220242353ff075865c32312e29117d88e`, 12,131,662 bytes. All 28 actual worker tests pass in 115.52 seconds, including per-pair native comparisons, shared factored query/exact-work counters, cancellation and stale-result controls. Package audit proves exact compression round-trip. Distribution verification passes: 152 artifacts, 8,410,207 asset bytes plus 15,428,054 raw WASM bytes. Geometry and aggregate budgets remain unchanged; the endpoint marker UI budget preserves the existing 879-byte margin over its measured 444,893 bytes. Archive: `mixed-wall-factor-package-audit.json`, `mixed-wall-factor-worker-tests.txt`. Same 13 mixed proofs, 32 unresolved distinct-face pairs and 3,128 pending products; whole-wall admission remains refused.

## Sharp endpoint diagnostic in the current interface

The interface now projects the native collapsed endpoint into a dedicated scene marker, only for a visible selected source body with distinct tangent limits. On the new factored-normal WASM, actual browser edge 0 produces marker coordinate `20,0,6` and the localized refusal. Selecting edge 1 removes the old marker (DOM count zero). Screenshot: `source-endpoint-marker.jpg`. Vue type checking passes. This locates a proven defect; it does not repair the sharp transition or certify the complete command matrix.

The actual in-app browser, using shipped endpoint-chord WASM, selects original edge 0 and runs its tangency command with Enter. The result identifies distinct limiting tangent planes at the end, displays the original coordinate (20, 0, 6) mm and unresolved parameter interval [0, 0], and advises changing the transition or processing neighboring edges. Retry remains available. `source-sharp-endpoint-ui-result.jpg` records the visible localized result. The selected edge is highlighted; a dedicated marker for the endpoint itself is not established by this screenshot. This is one error scenario, not the full mouse/keyboard command matrix or a qualified smooth end transition.


## Automatic original pair anchors (native, 2026-10-08)

`mixed-wall-auto-anchor-native.json` measures the unchanged restored annular
control with the same 1024 per-pair mixed cell limit and global budgets as
`mixed-wall-corner-integrated-native.json`. Candidate anchors come from at most
16 distinct authored pcurve control ends per face. Evaluated coordinate equality
only filters proposals; both original surface identities must be certified.
Preparation uses the existing exact budget, capped at 2,000,000 per pair.
The prepared paths remain bound to the original Body and ordered face pair.

- Complete mixed pair proofs remain **17/45**; no proofs were lost or added.
- Pending mixed UV products: **3124 -> 2714**. Product counts describe the
  adaptive decomposition, not a physical area or percentage of wall coverage.
- Exact preparation work: **37,683,566 -> 55,479,571**, within 100,000,000.
- Mixed geometric spans: **59,588 -> 92,369**, within 100,000.
- Mixed cells: **30,461 -> 23,565**; normal spans **11,647 -> 9,918**;
  domain cells **314,744 -> 289,302**; factored queries **4,562 -> 3,899**.
- Whole-wall admission remains false; 28 distinct-face pairs remain pending.
- Native source-support tests: 17 passed; additional automatic anchor selection
  and zero-budget refusal regression passed. No WASM/worker/UI qualification
  of this update has been performed yet, and no runtime speedup is claimed.


## Automatic anchors: packaged worker and current UI

Production WASM SHA-256:
`3ad1975343ec8db427089d3f06d713ab8c870f287fa754fdb04f29fc7d017dcb`,
12,142,818 bytes. The package audit confirms its exact Brotli/Base85 round trip;
28 real source-body worker tests pass (133.80 seconds overall), including
native/worker comparison of cells, spans, normals, domains, exact preparation,
queries, qualified pairs and pending products. Historical forged-response,
cancellation and late-result tests are included in that suite.

Current UI reloaded this package, restored the control body, prepared original
edges, cancelled a whole-wall request, and completed a fresh request at 5.99 mm.
It shows 28 unqualified pairs, mixed 17/45, 2714 pending UV products, self 27/27,
localized refusal and scene uncertainty highlights. `mixed-wall-auto-anchor-ui.jpg`
records that result. The result Retry control completes a fresh request with the same 28/17/2714/27 result.
Whole-wall admission remains false. This is wall-control qualification, not
completion of general fillets or independent bracket/flange/enclosure acceptance.

Distribution: 152 artifacts, 8,415,805 asset bytes plus 15,439,210 raw WASM bytes
= 23,855,015 bytes. Geometry chunk 4,050,304 bytes; named geometry budget retains
608-byte margin, total asset budget retains 761-byte margin. Other budgets stay
unchanged. Vue and MCP type checks pass. No end-to-end speedup is claimed.


## Reuse anchor enclosures across direction proposals (native)

`qualify_polar_anchored_chords_with_projections` validates 1..26 finite directions
before any early proof. Source derivative paths and normals are enclosed once
per UV product, then each proposed direction is independently checked by the
same positive chord cone criterion. No floating-point sample authorizes a proof.
Original Body/face ownership and geometric work limits are retained.

With the unchanged production 1024 mixed cell limit, complete mixed pair proofs
remain **17/45**, with no proofs lost. Geometric spans **92,369 -> 64,370**.
Cells **23,565 -> 30,461**, normal spans **9,918 -> 11,647**, domain cells
**289,302 -> 314,744**. Exact preparation remains **55,479,571**. Pending UV
products **2714 -> 3006** reflect a different adaptive decomposition: earlier
per-pair geometric work exhaustion stopped some subdivisions. Counts alone
are not a physical coverage metric. Whole-wall admission remains false.

The 4096 mixed-cell experiment exhausts the shared geometric span budget
(99,999/100,000), reaches only 16/45 proofs and leaves 4853 pending products.
It does not justify raising the production per-pair cell limit; keep 1024.
Both raw measurements are archived here. Native NURBS family: 32 tests pass.
Original annular regression, bounded-direction validation and budget-refusal
checks pass. This change has not yet been packaged into WASM; the current
worker/UI package remains the 3ad19753 artifact qualified above. No runtime
speedup or new full-wall qualification is claimed.


## Prepared normal factors in anchor products (native)

The whole-wall path now supplies original prepared normal factors to the
batched anchor criterion when its remaining query budget covers them. Each
supplied factor is conservatively charged before the call, including factors
not reached after a path refusal. Native source/pole validation is retained;
a source/pole incompatibility falls back to the ordinary normal path.

At the same 1024 mixed-cell limit: proofs remain **17/45**, pending products
remain **3006**, geometric spans **64,370 -> 64,252**, factored query charges
**4562 -> 4975** (limit 10000). Cells 30461, normal spans 11647, domain cells
314744 and exact preparation 55479571 remain unchanged. No complete proofs
were added or lost; whole-wall admission remains false. The original annular
regression passes. The increased query cost is recorded; no speedup is claimed.
This native update is not yet in the currently qualified 3ad19753 WASM package.


## Derivative-cone direction proposal (native)

The batched polar anchor gate may additionally try the sum of normalized
midpoint generator vectors. This rounded vector is only a proposal: the original
interval cone predicate must still prove strictly positive projections for every
generator and the requested normal-angle bound. Empty/invalid/zero midpoint
sets yield no proposal. The single-direction API keeps its previous semantics.

A regression gives an exact XY cone whose two nearly opposite generators have
no positive projection among all 26 integer directions. The derived direction
passes the existing interval predicate. Enlarging one generator across zero
still refuses. The original source-path regression additionally proves that the
batch can use its proposal with the same eight geometric spans while the
single zero-direction call refuses. Native normal tests: 18 passed, one existing
ignored; source-chord family: 32 passed; annular Body regression passes.

On the original annular whole-wall control this proposal adds **no new proof**:
17/45 complete mixed pairs, 3006 pending UV products, 64252 geometric spans,
30461 cells, 11647 normal spans, 314744 domain cells, 4975 charged factor queries,
55479571 exact preparation work. No proofs are lost. Whole-wall admission is
false. This diagnostic change remains native and has not replaced the qualified
3ad19753 WASM/worker/UI artifact. Remaining shared-edge products require stronger
source chord enclosures; no claim of general fillet qualification is made.

## Shared carrier parameter and derivative preparation

Native `source_shared_chord_map` binds an affine natural-boundary pcurve map to
one privately admitted Body edge and its two original owned surfaces. It uses
exact full source-map admission, not sampled world coincidence or promotion of
retained root endpoints. Curved and nonlinear pcurve maps remain unsupported.
The original annular faces 0/2 map to canonical s through u=s and u=3(1-s).
The regression checks both face orders, outward inverse intervals, invalid UV,
zero edge inspection budget and original shared tangent preparation.

`normal_alignment::derivative_path_bounds` retains source identity, coordinate,
and domain in private fields. It covers every intersected nonempty knot span,
including both sides at a knot and zero-width boundary charts, with an explicit
span budget. Invalid/disconnected source surfaces refuse. The Body map scales
these original derivative bounds by the original affine chart coefficient.

`shared_chord_cone_excluded` provides a sufficient interval Schur-complement
and copositivity test for an unrestricted common tangent coefficient and two
nonnegative transverse coefficients. It refuses opposing transverse vectors,
normal alignment and uncertain cancellation. This algebraic gate alone is not
a source chord or whole-wall certificate: the original transverse decomposition
still must be constructed and connected to the automatic pair controller.

Validation: 21 native normal-alignment tests passed, one existing ignored; the
annular original Body regression passed. These changes remain native. No new
whole-wall pair proof or general fillet completion is claimed; WASM remains the
previously qualified 3ad19753 artifact.

## Original shared transverse paths in pair coverage

The Body-bound boundary map now integrates each original fixed-coordinate
surface derivative from its natural chart boundary to the endpoint product.
The known inward direction and subtraction on side zero give nonnegative
transverse coefficients. The common carrier derivative retains one unrestricted
coefficient instead of two independent point anchors. Original endpoint normals
and all derivative/normal span costs are charged; incomplete work cannot exclude.
The mixed pair controller invokes this gate on terminal UV products. Original
retained-domain, positional-distance and existing anchored proofs remain active.

Native control with unchanged 1024-cell mixed limits:
17/45 complete pair proofs (none lost), pending UV products **3006 -> 2247**,
cells 30461, geometric spans **64252 -> 62406**, normal spans **11647 -> 12406**,
domain cells 314744, factor queries **4975 -> 4797**, exact preparation 55479571.
These product counts are subdivision diagnostics, not percentages of geometric
coverage or a complete wall certificate. Whole-wall qualification remains false.
The artifact `mixed-wall-shared-transverse-native.json` retains native pair
statuses and unresolved original UV products. All 17 source-support-shell tests
passed. This work has not yet replaced the qualified WASM/worker/UI artifact.

## Packaged shared carrier proof through WASM, worker and UI

The native batch/proposal/shared-carrier changes are now packaged in WASM SHA
`7525a8b08323619859bc129839db70df68e924c5a8fcd27e67a8e17feab8f63b`,
12,160,834 raw bytes. Production Brotli+Base85 audit verifies exact roundtrip,
zero external imports and the export surface. The 28 real source-body worker
suite tests pass in 110.99 seconds; the mixed control compares every ordered
pair's status, cells and pending count, plus all aggregate derivative/normal/
domain/exact/factor budgets against the native shared-transverse artifact.
The suite also retains cancellation, stale response and forged transport gates.
These tests are selected source-body scenarios, not the full P0 command matrix.

Current UI restored the control body, prepared source edges, selected edge zero,
set minimum thickness 5.99 mm, canceled the first whole-wall request, then
completed a fresh request and Retry. Both results show 28 unqualified pairs,
17/45 additional mixed pairs, 2247 pending mixed products and self 27/27, with
localized incomplete-wall diagnosis and orange scene highlights. No console
errors were observed. `mixed-wall-shared-carrier-ui.jpg` records the Retry result;
worker and package audit artifacts preserve the exact shipped SHA.

Vue and MCP typechecks pass. Dist verification passes 152 artifacts: 8,420,991
asset bytes + 15,457,226 raw WASM bytes = 23,878,217 total. Only geometry chunk
and total asset budgets changed for measured 5186-byte growth; prior 608-byte
chunk and 761-byte total margins remain. Other budgets are unchanged. No runtime
speedup or complete wall/fillet qualification is claimed. Original general
fillets, unsupported carrier maps, outstanding wall pairs, full part acceptance
and the wider P0/P2/P3 roadmap remain open.

## Boundary mean integration experiment

Native `boundary_mean_bounds` encloses the averaged original coordinate
surface derivative by covering each full interval of the normalized path
parameter. This uses interval integration with exact rational partition weights,
not sampled quadrature. The private result preserves source, original endpoint
rectangle, coordinate and natural boundary. Partial span coverage refuses.
A polynomial quadratic control narrows the mean derivative from the full
pointwise range [0,2] to a width below 0.26 around the exact mean 1. A rational
quadratic control covers independently evaluated analytic secants across its
endpoint interval. Native normal tests: 22 passed, one existing ignored.

An eight-piece transverse integration experiment on the same original annular
control yields no additional pair/product proof: 17/45, 2247 pending products,
all previous statuses retained. Derivative work increases 62406 -> 73032;
other native aggregate counters remain unchanged. The separate 4096-cell
numerical refusal control exhausted derivative work at cell 3488 instead of
4096. Its existing stopping regression was preserved, rather than weakened.

Therefore automatic wall coverage retains the original derivative-hull path.
`SharedBoundary::exclude_normal_chords_with_mean` exposes the optional bounded
original mean integration for further qualification, with explicit work counts;
it is not enabled in the automatic UI controller. The experiment artifact is
native-only and does not replace the qualified 7525a8b0 WASM. The observed lack
of gain motivates correlated secant bounds rather than a larger integration
budget. No claim of wall completion or performance improvement follows.

## Correlated original rational boundary secants

Native `normal_alignment::boundary_secant_bounds` constructs the homogeneous
Bernstein numerator H(u,v)W(u,b)-H(u,b)W(u,v), preserving its common free
coordinate. Its boundary coefficient is symbolically zero by antisymmetry of
the common polynomial product; the parameter factor is divided out in the
Bernstein basis before interval evaluation. The resulting private source-bound
certificate encloses the original coordinate secant and its boundary limit.
It supports one original clamped integrated Bezier span of degree 1..8 and
multiple original free-coordinate spans of degree up to 8. Unsupported layouts
refuse. Missing free-span coverage refuses with charged work. No sampled
coincidence or fitted coefficient authorizes the division.

Three tests cover polynomial/rational analytic secants, boundary limits, UV
transposition, multiple free knot spans, unsupported integrated layout and
budget/invalid boundary controls. The admitted annular Body regression also
checks the optional `exclude_normal_chords_with_secant` method.

The native experiment selected secants in place of transverse derivative hulls:
all aggregate counters and statuses are unchanged (17/45, 2247 pending products,
62406 geometric spans, 12406 normal spans). No new complete proof is claimed.
Automatic coverage therefore retains its previous path. The new secant method
is available separately for further native qualification; shipped WASM remains
7525a8b0. Original face 0 has a collapsed high-U control rail with unequal
weights, whereas face 2 has no collapsed U rail. Further directional proof must
handle this original zero factor explicitly rather than normalize a vector
whose interval contains zero. The experiment artifact records this boundary.

## Exact original pole factors in correlated secant directions

`boundary_secant_direction_bounds` removes a normalized nonnegative free
coordinate factor only when original clamped Bezier endpoint control rows share
one exactly equal pole. Unequal positive rational weights are supported:
H-cW has exactly zero original rows, so the rational chord numerator has the
corresponding factor before interval arithmetic. Bernstein degree reduction
preserves its original multiplicity. The private direction certificate reports
source, endpoint product and (high/low end, power). Its components are directions,
not physical derivative magnitudes. A whole constant chart refuses.

Five secant tests pass, including high-end unequal weights, a double low-end
factor, pole limits and an independent rational formula. Changing one rail
point by 1e-15 prevents factor admission. The optional Body-bound factored
secant method passes the existing rolling-boundary control. The experiment on
the full original annular control preserves all counters and statuses but adds
no proof: 17/45, 2247 pending products. Automatic coverage retains the previous
method; these additions remain native and shipped WASM is unchanged.

An exact dyadic source audit establishes why a second pole factor cannot simply
be assumed. Face 0's adjacent constant rail differs from the pole in position;
its weight profile is only approximately proportional to the pole profile.
The exact cross product w4[0]*w5[1]-w4[1]*w5[0] equals
-165809539722621 / 2535301200456458802993406410752 (approximately -6.54e-17),
not zero. `factored-secant-pole-weights.json` preserves original weights and
this exact result. Removing a second factor would change the represented
geometry; a supported correction requires its own bounded error and source
identity treatment. No tolerance-based coefficient deletion is performed.

## Whole-domain positional bound for a separable weight proposal

Native `qualify_weight_profile` binds a proposal to the original surface and
covers all original controls. Target weights are exact expressions
row[i]*column[j]/pivot using authored original row/column weights, not flattened
binary64 replacements. Positive rational basis partition of unity gives
T-S = sum B_ij (targetWeight-originalWeight)(P_ij-S)/targetW.
Original control coordinate extents bound P_ij-S, and the positive target-weight
lower bound supplies the denominator. Every arithmetic step rounds outward.
The private proposal certificate reports complete whole-domain axis bounds;
missing controls provide no certificate. Periodic layouts currently refuse.

Two native tests cover one-ULP changes, analytic planar evaluation, unchanged
original source, insufficient work, invalid anchors/tolerances and rejection
of a large change. The admitted original annular Body regression covers all
18 face-0 controls with pole-row anchor [5,0]. Axis deviation bounds are
[2.5515425836831596e-15, 7.294274923020359e-15, 1.1913036837556104e-15] mm;
weight deviation upper is 6.66133814775094e-16. The positional 1e-7 mm tolerance
passes. `weight-profile-position-native.json` records the native output.

This is a proposal bound only. It neither alters the represented source nor
qualifies tangent continuity, rebuilt neighboring carrier identities or a
replacement closed Body. Flattening the exact product expressions to f64 adds
a separate error and can reintroduce the pole residual. Applying a correction
requires retaining its exact factored representation and requalifying seams,
wall/volume, exchange and history. Shipped WASM and its 28 pending wall pairs
remain unchanged. No new whole-wall proof follows from positional proximity.

## Evaluation and pole directions of the exact proposed weight profile

`WeightProfile::enclose` evaluates exact row*column/pivot weight expressions
inside outward homogeneous B-spline extraction. A shared internal extractor
accepts those weight expressions; its original Surface path supplies unchanged
original point weights. Target coefficients are never flattened to new f64
weights. Rectangle validation, complete original span union and work budgets
remain required. Independent separable rational formulas and missing-span
controls qualify the new evaluation; the annular proposal also encloses its
original pole [20,0,6] within a 1e-10 mm point box.

`WeightProfile::boundary_direction` separately binds a private direction
certificate to the proposed profile identity, endpoint product, axis and natural
boundary. Exact separability cancels transverse chords for each constant source
control row even when those rows are different points. It removes the original
normalized nonnegative free-coordinate factor in the Bernstein basis. Supported
layouts currently require one clamped Bezier span per axis with degrees 1..8.
It does not reuse that corrected direction as an original Surface certificate.

For original annular face 0, proposal anchor [5,0], U=1 and V=0.5, the proof
establishes the high-U factor power 2. Remaining direction intervals are
[[2.1966991411013588,2.1966991411013996],
 [5.818466262050285e-14,5.81846626205039e-14],
 [-5.3033008588991475,-5.303300858899055]].
The independent control covers this double factor for distinct constant endpoint
rows and the continuous pole limit. This is a direction after factor removal,
not the physical secant magnitude. The native artifact preserves the result.

Validation after the shared extraction and direction implementation: all 753
NURBS lib tests pass (two existing ignored), and all 17 source-support-shell
tests pass. The original Body, its source definitions and shipped WASM remain
unchanged. Applying this proposal still requires a closed corrected-body
representation, neighboring original carrier verification, seam/wall/volume
checks and STEP/history acceptance. The 28 original pending pairs are not
promoted by this proposal certificate.

## Boundary-preserving native body proposal

`source_profile_body::prepare` binds exact profile recipes to an immutable
original Body. Admission checks all four natural boundaries, including collapsed
pole rails, and every original retained contour fragment. Boundary equality
requires exact original weight identity or an exactly constant control rail;
nearby floating-point values never establish identity. Contour control points
must all lie on a single preserved natural boundary. Interior trims currently
refuse admission.

The annular face 0 proposal with anchor [5,0] preserves all four contour
fragments using 44 control checks. Original world edges, root ends, Body
identity and source definitions are retained. Limits of 43 controls or fewer
than four fragments refuse; the exact 44-control/four-fragment budget accepts.
The profile kernel suite passes all six tests.

The definition stores original Body plus exact factor recipes, not flattened
weights or certificates. Restoration and corrected embedding, volume, seam,
wall and STEP qualification remain outstanding. This proposal is not an
admitted corrected `source_volume::Body`; it does not reduce the 28 unresolved
original-body pairs and is not included in the packaged WASM.

All 17 source-support-shell tests pass after the body-boundary admission and
budget refusal controls. No original Body qualification gate was relaxed.

## Exact-profile normal enclosure

`WeightProfile::normal_bounds` now covers complete original knot rectangles
using exact factor expressions in homogeneous extraction, quotient derivatives,
and their interval cross product. Both one-sided knot limits are included;
missing spans return no enclosure. Zero-containing bounds do not establish
regularity. Original Surface differentiation shares the same coefficient
calculus and keeps its original weights and translation preconditioning.

An independent analytic separable rational surface checks normal orientation
and magnitude at interior and boundary parameters. A multi-span refusal control
checks incomplete work and knot coverage. All 755 NURBS lib tests pass, with
two existing ignored tests. This supplies differential bounds for subsequent
corrected seam/wall admission; it does not itself qualify the corrected body or
its singular poles. Packaged WASM remains unchanged.

## Proposed/original tangent-plane comparison

`WeightProfile::inspect_original_normals` compares complete target-profile and
original-surface normal enclosures under one shared span budget. It retains
indeterminate results for incomplete work and singular normals, and does not
claim shared position, seam ownership or oriented normals from an angle test.
Eight profile tests pass, including parallel, tilted, collapsed and underbudget
controls.

The actual annular face 0 / face 5 seam on V=[0.49,0.51] yields squared-sine
bounds [0,0.007973550652571522], so a 1e-6 threshold remains unproven. This is
interval dependency over a finite segment; the segment is not promoted to a
qualified seam. Adaptive complete-seam coverage remains required.
At seam midpoint V=0.5, the corrected/original tangent planes qualify with
squared-sine upper bound 2.7218457621120856e-25. The actual annular regression
passes and explicitly keeps the finite segment unresolved. A point result does
not establish whole-seam tangency.

## Complete adaptive corrected/original seam angle coverage

`WeightProfile::inspect_original_path` covers both affine UV lines over the
entire canonical [0,1] interval. Outward coordinate maps include endpoints;
every accepted cell compares complete normal products. Cell and span limits
retain the remaining canonical intervals, and only an empty queue yields a
qualified angular result. The API does not claim positional edge identity or
shell orientation from supplied line coordinates.

Ten profile tests pass, including a curved line requiring subdivision, retained
pending work, angular mismatch and incomplete normal coverage. The actual
annular face 0 U=0 / face 5 U=1 seam with V running from 0 to 1 qualifies at
squared-sine tolerance 1e-6: 14697 inspected cells, 29394 normal spans, 7349
accepted intervals, zero pending intervals, upper bound
9.996374481646285e-7. The fixture asserts complete coverage and tolerance.

This qualifies this corrected/original seam's tangent-plane angle only.
Other seams, singular end transitions, corrected volume/wall/self-intersection
and STEP admission remain outstanding; no original pending wall pair or whole
body is promoted. The implementation remains native, outside packaged WASM.

## Owned corrected seam certificate

`Proposal::qualify_seam` obtains both affine UV lines from a privately admitted
original shared-carrier map. Callers choose face addresses and work limits;
they do not supply UV paths or angle certificates. The profile proposal's
boundary-preservation gate keeps that carrier's geometry unchanged. A private
`source_profile_body::Seam` binds the complete angular result to the proposal,
face pair, actual edge address and tolerance. Both changed incident faces
currently refuse this original-neighbor path.

The annular regression uses face pair [0,5], owned edge 5, instead of manually
specified UV lines. It checks proposal identity, full angular coverage and
refusal for zero edge-inspection work or incomplete normal work. The earlier
angular-only API remains explicit about its lack of positional ownership.
Neither certificate admits corrected volume, embedding, wall thickness or STEP.

## Two proposed incident faces

The complete adaptive angular path can now compare two exact `WeightProfile`
representations. Original/profile and profile/profile paths share coverage,
budget and pending-interval logic. Both normal calculations use their actual
homogeneous weight expressions. The owned proposal seam gate selects this path
when both incident faces are proposed, preserving original carrier authority
and binding its certificate to the complete proposal.

Eleven profile tests pass. A control uses different original interior weights
whose proposed separable factor recipes describe the same curved surface;
complete angular coverage qualifies, incomplete work remains pending, and a
changed control geometry rejects. This supersedes the preceding deliberate
refusal for two changed incident profiles; it does not expand positional
boundary admission or qualify corrected volume/wall/STEP.

The actual annular proposal selects faces 0 and 5 with anchors [5,0] and [0,0].
Boundary admission succeeds for both. Owned edge 5 then qualifies over its whole
length: 15121 cells, 30242 normal spans, 7561 accepted intervals, no pending
intervals, squared-sine upper 9.985969734463084e-7 at tolerance 1e-6.
The native annular regression passes, including the original/profile path and
its work-limit controls.

## Exact owned edge selection

`source_shared_chord_map::prepare_edge` restricts the carrier search to one
actual shell edge address. Its bounded scan cannot substitute another edge
between the same faces. `Proposal::qualify_edge` derives incident face addresses
from original shell uses and runs the profile/original or profile/profile gate
for that exact edge. Face-pair convenience selection remains a single selected
carrier result, not coverage of every edge between those faces.

The annular control checks a wrong face/edge combination, insufficient search
coverage, an invalid address, and the existing full edge 5 angle proof through
this exact-edge route. Initial incident-edge audit uses one angular cell/two
normal spans to expose unsupported carriers and pending intervals; it does not
claim whole incident seam qualification.

Native incident inventory for proposed faces [0,5] contains exactly edges
[0,1,5,6,8,15]. Under the initial one-cell budget, edges [1,5,8,15] retain
canonical intervals [0,0.5] and [0.5,1]; edges [0,6] have unsupported carrier
maps in this affine-boundary path. The separately fully funded edge 5 proof
continues to pass. The actual annular regression passes after exact selection.

## Original pcurve fallback for corrected seam angles

Exact owned edge selection now has an original-pcurve fallback when affine
natural-boundary maps are unsupported. It uses the privately admitted canonical
parameter correspondence, both original UV curves and their complete interval
span enclosures, then compares the actual original/proposed surface normals.
Source identity is checked against both retained owned face regions. Periodic
or incomplete original canonical maps remain unproven.

The fallback separately inspects canonical endpoints before subdivision.
Singular endpoint normals remain unresolved. Its shared span quota includes
both pcurve spans and normal spans; no partial angular result becomes a seam
certificate. Boundary geometry preservation still comes from proposal admission.
The zero/insufficient edge-search limit cannot activate this fallback.
Eleven profile kernel tests pass after exposing the direct profile/profile
normal comparison used by the fallback.

Actual original-pcurve results for proposal faces [0,5]: edge 6 qualifies over
its whole length with 291 inspected cells, 1164 combined curve/normal spans,
147 accepted intervals, no pending intervals and squared-sine upper
9.837155135567825e-7 at tolerance 1e-6. Edge 0 remains unresolved at an explicitly
inspected canonical endpoint: one cell, four combined spans, no accepted cells,
three pending intervals (whole carrier and both endpoint checks). The regression
asserts this refusal separately from the admitted edge 6 proof. These results
supersede only the previous unsupported-map status of those two edges.

## Remaining incident seams and endpoint preflight

Full funded baseline checks classify edge 1 as unresolved after 44063 cells,
88126 normal spans, 22021 accepted cells and 21 pending intervals at floating
parameter resolution. Edges 8 and 15 qualify over their entire carriers.

The affine owned-seam path now checks both literal canonical endpoints under
the same cell/span quota before full subdivision. A singular or unresolved
endpoint retains whole-carrier coverage as pending and cannot authorize a
certificate. Endpoints with insufficient normal work report the work limit.
Qualified endpoints are included in final angular bounds and work counters.

For proposed faces [0,5], edge 1 now refuses in two cells/four normal spans:
one regular endpoint accepted, whole [0,1] and endpoint [1,1] pending,
reason `profile-seam-endpoint-normal-unresolved`. This avoids spending the
previous 44063 cells on a known endpoint refusal; no accepted geometry expands.
Edge 8 still qualifies: 16149 cells, 32298 normal spans, 8076 accepted cells,
upper squared sine 9.99720292310633e-7, no pending intervals. Edge 15 qualifies:
23247 cells, 46494 normal spans, 11625 accepted cells, upper squared sine
9.996371266985573e-7, no pending intervals. Both use tolerance 1e-6.
The original/profile edge 5 result includes endpoint work (14699 cells,
29398 spans, 7351 accepted cells); its angular upper bound remains unchanged.

Current incident angle qualification covers edges [5,6,8,15]; endpoints of
[0,1] remain unresolved. This is not whole-body embedding, volume, wall,
self-intersection or STEP qualification, and remains outside packaged WASM.

All 18 source-support-shell tests pass after endpoint preflight, including the
explicit singular-endpoint and quota-refusal controls and full incident edge
5/6/8/15 angular checks. No original Body admission gate was relaxed.

## Corrected pole limiting-plane diagnostic

`WeightProfile::pole_limits` combines its exact nonnegative factored transverse
boundary direction with the exact profile's U derivative at both V endpoints
of a collapsed natural U rail. Its private source profile supplies all
homogeneous expressions. The returned diagnostic does not assert regularity
or authorize G1 at a pole. Unsupported factors and missing span work return no
diagnostic, and zero-containing normals cannot prove distinct tangent limits.

The actual corrected annular face 0 high-U pole [20,0,6] has distinct limiting
planes: squared-sine interval [0.9999999999997745,1]. It consumes four spans.
The limit normals point essentially along Z and X respectively. Thus the
profile correction does not remove the original sharp pole; admitting a smooth
end requires changing the transition geometry, not just accepting the corrected
weights or the zero normal. Twelve profile tests and the native actual pole /
endpoint-preflight regression pass, including underbudget and noncollapsed
controls. Other incident seam certificates remain separate from this diagnosis.

## Exact new section carrier for endpoint reconstruction

`WeightProfile::section` supplies a private `ProfileSection` view bound to one
exact proposed profile, cut coordinate and full natural free-parameter range.
Its position and incident-support normal enclosures use the existing exact
homogeneous expressions. No iso curve weights are flattened into new f64
controls. Incomplete span budgets return no position/normal bound; parameters
outside the owned section range are errors. This creates a new carrier view,
not a trimmed face, new cap or closed Body.

The annular face 0 cut at U=0.75 stays away from pole [20,0,6]. Its V=0.5
midpoint enclosure is [[19.847227256480796,19.847227256481446],
[1.950029346061504,1.950029346061577],
[5.942794293200402,5.9427942932005955]]. All 64 consecutive V intervals have
support normal enclosures excluding zero. Thirteen profile tests and the
native actual section/pole/endpoint regression pass. These bounds prepare a
regular new junction boundary; adjacent retained-face cuts, cap construction,
sewing, embedding and volume qualification remain required.

## Owned new cut endpoints on neighboring retained faces

`Proposal::cut_endpoint` binds a section end to an exact original Body edge and
both original retained uses. The proposed incident pcurve must be an affine
constant-weight curve on the preserved natural boundary; its neighboring
pcurve may be nonlinear. Source definitions must match both owned face regions.
The cut coordinate supplies an exact parameter recipe: original curve parameter
and canonical carrier parameter are enclosed with outward arithmetic, then
mapped onto the neighbor and original world carrier. Both mapped parameters
must lie inside the guaranteed retained endpoint ranges, including conservative
root enclosures. Otherwise no owned endpoint is returned.

The private endpoint retains proposal identity, actual edge address, section
coordinate and a replayable definition recipe. Returned coordinate intervals
are diagnostic enclosures of that recipe, not new rounded vertices or cached
ownership certificates. Complete curve-span work is required; foreign edge,
invalid axis and missing-work controls refuse.

For annular face 0, cut U=0.75, the V=0 end maps onto edge 0 / neighbor face 1
at canonical [0.24999999999999753,0.25000000000000205], neighbor UV
[[19.709782279111632,19.709782279111653],
[1.9365251050974668,1.9365251050975405]]. The V=1 end maps onto edge 1 /
neighbor face 2 at canonical [0.7499999999999988,0.750000000000001], neighbor UV
[[0.7499999999999892,0.7500000000000108],[1,1]]. Each uses two curve spans.
The native actual cut/pole/endpoint regression passes. New adjacent trim paths,
cap geometry and closed-body admission remain required.

## Root-valued source-edge partitions at the new section

`CutEndpoint::split_source_use` creates an unchanged original UV cut line and
independently certifies its unique intersection with the selected original
boundary pcurve. Both directed fragments use clones of the same private
`SourcePoint` root expression. Their full original surface/curve definitions
remain unchanged. The cut root is transported through the privately admitted
canonical world carrier into the original neighbor use, retaining strict
interior ordering and the same cut enclosure on both neighbor range views.
These neighbor ranges are expression views, not admitted replacement Fragments
or a new neighbor contour. A zero mapping budget errors before qualification.

Actual edge 0 split root parameters are
[[0.7499999999999971,0.7500000000000026],
[0.33333333333333204,0.3333333333333344]]; neighbor cut interval is
[0.24999999999999564,0.2500000000000046]. Actual edge 1 split roots are
[[0.2499999999999987,0.25000000000000117],
[0.6666666666666641,0.6666666666666689]]; neighbor cut interval is
[0.749999999999997,0.7500000000000033]. Each root uses one mapping cell.
The actual source-partition regression verifies root-valued shared joins,
unchanged source curves and equal transported neighboring cut ranges.
This prepares real selected-face boundary fragments; adjacent new trim paths,
cap geometry and closed-body qualification remain required.
Both new selected-face fragments also restore through the existing native
root-requalification gate. Their joins and full original definitions survive
the round trip; no cached endpoint bounds authorize restoration. The native
actual cut/pole/partition/restore regression passes.

## Fresh neighboring fragments and shared-edge halves

`CutPartition::qualify_neighbor` treats the neighbor cutter/selector as proposal
inputs. It independently certifies the original neighbor crossing, splits its
original directed Fragment, and freshly qualifies both common world-edge halves
through exact affine composition and actual endpoint-root identity. The two
half-edge queries share the exact-work allowance and remaining driver allowance.
A partial result is never issued as a complete neighbor partition.

For annular edge 1 / neighbor face 2, an original UV cutter U=0.75 with V from
-1 to 2 and root selector [[0.7,0.8],[0.61,0.72]] qualifies both half-edges.
Fresh exact composition/root identity uses 220781 work units. Both neighbor
Fragments retain the original curve and shared crossing root. The half-edges
correctly refuse complete-source coverage claims. A displaced U=0.8 cutter and
one-unit exact-work allowance refuse instead of matching nearby roots.

Both actual new shared-edge halves restore through existing original-source
qualification; their full definitions survive and they remain partial carriers.
The native actual cut/pole/partition/shared-edge/restore regression passes.
This advances neighboring contour preparation for edge 1. Edge 0's nonlinear
neighbor still needs a suitable exact cut representation or transported root
endpoint support. A full replacement contour, endpoint cap and closed-body
qualification are still outstanding.

## Original carrier root transport for nonlinear neighbors

`source_carrier_point::qualify` privately binds a certified original UV crossing
root to its admitted original shared carrier. Source Surface and boundary Curve
must equal the selected original use. Full original canonical maps are required;
both retained uses must strictly contain the transported root enclosure.
Complete original UV/world curve spans are charged before a Point is issued.
The Point retains original edge/root definitions and exact correspondence;
its UV, parameter and world boxes never become fixed endpoint coordinates.

Restoration independently requalifies the original shared edge, root equations
and carrier transport. Cached coordinate fields are ignored. An altered source
side fails source identity, and incomplete span work yields no Point.

Actual edge 0 transports its selected root into the nonlinear neighboring
pcurve at parameter [0.24999999999999564,0.2500000000000046], canonical
[0.24999999999999598,0.25000000000000416]. Edge 1 transports to neighbor parameter
[0.749999999999997,0.7500000000000033], canonical
[0.7499999999999981,0.7500000000000021]. Each costs three original curve spans.
Both actual carrier/root transport and fresh restoration regressions pass.

This establishes a native transported-root object for subsequent endpoint
integration. It is not yet a Fragment endpoint variant, a replacement contour,
new endpoint surface or corrected closed Body. Those integrations remain open.

## Incident-use restrictions at transported roots

`Point::retained_parts(side)` now represents both halves of either original
incident use, including the degree-five neighboring pcurve on edge 0. The halves
retain the private admitted carrier and original root expression. Directed
parameter bounds enclose that expression; they do not replace it with a scalar.
Joining requires the same source side, original carrier/root definitions and
first-to-second traversal. Coordinate overlap cannot establish a join.

Complete UV enclosures charge every intersected original curve span and refuse
insufficient work. The actual edge 0 and edge 1 regression checks both incident
uses, their outer endpoints, shared root bounds, reversed join refusal, foreign
side refusal and zero-span refusal. It passed in 2.41 seconds. All five existing
source-boundary-fragment tests passed in 0.06 seconds.

These are exact source restriction views. Integration into Fragment endpoints,
fresh qualification of both restricted shared carriers, restoration of those
restrictions and closed-body assembly remain required. No packaged WASM changed.

## Transported root Fragment endpoints

`Endpoint::Carrier` and `Fragment::split_at_carrier` now keep the admitted
original carrier/root expression at an actual source fragment endpoint. Source
surface and pcurve identity are checked against the chosen parent use; splits
require strict interior ordering. Joining two halves requires identical carrier
recipes and incident sides, without comparing coordinate proximity.

Restoration requalifies the original parent edge, root and transport. Nested
carrier parents are rejected before recursive restoration. This first restore
contract admits one carrier endpoint per fragment: it consumes the remaining
mapping allowance, with separate parent-use/root mapping bounds and fixed
100-million exact-work, 100000 driver-cell and span ceilings. A second carrier
endpoint requires a future shared-budget contract and is explicitly refused.

Actual edges 0 and 1 split and restore both incident fragments, including the
degree-five neighbor. Foreign source sides, zero restoration budget and nested
parent definitions refuse. The fixture passed in 2.44 seconds. Fragment tests
passed 5/5, mapped-edge tests 5/5 and shared-edge/restore tests 10/10.

Contour/fiber endpoint queries use conservative certified UV enclosures. Inverse
shear and exchange refinement refuse these endpoints until replay is supported.
Direct and mapped shared-edge admission also explicitly refuse Carrier endpoints;
matching both restricted halves to their parent correspondence is the next proof
step. Closed-body assembly and packaged WASM integration remain incomplete.

## Fresh admission of transported-root shared halves

Mapped shared-edge admission now recognizes a Carrier/Carrier or
Carrier/Boundary-Crossing endpoint pair through the private original parent.
The world Curve, both incident source definitions and both original affine maps
must be unchanged. Incident sides must differ. A crossing must retain exactly
the carrier's original root definition. Fresh source/world composition and
opposite traversal qualification still run before endpoint identity is used.

`CutPartition::qualify_transported_neighbor` integrates this route without an
independently authored neighbor cutter. It charges carrier spans and shares
exact-work/driver quotas across both admitted halves. Actual edges 0 and 1 pass
both mixed and Carrier/Carrier admission, including the nonlinear degree-five
neighbor. Mixed halves restore through fresh original definitions; changed
world geometry, altered maps, repeated incident side and insufficient work
refuse. No admitted half is labeled complete source coverage.

The control regression passed in 2.43 seconds; mapped tests passed 5/5 and
shared-edge/restore tests 10/10. These fixtures prove source-owned half-edge
correspondence only. Replacement face contours, corrected surface ownership,
end caps, closed-body assembly, embedding and packaged WASM remain required.

## Closed original-source contours at a transverse section

`propose_cut_contours` connects two privately owned cut partitions on one
original face. They must belong to the same Proposal, face and exact section
parameter, with opposite free ends and identical original UV contact curves.
Each endpoint boundary must have one unambiguous occurrence in the retained
wire. The two cyclic arcs preserve original source fragments and join through
a contact fragment whose endpoints retain both certified root expressions.
Every cyclic join is verified. Fragment work is bounded before construction.

The annular face cut at U=0.75 produces two closed source contour candidates.
Every fragment restores independently and all cyclic joins survive. Repeated
cut ends, different section parameters and insufficient fragment work refuse.
The qualification fixture passed in 2.91 seconds; contour regressions passed
2/2 in 0.11 seconds.

The present builder supports a single retained wire with original scalar outer
ends. Hole assignment is explicitly unclassified. Closure does not prove
containment, orientation, immersion or self-intersection freedom. The section
is still owned by the original Surface; it is not an exact shared boundary of
the corrected weight profile. Exact corrected-section ownership, neighbor
replacement wires, cap geometry and closed-body admission remain required.

## Exact section tangent and transverse derivative bounds

`WeightProfile::derivative_bounds` differentiates the exact factored homogeneous
profile with the existing translated quotient calculus. Original intersecting
spans, including both one-sided limits at knots, are all charged. Incomplete
span work returns no certificate. `ProfileSection::tangent_bounds` and
`transverse_bounds` select the free and fixed original coordinate derivatives
without flattening new weights or materializing a rounded Curve.

An independently evaluated rational planar example checks tangent
[0,3/(1+2v)^2,0] and transverse derivative [0.64,0,0] at three parameters. A
two-span example refuses one-span work for both derivatives. All 13 profile
tests passed in 0.02 seconds. On the actual U=0.75 annular cut, both derivative
vectors exclude zero over each of 64 full intervals covering the section.
The B-rep control regression passed in 3.37 seconds.

These derivatives provide exact-profile boundary data for transition
construction. They do not prove an independently authored cap tangent, shared
position, positive thickness or embedding. Corrected-section ownership in
shared-edge admission and actual end-transition surfaces remain open.

## Exact Hermite end-transition expression

`ProfileSection::end_transition` constructs an immutable exact expression
H(s,t)=h00(s) S(t)+h10(s) l D(t)+h01(s) P, with cubic Hermite basis,
original exact section S, transverse support derivative D, finite target P
and finite nonzero scale l. Neither section weights nor derivative coefficients
are flattened into an authored binary64 Surface. Full parameter rectangle
enclosures share span work across positional and derivative queries; exhausted
work returns no enclosure. The exact endpoint at s=1 is the target point.

At s=0, H=S, H_t=S_t and H_s=l D identically. The private expression and
nonzero source normal therefore establish the same regular boundary tangent
plane. `regular_join` returns no certificate when normal bounds contain zero.
This is structural boundary equality, not a sampled angular approximation.

Analytic rational planar evaluations and bounded-work refusals pass. All 13
profile tests passed in 0.03 seconds. The annular transition from U=0.75 to
[20,0,6] preserves exact section enclosures and a regular join on all 64
intervals covering the seam. Whole transition enclosure work completes for
each interval. The B-rep control fixture passed in 2.66 seconds.

This is transition geometry, not a sewn cap. Side rails still need coincidence
with neighbor replacement contours; the collapsed target has no regularity or
embedding certificate. Interior Jacobian, self-intersections, thickness,
corrected shared-edge admission and closed-body/WASM integration remain open.

## Owned transition rails and across derivatives

`ProfileEndTransition::across_derivative_bounds` encloses the derivative of
the exact Hermite expression with shared source/derivative span work. At the
section it specializes to the exact scaled transverse derivative; at the
collapsed target it is exactly zero. `rail(high)` binds each side to this
immutable transition and original free endpoint. Rail positions and tangents
query the same expression, without creating an independently rounded Curve.

Analytic rational fixtures check both rails at transition parameters 0, 0.5
and 1, invalid domains and incomplete work. All 13 profile tests pass in
0.03 seconds. Actual annular rails preserve section endpoint enclosures and
the common target [20,0,6]; their positions and derivatives enclose all 64
intervals covering each rail. The control fixture passes in 2.86 seconds.

The zero tip derivative is explicit evidence of a singular parameterization,
not a regular-tip certificate. Neighbor rail coincidence, rail tangent planes,
interior Jacobian, embedding and closed-body admission remain required.

## Interior transition Jacobian diagnostics

The existing homogeneous quotient jet evaluator now accepts an explicitly
owned coefficient net; original Surface evaluation delegates to the same
unchanged calculus. `WeightProfile::mixed_derivative_bounds` uses the exact
factored profile net and complete original spans. Transition free derivatives
include this mixed derivative of the transverse field. `normal_bounds` crosses
both transition derivatives while sharing their span quota; incomplete work
returns no normal certificate. The collapsed tip remains explicitly singular.

Analytic free derivatives, complete-work and work-refusal tests pass. Profile
tests passed 13/13 in 0.02 seconds; original normal-alignment regressions passed
22 tests with one pre-existing ignored test in 0.01 seconds. The actual B-rep
control fixture passed in 2.97 seconds.

On a coarse 8x8 full-interval cover of the annular end transition, all 64 normal
enclosures are complete, but zero remains possible in every enclosure:
0 regular cells, 64 pending cells. This proves neither degeneracy nor regularity.
Adaptive refinement and tighter correlated Hermite bounds are needed before
interior immersion can be admitted. No closed-body certificate or WASM changed.

## Adaptive transition regularity with factored Hermite derivatives

`inspect_regularity` uses a breadth-first cover of the full requested rectangle.
A cell is accepted only when its complete normal enclosure excludes zero in
at least one component. Unresolved cells split without dropping either child.
Work exhaustion preserves the current cell and every queued domain; resolution
limits remain pending. Reports never infer regularity from sampled points.

Derivative bounds now use the algebraically identical factorizations
h00=(1-s)^2(1+2s), h10=s(1-s)^2 and
H_s=6s(1-s)(P-S)+(1-s)(1-3s)lD. This reduces cancellation and repeated-parameter
interval inflation without changing geometry.

At the same 1000-cell/4000-span budget the annular transition improved from
16 accepted / 969 pending cells to 311 accepted / 379 pending cells after
factoring. Counts refer to adaptive cells, not the previous coarse 8x8 cover.
Accepted and pending rectangles retain exactly the entire original unit area.
Pending cells explicitly include the collapsed tip. The control fixture passed
in 5.49 seconds; all 13 profile tests passed in 0.04 seconds. Zero-cell work
returns the untouched domain; budgets and incomplete coverage remain explicit.

The full transition is still unqualified: 379 cells require tighter bounds or
further work, and the tip needs its own supported singularity treatment. Rail
coincidence, self-intersections, thickness and closed-body admission remain open.

## Deflated transition Jacobian near the collapsed target

The exact Hermite normal factors as N=(1-s)^3 F, with
F=[(1+2s)S_t+s l D_t] cross [6s(P-S)+(1-3s)lD].
`factored_normal_bounds` encloses F using four shared source span queries.
A nonzero F proves regularity only for s<1; it cannot authorize the actual
zero Jacobian at s=1. Adaptive acceptance explicitly requires the upper s
endpoint below one. Nonzero cells touching the tip subdivide across s and
retain an unresolved tip collar instead of silently dropping the boundary.

At the same 1000-cell/4000-span budget, the actual annular fixture now has
478 accepted cells and 45 pending cells, with complete unit-domain coverage.
The pending parameter area is 0.00001955032348633; this is parameter area,
not a physical surface area or an error tolerance. Every accepted cell
excludes s=1. A rational analytic test independently checks the factored
normal [7.03125,0,-1.21875] at the specified planar example point.

All 13 profile tests passed in 0.04 seconds, and the actual control fixture
passed in 4.70 seconds. Span exhaustion remains a refusal. The remaining
45 cells, limiting tip planes, neighbor rail ownership, embedding and
closed-body admission remain unresolved; no packaged WASM changed.

## Endpoint limiting-plane diagnosis for the collapsed transition

`ProfileEndTransition::tip_limits` compares independently enclosed factored
normal limits at both original free endpoints. All eight original span queries
share the work allowance. Incomplete work produces no limit report. Matching
endpoint limits would still require qualification of the complete tip fan.

The actual annular transition has distinct limiting planes: sine squared lies
in [0.9999999999581346,1]. This is a geometric counterexample to a regular G1
point cap, not an interval-budget shortfall. The first limit is essentially
parallel to the XY plane and the second to the vertical neighbor plane. The
regression requires this distinction and refuses seven-span work. A rational
planar reference independently passes coincident endpoint limit planes.

All 13 profile tests passed in 0.04 seconds; the actual control fixture passed
in 4.80 seconds. The proposed point-collapse transition cannot be admitted as
a smooth tip by further subdivision alone. Subsequent construction must use
a noncollapsed terminal boundary and a separate corner patch, or explicitly
qualify a supported sharp vertex with its full fan, embedding and neighbors.
Neither route is implemented here. The full CAD objective remains incomplete.

## Exact transition between two noncollapsed sections

`ProfileSectionBridge` owns two independent exact profile sections and two
finite nonzero transverse scales. It requires matching original free parameter
ranges. Its cubic Hermite expression preserves each section identically at
s=0 and s=1, with the corresponding scaled transverse derivative. Regular
support normals therefore establish the boundary tangent planes at both ends.
The second end is a section expression rather than an authored point.

Complete position enclosures share four source span queries; complete normal
enclosures share eight, including both mixed transverse derivatives. Neither
profile is flattened into rounded replacement weights. Insufficient span work
returns no certificate. A rational planar fixture independently checks bridge
positions and regular interior normals at s=0,0.5,1, plus budget refusals.

On the actual annular profile, a bridge from U=0.75 to U=0.9 preserves both
section boundary enclosures and regular tangent planes over all 64 intervals
of each boundary. A coarse full 8x8 interior cover yields complete normal
enclosures but 0 regular / 64 pending cells; it proves no interior admission.
All 13 profile tests passed in 0.04 seconds; the control fixture passed in
5.12 seconds.

This supplies a noncollapsed transition construction for a subsequent corner
patch. The specific terminal section is a control example, not a finished
end cap. Side-rail ownership, terminal corner geometry, interior refinement,
self-intersections, thickness and closed-body/WASM integration remain open.

## Complete regularity cover of the noncollapsed control bridge

Bridge transverse derivatives now combine the two opposite position terms
as 6s(1-s)(S1-S0), retaining exact expression identity while removing absolute
coordinate cancellation. `inspect_regularity` adaptively covers the full
requested rectangle; only complete nonzero normal enclosures qualify a cell.
Budget exhaustion retains every unproved queued domain.

The actual annular U=0.75 to U=0.9 bridge now completes its full closed unit
parameter rectangle: 478 regular cells, 0 pending, 955 visited cells and
7640 source spans. The regression requires complete coverage and unit area.
Both natural boundaries remain included; there is no collapsed-tip exclusion.
The original coarse 8x8 cover still has 64 unresolved cells, so this result
comes from adaptive full-cell proofs rather than sampled normals.

The rational planar bridge also qualifies its entire rectangle with a larger
12000-cell/96000-span allowance; zero-cell work preserves the untouched
domain. All 13 profile tests passed in 0.64 seconds; the actual control
fixture passed in 9.36 seconds.

This qualifies local immersion of this bridge only. It does not prove global
injectivity, self-intersection freedom, thickness, neighbor side-rail identity,
terminal corner closure or a sewn Body. Those remain required before WASM/UI
admission, general geometry claims or the full CAD objective can be complete.

## Fixed-projection injectivity check for exact bridges

Bridge derivatives are now exposed as complete source-owned interval hulls.
`inspect_injectivity` checks the positive definite symmetric part of a fixed
2D projected Jacobian on every adaptive cell. On the convex domain, integration
of d^T J d along any joining segment proves strict monotonicity, hence global
injectivity, only when the entire cover qualifies. No sampled point or local
regularity certificate establishes this result. Original internal knot
multiplicities must guarantee a single-valued piecewise C1 transverse field;
unsupported continuity returns the untouched pending domain.

The rational planar reference qualifies the full rectangle; zero projection
and a deliberately folded bridge refuse global admission. All 13 profile
tests passed in 0.86 seconds. For the actual annular bridge a fixed projection
proposed from the midpoint dual vectors is freshly checked, not trusted.
At 1000 cells/8000 spans it accepts 76 cells with 849 pending. At 12000
cells/96000 spans it accepts 3287 cells with 5427 pending, retaining unit-domain
coverage. The latter control run passed in 52.42 seconds.

The annular bridge remains globally unqualified. Pending cells are neither
proved defects nor proved injective regions. Another projection, tighter
correlated derivatives or independent pairwise separation is required. Existing
full local regularity remains valid; side rails, corner geometry, whole-body
self-intersections, thickness and packaged WASM remain incomplete.

## Complete injectivity proof with projected principal minors

The injectivity gate now checks all 2D projected principal minors: j00>0,
j11>0 and det(J)>0 over every complete cell. Positive symmetric-part bounds
remain an alternative sufficient proof of the same P-matrix hypothesis.
Global injectivity on the closed rectangular domain follows from Theorem 4
of [Gale and Nikaido (1965)](https://fig.if.usp.br/~marchett/fluidos/jacobian-matrix-global-univalence-mapping_gale-nikaido.pdf).
This replaces the unnecessary symmetric positivity requirement; original
continuity, fixed projection and full-domain coverage remain mandatory.

Independent matrix regressions accept [[1,8],[-1,1]], whose symmetric part
is indefinite but whose principal minors are positive. Negative diagonals,
negative determinant and zero-containing diagonal bounds refuse. The planar
reference and deliberately folded-bridge regression remain covered.

At 1000 cells/8000 spans the actual annular bridge improves to 308 accepted
and 385 pending cells. With a 4000-cell/32000-span allowance it completes:
812 accepted, 0 pending, 1623 visited cells and 12984 original source spans.
The regression requires full injectivity and unit-domain coverage. The same
fixed midpoint-proposed projection is used; no point samples grant authority.

This proves the exact U=0.75 to U=0.9 control bridge has no self-intersections,
including its parameter boundaries. It does not qualify the entire fillet Body,
other bridges, neighbor intersections, side-rail sewing, terminal corner patches,
wall thickness or packaged WASM/UI admission. Those remain required.

## Exact bridge rails expose a cylinder sewing mismatch

`ProfileSectionBridge::rail` now retains each side as the same immutable
bridge expression. Complete rail positions and tangents query original
section data with shared span budgets; no independently rounded Curve is
substituted. Actual rail endpoints agree with the incident sections, and
position/tangent work completes at parameters 0,0.5,1.

The high-free rail midpoint fails the nominal radius-20 cylinder constraint:
x^2+y^2-400 lies in [-0.00007403168314112919,-0.00007403165307096059].
This strictly excludes zero, corresponding to approximately 1.85e-6 model
units of inward radial displacement. The regression requires this refusal;
regularity and injectivity cannot authorize sewing onto the unchanged cylinder.

All 13 profile tests passed in 1.15 seconds and the actual control fixture
passed in 14.31 seconds. The verified noncollapsed bridge is still a construction
proposal. Its side geometry must be constrained to the original neighbor
supports, retaining exact source rail expressions and compatible endpoint
derivatives. The needed side correction and resulting renewed regularity/
injectivity checks are not implemented here. Whole-body admission remains open.

## Source-rail position correction of the bridge

`constrain_original_rails` binds both natural side rails to the same original
profile used by both sections. It requires matching axes, exact affine
coordinate width and matching endpoint scales, and independently qualifies
both preserved original natural boundaries. Exact subtraction is admitted
only for zero endpoints or the Sterbenz range; other arithmetic and rounded
scales refuse. In particular authored 0.15 is not the exact width of binary64
section parameters 0.75 and 0.9.

The new expression adds linear free-coordinate weights of the two source-rail
minus Hermite-rail errors. On each side it specializes to the original rail
expression; at each section it preserves the original section. Interior
position bounds share all 14 source queries and refuse insufficient work.
No regularity or injectivity facts are inherited from the old bridge.

The actual corrected high rail midpoint has nominal cylinder squared-radius
residual [-1.3926637620897965e-11,1.1084466677857565e-11], containing zero,
compared with the previous strictly negative residual near -7.403e-5.
Structural source ownership, not this numerical enclosure alone, establishes
the side position. The control fixture passed in 14.51 seconds. An independent
rational planar reference checks the corrected interior positions against
[4u/(1+3u),v,0], plus exact boundary behavior and work refusal.

This is position geometry only. Corrected derivative fields, regularity,
injectivity, side tangent planes, native shared-edge admission and corner
closure remain required before the corrected bridge can be sewn into a Body.

## Derivatives of the source-rail position correction

The constrained bridge now encloses both first derivatives using the source
profile jets and the original bridge jets, with shared work accounting.
Interior derivative queries require 36 spans on the control fixture; 35
refuses. Normal comparison against the source profile requires 37 spans;
36 returns an unresolved report. Construction refuses internal knots whose
multiplicity permits first-derivative jumps, so endpoint jet specialization
does not silently assume continuity.

An independent rational planar reference checks both derivatives at nine
parameter samples against [0,1,0] and [1/(1+3u)^2,0,0]. The 14 profile tests
passed in 4.00 seconds. The native shell control regression passed in 102.98
seconds, including the work-refusal assertions.

At the side-rail midpoint, squared sine enclosures are
[0,2.717103622480158e-23] on the planar side and
[1.7757096542737398e-9,1.7757127517940984e-9] on the cylindrical side.
Both pass the configured 1e-6 threshold at that sample. The strictly positive
cylindrical lower bound proves that position correction alone does not
produce exact G1 tangency. This comparison uses the source profile support,
not fresh neighboring Body ownership, and does not qualify the whole seam.

Next, correct the free transverse jets as well as position on both side
rails, then requalify full-domain regularity, injectivity and seam angles.
Closed Body assembly, corners, wall thickness, whole-body intersection
checks and WASM/worker/UI integration remain open. These changes are native
only; the previously packaged WASM does not contain them.

## Mixed jet for transverse tangent correction

The two-section bridge now bounds its mixed derivative with respect to the
original free coordinate and the normalized across coordinate. It combines
original section tangent and mixed profile jets using differentiated Hermite
coefficients; no rounded intermediate Surface is introduced. All eight source
queries share one work allowance. Seven spans refuse on the reference.

An independent bilinear surface S(u,v)=[u,v,3uv] gives mixed bridge derivative
[0,0,1.5] for section width 0.5. Both section orientations, nine samples each,
the full parameter rectangle, insufficient work and an invalid domain are
covered. The 15 profile tests passed in 5.10 seconds. This supplies the mixed
jet needed to differentiate the forthcoming side transverse-field correction;
that correction and renewed whole-seam/body qualification remain open.

## Full side Hermite position expression

`enclose_with_transverse_correction` is a separate expression query. It adds
Hermite free-coordinate weights of source-position errors and source-free-jet
errors to the original bridge. Both side positions and free jets therefore
match the original source expressions algebraically. Section positions remain
specialized to the original sections. Interior point queries consume 32
spans on the references; 31 refuses. This query does not reuse the normals or
regularity certificates of the position-only correction.

An independent polynomial patch S(u,v)=[u,v,u^4*v^2] checks nine interior
points. Its quadratic dependence on the free coordinate is exactly recovered
by the full side Hermite expression. The position-only correction excludes
the expected value at those same samples, so the reference exercises the
transverse correction. The rational planar reference also checks nine
positions and work refusal. All 16 profile tests passed.

The new expression still needs first-derivative bounds, full-seam normal
qualification and renewed interior regularity/injectivity. No Body or WASM
admission follows from these position tests.

## First jets of the full side Hermite expression

`derivatives_with_transverse_correction` differentiates both Hermite error
fields using original profile first and mixed jets and bridge first and mixed
jets. Interior queries share 72 spans on the references; 71 refuses. Side
boundaries specialize to the original profile jet with the exact across
scale, while section boundaries specialize to the original bridge jet.

The independent quartic patch verifies both derivatives at nine interior
points. Its complete side intervals are also enclosed with one source span,
checking analytic values at both endpoints and the midpoint. A zero side
span allowance produces the source API's invalid-budget error. All 16 profile
tests passed in 2.23 seconds.

This is native derivative support for the new expression. Full-domain normals,
regularity, injectivity and neighboring Body seam ownership still require
qualification before sewing or exposing this bridge through WASM/UI.

## Normals and complete regularity cover of the full correction

`normals_with_transverse_correction` crosses the fresh first jets of the full
Hermite expression. `inspect_transverse_corrected_regularity` adaptively
partitions the entire requested rectangle and accepts only cells whose normal
enclosure excludes zero in at least one component. Every unresolved rectangle
is retained on work exhaustion or floating-point subdivision limits.

The independent quartic patch has a complete regularity cover of the closed
unit rectangle. The accepted rectangles have total area one. Zero cell work
retains exactly the original full rectangle as pending. All 16 profile tests
passed in 54.47 seconds. The native annular control is being checked separately;
no result for it is claimed by this reference qualification.

## Whole-side source joins and annular regularity result

`inspect_transverse_corrected_side_join` uses the full Hermite expression's
algebraic first-jet identity and independently bounds the source profile
normal over the entire affine rail interval. It keeps a complete interval
partition on budget exhaustion. This checks source profile supports only;
original neighboring Body support ownership must still be requalified.
The quartic reference qualifies both complete side intervals and checks
zero-work refusal. Its focused regression passed in 29.76 seconds.

The native annular control completed in 138.60 seconds. The full correction's
regularity diagnostic reports 0 accepted cells, 1001 pending rectangles,
1000 visited cells and 72000 source spans. Accepted plus pending area remains
one. The test passes because it checks coverage and truthful unresolved
reporting; it does not certify annular regularity. These results supersede
any inference from the original uncorrected bridge's regularity certificate.
Tighter correlated derivative bounds are needed before Body admission.

## Reuse of identical source derivative queries

The full correction now reuses each side's original profile first jet and
bridge first jet when differentiating both error fields. Reuse is local to
one query and one exact domain; no certificate or work allowance is reused
across different cells. Interior reference queries require 54 source spans
instead of 72. The independent quartic derivative, whole-domain regularity
and full-side join regression passed in 39.82 seconds, including refusal at
53 spans. This establishes the query reduction, not a runtime speedup claim.
The annular point-normal and whole-side diagnostics are being tested separately.

## Annular source-side qualification after shared-jet reuse

The annular full correction has nonzero normal enclosures at paired free/across
parameters 0.25, 0.5 and 0.75, using 54 spans each. The source-side join
qualification covers both complete across intervals: 9 cells/spans for the
planar side and 5 for the cylindrical side, with no pending intervals.
This uses exact Hermite source-jet identity and nonzero source normals.
Neighboring Body support ownership is still not qualified by these results.
The separate full-domain regularity calculation remains necessary; point
normals and side regularity do not certify the bridge interior.

The updated native control completed successfully. Full-domain regularity
remains unresolved: 0 accepted cells, 1001 pending rectangles, 1000 cells,
54000 spans. Shared jet reuse reduced source query work but did not tighten
the geometric enclosure enough for admission. The next correction must
retain dependencies between the source and bridge error fields rather than
merely reuse their independently bounded jets.

## Source second partials and common-coordinate differences

The profile exposes every second partial through the same exact homogeneous
quotient jet, preserving all incident span coverage and explicit work refusal.
The mixed API delegates to this implementation. An independent rational
reference checks x''=-24/(1+3u)^3 and zero other partials at nine points;
invalid axes and incomplete multispan work are covered. Its focused test passed.

`coordinate_difference_bounds` integrates one common derivative enclosure
to bound the difference between source points at two fixed coordinates.
It avoids independent absolute-position subtraction, requires continuity
along the interval, supports reverse traversal and specializes equal
coordinates to zero. A translated planar reference at coordinate 1e8
checks the exact difference and a tighter enclosure than direct point-box
subtraction. Disconnected knot input is rejected during profile qualification.
The final focused difference test passed. The initial attempted 1e12
reference exceeded the existing 1e9 coordinate limit and was corrected.

These source queries provide ingredients for a correlated bridge error
bound; they are not yet integrated into the full correction. The annular
regularity result remains 1001 pending rectangles until requalification.

## Common source displacement in bridge derivatives

The full correction's base and both side first jets now bound the difference
between incident section positions by integrating one common source-profile
derivative over the intervening coordinate interval. This enclosure is
intersected with the independent position difference, so it cannot widen
that term. Source pointer and axis identity are required. Public original
bridge derivative behavior and its work allowance remain unchanged.

The translated planar bridge reference proves a strictly narrower across
derivative interval containing the independent exact value. Insufficient
work refuses. The full corrected interior reference requires 57 spans and
refuses 56. All 18 profile tests passed in 95.65 seconds, and both bridge
tests passed in 0.01 seconds. The native annular requalification is running
separately; the earlier 1001 pending rectangles remain the last qualified
whole-domain diagnostic until that run completes.

## Correlated-displacement annular result and local refinement order

The native run with common source displacement bounds completed in 169.28
seconds. Whole-domain breadth refinement still reports 0 accepted cells,
1001 pending rectangles, 1000 visited cells and 57000 spans. This confirms
that tightening the displacement term alone does not qualify the interior.

The corrected bridge regularity traversal now finishes local subdivision
before traversing untouched siblings. All siblings remain in its queue and
are copied to pending on exhaustion; requested-domain coverage is unchanged.
The independent quartic regression passed in 58.34 seconds, including full
regularity, a 100-cell partial cover containing certified cells, and area
one for certified plus pending rectangles. Its annular native check is
running separately; no new annular regularity result is claimed yet.

## Local annular refinement result and Taylor remainder bounds

The native local-refinement run completed in 141.88 seconds with 495
accepted rectangles and 11 pending rectangles, 1000 visited cells and
57000 source spans. Certified plus pending area remains one. This is a
partial regularity cover, not complete bridge admission.

The profile now bounds higher quotient partials through order four on
either axis, including mixed [4,1]/[1,4] partials. It differentiates the
homogeneous quotient recursively with exact binomial factors and all
incident span-side coverage. A rational reference independently verifies
the fourth derivative -2592/(1+3u)^5 and zero mixed partials.

The full bridge correction intersects independent error intervals with
Taylor remainder bounds. Cubic Taylor terms cancel identically in the
Hermite value and derivative. For normalized t and width d, the value
remainder is bounded by M*d^4*(t^4/24+h01/24+abs(h11)/6); the across
derivative remainder by M*d^4*(t^3/6+abs(h01')/24+abs(h11')/6). The same
calculation for the free jet uses the mixed fourth partial. Every factor
is evaluated with outward interval arithmetic. This narrowing requires
C3 continuity across interior knots; otherwise the independent enclosure
remains in use. Original source/profile ownership is unchanged.

The corrected interior reference uses 61 source spans and refuses 60.
All 19 profile tests passed in 12.30 seconds, including complete quartic
regularity coverage, independent first jets and work refusal. The annular
remainder-bound native run is being checked separately. Whole-body sewing,
injectivity, wall thickness and neighbor support admission remain open.

## Complete annular regularity and corrected injectivity qualification

The native remainder-bound run completed in 94.83 seconds. The full corrected
annular bridge has complete regularity coverage: 222 accepted rectangles,
0 pending, 443 visited cells and 27023 source spans. Total accepted area is
one. This supersedes the previous partial 495/11 diagnostic for this exact
fixture. It certifies immersion of this transition expression, not closed
Body assembly or general fillets.

`inspect_transverse_corrected_injectivity` applies one fixed projection to
the freshly corrected first jets and requires positive projected Jacobian
principal minors on every rectangle. It retains complete pending coverage,
checks continuity and validates finite projection/work limits. The original
bridge's certificates are not reused. The independent quartic reference
qualifies full-domain injectivity and rejects zero/nonfinite projections;
its focused test passed in 19.95 seconds. A genuinely folded quadratic
reference has coincident distinct parameter points and correctly refuses
a complete certificate. All three bridge tests passed in 0.78 seconds.

The annular corrected injectivity run is being checked separately. Neighbor
shared-edge admission, global assembled-body intersection checks, thickness,
corner closure and WASM/worker/UI integration remain open.

## Complete corrected annular injectivity and restricted neighbor seams

The native corrected injectivity run passed in 163.22 seconds: 328 accepted
rectangles, zero pending, 655 visited cells and 39955 source spans under
one fixed projection. Together with the 222-cell regularity cover, this
qualifies the exact transition expression on the complete unit rectangle.
It does not qualify assembled-body intersections or closed shell topology.

`Proposal::qualify_edge_interval` freshly maps both owned original pcurves
and bounds the corrected/original support normals on one canonical interval.
A distinct RestrictedSeam retains Proposal identity, both faces, edge,
canonical interval, threshold and angular upper bound; it cannot stand for
the remaining source edge. The full-edge API delegates with [0,1] and
preserves its singular endpoint preflight.

The owned annular neighbor regression passed in 11.85 seconds. Edge 0
on [0.1,0.25] qualified with 11 cells/44 spans and upper squared sine
4.2472248031705913e-23. Edge 1 on [0.75,0.9] qualified with 2255 cells/9020
spans and upper squared sine 9.981695392026196e-7 under threshold 1e-6.
These are finite angular qualifications, not exact zero-angle proofs.
The test covers Proposal identity, selected faces/edge/interval, work
refusal and invalid intervals/edge IDs.

These explicitly requested canonical intervals are not yet bound to the
bridge's root-ended carriers. That binding must enclose the exact transformed
end coordinates, retain source ownership and restrict neighbor contours
before a sewn Body can be admitted. Corner/terminal closure remains open.

## Root-ended carrier seam ownership

`qualify_root_ended_seam` accepts two source-owned CutPartitions on the same
Proposal, face, neighbor, original edge, section axis and natural free end.
It freshly admits both UV roots through the original shared carrier, proves
strict canonical order, and uses the hull of their transported enclosures
for original neighbor angular qualification. A RootEndedSeam retains both
Point root definitions alongside the interval-qualified RestrictedSeam.
No independently authored rounded canonical endpoint replaces a root.
Carrier transport and angular checks share one source-span allowance.

The final owned-neighbor regression passed in 13.57 seconds. Edge 0's roots
produce canonical cover [0.09999999999999563,0.25000000000000416]; edge 1
produces [0.7499999999999981,0.9000000000000014]. Both use six carrier spans
before whole-interval neighbor qualification. Every root definition restores
with fresh original carrier/root checks. Repeated ends, a distinct Proposal
over identical source geometry and insufficient carrier work refuse.
The preceding complete bridge regression also passed in 147.05 seconds.

These facts still do not create the two-root retained incident fragments or
sew a closed body. Fragment restoration currently allows one Carrier end
per fragment; bounded two-end restoration and fresh mapped-edge admission
are the next steps before assembling neighbor contours.

## Two-root incident fragments and fresh retained shared edges

RootEndedSeam now constructs the original interval between its admitted
carrier roots on both incident uses. It requires strictly separated source
parameter enclosures and follows each original use's traversal. Both fragment
ends remain Carrier expressions; no rounded parameter endpoint is created.

Fragment restoration reserves disjoint mapping allowances for both carrier
ends and divides its fixed exact-work, driver and span caps between their
original parent definitions. Single-carrier behavior is retained. Every
root/parent is freshly qualified; insufficient allowance refuses. Nested
carrier parents remain rejected by the carrier restorer.

`qualify_incident_edge` freshly qualifies both retained compositions against
the original world curve and original canonical maps. The final annular
regression passed in 12.61 seconds. Both retained edges qualify and restore
with identical definitions: edge 0 uses 236260 exact work, edge 1 uses
109463. No extra endpoint driver/root checks were needed because freshly
admitted shared carrier-root identity establishes corresponding endpoints.
Four incident fragments restore with both Carrier ends and original traversal.
Tiny restore/edge allowances refuse. The five fragment tests and ten shared
edge tests also passed.

This admits the selected trimmed original 3D edges between two source roots.
Transition-side binding, replacement contour assembly, terminal/corner
closure and closed corrected Body admission remain open.

## Full transition-side binding to root-ended retained edges

RootEndedSeam retains the selected source face, axis, authored section
parameters and natural free-end identity from both owned CutEndpoints.
`qualify_transition_side` accepts only a constrained bridge using the exact
Proposal-owned profile, that axis and those ordered section parameters.
The bridge factory already fixes exact affine width/scales and original
natural rail preservation. The full Hermite side first-jet identity is
then checked for complete regular support coverage.

The resulting TransitionSide refers to the root-ended seam and exact
transition recipe. Selected source face ownership is retained even when
both adjacent faces have corrected profiles; automatic neighbor ordering
is not allowed to substitute the other face's profile. This binding refers
to the full Hermite-corrected expression, not the position-only variant.

The final annular regression passed in 11.58 seconds. Both sides bind to
face 0, axis 0, section parameters [0.75,0.9] and their respective natural
free ends. An altered terminal parameter, a distinct profile owner over
identical source geometry and zero-cell allowance refuse. Existing retained
shared-edge and two-root restoration checks also pass in this regression.

This binds transition rails to admitted trimmed original edges. It does
not yet build replacement contours or an admitted closed transition face;
terminal/corner closure and whole-body qualification remain required.

## Root-owned four-edge transition boundary

TransitionBoundary now joins the two qualified opposite transition sides
under one immutable Proposal, selected face, section axis and ordered
section parameters. Input side order is normalized. Duplicate sides and
mismatched owners or recipes refuse. Four carrier-root corners remain
owned by the original retained seams.

The contour consists of the low natural rail, terminal source section,
reversed high rail and reversed entry source section. Its enclosure API
evaluates the exact owned profile boundary specializations of the full
Hermite expression. Affine parameter mapping uses outward intervals; exact
segment endpoints specialize directly to original section coordinates.
No rounded replacement surface or vertex is introduced.

The final native annular regression passed in 11.87 seconds. All four
complete edges enclose with one source span. Adjacent endpoint evaluations
produce identical profile enclosures, and all eight endpoint evaluations
overlap their corresponding admitted carrier-root world bounds. Overlap
is a diagnostic check; shared owner, recipe and root identity establish
endpoint authority. Duplicate sides, invalid edge indices and invalid
parameter intervals refuse. Existing retained-edge and restore checks pass.

This is a transition boundary proposal. Interior face admission, terminal
closure, neighboring replacement contours and closed Body admission remain
open. These changes are native-only and are not in the packaged WASM.

## Owned transition-expression face admission

TransitionBoundary::qualify_face reconstructs the full Hermite-corrected
expression directly from the boundary's immutable Proposal profile and
section parameters. The exact rail-constrained factory must succeed.
A fresh fixed-projection injectivity check covers the entire natural
free interval and normalized across interval. Complete positive principal
minors also establish immersion. An externally supplied report cannot
create TransitionFace; its constructor and fields are private.

The admitted expression keeps a reference to the exact root-owned boundary
and the projection used for qualification. Refusal retains the full
interior pending report. Zero cell/span allowance leaves one pending
rectangle and admits no face; invalid nonfinite projection errors and
insufficient control allowance refuses.

The positive annular regression passed in 31.79 seconds with 328 accepted
rectangles, 655 cells, 39955 source spans, zero pending rectangles and
coverage area 1. Boundary owner identity and stored projection are checked.
The refusal-only regression also passed in 7.75 seconds.

This admits an embedded exact transition expression with its owned contour.
It does not convert the expression into a rounded Surface or admit the
surrounding shell/Body. Neighbor replacement contours, end closure, volume,
wall thickness and whole-body intersection checks remain required. The
implementation remains native-only pending WASM/worker/UI integration.

## Incident original contours split at both shared roots

RootEndedSeam::propose_incident_contours now uses the original shell's
incident-use addresses to subdivide both face contours at the same admitted
carrier roots. The source use definition must match the owned carrier use.
The original directed edge becomes head, retained middle and tail; middle
endpoints remain Carrier expressions and traversal follows the original use.
All other edges and all other loops are preserved as source definitions.
The result records each face and the middle use address for later sewing.

The total fragment budget is checked before contour cloning and construction.
Insufficient allowance refuses; allowances above the hard cap error. Every
resulting loop must close using source-expression endpoint identity. Tests
check both incident faces, exact middle definitions, restoration of all three
segments, joins and preservation of every unaffected edge.

These are subdivided original contours ready to attach retained rails. They
do not admit a replacement shell: transition-facing incidence, selected-face
region replacement and terminal closure still need assembly and qualification.

The final combined annular regression passed in 33.91 seconds, including
complete owned transition-face admission (328 accepted, zero pending).

## Oriented rail incidence of the admitted transition

TransitionFace::qualify_rail_incidence freshly admits each retained shared
3D edge and reconstructs both incident source contours. The selected use
comes from each carrier root's original source-side authority. The opposite
use must be the exact middle fragment in the neighbor contour.

Strict canonical root ordering determines each directed transition rail.
The high rail follows the reverse contour direction. Both neighbor uses
must admit the same whole-face orientation reversal, and after that reversal
each transition rail must oppose its neighbor's directed canonical use.
An orientation conflict refuses. The private incidence result retains the
admitted transition face, both fresh shared edges, neighbor contours and
orientation.

Regression cases include insufficient edge work and insufficient contour
allowance. The positive case checks both opposite traversal relations, exact
neighbor middle-use definitions and transition-face owner identity.

This establishes the two rail incidences only. Entry/terminal edge incidence,
selected-region replacement, closed-shell assembly and whole-body embedding
remain open; it is not a closed Body or a general radius certificate.

The combined native regression passed in 70.19 seconds with complete
transition interior coverage (328 accepted, zero pending) and both rail
incidence checks. Packaged WASM is unchanged.

## Entry and terminal contour binding

TransitionBoundary::propose_section_contours binds each entry/terminal cut
to the exact boundary recipe. Both supplied CutPartitions must belong to
the same immutable Proposal and selected face, axis and section parameter,
the matching low/high rail and original edge. Each SourcePoint definition
must equal the corresponding admitted carrier root's original crossing
definition. Coordinate-box overlap cannot substitute for this identity.

The original cut-contour constructor then freshly builds the two
complementary closed source-chart contours. The private bound section
retains its transition boundary and records contour edge 3 for entry or
edge 1 for terminal. These remain original chart contour proposals whose
geometry must be evaluated under the owned corrected profile.

The final combined native annular regression passed in 26.73 seconds.
Both section bindings succeed and all complementary loops close. Supplying
the other section, reversed low/high partition order or insufficient
fragment allowance refuses. Prior face and oriented rail admission still
pass, with 328 accepted interior rectangles and zero pending.

Remaining work includes selecting and admitting the exterior retained
regions, matching their directed section uses to the transition, assembling
the closed expression shell and qualifying whole-body volume/intersections
and wall thickness. No new WASM is packaged by this stage.

## Exterior section-side qualification

TransitionSection::qualify_exterior classifies both complementary source
contours against the owned straight section coordinate. Every noncontact
arc must lie wholly on one side. Original positive-weight control hulls
prove strict sides for unaffected curves. Arcs ending at the implicit
section root use fresh whole-source monotonic coordinate-driver proofs
through source_halfplane_side, preserving the original crossing equation.
No sampled point or tolerance-based coordinate snap selects a region.

Both loops must be qualified on opposite sides before selecting a kept
loop. Entry keeps the lower coordinate side; terminal keeps the higher
side. The private ExteriorSection borrows the bound section and exposes
kept/removed source-chart loops. Controls and monotonic driver cells share
explicit total allowances; insufficient work refuses admission.

The regression checks both cuts, owner identity and refusal with zero work.
It independently checks that entry retains the original coordinate-0 end
and terminal retains coordinate-1, and that those ends are absent from the
corresponding removed loops. This qualifies side selection, not profile
face immersion, simplicity or whole-body embedding. Those checks and
terminal directed incidence remain required before shell admission.

The final strengthened combined annular regression passed in 57.34 seconds;
the preceding classification regression passed in 35.24 seconds. Interior
coverage remains 328 accepted rectangles with zero pending. Native-only.

## Four-edge transition-patch incidence

TransitionRailIncidence::qualify_sections now binds the two classified
exterior section contours to the already admitted transition and rail
incidence. Both sections must refer to exactly that boundary instance and
cover opposite entry/terminal ends. Reversed input order is normalized.

For each section, the retained directed contact's free-coordinate traversal
must oppose the corresponding transition edge after the common orientation
reversal established by both rails. Each directed contact endpoint must
retain the exact corresponding original carrier root's crossing definition
with Contact role. Root-box proximity is not accepted. The private patch
result retains the rail incidence and the two original retained contour
definitions in entry/terminal order.

Tests verify positive four-edge incidence, normalized section order, exact
retained definitions and rail owner identity. Duplicate sections and
sections bound to another boundary instance over identical source geometry
refuse. The initial combined regression passed in 49.49 seconds.

This closes incidence around the transition patch, not the surrounding
shell. Retained profile-face immersion/embedding, original edge segmentation
assembly, global pair intersections, volume and wall thickness still require
qualification before a closed Body can be admitted. Native-only.

The final owner-refusal regression passed in 44.03 seconds, including
complete transition interior coverage (328 accepted, zero pending).

## Retained exact-profile regularity coverage

WeightProfile::inspect_regularity now adaptively covers a supplied source
rectangle using fresh exact-profile normal hulls. A nonzero component admits
a cell. Work exhaustion, zero-containing normals and resolution limits
retain every unresolved rectangle, including untouched queued siblings.
The coverage is diagnostic immersion evidence; it does not prove global
injectivity or admit the surrounding Body.

ExteriorSection::inspect_regularity derives a rectangle containing its
entire qualified exterior source region. It evaluates the owned profile,
not inherited original Surface normals. Entry covers the natural chart up
to its cut; terminal covers the chart after its cut.

An independent bilinear patch S(u,v)=(u,u*v,0) proves that a region away
from the collapsed u=0 edge completes, while a region including that edge
retains pending coverage. Accepted plus pending area stays 1. Zero work
refuses and invalid rectangle ordering errors. The full 20-test profile
suite passed in 16.23 seconds.

On the annular fixture with 128 cells/256 spans, entry completes with
46 accepted rectangles, zero pending, 91 cells and 91 spans. Terminal has
59 accepted, 11 pending, 128 cells and 128 spans. Both reports cover their
entire requested region (areas 0.75 and 0.1). Terminal regularity remains
unproven, including its original singular boundary; no retained-face or
closed Body certificate is issued from the incomplete cover. Native-only.

The final combined regression, explicitly requiring complete entry-region
regularity, passed in 40.83 seconds. The initial run passed in 41.65 seconds.

## Exact collapsed-U normal factor

WeightProfile::factored_pole_normal_bounds factors the collapsed-U position
numerator under the exact profile weight expression. For a clamped single
Bezier span with p coincident pole rows, Q=N-P*W=f^p*A, where f is the
normalized distance from the collapsed U boundary. The ordinary normal
factors as f^(2p-1) times the returned normal expression. Bernstein
coefficient reduction uses outward binomial ratios; recentering uses
original control differences and does not flatten the profile weights.
Both high and low collapsed boundaries and original parameter width are
handled. Six homogeneous jet hull queries are required; insufficient work,
unsupported span structure and missing collapse refuse.

The returned diagnostic explicitly records pole point, boundary side and
removed power. It does not certify immersion at the singular boundary or
a smooth common tangent plane. Independent references cover bilinear
collapse on both ends, third-power normal factors for quadratic position
collapse, nonunit parameter width and geometry translated by 1e8. The
initial 21-test profile suite passed in 13.96 seconds. A rational reference
S=(4u/(1+3u),4uv/(1+3u),0) checks the factored normal
(0,0,16/(1+3u)^3) at five U values including the collapsed endpoint.

On the annular terminal cover, 5 of the 11 ordinary-normal pending cells
have nonzero factored normal bounds; 6 still require refinement. The
ordinary immersion cover is not rewritten as complete. Adaptive factored
coverage and singular-boundary topology/embedding admission remain open.
Native-only; packaged WASM remains unchanged.

The final 22-test profile suite passed in 17.75 seconds, including the
rational reference. The combined annular regression passed in 65.70 seconds.

## Adaptive factored-pole coverage

WeightProfile::inspect_factored_pole_regularity now covers an entire
rectangle using fresh exact factored-normal bounds. ProfilePoleCover is
a separate diagnostic type with complete_factored, removed power, pole
point and boundary side. It never sets ordinary complete_regular or
claims a smooth pole. Work and resolution refusals retain all queued
rectangles; accepted plus pending area covers the requested rectangle.

ExteriorSection derives the containing rectangle from its owned exterior
region for both ordinary and factored checks. The new report and normal
types are exported through the existing normal_alignment facade.

Independent tests confirm complete bilinear factored coverage, exact work
refusal and full pending-area preservation for a patch also collapsed in V,
where the factored normal is truly zero. The 23-test profile suite passed
in 14.62 seconds.

The annular terminal rectangle [0.9,1] x [0,1] with 2048 cells and 12288
jet queries has 201 accepted rectangles and 847 pending, area 0.1 and
removed power 1. This is a finer adaptive cover than the previous 11-cell
ordinary-normal remainder; the counts are not directly comparable. Full
factored regularity remains unproven. The initial combined native run
passed in 42.52 seconds.

The final region-bound API regression passed in 56.06 seconds. Diagnostic
point hulls at U=1 and V=0,0.5,1 all contain zero even after the current
first-power removal. Sample pending rectangles reach U intervals near
0.9999999999999983 and V widths around 8.88e-16. This points to an
additional transverse collapse factor or higher-order singular structure;
it does not justify increasing the budget or declaring completion. The
next step is exact transverse numerator factor qualification.

## Independent transverse collapsed-row factor

The exact profile normal factor now distinguishes p coincident pole rows
from r rows that are individually constant across V. Exact separable
profile weights make each such row independent of V even when its point
differs from the pole. Subtracting each original row's own V baseline
produces E=f^r*B; the removed baseline's rational blend depends on U only.
Consequently Su=f^(p-1)*A and Sv=f^r*C, giving normal factor f^(p+r-1).
This supersedes the earlier sufficient but incomplete f^(2p-1) removal.

When r=p the existing six-query formula is retained. When r>p a separately
reduced transverse numerator requires seven homogeneous jet hull queries.
Outward binomial ratios, exact row/column/pivot expressions and original
control differences remain authoritative. Insufficient work refuses, and
adaptive coverage preserves pending rectangles when the seventh query
cannot be afforded.

Independent references cover S=(u,u^2*v,0), both low/high collapsed ends,
and a rational V blend S=(f,f^2*4v/(1+3v),0). Their factored normal is
respectively (0,0,+/-1) or (0,0,+/-4/(1+3v)^2), with removed power 2.
The final 24-test profile suite passed in 22.44 seconds.

For the annular terminal region the corrected factor is power 2. All 11
ordinary-normal pending cells have nonzero factored normals. Complete
adaptive coverage now requires only 4 accepted rectangles, zero pending,
7 cells and 49 queries, covering area 0.1. Pole-normal point hulls at
V=0,0.5,1 are nonzero after the second-power removal. The initial combined
native regression passed in 90.28 seconds.

This qualifies regularity in the interior and nonzero scaled pole limits.
It does not turn the singular boundary into a smooth pole, establish a
common tangent plane, admit retained-face global embedding or construct
a closed Body. Those gates and whole-body qualification remain required.

The strengthened native regression explicitly requiring complete terminal
factored coverage, zero pending and power 2 passed in 43.90 seconds.
Packaged WASM remains unchanged.

## Exact-profile global quotient injectivity and retained-face admission

The existing weighted quotient dominance proof now accepts exact profile
weights through certify_profile_source_frame. Its original-control source
frame and polynomial proof are reused, but each weight is freshly enclosed
as row[i]*column[j]/pivot. Existing ordinary Surface callers retain their
original weight path. No rounded replacement Surface or inherited original
injectivity report supplies authority.

ExteriorSection::inspect_quotient_injectivity qualifies the entire owned
profile chart. A successful global proof covers both retained subregions,
identifying only the original collapsed U row with a single pole. The
private RetainedProfileFace constructor then requires fresh complete
region regularity: ordinary normals when the pole is outside the retained
rectangle, or complete factored normal coverage when it is included. The
result explicitly records the factored boundary and borrows its exact
ExteriorSection. Failed global or local checks issue no face.

Independent tests exercise a nonseparable original weight changed by the
profile proposal, work refusal, the wrong collapsed boundary and a genuine
fold S=(u,2u^2*v*(1-v),0), where V=0.25 and 0.75 coincide. All eight
quotient tests passed in 0.04 seconds.

On the annular fixture the full-chart exact-profile proof succeeds with
256 cells and weighted bounds [9.824382008237144,0.6195945915904447,
1.185833747547404,4.309243282738274]. Entry uses ordinary complete
regularity; terminal uses complete second-power factored coverage. The
initial combined quotient regression passed in 56.06 seconds.

Retained face admission is separate from patch-to-retained incidence and
closed shell assembly. It does not qualify intersections between distinct
faces, volume, radius or wall thickness. Native-only.

The final owned retained-face regression passed in 194.84 seconds. It
requires both global profile proofs, entry ordinary regularity, terminal
factored regularity, exact exterior owner identity and the explicit
collapsed-boundary classification. Insufficient quotient or regularity
allowance issues no face. Packaged WASM remains unchanged.

## Fresh outer retained 3D-edge admission

TransitionPatchIncidence::qualify_outer_edges now requires both admitted
retained-profile faces under the exact same transition boundary. Duplicate
entry/terminal faces or foreign boundary ownership refuse. Input order is
normalized and each admitted exterior contour must equal the patch's
retained contour definitions.

For each rail and each exterior section, the selected original use is
located by source surface/curve and exact Crossing root definition. Its
neighbor head/tail fragment is located by the corresponding Carrier root
and neighbor source-side authority. The original world curve and reordered
original affine maps are then freshly admitted through source_mapped_edge.
The selected profile's complete natural rail preservation remains tied to
the immutable Proposal; no rounded profile boundary is substituted.

A shared total exact-work and driver allowance covers all four fresh edges.
The private assembly retains the patch and four admitted SharedEdges, plus
the ordinary/factored retained-face classification copied only from the
admitted face objects. Tiny work and duplicate faces refuse. Tests check
opposite directed uses and exact patch ownership.

The first combined annular regression passed in 72.91 seconds. Four outer
edges require 692448 exact work and zero extra root drivers. Together with
the two middle rails and two section incidences, this admits the split
edge uses around the selected replacement patch. Complete shell incidence
assembly, distinct-face contacts, volume and wall/radius qualification
remain required. Native-only.

The final combined regression passed in 49.97 seconds. All four outer
SharedEdges restore from their mixed Crossing/Carrier endpoint definitions
with identical source recipes, followed by fresh admission.


## Closed oriented expression-shell incidence

OuterEdgeAssembly::assemble_shell replaces the selected original face by
its two admitted retained regions and the admitted exact transition face.
Both neighbor contours are subdivided at the owned roots; unchanged faces,
inner loops, original shared edges and collapsed pole uses are preserved.
Four newly admitted outer edges, two middle rails and two section contacts
are remapped to the resulting directed use addresses. Every use must have
exactly one paired edge or one admitted collapsed pole; paired traversal
must be opposite. Source contour joins are freshly checked.

Endpoint union and the vertex link graph are rebuilt from the final use
incidence. Each vertex must have one connected degree-two link, rejecting
branching and two closed fans touching at one vertex. Component count is
preserved; the replacement adds four vertices, six edges and two faces.
The use allowance is checked before cloning the complete shell. The
private ExpressionShell borrows its admitted OuterEdgeAssembly, so callers
cannot construct an admission from public report numbers.

The combined native annular regression passed in 72.33 seconds: 120 uses,
all remapped and closed, including the original collapsed poles. This is
closed oriented manifold incidence. Global embedding, distinct-face
contacts, volume, radius and wall thickness are not admitted by this
stage. Geometry evaluation/serialization and packaged WASM integration
remain required.

The final combined regression passed in 81.11 seconds after adding exact
use-budget admission/refusal and opposite traversal assertions. The
independent vertex-link regression passed, including disconnected closed
fans, branching, a normal triangle fan and a valid two-edge lens.


## Exact expression-shell support position enclosures

ExpressionShell::enclose_support dispatches by the admitted face kind.
Original faces use outward interval NURBS span enclosures with an explicit
span allowance checked before evaluation. Retained faces use the exact
WeightProfile expression, including factored original weights. Transition
faces reconstruct the owned ProfileSectionBridge and constrained natural
rails, then evaluate the full transverse Hermite correction. Insufficient
control or span work returns no enclosure. Invalid face indices and
out-of-domain parameters refuse.

Coordinates are source (u,v) for original/retained faces and (source free,
normalized across) for the transition. These conservative bounds cover
the supporting expression. They do not certify membership of a rectangle
in the trimmed region or absence of contact between two faces.

The combined annular regression passed in 48.22 seconds. Every unchanged
original face was checked against a separately evaluated point, the
transition entry matched the exact profile section enclosure, its
interior returned a bounded expression, retained dispatch preserved the
exact profile bounds and zero-work cases returned no enclosure. Native
cargo check and diff whitespace checks passed. Packaged WASM is unchanged.


## Adaptive expression support-pair exclusion

ExpressionShell::inspect_support_pair accepts two distinct owned faces
and their support parameter rectangles. Exact outward position bounds
exclude a product only when one world coordinate has a strict gap. An
ambiguous product is bisected along its largest normalized parameter
width. Shared seams, touching bounds and actual crossings are not admitted
as safe. The shared span allowance covers both evaluations; the cell
allowance covers all visited products. Control work is separately bounded
per transition enclosure. On exhaustion, the current product and every
queued sibling remain pending. Unsplit representable point products also
remain pending. This report does not confer Body or contact authority.

Independent regressions passed for separated parallel planes, coincident
planes, perpendicular intersecting planes and zero work. Product volume
of excluded plus pending cells equals the original parameter product;
the known intersection line remains in the pending cover. Both regressions
passed after final compilation in 0.01 seconds.

The annular integration regression passed: the two retained support
rectangles [0,.75] x [0,1] and [.9,1] x [0,1] are completely separated
in one product cell and two span evaluations, with no pending product.
The final regression explicitly requires complete separation. This is
one pair; remaining original/retained/transition pairs and shared seam
neighborhoods still require geometric qualification. Native-only.


## Complete expression-shell support pair inventory

ExpressionShell::support_domain derives a conservative containing
rectangle from the privately owned face recipe: the original active knot
domain, the entry/terminal exterior half rectangle, or the exact transition
free range times normalized across [0,1]. Trimmed-out support regions are
retained conservatively.

inspect_shell_supports visits every unordered face pair, including
adjacent faces. Each record identifies the owning faces, all shared
incidence edge IDs, excluded products and pending products. A global cell
and span allowance covers the full run, with a separate cell cap per pair.
Even after exhaustion, every remaining pair retains its full pending
parameter product. The complete report is bounded to 100000 pairs before
allocation. Larger shells need a streaming report implementation.

The combined annular regression passed in 34.53 seconds: all 406 pairs of
29 faces are represented; 149 support pairs are completely separated and
257 remain pending, using 663 cells and 3062 spans. Every adjacent pair
remains pending. The zero-cell run retains all 406 full products. After
adding the allocation guard, final compilation and both independent
support-pair regressions passed. This is conservative support exclusion,
not complete trimmed-face self-intersection qualification; region
classification and certified seam neighborhoods remain necessary.


## Exact source-contour exclusion inside expression pair traversal

inspect_trimmed_pair combines strict world separation with whole-rectangle
source_contour_winding classification on the final assembled contours.
The original and retained faces use their preserved source fragments,
including Carrier/Crossing root endpoints and holes. The transition keeps
its full parameter chart. Only Location::Outside excludes a product;
Inside and Unresolved continue conservative world subdivision. Region
work has a separate shared allowance. Zero region work never classifies.

Outside products are recorded separately from world-separated products.
complete_excluded requires no pending products; complete_separated also
requires that none were removed by trimming. The independent region test
checks that the union of outside, separated and pending product measures
still equals the input product and that every outside record lies wholly
in the declared excluded half-space. All three pair regressions passed.

An initial test requiring a far sibling to be visited within 31 DFS cells
failed: a near ambiguous branch consumed the allowance first. This did
not lose coverage; all unvisited siblings remained pending. The test was
corrected to verify exclusion and coverage without assuming traversal
reaches a distant branch within that allowance. Fair traversal refinement
remains a performance consideration.

The combined native regression passed in 40.71 seconds. A coincident
support product at source (u=.8,v=.5), lying in the removed transition
strip, was excluded by the owned retained contours in 12 domain cells.
It remains pending with zero domain allowance. All 406 support pairs
retain the prior 149 separated / 257 pending baseline. Whole-shell trim
classification and shared-seam contact certificates remain required.
Packaged WASM is unchanged.


## Whole-shell trimmed support traversal

inspect_shell_trimmed applies the owned contour classifier during every
unordered expression face pair traversal. The shared report preserves
world-separated, trim-outside and pending products independently.
pending_pairs now uses complete_excluded, so a fully trim-excluded product
is not mislabeled as pending or as world-separated. The existing support
inspection keeps its previous semantics because it never records outside
products.

A shared domain-cell allowance and a domain-cell cap per pair accompany
the existing global/per-pair product-cell and shared span allowances.
Every unordered pair remains represented after exhaustion. Both APIs use
one internal inventory implementation, including the complete report
allocation guard and shared incidence IDs. This does not skip adjacent
pairs or admit shared contact neighborhoods.

The first combined whole-trimmed regression passed in 72.45 seconds:
406 pairs, 162 completely excluded / 244 pending, 149 outside products,
2156 product cells, 11256 spans and 6494 domain cells. Adjacent pairs all
remain pending, and zero product work preserves all 406 pending pairs.
The independent three pair regressions passed after this integration.

A matched-budget comparison then passed in 59.27 seconds. At the same
4096 total / 8 per-pair product-cell and 20000 span allowances, support-only
inspection excludes 158 pairs and leaves 248 pending (2160 cells, 11264
spans). Owned trim classification excludes 162 and leaves 244 pending
(2156 cells, 11256 spans, plus 6494 domain cells). Thus trim classification
completely excludes four additional pairs at that budget, while also
removing 149 partial outside products. The previous 149/257 result had a
smaller two-cell per-pair allowance and is not an equal-budget comparison.
Full shared contact neighborhoods, remaining distinct-face products,
volume, wall thickness and radius qualification remain required.


## Fresh planar contact authority for unchanged expression faces

qualify_original_planar_contact maps two distinct Original expression
faces to the privately owned original shell, requiring identical complete
contour definitions (all loops, every fragment and endpoint) before fresh
source_allowed_contact qualification. It issues a private certificate
borrowing the actual final ShellFace owners and retaining the fresh source
contact certificate. Retained, transition and subdivided original faces
refuse transfer; their changed geometry/contours need separate proofs.
Zero exact or span allowance issues no certificate.

This admits supported shared-line contact without treating adjacency or
an old report number as absence of extra intersection. Unresolved pair
products are kept by the trim report; the independently owned contact
certificate is an additional geometric proof for the whole unchanged
pair. The test attempts every pending pair under shared exact/span work
and checks actual face-owner pointer identity for each admitted contact.

The first combined native run passed in 42.71 seconds. Ten of the 244
pending trimmed pairs received fresh unchanged-planar contact certificates,
using 1288456 exact work and 288 spans over all attempted pairs. A final
run explicitly requires these ten certificates and rejects reflexive,
retained and transition contact transfer. These ten pairs are qualified
for allowed contact, not classified as disjoint. The product report still
preserves its 244 pending entries; combining it with these ten separate
contact certificates leaves 234 pairs without a complete pair proof.
Changed/subdivided faces and curved seam neighborhoods remain required.


## Fresh boundary-fiber authority for unchanged curved expression pairs

qualify_original_fiber_contact freshly attempts source_fiber_contact in
both face orientations, then source_paired_fiber_contact under one shared
exact-work/driver allowance and a shared span allowance where applicable.
The complete original-contour identity gate is factored into the same
private helper used by planar transfer. A private OriginalFiberContact
borrows the actual final face owners and retains its Boundary or Paired
source certificate. Reports, topology flags and public numbers cannot
construct this admission. Changed contour or expression faces still need
new proofs rather than inherited source authority.

The integration attempts fibers only for pairs left pending after trim
inspection and not admitted by planar contact, preventing double counting.
Each admitted certificate is checked against the actual expression-face
owner pointers and zero-work admission is rejected. Native cargo check
passed before integration testing.

The first combined native run passed in 64.81 seconds. Nineteen additional
pending pairs received fresh unchanged fiber certificates after excluding
the ten planar-qualified pairs, using 948414 exact work, 511 spans and zero
driver subdivisions. The final run requires exactly nineteen certificates
and checks the fresh source certificate's unordered face IDs against the
actual Original expression-face mapping. Retained faces and zero work
refuse transfer. With 162 trim/world excluded pairs, 10 planar contacts
and 19 fiber contacts, 215 of the 406 pairs still lack a complete pair
proof. These are supported fixture results, not full shell qualification.


## Fresh owned-vertex and interior-fiber contact transfer

qualify_original_additional_contact uses the same exact final-to-original
contour identity gate. It freshly attempts source_vertex_contact, then
source_interior_contact in both face orientations (without linear-chart
fallback). One shared exact-work allowance covers all attempts, and the
interior attempts share span and driver allowances. The private resulting
certificate borrows final face owners and retains the fresh Vertex or
Interior source certificate.

Integration tries this only after trim/world exclusion, planar contact
and boundary/paired fibers have failed, avoiding duplicate qualification.
Source face IDs and actual owner pointers are checked for every admitted
certificate. Changed faces and zero exact work issue no contact authority.

The first combined native run passed in 65.20 seconds: 45 additional
vertex/interior pairs were admitted after 10 planar and 19 boundary-fiber
pairs, using 1041307 exact work, 310 spans and no driver subdivisions.
The final regression requires these 45 fresh certificates, actual face
owner identity, source face ID correspondence and reflexive/changed/zero
work refusal. Combining all independent pair proofs with the 162 excluded
pairs leaves 170 of 406 pairs without full qualification. Modified source
contours, exact profile and transition contacts still require admission.


## Complete source-arc refinement admits subdivided neighbor contacts

The private original-pair gate now accepts the exact full source region
with its two owned rail-root subdivisions. For each original fragment it
reconstructs the expected final use sequence from the privately admitted
neighbor IncidentContour. A replacement must be exactly head/middle/tail
on the same original surface and curve, with the original traversal,
original extreme endpoint definitions and identical intermediate endpoint
definitions. Neighbor source-side/address ownership comes from the admitted
root-ended seam; strict endpoint ordering was freshly established by
Fragment construction and root admission. Every loop and unchanged
fragment remains accounted for, including holes. Arbitrary new cuts,
missing arcs and retained/profile/transition faces cannot inherit.

The three contiguous restrictions therefore cover the same original
parameter interval and UV material region. Fresh original planar/fiber/
vertex/interior contact proofs can transfer across this refinement without
substituting a rounded boundary or treating adjacency as a proof. The
certificate still borrows the actual final refined face owners.

The independent full-partition test passes for forward and reversed source
traversal and refuses a gap, wrong traversal, missing arc or changed curve.
The first annular run passed in 54.01 seconds: 12 planar, 23 boundary-fiber
and 54 vertex/interior pairs, compared with 10/19/45 before refinement.
The final regression requires all 12/23/54 results with exact intermediate
endpoint identity checks enabled. With 162 excluded pairs, 155 of 406
pairs still need a complete geometric proof. Transition/profile contacts
remain separate; packaged WASM is unchanged.


## Unified private expression contact audit and transport diagnostics

ExpressionShell::inspect_contacts freshly creates the complete trimmed
pair inventory and attempts planar, boundary/paired fiber and vertex/
interior certificates for its pending pairs. One exact/span/driver budget
covers the entire combined audit, with the existing global/per-pair cell
and domain budgets. Each pair retains its support cover and a Pending,
Excluded or privately owned typed contact proof. The audit borrows the
actual immutable expression face array; its fields are private and it
accepts no caller-supplied pair report or certificate.

The diagnostic Value records all face IDs, shared incidence edges,
proof kinds, refusal reasons, pending/outside/separated parameter products
and total work. It is transport data only and has no restoring admission
API. Zero allowances retain all unordered pairs as Pending. No Body or
volume/thickness/radius admission follows from this partial pair audit.
Native cargo check passed before the integrated regression.

The first combined audit regression passed in 38.20 seconds. Its counts
match the separate proofs exactly: [155 pending,162 excluded,12 planar,
23 fiber,54 vertex/interior]. Total combined work is 2156 product cells,
12595 spans, 6494 domain cells, 3970573 exact work and zero drivers. Each
certificate's final owner pointer is verified; zero allowances preserve
406 pending pairs. The final regression passed in 48.96 seconds including
exact binary diagnostic encode/decode roundtrip and JSON parse with all
406 records and 155 pending count preserved. Native-only; these payloads
are not yet exposed through packaged WASM/worker/UI. Support pending
products remain present even for contact-certified records; consumers
must interpret them together with the pair proof kind, not as additional
unqualified pairs.


## Transition natural-rail plane preimage qualification

qualify_transition_plane_contact requires an owned sewn Rail pair and its
Original neighbor. The neighbor control net is freshly certified planar
with exact predicates. Every original control point on the corresponding
natural free rail must be exactly coplanar. Positive exact profile weights
and the clamped preserved-rail factory retain the full rail image in that
plane. The exact full-Hermite expression is rebuilt from its owned sections.
An outward plane-normal enclosure projects exact position and free-jet
bounds. Anchored rectangles can be accepted only when their full free
normal derivative excludes zero; other rectangles require strictly
nonzero plane-position bounds. The entire rectangle cover is retained on
work exhaustion. Only a complete cover creates the private plane-contact
certificate attached to the actual transition/neighbor owners.

The independent polynomial t*(1-2*t) test retains its additional zero at
1/2; accepted plus pending coverage preserves the whole domain. A linear
anchored case is admitted and zero work keeps the full domain pending.
Coverage summation uses a 1e-12 arithmetic tolerance only in the test,
after its initial exact-f64 sum assertion differed from one by 1e-16;
the geometric predicate and enclosures were not relaxed.

The first native run passed in 63.32 seconds but issued no contact proof:
rail 0 exhausted 512 cells / 31232 spans with 513 pending rectangles;
rail 1's neighbor was not planar. The latter needs a curved-surface contact
proof. The report additionally checks adjacent natural-rail control rows
for exact coplanarity: together with clamping this identifies zero first
normal jet (anchor order 2). Such tangency needs second-jet qualification;
first-order interval failure is not treated as a geometric defect or safe
contact. The unified audit remains at 155 pending pairs.

The final native run passed in 77.31 seconds. Rail 0 has freshly confirmed
anchor order 2 (8636 exact work) and remains at 513 pending rectangles;
rail 1 remains nonplanar (2932 exact work). Neither receives a certificate.
Next required work is full corrected second free-jet bounds for tangent
plane contacts and a curved-neighbor proof for rail 1. No packaged WASM
or full Body admission is claimed by this stage.

## Full corrected second free derivative (2026-10-09)

The transition now encloses its second free derivative, including all four
Hermite rail corrections. Natural free rails are not replaced by source
second derivatives: the construction preserves position and first jets,
which does not imply preservation of the second jet. Across endpoints use
the exact source section derivative because the correction vanishes there.
Shared span exhaustion returns no enclosure.

An independent quartic fixture checks C_vv=(0,0,2u^4) and a rational
positive-weight variant checks C_vv=(0,0,4u^4/(1+u^4)). All four
profile_section_bridge tests passed (0.54 s). The second-order contact
cover additionally refuses t^2(1-2t), which has an interior zero.

The native shell fixture passed in 77.42 s. With 512 cells the tangent
plane rail retains 21 pending rectangles (16412 spans, 8636 exact work),
down from 513 before second-derivative bounds. It still has no complete
contact certificate. The curved neighbor remains unsupported by this
plane-specific proof. Combined audit still has 155 pending pairs; neither
packaged WASM integration nor global Body validity follows from this test.

Whole-rectangle analytic checks also passed in the four-test group
(3.78 s). The expanded native run initially failed at the API work guard:
300000 spans exceeded its documented 100000 cap. The call is corrected
to 4096 cells / 100000 spans; no geometric predicate or guard is relaxed.

The quartic and rational whole-rectangle checks now run with both source
parameter axes (transposed control and weight nets). All four bridge tests
passed in 6.76 s. This covers orientation of the derivative transport; it
does not certify arbitrary NURBS input or the pending shell contacts.

The corrected expanded native call passed in 282.00 s. Rail 0 exhausted
100000 spans after 3125 cells, still retaining 21 rectangles. Its pending
list includes the untouched half free=[0.5,1], across=[0,1], while the
active rectangle had narrowed to free=[0.12615966796875,0.126220703125],
across=[0.2520751953125,0.252197265625]. This demonstrates depth-first
budget starvation; it is not a proof of an actual intersection.

The plane-contact cover now processes refinement levels in breadth-first
order. All unprocessed products remain pending on exhaustion. A regression
checks that seven evaluations reach all four coarse siblings and preserve
the whole pending area. All three plane_cover tests passed. An initial
filter by the source filename selected zero tests; the corrected function
filter explicitly selected these three. Native full-shell rerun is required
before claiming any additional pair certificate.

The analytic second-jet fixtures additionally check enclosure convergence:
a 2e-5-wide rectangle around (0.5,0.5) has less than one percent of
the full-domain z-derivative enclosure width, for both axes and both
weight cases. All four bridge tests passed in 1.73 s. This establishes
convergence for these fixtures, not for every source profile.

The breadth-first native fixture passed in 151.08 s. Rail 0 remains
unqualified: 3114 pending rectangles, 3124 evaluated cells, 100000 spans.
The pending count is not directly comparable to the depth-first count:
it now describes broadly refined coverage rather than a few large
unvisited siblings. No new contact certificate was issued.

A subsequent enclosure improvement uses an exact coordinate plane equation
when all three already-qualified plane points share a coordinate. Its
zero normal components are skipped exactly, avoiding outward cross-product
noise in structurally zero terms. General planes retain the cross-product
enclosure. Four plane_cover tests passed, including all three coordinate
plane orientations. Full native verification of this improvement follows.

An additional tangent-contact fallback is implemented for nonanchored
rectangles whose position plane interval contains zero. It evaluates the
full corrected second normal derivative over the entire free path from
the owned natural rail to that rectangle, with the same across interval.
A strict sign, together with the exact zero position and first normal
jet at the rail, proves nonzero position by twice integration. It never
uses a local second derivative away from the rail as a contact proof.
Fallback work is charged to the same remaining span allowance. The
interior-zero polynomial regression also exercises this complete-path
criterion; a final focused test run is pending.

The coordinate-plane-only native run passed in 140.64 s and retained
the same 3114 pending rectangles (3124 cells, 100000 spans). The complete
path second-derivative fallback then passed the native fixture in 104.34 s:
rail 0 retains 1288 rectangles after 1478 cells / 100000 spans; no complete
certificate. Rail 1 still needs a curved-neighbor criterion. Summing the
exact dyadic rectangle areas from the logs gives pending area 0.84375
before the fallback and 0.38037109375 after it. Remaining free extent is
[0.578125,1]. This is parameter-domain area, not world-space face area,
and is not a count of actual intersections. The combined shell audit
remains at 155 unqualified pairs.

The final four plane_cover tests also verify early span exhaustion after
three queries: all unprocessed area remains pending. The full-path
interior-zero regression passed. This stage is native only.

## Integrated normal-position enclosure (2026-10-09)

Nonanchored rectangles with inconclusive direct position and second-path
bounds now have an additional first-derivative integral enclosure. Eight
whole derivative intervals cover the fixed path from the exact zero rail
to the near endpoint; a final whole target interval encloses displacement
from that endpoint to every target parameter. Descending paths retain
signed widths. The result intersects the direct plane-position interval.
No point sampling or unchecked quadrature grants proof; every interval
query is charged to the same span budget. Incomplete paths return none.

Five plane_cover tests passed (0.01 s), including both integration
orientations, t^2(1-2t)'s additional zero, and work exhaustion. The full
native fixture is being rerun before any additional certificate claim.

The integral regression now enumerates all nine incomplete work budgets
and each of nine missing derivative queries. Every partial path returns
none, including paths with an already positive accumulated integral.
All five focused tests passed. Native full-shell evaluation is still live.

The integrated-position native fixture passed in 114.84 s and issued the
first complete owned transition-plane contact certificate for rail 0:
zero pending rectangles, 89 cells, 38512 spans, 8636 exact work. The
fixture now requires this certificate instead of merely printing its
status. Rail 1 remains nonplanar and needs another proof family. Combined
audit integration is next; the last combined audit still reported 155
pending pairs because it did not consume transition-plane certificates.
This one pair proof does not establish global Body validity or fillet
radius, wall thickness, or packaged WASM integration.

## Combined audit consumes owned transition-plane proof

The unified audit freshly qualifies pending pairs that have an owned Rail
edge, using the same remaining cells, spans, and exact work as all other
proof families. A private TransitionPlane certificate is retained as a
separate proof variant and emits diagnostic kind transition-plane.
Support rectangles are still present as inspection history; the typed
proof determines whether a pair is qualified.

The native fixture passed in 88.26 s. Counts are now [154 pending,
162 excluded, 12 original planar, 23 fiber, 54 vertex/interior,
1 transition-plane]. Combined work is 2245 cells, 51107 spans, 6494
domain cells, 3982141 exact work, 0 driver cells. Tests check actual owner
pointer identities, JSON/binary diagnostic round trips with 154 pending
pairs, all 406 inventory pairs, and zero budgets retaining all 406 pending.
This does not qualify the remaining curved-neighbor contact, all other
pairs, shell volume, wall thickness, or packaged WASM/UI runtime.

## Curved neighbor inventory

Fresh fixture inspection identifies rail 1 neighbor as original face 2,
degree [5,1], clamped domains [0,3] x [0,1], a 6x2 positive-weight net.
For each of its six rows the XY control coordinates and both weights
are identical across the linear parameter; only Z changes. Its complete
XY projection is therefore the original rational degree-five curve,
independent of that parameter. This is a ruled support, not an assumed
exact cylinder. The transition source has free axis 1 and its high free
rail coincides with the wall's upper curved boundary through the privately
owned shared-edge construction. A projected curve/Jordan exclusion proof
is the next relevant family; the current plane certificate cannot certify
this curved neighbor. No additional pair admission follows from inventory.

The inventory native fixture passed in 141.07 s, retaining the same
combined counts [154,162,12,23,54,1] and owner/transport/budget checks.

## Exact ruled trace certificate

A private borrowed certificate now reduces a positive-weight ruled support
to its exact projected source curve. It requires a clamped degree-one
linear axis with two rows, identical projected coordinates and weights
for each corresponding pair, and a bounded complete source layout. The
nonlinear degree, original knot vector and domain, control coordinates,
weights, and periodic flag are copied unchanged; no normalization or
rounded fitted curve is substituted. The certificate retains the actual
immutable source owner. Shape and cardinality guards precede deep source
validation to keep work inside the control allowance.

Focused tests cover both parameter orientations, non-unit original domains,
zero and insufficient control budgets, duplicate projection axes, one-bit
coordinate and weight perturbations, and independent nonprojected changes.
The initial ruled_trace filter selected these two tests and an existing
residual-cancellation refusal test; all three passed. A final focused
and full native run are in progress. Reduction does not by itself prove
that the transition avoids the projected curve away from its owned rail.

Final ruled-trace tests passed, including independent source-surface versus
projected-curve evaluation over both non-unit parameter domains and both
parameter orientations, plus reversed projection axes. The 1e-12 comparison
tolerance applies only to independent floating-point test evaluation;
certificate admission still uses exact coordinate and weight equality.
The native fixture passed in 252.80 s and freshly admitted face 2's trace:
curve degree 5, unchanged domain [0,3]. Combined pending pairs remain 154.

A new native XY-projection injectivity check is prepared for the complete
corrected transition. A midpoint-derived inverse only chooses a finite
preconditioning matrix in XY; the existing whole-domain principal-minor
enclosures alone grant proof. Failure or incomplete coverage retains
the projected contact as unqualified. This new check is being run next.

## XY injectivity and first jets from preserved rails

The direct XY native run passed in 120.23 s, but projection admission
remains incomplete: 476 accepted rectangles, 49 pending, 1000 cells,
61000 spans. No projected face certificate or curved-contact proof follows.

A new bounded first-free-jet enclosure integrates the full corrected
second jet from the exact source first jet at the nearest natural rail.
The whole connecting path is enclosed. Internal free knots permitting
C1 jumps are refused, including knots at the requested path endpoints.
An explicit stronger injectivity policy intersects these first-jet bounds
with the direct bounds and rechecks projected principal minors, charging
all additional queries to the same span budget. The original policy keeps
its existing work allocation.

Four bridge tests passed in 5.72 s, including first derivatives of both
quartic and rational fixtures along both parameter axes, and refusal of
a genuinely folded patch under the stronger policy. The new native XY
policy is being run; incomplete coverage cannot grant contact admission.

The first-jet path guard has a dedicated derivative-jump regression. A
degree-two free chart with a double knot at 0.5 is refused by the normal
rail factory. The test additionally bypasses that private factory only
to exercise the local defensive guard: paths ending at, starting at,
or crossing the knot all return none with zero derivative queries.
Five bridge tests passed in 14.39 s. The native stronger XY policy run
remains live; no projected injectivity/contact certificate is yet claimed.

The rail first-jet anchor now has an exact separable-weight endpoint
formula for a single clamped nonlinear span (degree at most eight). It
forms adjacent original control-point differences before weighting,
bounds their rational curve in the unchanged source parameter interval,
and multiplies by the exact degree/span and adjacent/boundary weight
ratio. The common separable factors cancel algebraically. Unsupported
nonlinear layouts retain the existing general derivative enclosure; no
rounded replacement controls are constructed.

A translated-world regression initially used 2^40 and was correctly
rejected by the source coordinate cap of 1e9. It now uses valid 2^29
coordinates and proves a strictly positive rail derivative from a one-ULP
control difference. The cap was not changed. Six bridge tests passed in
15.49 s, including rational weights, both axes, folds, and derivative
jump refusals. The preceding native stronger-policy run is still live;
its binary predates this more stable endpoint formula.

The native first-jet integration run (before the stable endpoint formula)
passed in 684.23 s. XY coverage remains incomplete: 352 accepted,
179 pending, 1000 cells, 90859 spans. Counts alone cannot compare
different refinement trees; the next native fixture reports pending
parameter area and rectangles alongside a direct-policy run with the
same 1000-cell / 100000-span limits. Combined audit remains at 154
pending pairs. The stable endpoint formula is now included in that
next native executable; the previous live handle finished before rerun.

## Shared rail displacement in projected determinant

The stronger policy now retains its first-free-jet decomposition
A(across) + delta * B(free,across), where A is the exact preserved rail
first jet and B encloses the average full second jet along the path.
It bounds the projected determinant as det(A,T) + delta*det(B,T),
where T is the whole across-column enclosure. The same delta occurs
in both entries of the free column; expanding it before interval
evaluation avoids losing this dependency. Both projected diagonal
entries must still be strictly positive. No diagonal or determinant
predicate is weakened; all source queries use the same work allowance.

An independent affine-column regression uses A=(epsilon,0.1*epsilon),
B=(-1,-0.1), delta in [-1,0], T=(1,1). Independent column boxes fail
while the expanded determinant is strictly positive. Reversing B
retains the real interior zero. Seven bridge tests passed in 4.59 s,
including the existing folded-patch refusal under the stronger policy.
The currently live native comparison predates this determinant change;
no additional contact or projected-face certificate is claimed yet.

### Whole-domain scheduling for rail-based projected injectivity

The stable source-rail native run reported 486 accepted and 29 pending cells
at 1000 visited cells / 91673 spans. Pending parameter area remained one third,
as in the direct baseline (476 accepted / 49 pending / 61000 spans). Its
pending rectangles included a corner refined below 1e-10 alongside the entire
[0.5,1] x [0.5,1] rectangle. Fewer pending cells therefore did not demonstrate
better coverage.

The explicit from-rails policy now uses breadth-first subdivision so a difficult
corner cannot consume the cell budget before other coarse regions are visited.
The original policy retains its traversal. Principal-minor tests, full-path
bounds, interval determinant expansion, and pending-on-exhaustion semantics
are unchanged. Seven profile-section bridge tests passed (3.15 seconds),
including genuine-fold refusal and common-displacement determinant tests.
The full native fixture must still verify this scheduling change; it is not
an admission of the XY projection or a global shell contact certificate.

The preceding stable-rail native fixture subsequently completed successfully
in 550.70 seconds. Its combined owned-shell audit still reported
[154 pending, 162 excluded, 12 planar, 23 fiber, 54 vertex/interior,
1 transition-plane] across 406 pairs. That executable predated the coupled
Jacobian determinant and breadth-first policy, so its passing result cannot
qualify those changes. The replacement full fixture is running with both
changes and explicit regular-plus-pending parameter-area checks.

The combined determinant plus breadth-first native run has now emitted its
XY cover: 464 accepted / 73 pending, 1000 cells, 84828 spans, pending area
0.02392578125 (2.392578125%). Compared with the direct baseline's one-third
pending area, this proves more complete coverage at the same cell cap, not
whole-domain admission. Most remaining rectangles touch free=1; the rest
are near the upper across section. The fixture's total-area checks passed
before it continued to shell incidence/contact checks. Exact high-rail
projected first-jet diagnostics are being added to distinguish boundary
projection degeneracy from coarse-box overestimation. Full native execution
is still in progress.

The breadth-first full fixture completed successfully in 143.22 seconds,
retaining the combined 154 pending contact pairs. The next native executable
emitted exact high-rail projected first-free bounds (one span each):
across=0: first component [2.4967469075612617e-13,2.4967469075613293e-13];
across=0.5: [1.4432026944035643e-13,1.4432026944036127e-13];
across=1: [5.518449039608263e-14,5.518449039608565e-14].
Their second projected components are negative, respectively approximately
-2.2672e-15, -1.1500e-15, and -3.8161e-16. These are interval bounds of
stored source data, not a tolerance-based zero classification. They show
that the first principal minor is positive but extremely small at the rail;
coarse direct derivative boxes can overwhelm it. Three point bounds do not
prove its sign along the entire rail, nor exclude contacts with the full
ruled-neighbor trace. A full-path curvature/factor proof remains required.

Whole high-rail first-jet diagnostic (across=[0,1], one source span) encloses
its first projected component in
[5.518449039608263e-14,2.496746907561329e-13]. Thus this principal minor is
strictly positive along the full original rail, despite its small scale.
Its second component spans [-3.601831891585211e-15,9.530326499677803e-16];
no sign assertion is made for that component. The fixture now requires the
full-rail positive bound. Two complete near-rail second-derivative rectangles
also have strictly negative world-X bounds: [-0.41591,-0.36639] and
[-0.22425,-0.18165], each using 36 spans. These component statements are
insufficient alone to qualify the complete projected Jacobian determinant.

The full whole-rail diagnostic fixture completed successfully in 143.36
seconds. The new `TransitionBoundary::qualify_face_from_rails` reconstructs
its exact owned Hermite recipe and runs the complete rail-based injectivity
policy itself, creating a `TransitionFace` only for a complete report.
It accepts no caller-provided certificate or diagnostic report. The original
`qualify_face` route remains unchanged. Native checks now cover zero-budget
refusal, invalid projection rejection, and a fresh successful 3D admission
with exact boundary-owner identity and projection preservation. Compilation
passed; the new successful-admission fixture is still running.

A requested 200000-span diagnostic run was rejected by the existing bridge
input cap (`Bound Hermite bridge domain and work`), rather than failing a
geometric predicate. The follow-up uses 2000 cells / 100000 spans within the
existing contract; no numerical sign criterion or resource cap was weakened.

### Preserve rail displacement through projection of the first column

A remaining dependency loss occurred before the principal-minor predicate:
world-coordinate boxes of A + delta*B were projected independently, losing
the common delta when the projection mixed X and Y. The stronger policy now
also intersects that column with P*A + delta*(P*B). This is the same exact
FTC expression, enclosed with outward interval operations, and requires no
additional derivative queries. Both diagonals and determinant still require
strict positivity. A regression has A=(1e-12,0), B=(-2,1), delta=[-1,0],
projection=(1,1): independent boxes include negative values but the coupled
projection proves positivity. Reversing B retains the genuine interior zero.
All eight bridge tests passed in 1.38 seconds, including folded-patch refusal
and both work-limit coverage stop conditions. Full native confirmation of
this new column intersection remains pending.

The pre-column-intersection run with 2000 cells / 100000 spans emitted
516 accepted cells and 127 pending rectangles after 1159 cells, with exactly
100000 spans consumed. Pending parameter area was 0.0113525390625
(1.13525390625%). This is a derivative-budget stop, not the 2000-cell stop
and not a geometric intersection witness. Its result is the comparison
baseline for the new shared-displacement projected-column intersection.

### Full XY injectivity confirmed after projected-column correction

The new native run emitted a complete XY cover: 456 accepted rectangles,
zero pending, 911 cells, 75773 spans. Regular-plus-pending area checks passed.
The fixed projection is
[[8.370299227358235,0.9154746877371366,0],
 [0.058483674244829885,-0.8488064130710989,0]].
No tolerance or principal-minor condition was weakened. The fixture now
requires this complete cover and separately requests a fresh owned XY face
through `qualify_face_from_rails`, verifying owner identity, exact projection,
and complete regularity. The direct baseline remains unqualified. Complete
XY injectivity does not yet prove exclusion of the full ruled-neighbor trace
outside the owned subrail; the combined audit still has 154 pending pairs.

### Exact monotone ruled-neighbor trace

A private certificate now proves a complete single-span positive-weight
rational Bezier trace is strictly monotone in a selected projected coordinate
when all successive controls are strictly ordered. The rational derivative
numerator is a positive sum of weighted ordered control-pair differences;
strict adjacent differences also cover the two endpoints. Original knots,
weights, domain, and borrowed trace owner are preserved. Mixed ordering,
periodic/multispan layouts, insufficient edge budget and invalid coordinate
retain refusal. This proves a simple full trace, not transition contact.
The nonunit-domain/reversed/fold/work-limit test passed. Native curved-neighbor
diagnostics now require this property for the actual full face-2 Y trace;
that native assertion is awaiting execution.

The monotone trace certificate now encloses a requested coordinate over any
complete original-domain subinterval using outward-rounded endpoint interval
evaluations. Monotonicity supplies the interior bound; no sampled interior
admission is used. Nondegenerate intervals require both endpoint evaluations,
point intervals require one; insufficient work returns no enclosure and an
out-of-domain request errors. The complete exact-trace suite (three tests)
passed, including unequal weights and nonunit domains. This supports later
exclusion of external trace portions but does not itself certify contact.

A bounded coordinate-preimage cover now eliminates trace parameter cells
only when their certified coordinate images are strictly disjoint from a
target interval. Cells entirely inside the target remain candidates; all
queued cells remain candidates at budget exhaustion. The three exact-trace
tests passed, covering an empty outside target, zero-budget whole-domain
retention, and containment of all independently evaluated target preimages.
Native diagnostics connect this filter to a fresh full corrected-transition
Y enclosure and the actual monotone neighbor trace with 128 cover cells.
Compilation passed. This connection only narrows candidate parameters; it
does not authorize a pair contact or reduce the reported pending-pair count.

Fresh owned XY transition admission completed the full native fixture in
420.05 seconds, including zero-budget refusal, owner/projection identity,
complete coverage and the unchanged 154-pending-pair combined audit.
The coordinate-preimage report now borrows the exact monotone certificate
and retains its immutable requested target. Tests check both identities and
retain a target touching the exact original endpoint. All three exact-trace
tests passed (0.01 seconds). These reports remain conservative candidate
covers, not complete pair-contact certificates. Native filtering is running.

Native trace-filter diagnostics now transport the actual owned carrier-root
UV intervals into the neighbor trace's nonlinear parameter axis. They retain
candidate portions outside the definitely interior shared-rail interval,
including both uncertain endpoint bands. No midpoint replaces a root and no
candidate list grants contact authority. Compilation passed; native results
for these root-chart diagnostics are still pending.

Exterior trace candidates now use a tested conservative endpoint-band split.
Both original root intervals must lie in the complete trace chart. Either
endpoint order is accepted; overlapping root intervals retain the entire
candidate cover. Definitely interior intervals alone are removed, with their
closed limits retained in exterior bands. Three exact-trace tests passed,
including swapped roots, overlapping uncertainty, and outside-chart refusal.
This utility does not bind arbitrary supplied roots to a geometric carrier;
native diagnostics supply the actual owned carrier UV intervals, and no pair
contact certificate is created from this split.

A stronger route is under investigation: the corrected transition specializes
to exact original source curves on all four boundaries. A fresh global
projected-Jordan certificate for the original source, together with the
owned transition's complete XY injectivity, could establish equality with
that source subregion's projected interior. This still requires proving the
boundary specialization/ownership and exclusion of the full wall trace;
no certificate is transferred merely from equal-looking diagnostics.
Native diagnostics now request the original source's XY Jordan certificate
within declared nonzero work/cell budgets. Compilation passed; qualification
of this original-source certificate has not yet been observed.

Native full neighbor monotonicity completed successfully in 546.97 seconds.
The first full-expression scalar filter was unhelpful: the unsplit Y enclosure
[-55.20937163996749,33.99841160034473] kept the entire [0,3] wall trace.
This is interval overestimation, not a measured geometric extent. Diagnostics
now enclose a complete 8-by-8 partition of the corrected transition and union
all 64 Y bounds before intersecting the original broad enclosure. All cells
must return complete bounds within one shared span budget; otherwise the
original broad target is retained. Grid endpoint values are reused exactly,
including the original domain extremes. The new native run also checks the
original-source projected Jordan route; both results remain unverified.

A new owned `OriginalProjectedRegion` route pairs the already admitted
transition with a fresh original-source XY Jordan certificate. It requires
an interval-proven invertible XY projection, section cuts strictly inside
the original chart, and no collapsed original boundary in the uncut free
direction. Exact Hermite boundary specializations then identify the same
simple projected boundary; injective interiors and Jordan separation identify
the same planar region. The certificate retains the actual transition owner,
original certificate and original UV subrectangle. This does not admit a 3D
Body or a neighbor contact. Zero budgets refuse and invalid tolerances error.
Compilation passed. Native owner/domain/refusal checks and actual original
qualification are running; no success is assumed for the original source.

The projected-region transfer additionally requires exact identity between
all original weights and the profile's unrounded factor expressions. This
condition was missing from the initial route: an approximating WeightProfile
cannot inherit original geometric authority just because its controls match.
Fresh exact Orient2 predicates prove row[i]*column[j]-originalWeight*pivot=0
for every control. Nonzero/uncertain results refuse; all work is charged before
passing only the remaining budget to the original Jordan routine. Tests pass
for exact separable weights and refuse the rounded-product near-match
1.1*1.1 versus binary64 1.21, as well as zero work. The test passed after the
shared-budget correction. Previous native fixtures predate this added guard.

The original-source Jordan/tiled fixture completed in 278.84 seconds. The
initial projected-region fixture completed in 218.99 seconds but correctly
refused original orientation at its 10000000-work cap. Its measured full
original Jordan cost is 12488524, so the next guarded run uses 20000000.

A new private-construction `TransitionRuledContact` route requires an owned
original projected-region certificate for the very same transition boundary,
a fresh exact ruled-neighbor reduction, exact complete Bezier boundary-trace
identity (possibly reversed, preserving positive weights), and a fresh 3D
root-ended incident-edge certificate. The full wall trace is the original
Jordan boundary: source/transition interior images cannot meet it, and the
clipped source subregion touches it only on the selected natural rail.
No old pair report or approximate trace equality creates authority. The
exact-identity test passed, including one-bit coordinate/weight refusal,
reversed nonunit domains and periodic refusal. The full guarded native fixture
now requires this owned high-rail contact; it is running. The combined audit
has not yet been changed and still reports 154 pending pairs.

The guarded native projected-region route exposed a real fixture limitation:
exact original weight identity is unproven after 888 predicate-work units.
The fixture's WeightProfile is an exact factor-expression proposal but differs
from its original authored weight matrix. Its original Jordan certificate
therefore cannot establish boundary identity for the profile. The initial
native expectation of a successful original transfer failed and has been
corrected to require this refusal; no geometric guard was weakened.

Combined auditing now has an optional owned-original-region entry point and
a transition-ruled proof kind, accounting fresh edge exact/driver work against
remaining shared budgets. It never creates that proof without the valid
region certificate. The current fixture must retain 154 pending pairs, with
zero transition-ruled admissions. Compilation passed; the corrected full
native refusal/audit fixture is running. The next required geometry work is
projected-Jordan authority for the exact factor-expression WeightProfile,
including its actual boundary trace; original-surface authority is insufficient.

### Exact factorized-weight projected Jacobian

Added `cad_predicates::factorized_surface_projected_jacobian` for authored Cartesian controls and positive row/column weight factors. The common positive pivot cancels from the rational map. Target weights are multiplied as exact expansions, including nonrepresentable binary64 products; no rounded replacement surface is used. Whole grids and individually selected coefficients share the existing determinant algorithm and budget refusal. This is orientation evidence only, not a Jordan-boundary or shell-contact certificate.

Validation: all 39 cad-predicates library tests pass. New regression covers nonrepresentable 1.1×1.1 products, all selected coefficients, reversed projection, a real fold, nonpositive factors and exhausted budgets. Existing native restricted-profile seam fixture passed (219.87 s), retaining 154 pending contact pairs and counts [154,162,12,23,54,1,0]. No WASM rebuild or UI admission was performed. Remaining work: bind this predicate to the actual WeightProfile orientation certificate and prove its exact boundary before region/contact admission.

### WeightProfile-owned orientation

`nurbs_core::profile_projection_jacobian::certify` now borrows the actual WeightProfile and certifies the full projected Jacobian coefficient grid of its exact factorized weights. Single clamped nonperiodic Bezier charts (degrees 1..8) are supported. Each row and coefficient fallback consumes one aggregate exact-work allowance. Missing coefficients, mixed signs and rank deficiency cannot produce an orientation certificate. The certificate preserves profile ownership, axes and orientation; it does not assert Jordan boundary simplicity, global injectivity or contact.

Validation: two new profile tests pass (nonunit domains, nonrepresentable weight product, owner identity, axis reversal, budgets, fold and rank deficiency); ten projected-Jacobian predicate tests pass and nurbs-core checks successfully. The full restricted-profile seam fixture is running in `/tmp/exact-profile-orientation-native.log`; its outcome is pending. No reduction of the 154 pending pairs is claimed.

### Exact factorized-profile Jordan boundary

Added `profile_projected_jordan::certify`, preserving ownership of the actual WeightProfile. On each natural boundary the fixed-axis positive factor cancels; boundary curves therefore use the remaining authored row or column factors exactly, including reversed natural-edge orientation. No original source weights or rounded product weights enter boundary admission. Exact profile orientation, boundary simplicity and necessary adjacent-curve separators together qualify global projected interior injectivity. This is not yet a closed-body or contact certificate.

Focused Jordan tests pass for the original and profile paths, including rectangular and collapsed triangular boundaries, nonunit source domains and exact factor weights. Added profile fold refusal. The annular native orientation fixture remains live in session 14402; no result for its new orientation step or pair reduction has been claimed. Next: use the profile Jordan authority in the owned transition-region/contact pathway and verify the annular fixture.

### Annular exact-profile resource diagnosis and dependency pruning

The previous full native fixture passed in 236.24 s with 154 pending pairs. Its new profile orientation was correctly refused: exact reason ResourceLimit, work 1,999,995, no complete coefficient grid. A separate focused annular probe confirmed the actual factors and resource cause; it did not establish a fold.

Projected-Jacobian minor products now compute only coefficient indices that can contribute through H to a selected determinant coefficient/row. Whole-grid behavior is preserved; ten predicate tests including whole/row/coefficient equality pass. After pruning, the annular probe advanced to work 12,974,983 but still refused due ResourceLimit inside a bounded query (5.48 s). Aggregate allowance remains 20 million and per-context limits remain unchanged. Two profile-orientation and three Jordan tests pass. Added explicit first-uncomputed coefficient diagnostics and a focused native probe; latest run is live in session 15461, log `/tmp/exact-annular-profile-window-pending.log`. Region/contact admission remains unavailable until orientation is fully proved.

### Exact unit and independent factor normalization

The focused annular probe identifies the first uncomputed coefficient [6,2], ResourceLimit, rather than a certified fold. Exact expansion multiplication now preserves the operand directly for an exact unit, retaining charge, cancellation and expansion-capacity checks. Factorized projection inputs normalize Cartesian coordinates plus their homogeneous unit separately from row/column factors; the resulting positive common homogeneous scale preserves every determinant sign. Translation/power-of-two scale regressions and low-term/unit/refusal regressions pass; all 40 cad-predicates tests pass.

Measured probe results: unit optimization still refused at [6,2], 12,798,479 work / 5.89 s; separate normalization still refused at [6,2], 12,788,602 work / 5.08 s. Both probes terminated successfully as diagnostics; neither grants orientation/contact admission. Current pending pair count remains 154. Further work must reduce the intermediate determinant cost or use a rigorous interval filter with exact fallback, while retaining existing limits.

### Filtered exact-profile qualification and owned-region/contact integration

Added an outward interval Bernstein Jacobian filter for factorized weights. Only strictly positive/negative interval bounds authorize coefficient signs; intervals containing zero trigger exact coefficient fallback. Full bounds are all-or-nothing under the filter budget. Filter and exact fallback consume the same aggregate profile work allowance. Cross-check regressions compare strict filtered signs with exact results for rational weights, translated charts, reversed axes, real folds and collapsed boundaries; zero coefficients are never inferred from rounding. All 41 cad-predicates tests, two profile-orientation tests and three Jordan tests pass.

The annular factorized profile now fully qualifies: orientation work 1,133,191 with 74 filtered coefficients; exact zero fallback covers the remaining boundary coefficients. Its Jordan certificate also qualifies at work 1,459,874 / 0 boundary cells. Focused native `exact_annular_profile_projection_is_globally_injective` now requires both owned certificates and passes in 3.36 s.

Added distinct ProfileProjectedRegion authority for the owned transition face; the original-source path retains exact source-weight identity refusal. Added fresh ruled-contact and combined-audit paths accepting the profile region and exact boundary-trace identity. Full restricted-profile native test is running in session 88768, log `/tmp/exact-profile-region-contact-native.log`; expected 153 remaining pairs is an assertion still awaiting verification, not a reported result. No new WASM or UI admission has been packaged.

### Verified annular ruled contact: 153 pending pairs

The full restricted-profile native test completed successfully in 210.16 s. The actual factorized profile region and its owned transition face qualify, then fresh exact ruled contact qualifies with 107,483 exact work / 0 driver cells. Complete combined audit counts are [153 pending,162 excluded,12 planar,23 fiber,54 vertex-or-interior,1 transition-plane,1 transition-ruled], covering all 406 pairs; total audit work 4,089,624, cells 2,245, spans 51,107, domain cells 6,494. Original-source weight-identity refusal remains enforced. Diagnostics round-trip through binary and JSON with 153 pending. This is one qualified contact in one fixture, not global Body admission.

Exact trace identity now also permits common positive homogeneous weight scaling, proved with exact dyadic cross-products and shared work. Incident-edge qualification receives the remaining budget. Focused regression accepts exact scaling and reversed/nonunit charts; rejects one-bit coordinate/weight changes, periodic curves and exhausted work. Optional BREP_CONTACT_AUDIT_PATH exports the full fixture diagnostics for analysis of remaining pairs. WASM/UI remain unbuilt for this milestone.

### Remaining pair classification and diagnostic owner metadata

Latest full native fixture passed in 195.71 s after the homogeneous trace-identity extension. Saved its unchanged transport report as `contact-audit-153.json` (SHA256 83ea75d6e92cf21b261e5cd23d38d81ec3424e8b71fd8f5db2354db2a59c81fc). The report has 406 pairs and 153 pending: 62 source-fiber-chart-unproven, 48 source-interior-fiber-support-or-ownership-unproven, 43 expression-original-contact-face-changed.

For this fixture, source shell construction places Retained(0) at face 0, Retained(1) at 27 and Transition at 28. The pending groups are 110 original-original, 17 original-retained, 24 original-transition, 2 retained-transition. The retained/transition candidates are [0,28] with shared edge 57 and [27,28] with edge 58. Retained-retained is already excluded. These counts classify uncertainty, not defects.

Added additive diagnostic fields `faces`, `pendingByReason` and `pendingByFaceKinds`, preserving original face indices and retained-side indices. A focused test verifies only pending proofs are counted, reversed pair ordering is canonicalized, owners are retained and binary diagnostics round-trip. The saved fixture artifact predates these additional fields; its pair records remain authoritative for the classification above. Next geometric work: refine incomplete exclusions of the 110 original pairs and qualify their remaining charts/owned contact loci; use the exact profile-region authority for the two section contacts.

### Resume pending original-pair exclusions under shared limits

Added `inspect_contacts_with_profile_region_refined`: produces the complete fresh owned audit first, then revisits only pending original-original products. Remaining cells/spans/domain work are distributed across those pairs and their existing frontiers; previously excluded products and qualified contacts are retained. Only a fully excluded frontier can replace Pending with Excluded. Inputs are owned geometry and the typed profile-region certificate, not serialized diagnostics. Default base audit remains unchanged.

The internal refinement helper updates a report only after all steps succeed; callback failure or excess work leaves its original cover intact. Two focused regressions pass: aggregate cover/budget/transaction behavior, and repeated actual conservative support queries on intersecting perpendicular planes, retaining an explicit intersection witness. A related root-refinement test also passes in the selected test run.

The full restricted-profile fixture now compares base and refined audits, preserves every existing proof and owner pair, enforces global budgets, and exports refined diagnostics when BREP_CONTACT_REFINED_AUDIT_PATH is set. Running session 44284 / `/tmp/profile-refined-contact-audit-native.log`, expected artifact `/tmp/profile-refined-contact-audit.json`; no new pending-count reduction is claimed until it completes. Last verified count is 153.

### Verified shared-budget refinement: 131 pending pairs

The full restricted-profile native fixture passed in 207.04 s. Refinement excluded 22 complete original-original pair frontiers, preserving all prior contact proofs and the global work limits: 3,939 cells, 54,495 spans and 18,055 domain cells. Remaining pending groups: 88 original-original, 17 original-retained, 24 original-transition and 2 retained-transition. Saved `contact-audit-131.json`, SHA256 74f083b1d3accbeb4083d838074fea03ab83c4254a859e3464654715cd2991c4. These are unresolved pairs, not identified intersections; global body admission remains unproven.

The new section-contact protocol is undergoing native validation. It uses owned crossing endpoints to restrict the extended cutter to the natural profile section, together with the full profile Jordan certificate and retained-domain separation. No newer count or WASM/UI delivery is claimed before that validation completes.

The saved 131-pair report further separates pending adjacency: original-original has 15 pairs with shared edges and 73 without; original-retained has 5 with shared edges and 12 without; all 24 original-transition pairs have no shared edge. The two retained-transition pairs share their cut sections. This distinguishes pending contact-locus proofs from incomplete separation queries; absence of a shared edge does not establish nonintersection.

### Verified owned section contacts

The full native restricted-profile fixture passed in 266.87 s. Both cut-section contacts qualify with retained/transition owners; zero and insufficient control budgets refuse, invalid end indices error. Base audit covers all 406 pairs: [151 pending,162 excluded,12 planar,23 fiber,54 vertex-or-interior,1 transition-plane,1 transition-ruled,2 transition-section]. Refined audit has 129 pending, preserving all previous proofs and shared limits. The retained-transition pending group is now empty. Global embedding, thickness/radius qualification, final Body admission and WASM/UI delivery remain incomplete.

Saved reports: `contact-audit-151.json` SHA256 b5b68609a467407c69c3f2dca601d5edbe71fcb50b85881144012a07094ab391; `contact-audit-129.json` SHA256 9c18655564f0630b3b81c6cc8b857f08fa2dcef4503ad4a0c6cba536d10ceeaa.

### All-face pending refinement under native validation

Added `inspect_all_contacts_with_profile_region_refined` alongside the existing original-only path. It resumes every pending owned pair with the same global budgets and conservative support/trim queries. Retained faces use exact WeightProfile bounds; transition bounds use the owned bridge and transition trim exclusion remains refused. Previously qualified contact loci are retained unchanged. Focused frontier transaction/cover and real-intersection-witness regressions pass. The full fixture compares all 406 owner pairs/proofs and enforces every aggregate work limit. Native run session 40530, log `/tmp/profile-all-refined-native.log`, optional output `/tmp/profile-all-refined-audit.json`; latest verified pending count remains 129 until this run finishes.

### Verified all-face refinement and allocation tradeoff

Full native fixture passed in 248.16 s. All-face refinement uses exactly 4,096 cells, 68,187 spans and 22,762 domain cells; all existing typed contacts remain unchanged. It leaves 133 pending: 95 original-original,14 original-retained,24 original-transition. Compared with original-only refinement (129 pending), it newly excludes [[11, 27], [15, 27], [19, 27]] but no longer reaches seven original-original exclusions within the same shared budget: [[4, 12], [11, 20], [11, 21], [11, 25], [12, 19], [14, 21], [18, 24]]. Thus wider fair allocation closes three retained-face pairs but is less effective overall at this limit. Both explicit methods remain available; no automatic replacement of the 129-pair result is justified. Combining independent reports would require accounting for both computations and fresh owned authority, not importing serialized diagnostics. Saved `contact-audit-all-refined-133.json`, SHA256 a4ffe17834acdbce147b6d4908cebd5969b7319fbd4eb43f5e653c4993351a6b. Global Body admission and WASM/UI remain incomplete.

### Owned cumulative continuation under validation

Added `resume_contact_audit` accepting only a live typed audit whose face-slice identity belongs to this immutable shell. It refines only Pending products, retaining all qualified contacts/exclusions. Every limit includes previous work; smaller cumulative limits and foreign owners refuse before mutation. Zero additional per-pair cells preserve the report. Focused transactional frontier regression passes. Full native fixture adds unchanged-report checks for insufficient and zero budgets, then continues the 129-pair owned report under 8,192 cumulative cells, with all other limits unchanged. It checks proof preservation, monotone counters and aggregate caps. Native log `/tmp/profile-resumed-contact-native.log`, requested artifact `/tmp/profile-resumed-contact-audit.json`; result remains unverified while running.

### Optional linear-chart contact API

Added `qualify_original_additional_contact_with_linear_chart`, with cumulative `linear_cells` across both plane/fiber contact orders. Existing additional-contact API delegates with zero linear cells. Maximum work bounds and unchanged-contour ownership checks remain mandatory. `cargo check -p brep-core` passes. Underlying `root_clipped_curved_wedge_closes_with_new_planar_cap` regression passes in 25.56 s: all 15 face pairs qualified, supported interior side/cap contact confirmed, insufficient driver work refuses. This regression validates the underlying protocol; no new expression-shell contact count or general NURBS qualification is inferred from it. Integration into the resumed typed audit remains to be done after its current full validation (session 84887).

### Verified cumulative continuation: 110 pending pairs

Full native fixture passed in 658.59 s elapsed under concurrent host load. Continuing the live 129-pair audit, rather than rebuilding it, excludes 19 additional complete pair frontiers and preserves every previous proof. Cumulative work: 8,085 cells (limit 8,192),89,177 spans (limit 100,000),40,607 domain cells (limit 100,000). Zero added cells leave the report unchanged; a limit below spent work refuses before mutation. Remaining groups {'original-original': 72, 'original-retained': 14, 'original-transition': 24}. Saved `contact-audit-resumed-110.json`, SHA256 ea2afef3fa8efead8c30d3e9442b075bb168aa883d631d947c5e954223bfb8ae. This count does not include the new optional linear-chart pass, which still needs full fixture validation. Global Body admission, thickness/radius and WASM/UI remain unqualified.

The typed audit now exposes cumulative `linearCells` separately from trim domain work. `qualify_remaining_original_contacts` revisits only pending unchanged-source pairs, retains all prior proofs, and shares remaining exact/spans/driver limits as well as the explicit linear budget. Diagnostic owner/grouping regression passes. Full fixture additionally tests zero/invalid linear budgets, prior-proof preservation and cumulative caps. Running session 54456, log `/tmp/profile-linear-contact-native.log`, requested artifact `/tmp/profile-linear-contact-audit.json`. No result from that pass is claimed yet.

Cumulative continuation validation now has a focused owner/limit regression: identically described faces at a different allocation are rejected; limits below each spent counter, nonfinite/zero tolerance and excessive caps refuse without changing diagnostics. The four selected assembled-shell audit regressions pass (owner/limits, complete-cover transactional refinement, retained real-intersection witness and diagnostic groups). Full linear-chart fixture remains running in session 54456; no additional pair reduction is claimed.

### Verified optional linear-chart pass: unchanged 110 pending

Full native fixture passed in 260.30 s. Fresh original-pair linear-chart fallback spent 844 cells and preserved all existing proofs; zero/invalid linear limits preserve diagnostics. It did not close any additional pair. Cumulative spans 89,449, exact work 4,919,242, driver cells 0; all caps remain respected. Remaining reasons {'expression-original-contact-face-changed': 38, 'source-fiber-chart-unproven': 20, 'source-interior-fiber-support-or-ownership-unproven': 52}. Saved `contact-audit-linear-110.json`, SHA256 25ac1958129a9dfe4ea993ee3783fc79fc90f8135ce7626414422a8b16d30c8c. This verifies conservative refusal and accounting, not successful general contact qualification. Next investigation must inspect why remaining fresh protocols refuse (including chart/support/ownership constraints), rather than infer contact from counts or merely increase resource caps. The source Geometry has typed immutable admission but any transfer to modified shell requires fresh complete-region and contact-locus ownership equivalence; its public diagnostics alone cannot authorize transfer.

### Fresh exact hull-separation pass under validation

Added typed `OriginalHullContact` and `Hull` pair proof. Before fresh native hull separation, the existing exact full-region refinement comparison must establish both original contours unchanged; retained/transition and incomplete refinements refuse. `qualify_remaining_original_hulls` checks live audit ownership, shares remaining cumulative exact work fairly, and retains prior proofs. It calls `source_hull_separation::certify_shell` afresh, not source Body diagnostic counters. Three focused hull regressions pass: rotated shaft-face strict separation, shared cone-pole refusal, and rounded-midpoint zero-sign refusal. Four audit regressions also pass. Full fixture tests zero remaining work, invalid face pairs, immutable owner pointers and all prior proofs, then exports `/tmp/profile-hull-contact-audit.json`. Native log `/tmp/profile-hull-contact-native.log`; latest verified residual remains 110 until completion.

### Fresh pole-contact integration awaiting full validation

Added `OriginalPoleContact` with distinct paired-pole and planar-pole native proofs, after exact complete unchanged-region comparison. Both attempts share exact/driver limits. The resumed audit charges their work and preserves existing proofs; zero remaining exact work skips queries. Focused paired-cap-meridian regression passes for both orders and exhausted work; four audit regressions pass after integration. Full fixture now includes invalid/zero-work refusal, prior-proof preservation and actual owner pointers for pole contacts. The already running hull-only compiled fixture remains session 84536; it has not yet exercised the new pole phase. No pole reduction is claimed before a later full run.

### Verified fresh hull separation: 53 pending pairs

Full native fixture passed in 320.01 s. Fresh strict hull separation qualified 57 additional original-original pairs while preserving all previous proofs and actual owner pointers. Remaining 53 pairs: {'original-original': 15, 'original-retained': 14, 'original-transition': 24}. Exact work 5,607,024 (limit 100,000,000), all other cumulative counters unchanged. Empty remaining work preserves diagnostics; invalid/same faces and exhausted work refuse. No source Body diagnostic report was inherited. Saved `contact-audit-hull-53.json`, SHA256 1160b8236647bcbd1ec523dd8b467047427c58793b850f8cdb5eb769c9330294. Pending adjacency groups: {('original-retained', True): 5, ('original-retained', False): 9, ('original-transition', False): 24, ('original-original', True): 15}. These are unresolved contacts, not identified defects. The newly added pole pass was absent from this compiled run and still requires full validation.

Full pole-inclusive validation now runs in session 16752, log `/tmp/profile-pole-contact-native.log`, requested artifact `/tmp/profile-pole-contact-audit.json`. The 15 remaining original-original pairs all have shared edges; remaining modified-face work is 14 original-retained (5 shared-edge contacts,9 unresolved nonadjacent separations) and 24 nonadjacent original-transition pairs. Latest verified residual is 53.

### Profile control-hull separation prepared for validation

Added distinct `ProfileHullContact`: exact positive row/column/pivot factors and actual source/control ownership let the full raw rational control hull contain every retained factorized profile. Fresh strict separation uses raw source control hulls only; original planar material trims are explicitly forbidden for this target. The method refuses other face-kind pairs, mismatched source surfaces, invalid indices and exhausted work. It accepts no source diagnostic admission. Cumulative audit pass preserves previous proofs and charges exact work. Four focused audit regressions pass after compilation. Full fixture now checks owner pointers and that every accepted profile separator has no material-hull substitution, but this phase is not in the currently running pole-inclusive binary (session 16752). Latest verified pending count remains 53.

### Verified pole-contact pass: 39 pending pairs

Full native fixture passed in 247.07 s. Fresh paired/planar pole protocols qualified 14 additional original-original pairs, preserving all previous proofs and pointer owners. Remaining groups {'original-original': 1, 'original-retained': 14, 'original-transition': 24}. Cumulative exact work 6,002,302, driver cells 0, all limits respected. No remaining-work queries leave diagnostics unchanged; invalid face and exhausted driver work refuse. Saved `contact-audit-pole-39.json`, SHA256 56d04900b969e88bc90a60e3f9f5b9ad513831ec9148ec1096a44662b5feddad. Profile-control-hull separation remains unvalidated for this fixture; global embedding and Body admission remain incomplete.

Profile-hull-inclusive native validation is running in session 90601, log `/tmp/profile-material-hull-native.log`, requested artifact `/tmp/profile-material-hull-audit.json`. The one remaining original-original pair is [10,12] with source-pole-planar-contact-unproven; its shared edge needs another fresh contact-locus protocol. Latest verified residual remains 39.

### Remaining original boundary protocols prepared

Added `OriginalBoundaryContact` with fresh paired-plane fibers and ruled Jordan projection, after complete unchanged-region comparison. Both attempts share cumulative exact/driver work; the audit preserves old proofs and actual owner pointers. Three paired-fiber regressions pass: straight neighbor rails, sphere join both orders with budget refusal, and coincident curved-face refusal. Ruled projection coordinate/weight identity regression also passes. Full fixture adds zero-work, invalid-tolerance, exhausted-driver and prior-proof checks. This new phase is absent from current profile-hull-inclusive compiled run (session 90601); no [10,12] contact result is claimed yet.

For the 24 pending nonadjacent original-transition pairs, a potential next proof is strict XY control-hull separation: the owned ProfileProjectedRegion certifies the complete transition projection equals the clipped exact profile image. Only a fresh vertical-plane separator of the actual complete original/profile XY hulls could transfer that containment. Arbitrary 3D source hull planes do not bound the corrected transition. This is a proposed proof path, not implemented authority.

### Verified conservative profile hull pass: unchanged 39 pending

Full native fixture passed in 258.09 s. Full-profile control-hull checks preserved all proofs and respected cumulative work (6,045,574 exact operations). They closed no pending pairs: these original-retained support hulls still have nonstrict sides. Saved `contact-audit-profile-hull-39.json`, SHA256 3a29b525307868c1ec15b8cf7b47c80f01d06574007d2ab069c090b0ea02b144. This validates refusal/accounting for this fixture, not positive-case success or complete profile contact qualification. Next full run adds explicit positive-case tests on otherwise separated original/profile pairs and the [10,12] boundary protocols.

Boundary-inclusive full native validation now runs in session 19272, log `/tmp/profile-boundary-contact-native.log`, requested artifact `/tmp/profile-boundary-contact-audit.json`. It also requires an actual positive profile-hull certificate in both owner orders on this fixture; the pending-only profile pass has only established conservative refusal so far. Latest verified residual remains 39.

### Exact XY hull separator primitive verified

Added `source_hull_separation::certify_xy`. XY control projections propose a plane only; exact orientation in XY must prove its normal-Z component is zero, then every actual original 3D control is freshly checked against the same strict side. All projected and original checks share exact work. Returned native certificate uses the real original regions and no material-hull substitution. Four hull regressions pass, including a new isolated exact-plane test: separated XY hulls with interleaved large Z values qualify in both orders; same or touching XY projections refuse even when Z hulls are disjoint; exhausted work refuses. As with the existing midpoint test, synthetic points test the private plane arithmetic only; public authority derives all points from actual original regions. Integration with the owned ProfileProjectedRegion and transition face remains the next step. Boundary-inclusive full fixture is still running in session 19272; latest verified pending count remains 39.

### Verified fresh boundary contact: 38 pending pairs

Full native fixture passed in 422.95 s. Fresh boundary protocols closed the last original-original pair while preserving prior proofs and actual face owners. Remaining 38 modified-face contacts are unresolved; global embedding and Body admission remain incomplete. Cumulative exact work is 11,782,396, driver cells 0. This run also required and passed an actual positive profile-hull certificate in both owner orders. Saved `contact-audit-boundary-38.json`, SHA256 1d7472d654ff70982dab4b17744e0bd624d54b7f0fb4d925a93005fb8f8d9e1c.

Transition XY hull integration is now under full native validation, log `/tmp/profile-transition-xy-native.log`, requested artifact `/tmp/profile-transition-xy-audit.json`. The new protocol requires the actual owned transition face and exact profile identity, positive profile factors, and fresh vertical raw-control separation. The fixture checks positive certificates in both orders, zero-work refusal, wrong face kinds, cumulative accounting and preservation of existing proofs. No reduction below 38 is claimed before that run passes.

### Clipped exact-profile XY bounds prepared

Added a distinct native `TransitionXyBoundsContact`. Actual transition/profile pointer ownership and unchanged original surface controls are required. Fresh `WeightProfile::enclose(region.source_domain())` supplies outward bounds of the exact clipped profile; owned planar-image equivalence transfers those XY bounds to the complete transition, without claiming its Z lies in the source hull. Positive finite original weights justify raw control coordinate bounds. Only a strict finite coordinate gap admits separation; touching, overlapping, invalid or exhausted intervals refuse. A resumed pending-only audit charges span work cumulatively and preserves prior proofs. All 16 assembled-shell regression tests pass, including strict-bound arithmetic refusal; fixture additions for real owners, reverse order, spent-budget no-op and proof preservation compile. These additions are absent from the still-live hull-inclusive session 8979, so no actual clipped-bound positive certificate or pending reduction is claimed yet. Latest verified full fixture remains 38 pending.

### Full XY hull positive-case expectation failed

The hull-inclusive native run ended with exit 101 after 248.41 s at the assertion requiring at least one positive full XY hull separation. No positive certificate was found; its preceding boundary phase still reached 38 pending pairs. This failed run proves no new reduction. The bounds-inclusive binary containing the same assertion was explicitly cancelled (exit 130) before replaying that obsolete expectation. The fixture now expects full-hull refusal on this model and requires nonzero exact predicate work, with per-reason diagnostics, so owner rejection cannot masquerade as geometric refusal. All 16 assembled-shell regression tests and two exact profile-geometry enclosure tests pass. A fresh full run uses `/tmp/profile-transition-xy-bounds-rerun.log`, with separate full-hull and clipped-bounds audit artifacts; both actual fixture refusal accounting and clipped-bound positive cases remain awaiting that result.

### Corrected transition expression ownership

Source inspection found the actual reason that the previous positive XY expectation failed: the expression assembly borrows `face`, whereas the image certificate borrows a separately admitted `xy_face` with a different proof projection. Both borrow the identical immutable `TransitionBoundary`, which defines their geometry; `TransitionFace` adds only the proof projection. Requiring face-object pointer equality incorrectly refused this same expression before any geometry query. Both new protocols now require exact boundary pointer identity plus exact profile pointer identity, matching the existing qualified section/ruled protocols. Consequently the earlier statement that this fixture's complete XY hulls necessarily overlap is unproven and withdrawn. The test requires nonzero fresh exact predicate work; full-hull success/refusal must be measured after ownership succeeds. All 16 assembled-shell regressions pass. The obsolete run was explicitly cancelled (130), and the corrected full native run writes `/tmp/profile-transition-xy-owned-native.log`, `/tmp/profile-transition-xy-owned-audit.json`, and `/tmp/profile-transition-xy-owned-bounds-audit.json`. Latest verified residual remains 38 until this run finishes.

The fixture now explicitly checks that its assembly face and projected-region face are different objects with different admitted projections, while their immutable boundary pointers agree. This preserves the actual regression precondition for the corrected expression-owner guard. The test source compiles and all 16 assembled-shell tests pass; these extra precondition assertions are not in the already running corrected full binary. Session 5561 remains live, so no new residual count or positive XY body certificate is asserted yet.

### Verified transition XY protocols: 25 pending pairs

Full native fixture passed in 260.29 s. Correct immutable expression ownership reaches fresh geometric predicates and an actual positive full XY hull certificate in both orders. Full hull separation closes 10 original-transition pairs (38 to 28), then exact clipped profile coordinate bounds close 3 more (28 to 25). Existing proofs and actual shell-face owners are preserved; reversed bound cases qualify. Remaining groups are 14 original-retained and 11 original-transition, all unqualified rather than identified defects. Exact work 11,908,922; spans 89,463; driver cells 0, within cumulative limits. Global embedding, radius/thickness and Body admission remain incomplete.

Saved `contact-audit-transition-xy-28.json`, SHA256 663542eb33bfdcecce468f93473378b43ba58bcc3c17a6ca27ce6d3471530e87.

Saved `contact-audit-transition-xy-bounds-25.json`, SHA256 7dbe2b9e52c706b0c224e3d2f544aae91f6cc8fcdb01b0313c5d1b1f79004a0e.

### Fresh original planar material bounds prepared

Extracted complete original-face equivalence from the already qualified pair protocol, preserving exact original loops and owned full-arc subdivisions. New optional `qualify_transition_xy_material_bounds_contact` first tries the clipped-profile/raw-control bounds, then freshly qualifies the unchanged original planar material hull after full contour identity. Only the original face receives this substitution; the transition remains bounded by its owned exact clipped-profile image. Certificates retain the native material-hull proof. Span and exact work are shared cumulatively across pending pairs; spent exact work skips all queries and leaves diagnostics unchanged. All 16 assembled-shell regressions pass, including full-partition gap/reversal/changed-curve refusals. The full fixture includes real pointer owners, retained prior proofs, bounded work, native material certificates and reversed owner cases. Native validation writes `/tmp/profile-transition-xy-material-native.log` and `/tmp/profile-transition-xy-material-audit.json`. Latest verified residual is 25; no material-bound reduction is claimed yet. No new WASM/UI admission is packaged.

The source planar-material regression passes on this checkout: distant top material separation qualifies in both owner orders (10,178 exact operations), exhausted work and neighboring-contact separation refuse. New transition material diagnostics preserve the native material-hull refusal reason rather than reporting every unsupported hull as coordinate overlap. All 16 assembled-shell tests pass after this diagnostic-only change. The already-live full material run predates this reason refinement; its geometric algorithm and accounting are unchanged. Latest fully verified residual remains 25.

### Verified original material bounds: 17 pending pairs

Full native fixture passed in 273.08 s. Fresh original planar material bounds closed 8 more original-transition pairs (25 to 17), preserving every prior proof and actual owner pointers. Accepted new cases retain material-hull certificates, prove strict coordinate gaps and qualify in reversed owner order. Remaining groups: 14 original-retained, 3 original-transition ([4,28], [6,28], [9,28]). Cumulative exact work 11,938,576, spans 89,474, driver cells 0; all limits respected. Saved `contact-audit-transition-material-17.json`, SHA256 47eac38ac3f9cc8438ac7518aee2a46553820766db059a1026e9ae218a8cc493. The compiled run predates diagnostic-only native refusal reason refinement; its geometric algorithm and work accounting match current code. Global embedding and complete Body admission remain incomplete.

### Focused full-3D support refinement prepared

A temporary native inspection identified remaining original-transition source faces: 4 and 9 are planar bottoms at exact Z=0; face 6 is planar top at exact Z=6. Fresh original material-hull qualification succeeds for all three. This inspection diagnoses bounding limits only, not separation. Added `resume_transition_contact_audit`, preserving the existing complete pending product frontier and rigorous actual expression enclosures. It spends remaining cumulative support work only on pending original-transition pairs, retaining other pairs and every qualified proof. The fixture expands only cumulative cell capacity to 16,384 and requests 256 extra cells per candidate; span/domain/exact/driver limits stay at their previous caps. All 16 assembled-shell regression tests pass. Full native validation logs `/tmp/profile-transition-support-native.log`, with artifact `/tmp/profile-transition-support-audit.json`. Latest verified residual is 17; no 3D refinement result claimed yet.

### Exact retained-profile vertex protocol prepared

Added `ProfileVertexContact`: fresh original single-vertex contact, complete unchanged-neighbor contours, exact selected source/control identity and positive finite exact profile factors. The old selected material hull is forbidden. A matching natural corner must actually remain on the retained contour and match the original vertex's source carrier. A fresh native single-Bezier corner-plane proof confines the whole positive profile's plane image to that owned world point: exact control signs, positive Bernstein bases and collapsed zero edges do not depend on rounded replacement weights. Certificate retains both native proofs and actual shell-face owners. Resumed candidate-only audit shares exact work and preserves previous proofs; fixture adds invalid/zero-work refusals, owner pointers, reverse cases and cumulative bounds. All 16 assembled-shell tests plus two corner-plane adversarial tests pass. This new phase is absent from the currently running 3D-support binary (session 65093); no positive retained-profile vertex certificate is claimed before full validation. Latest verified residual remains 17.

### Verified focused support refinement: unchanged 17

Full fixture passed in 253.10 s. Targeted full-3D refinement preserved all qualified proofs and unchanged nontransition diagnostics, but closed no further pair. It consumed the full cumulative span limit (100,000); cells 8,430 of 16,384 and domain cells 41,331 of 100,000. The three transition pairs remain unqualified; more cells alone cannot improve this exhausted-span audit. Saved `contact-audit-transition-support-17.json`, SHA256 797a11b8ca773d649e7dc0d1db6aa471030885086b2546fcedad907c805d4ea9.

Retained-profile vertex-inclusive full validation now logs `/tmp/profile-vertex-native.log` and requests `/tmp/profile-vertex-audit.json`; latest verified residual remains 17.

### Direct complete transition coordinate cover prepared

Source review confirms generic support subdivision already normalizes parameter widths, but still spends work subdividing the unchanged planar chart, whose constant Z enclosure cannot improve. Added distinct `TransitionCoordinateContact`: exact equal finite original controls and positive source weights place the entire unchanged support in a constant-coordinate plane. Actual transition expression intervals alone are adaptively covered over its whole natural rectangle. Every accepted closed leaf interval must strictly miss that plane; missing work retains the complete remaining frontier, touching/crossing stays unproven. Native certificate retains actual face owners, axis/value and complete admitted leaf cover. Cumulative pending-only integration charges cells/spans and preserves previous proofs. Seventeen assembled-shell tests pass, including a real interval crossing, touching, exhausted frontier and over-budget query refusal. Fixture additions compile and check actual owners, strict leaf bounds, reversed cases, spent-work no-op and cumulative limits. This phase is absent from current vertex-inclusive binary (session 94184), so no actual coordinate contact certificate is claimed yet. Latest verified residual remains 17.

### Verified retained-profile vertex contacts: 13 pending

Full fixture passed in 252.59 s. The fresh positive-profile single-point protocol qualified four original-retained pairs (17 to 13), preserving all prior proofs and actual shell owners. Source/image points agree, reversed cases qualify, exhausted work leaves diagnostics unchanged. Remaining groups: 10 original-retained and 3 original-transition. Cumulative exact work 12,174,378; spans 100,000 after optional generic 3D refinement. Saved `contact-audit-profile-vertex-13.json`, SHA256 c5038f0a8dbb1d1ffc864e43ac6a57c35dda1a7ba12bad656a601a269293f6ed. Neither global Body embedding nor all original-retained contact loci are qualified.

Coordinate-inclusive full validation now logs `/tmp/profile-coordinate-native.log`, with artifacts `/tmp/profile-coordinate-audit.json` and `/tmp/profile-coordinate-vertex-audit.json`. Coordinate separation is attempted before the span-exhausting generic support refinement. Latest verified residual remains 13.

### Removed vertex locus exclusion prepared

Extended the fresh profile vertex protocol with a distinct `profile-vertex-excluded` outcome. If the original owned corner is absent from the retained contour, a fresh admitted corner-plane certificate is required first. Exact control-plane signs then enumerate all possible natural zero corners and fully collapsed zero edges of the positive single-Bezier chart. Exclusion requires none of these closed loci to intersect the actual containing retained support rectangle; chart interior and partially nonzero natural edge interiors have strict signs. Positive exact factorized weights preserve that classification. Certificate stores the actual excluded rectangle; no original geometric report is inherited. Eighteen assembled-shell tests pass, including exhausted exact work, closed trim boundaries and a collapsed edge intersecting the trim despite both natural corners being outside. Full fixture additions compile and compare excluded domain to actual retained support, preserve owner/reverse checks and distinguish exclusion diagnostics. This phase is absent from current coordinate-inclusive binary (session 32103); no new excluded body pair is claimed before full validation. Latest verified residual remains 13.

### Verified coordinate pass: conservative refusal

Full coordinate-inclusive native fixture passed in 251.37 s. Direct coordinate cover closes no transition pairs; it reaches cumulative spans 100,000 at cells 8,415. All three transition coordinate covers remain incomplete. No positive coordinate certificate is established on this body, and no defect is inferred from interval overlap. The following profile vertex phase again reaches 13 pending, with unchanged qualified source/image contacts. Saved coordinate-stage `contact-audit-coordinate-17.json`, SHA256 cddb580e4a70d02813393ac02ec28f4515234228f2405a9aa4faae23eeb5ddfe. The current removed-corner-inclusive binary logs `/tmp/profile-corner-exclusion-native.log`, artifact `/tmp/profile-corner-exclusion-audit.json`; no new exclusion result claimed yet.


### Verified removed corner exclusion: 11 pending pairs

Full native fixture passed in 251.34 s. Fresh positive-profile corner-locus exclusion closed two more pairs, [0,23] and [6,27], reducing 13 to 11. Closed trim boundaries and collapsed zero edges remain conservative refusal cases. Prior proofs, reversed owners and cumulative budgets passed. Remaining groups: 8 original-retained and 3 original-transition. Cumulative exact work 12,191,096; spans 100,000; cells 8,415. Saved `contact-audit-profile-corner-exclusion-11.json`, SHA256 530c7aad200b4813b7de992f0b593000ca661a15cf79f729f60c7d62570e2968. This verifies the control fixture only; complete Body admission, WASM packaging and UI acceptance remain incomplete.

### Anchored Hermite enclosure prepared

The position enclosure now intersects the previous direct interval with two algebraically equivalent anchored forms using h00+h01=1. Monotonic h01 on [0,1] gives outward endpoint bounds; no source geometry substitution or sampling authority is used. Four profile-section bridge regressions and 18 assembled-shell regressions pass. Full control-fixture qualification is running; no further contact reduction is claimed.


### Natural profile boundary identity prepared

Added a typed native certificate retaining the actual WeightProfile owner and chosen clamped natural boundary. Fresh exact dyadic determinants prove row[i]*column[j] = pivot*source_weight for every boundary control, without flattening products or comparing rounded ratios. Source controls and knots are unchanged, so this proves pointwise curve identity on every trimmed subinterval of that boundary. It does not authorize cross-face contact, corner ownership or Body admission; those require subsequent native gates. Regression passes for both chart axes, rejects a one-ulp weight residual, exhausted work and invalid axis. The tighter Hermite full fixture is still running; latest complete verified contact count remains 11.


### Positive profile boundary/pole plane transfer prepared

The new typed PlaneImage combines fresh exact boundary weight identity with a fresh source boundary/pole plane-image certificate. Positive profile factors preserve control-plane signs and collapsed pole world points; the unchanged boundary identity preserves the full remaining plane locus. Cross-face retained ownership is still separate and no pair is reclassified by this preparation. The actual annular profile passes all four natural boundary identities (384 or 768 exact operations), and transfers the source face-0/face-1 plane image with matching poles, side and actual profile owner. Source face-0/face-5 cap contact remains explicitly unqualified. Synthetic regression retains one-ulp, zero budget and all-zero plane refusals; actual annular regression passes in 0.20 s.

The full anchored Hermite run stopped after 202.49 s at the frozen initial count: actual pending 150 versus old expected 151. The exact initial fixture expectation is updated to 150 pending and 163 excluded, preserving all other proof counts, owner and budget gates. A complete requalification is running; the final residual and identity of the newly excluded pair are not yet verified. Latest completed full audit remains the 11-pair report.


### Verified anchored initial audit stage

The repeated full fixture passed the exact initial type-count gate [150,163,12,23,54,1,1,2,0,0,0,0,0,0,0,0], including every actual owner pointer, codec roundtrip and cumulative budget. Comparison with the saved 151-pending initial report identifies exactly one changed pair: [9,28], pending to expression-contact-products-excluded, with two rigorously separated product cells. Saved `contact-audit-anchored-base-150.json`, SHA256 ca091e1aaee2ffcf5cd49be7e3064780e8859e784b71b73df01a592cf09d4678. This is intermediate-stage evidence; full repeated fixture is still running and the final residual is not yet qualified.


### Verified anchored Hermite full audit: 9 pending pairs

Complete native control fixture passed in 250.59 s. Anchored interval bounds closed bottom-original/transition pairs [4,28] and [9,28], reducing the final residual from 11 to 9. Remaining groups: 8 original-retained and 1 original-transition [6,28]. Prior proof preservation, actual owner pointers, reversed cases, codec restoration and every cumulative budget gate pass. Cumulative cells 7,949; spans 84,868; exact work 12,058,349. Saved `contact-audit-anchored-final-9.json`, SHA256 7d30ba8d4339fb30e76579460362ed557e5249dcafffa8cdb0c28378dc2adcbb. This qualifies the recorded fixture only. Remaining retained contacts and upper transition contact, complete Body admission, radius/wall qualification, WASM/UI delivery and full part acceptance remain incomplete.


### Current retained boundary ownership protocol prepared

Added `qualify_profile_boundary_contact`: fresh original boundary/pole plane contact, exact positive-profile boundary transfer, complete unchanged neighbor contour, and actual current shared addresses. Every retained natural-fiber fragment must be an owned current shared use. Potential endpoints require an adjacent entire owned fiber fragment on the same qualified closed contour; unresolved loci refuse. Included collapsed poles require an actual natural corner of a current shared fragment; only poles outside the full conservative retained rectangle can be excluded. Current Outer pairs retain their assembly-time root-ended canonical edge authority. Exact work and driver limits are cumulative within the report. All 19 assembled-shell regressions pass, including missing ownership, endpoint-only and unresolved-locus refusals. Full fixture now checks [0,1] and [1,27], actual pointers, reverse owners and exhausted budgets; native run is in progress. No new contact is reclassified until full validation; latest completed residual remains 9.


### Cumulative retained boundary audit integration prepared

Added a separate ProfileBoundary pair-proof kind and candidate-only cumulative pass. It first validates actual audit owners and every cumulative limit through the existing continuation gate, fairly divides remaining exact and driver work, and skips all queries when either budget is spent. Existing nonpending proofs are retained. The full fixture checks spent-budget diagnostics equality, prior-proof preservation, actual owner pointers, current shared-address membership and reversed positive cases. The 17-type initial count remains exactly fixed, with the new kind zero before this pass. All 19 assembled-shell regressions pass after integration. The already-live direct ownership fixture predates this cumulative phase; it remains running, so neither direct positive ownership nor final cumulative reduction is claimed yet. Latest completed residual remains 9.


### Verified direct retained boundary contacts

Complete native ownership fixture passed in 246.21 s. Pairs [0,1] and [1,27] qualify in both owner orders, including actual pointer owners, current root-ended shared edges and native plane-image identity. Each direct qualification uses 31,246 exact operations and one driver cell. This run predates cumulative ProfileBoundary classification, so its final residual remains 9; no lower cumulative count is claimed.

### Fresh neighbor-only material hull prepared

Added `certify_shell_with_raw_side`: retain the designated full positive control hull and freshly qualify material only on the other original side. Profile hull contact validates complete unchanged neighbor contours, keeps the modified selected side raw, and can now use this fresh neighbor material certificate. Four native hull regressions and 19 assembled-shell regressions pass. The fixture guard now requires the retained side material certificate to be absent while allowing a fresh unchanged-neighbor certificate. A full cumulative run is active; its compiled fixture predates this guard update and may stop at the obsolete all-material-certificates-absent expectation if a new positive material case occurs. It will not be called complete on partial output. Latest fully verified residual remains 9.


### Verified neighbor-only material separation on residual source pairs

The native annular regression passes in 0.26 s with all six exact positive cases: [0,11], [11,0], [0,15], [15,0], [0,19], [19,0]. Each certificate has no material substitution on the designated raw selected side, a fresh original planar-material certificate on the neighbor, opposite strict signs and matching source controls. Work ranges 9,158 to 14,054 operations. A one-operation budget refuses and invalid raw-side index errors. Exact positive-case coverage is frozen in the regression, rather than accepting a zero-positive result. The complete cumulative expression fixture remains active and predates the updated retained-only material guard; full residual reduction is not yet qualified. Latest completed full residual remains 9.


### Retained profile / ruled-wall image protocol prepared

Added a typed image-only protocol for actual retained/original owners with an existing Outer edge. It freshly qualifies full exact positive-profile XY Jordan projection and whole unchanged ruled-wall projection, then checks the natural boundary curve identity with exact homogeneous weight determinants (including reversed control order). Complete unchanged neighbor contours and actual current outer-edge ownership are required. The certificate retains both native proofs and owners; it does not yet classify cross-face contact or grant retained root/pole ownership. All 19 assembled-shell regressions pass. The full fixture now requests the image proof for [0,2] and [2,27] in both owner orders with zero-work/invalid-pair refusals.

The previous cumulative run terminated after 364.04 s at the known obsolete all-material-certificates-absent guard. It produced a fresh permitted neighbor-material proof, but no final cumulative report, so no new final count is claimed. A new full run uses the retained-side-only material guard, cumulative boundary phase and new ruled-image assertions. Latest fully qualified final residual remains 9.


### Owned retained ruled-boundary contact prepared

Added `qualify_profile_ruled_contact`: fresh exact full positive-profile Jordan image, complete ruled-wall trace, exact natural-boundary source weight identity, current retained contour classification and actual shared source addresses. Possible endpoints require adjacent entire owned natural fiber fragments; unresolved loci refuse. Every collapsed projected boundary is rechecked for an exactly constant full 3D source control row/column; a vertical world curve collapsed only in XY is refused. Included world poles require current shared-fragment natural-corner ownership; conservative retained-domain exclusion is allowed only outside that entire rectangle. The certificate retains projection, exact boundary identity and current shared addresses. All 19 assembled-shell regressions pass after the full fixture adds [0,2]/[2,27], reversed owners, actual pointers and exhausted work/driver cases. The already-running cumulative fixture predates these direct contact assertions; it remains live. No ruled-contact classification or new full residual is claimed before the next full qualification. Latest complete residual remains 9.


### Cumulative ruled-profile contact integration prepared

Added a distinct ProfileRuled audit kind and pending original/retained-only pass. Existing owner and cumulative-limit continuation checks run before any queries. Remaining exact work, driver cells and domain cells are fairly divided; projected Jordan boundary work is charged to domain cells and zero controls or any spent budget skip queries. Existing qualified proofs are retained. Full fixture now checks spent-budget diagnostics equality, owner pointers, exact profile identity, shared current addresses and all cumulative counters; initial exact type counts include zero new ruled proofs. All 19 assembled-shell regressions pass. Current live cumulative run predates this ruled phase and direct ruled ownership assertions; no new final count or direct ruled positive claim is made before full qualification. Latest complete residual remains 9.


### Verified cumulative neighbor hull and boundary audit: 4 pending pairs

Full native control fixture passed in 273.41 s. Fresh neighbor-only material hull separation closes [0,11], [0,15] and [0,19]; current retained boundary ownership closes [0,1] and [1,27]. Final residual decreases from 9 to 4: [0,2], [0,5], [2,27], [6,28]. Cumulative exact work 12,199,519; spans 84,868; driver cells 2. Owner pointers, reversed positive cases, current shared addresses, retained-side no-material guard, prior-proof preservation, spent-budget diagnostics and cumulative caps pass. Saved `contact-audit-profile-boundary-final-4.json`, SHA256 4d01f15a776d34d2c41a34cccb9eec8938a4e7e7eb7abad3cb279b1ca43e5df1.

The same full run also verifies fresh ruled-wall image certificates for [0,2], [2,0], [2,27] and [27,2], each using 1,459,874 exact operations. It predates the direct ruled ownership assertions and cumulative ruled-contact phase, so these pairs remain pending in the verified report. A complete current-code run is active to qualify those remaining prepared phases; no reduction below 4 is claimed. Full Body admission, upper transition/cap contacts, radius/wall checks and WASM/UI/part acceptance remain incomplete.


### Residual [0,5] is an adjacent curved patch, not a planar cap

A temporary source inspection (removed after diagnosis) identified face 5 as a degree-[2,2] rational curved support, with no exact ruled XY trace on either axis. Replaced that diagnostic with a native regression: `source_paired_fiber_contact` qualifies [0,5] and [5,0], each using 13,263 exact operations and no driver cells. Both fibers use the same exact plane with opposite strict sides and retain actual shared edges. Fresh positive-profile plane-image transfer on selected face 0 also qualifies, preserving the actual profile owner, exact natural boundary, plane and side; no extra pole image occurs. A one-operation native budget refuses. The final regression passes in 0.15 s. This establishes the native transfer ingredients; actual retained current-edge ownership and cumulative reclassification remain to be wired and fully qualified. Current full ruled audit remains live; latest completed residual is 4.


### Verified complete ruled-profile audit: 2 pending pairs

Full native control fixture passed in 260.89 s. Current retained ruled ownership closes [0,2] and [2,27] in both owner orders; each direct certificate uses 1,460,642 exact operations and one driver cell. Native Jordan image, exact boundary identity, current root-ended shared addresses and included world-pole ownership pass. Cumulative ruled phase preserves every prior proof, spent-budget diagnostics and actual owners. Final residual decreases from 4 to 2: [0,5] and [6,28]. Cumulative exact work 15,120,803; spans 84,868; driver cells 4. Saved `contact-audit-profile-ruled-final-2.json`, SHA256 3305002c62f0b59b2553ea09ee61ba25b3ffc6ba291df04731d86d176def4c1f. This remains fixture qualification, not full Body admission.

### Current ownership and cumulative paired-fiber protocol prepared

Added a distinct ProfilePaired certificate and pending-only cumulative pass. Fresh original opposite plane fibers, exact positive-profile boundary transfer and complete unchanged neighbor contours are linked to current Original shared-edge pairs. Both actual source-use definitions must match in native edge order, every source contact edge must remain covered, and full canonical source coverage is required. Extra pole images or unsupported root-ended partial covers refuse. The current fixture adds [0,5]/[5,0], invalid/zero work, actual owners/current addresses, prior-proof preservation and spent cumulative budget gates. All 19 assembled-shell regressions pass. Full current-code qualification is running; no reduction below 2 is claimed until it completes. Full body/radius/wall/UI/STEP acceptance remains incomplete.


### Verified paired-profile audit: 1 pending pair

Full native control fixture passed in 242.84 s. Current opposite-plane-fiber ownership closes [0,5], including reversed owners, ordered source-use identity, complete canonical edge cover and exact positive-profile transfer. Final residual decreases from 2 to 1: [6,28]. Cumulative exact work 15,139,582; spans 84,868; driver cells 4. Prior proof preservation, spent-budget diagnostics and all caps pass. Saved `contact-audit-profile-paired-final-1.json`, SHA256 95395f96af868e3a7a78fa7d3306f1f49c4c0ebb39e9ce4b049144089bda984e. Full self/radius/wall/Body qualification and product delivery remain incomplete.

### Diagonal transition/material XY separation prepared

Added a separate typed vertical-plane certificate: actual immutable transition/profile owners, complete original-contour identity, fresh original planar material hull and outward clipped positive-profile XY bounds. Approximate projected ranges only propose planes; exact nondegeneracy and strict dyadic signs of every native material control and every enclosing rectangle corner provide authority. The plane is exactly vertical because its first and third points share identical XY coordinates. Affine convexity extends corner/control signs over the complete owned images. No source Z substitution or sample authority occurs. All 20 assembled-shell regressions pass, including overlapping coordinate boxes that separate diagonally, true projected contact, invalid intervals and spent work. Full fixture checks [6,28] and reverse order with actual owners and zero work/span cases; native qualification is running. No reduction below 1 is claimed yet.


### Cumulative diagonal XY plane audit integration prepared

Added a distinct TransitionXyPlane proof kind and pending original/transition-only pass. Actual audit owner identity and cumulative limits are validated before any queries. Remaining exact work and spans are fairly allocated; either spent budget skips all queries and retains diagnostics. Native material controls and the profile enclosure remain independently bounded. The full fixture checks separate exact-work/span exhaustion, prior-proof preservation, actual owners, clipped-domain equality, exact vertical-plane structure and reversed qualification. Initial exact type counts include zero diagonal proofs. All 20 assembled-shell regressions pass. The currently running direct diagonal fixture predates this cumulative phase and remains live; no new final contact count is claimed. Latest fully verified residual remains one pair [6,28].

### Verified direct diagonal transition/material separation

The full native fixture with direct diagonal assertions passed in 243.24 s. Both [6,28] and [28,6] qualify with 8,596 exact operations and one enclosure span each. Actual immutable owners, clipped profile domain, exactly vertical plane and zero-work/span refusal checks pass. This binary predates the cumulative diagonal phase, so its final audit still reports one pending pair. A current-code cumulative run is required before reporting zero pending cross-face pairs. Single-face regularity/injectivity, volume, radius, wall thickness, full Body admission and WASM/UI delivery remain separate requirements.

### Verified complete cross-face contact audit: zero pending pairs

Full native control fixture passed in 238.92 s. The cumulative diagonal phase closes the last pair [6,28]. All 406 unordered pairs of the 29 expression faces now have native contact/exclusion proofs; zero remain pending. Exact work is 15,148,178; enclosure spans 84,869; domain cells 38,978; driver cells 4. Prior-proof preservation, actual immutable owners, reversed direct separation and spent exact-work/span refusal checks pass. Saved `contact-audit-transition-xy-plane-final-0.json`, SHA256 99d4093f0ec7c8792acb109330066b0b503edb3741cfc50c6fb53e9964e399f9.

This run predates the exhaustive pair-coverage method and joint embedding gate. Those additions compile and all 21 assembled-shell regressions pass; a full current-code run is active to verify the gate on this fixture. The gate requires the actual same shell owners, complete distinct pair coverage and unchanged complete original contours; admitted retained/transition charts and manifold incidence come from the immutable assembly. Volume/material orientation, radius/tangency/thickness qualification, general fillets, WASM/UI and part acceptance remain incomplete. Zero cross-face residual on this fixture is not universal geometry qualification.

### Verified joined expression embedding gate

The full native fixture passed in 279.10 s with exhaustive distinct-pair coverage and the joint embedding gate. Incomplete contacts refuse; final complete contacts admit geometry tied to the actual expression shell and audit pointers. This verifies the control shell's combined chart/incidence/contact admission, before volume and material orientation. Native support first-derivative and flux wrappers plus conservative trim-mask flux are prepared; all 21 assembled-shell regressions compile and pass, but those focused tests do not execute the new fixture assertions. A full current-code run is active to check direct profile derivative identity, support flux equality, work accounting and zero-classification masking against classified bounds. Volume integration and solid Body admission remain incomplete.

### Adaptive expression volume integration prepared; mask rounding corrected

Added bounded largest-uncertainty-first flux subdivision on admitted expression geometry. Every initial face must be enclosed before any total is reported; unsuccessful subdivision retains the complete previous total. Exact chart domains, directed material flux, cumulative cells/spans/classification work and outward parent sums are preserved. Reports distinguish requested interval accuracy from unresolved volume sign and do not construct a Body.

The first full material-flux fixture failed after 139.14 s at mask/full-flux nesting. Mask area used a different outward multiplication order from the full flux area. Both now use the same outward arithmetic, and a regression checks containment for positive, negative and sign-changing densities on three parameter domains. Probe rectangles now lie within each retained face's owned support domain. All 22 assembled-shell regressions pass. A new full native run is active with derivative/flux identity, material masking, zero-volume-work refusal and a 512-cell adaptive volume inspection. No actual volume interval or new material-flux full-pass claim is made before it finishes.

### First complete volume enclosure run; precision still insufficient

Full native fixture passed in 285.11 s. All 406 cross-face pairs remain qualified. The direct derivative/flux identities and mask nesting pass. Adaptive volume inspection returns signed bounds [1616.6173597548361, 14790.440285867677] mm3, incomplete, at 511 cells, 2,153 spans and 10,000 classification cells. Requested width 20 mm3 is not achieved. The reported uncertain support is original face 23 on [[5,10],[2.4999999999999987,4.999999999999999]]. Saved `expression-volume-initial-512.json`, SHA256 3c24a735e36888a6725ffa673ecddc5d6649310ea5360a90379ed331adfd5351. This binary predates monotone parent intersections and highest-uncertainty budget-stop diagnostics.

Prepared fresh native planar Green flux transfer only after complete unchanged original-contour identity. It uses the original admitted geometry and actual directed world carriers, charges native work to cumulative cells/spans and never transfers to retained or transition faces. All 23 assembled-shell regressions pass. A full current-code run is active to qualify this transfer together with monotone volume refinement and corrected stop diagnostics. No tighter volume result or Body admission is claimed yet.

### Planar-transfer budget starvation observed and corrected

Full fixture passed in 267.77 s, but volume initialization used all 512 cells before face 17 was enclosed. It correctly returned no total bounds: partial face coverage cannot certify volume. Used 531 spans and 10 classification cells. Saved `expression-volume-planar-starved-512.json`, SHA256 d0bf4bf1f88d9257ada9564389360f4192250864efb82c77ef34ee4af5cac65f. Planar work now receives a fair remaining-face share with a reserved enclosure cell per remaining face. A transferred planar enclosure is fixed only after meeting its allocated error share; otherwise it remains eligible for subdivision. Also prepared exact original Taylor flux and unchanged whole-chart material transfer with direct native comparison assertions. All 23 assembled-shell regressions pass. A full run with 8192 cells, unchanged 20 mm3 requested width and unchanged span/classification caps is active. No adequate volume precision or solid admission is claimed.

### Verified fair-budget and original-flux volume enclosure

Full fixture passed in 273.71 s. Direct unchanged original Taylor-flux comparison, zero classification work for a whole material chart, exact profile derivative/flux identities, masked material nesting, exhaustive cross-face contacts and joined geometry pass. Volume initialization covers every face. Twelve original planar faces meet their allocated error share. Signed volume interval is [7039.191767844694,7068.556863369501] mm3, width about 29.3651 mm3; the requested 20 mm3 width remains unachieved. Used 8191 cells, 49,685 spans and all 10,000 classification cells. Highest uncertainty is retained face 0 on [[0.46875,0.4921875],[0.296875,0.3125]]. Saved `expression-volume-fair-8192.json`, SHA256 721750082a9591f6dca5029cecfdde0aac1aef70c725ebd1b92df629e486aedd.

Prepared a separate classification cap per rectangle, preserving the conservative unknown-material mask when a local query exhausts work. The next run uses 32,768 cells, 200,000 spans, 100,000 total classification cells and 256 classification cells per rectangle, with the same 20 mm3 requested width. All 24 assembled-shell regressions pass. A full current-code run is active; no tighter result or Body admission is claimed yet.

### Verified expression volume reaches requested width

Full native fixture passed in 291.20 s. All cross-face contacts remain qualified. Adaptive oriented material flux encloses signed volume in [7049.390204343811,7069.387602073336] mm3, width 19.99739772952489 mm3, meeting the requested 20 mm3 width and excluding zero with positive sign. Used 6,317 cells, 51,303 spans and 70,043 classification cells; 12 planar faces meet their allocated error share. No uncertain face/domain remains. Saved `expression-volume-qualified-20.json`, SHA256 d5245f3b3c905991b508caab6ceb5840c52071fd7ac68994584286b9dbf4dd74.

The run predates the typed oriented-volume certificate. That gate recomputes native volume from the actual embedded expression and retains its immutable geometry owner; caller diagnostics cannot restore authority. Pending computation, absent bounds, zero-sign intervals and excess width refuse. All 25 assembled-shell regressions pass. A full current-code run is active to verify actual certificate issuance, geometry pointer identity, positive bounds and orientation. Radius, tangency, wall thickness, multiple-component material containment, solid representation/restore/export and WASM/UI delivery remain incomplete.

### Whole seams and collapsed-pole limitation

The strengthened fixture passed in 278.56 s, including mandatory owned volume certificate issuance for the complete nonzero interval. Whole-edge profile tangency remains unqualified for edges 0 and 1 because endpoint normals are unresolved.

The subsequent diagnostic run produced `expression-whole-seams-poles.json` before its full contact/volume stages finished. Exact changed-profile factor removal at the high U pole [20,0,6] gives squared sine [0.9999999999997039,1], proving distinct limiting planes. This pole cannot acquire a regular G1 certificate. The low U boundary has no admitted collapse-factor recipe; it is not labelled smooth. These diagnostics do not admit the new body or replace per-edge tangency proofs. The full diagnostic run remains pending.

That diagnostic run subsequently passed in 287.69 s. A separate focused test now covers the actual owned nonlinear pcurve correspondence with exact profile pole-factor normal bounds. Edge 0 qualifies its full canonical interval including one-sided limits in 3 cells / 30 spans. Edge 1 correctly refuses at 1,000 cells / 10,000 spans, preserving five pending intervals; refinement with 10,000 cells / 100,000 spans qualifies in 3,069 cells / 30,690 spans, with squared-sine upper bound 9.992575164150855e-7 <= 1e-6. The focused test passed in 6.05 s. This diagnostic angle coverage does not create a regular G1 certificate at the pole and does not prove transition-section joins, radius or thickness. The newer nonlinear-limit code still requires a full combined contact/volume run.

### Section joins and native expression radius checks

The subsequent combined section-join fixture passed in 278.76 s. Both exact corrected section boundaries have complete regularity coverage in 9 cells / 9 spans each. A private `TransitionSectionJoin` now binds fresh full coverage to its actual owned section; caller reports cannot construct it. All 24 profile regressions passed. A full run including the owned section gate is still active.

Native `profile_expression_radius` adaptively checks every point of the corrected expression against center and radius curves. Interval bounds give both a conservative maximum error and a proven minimum error; a mismatch records its UV rectangle and error interval. Work exhaustion preserves pending rectangles and never issues a certificate. Radius curves can use original parameter subdomains without rounded trimming. Ten radius regressions passed, and the independent rational-cylinder test with restricted law domains passed in 54.15 s. These fixture tests do not qualify the annular transition radius.

`TransitionFace::qualify_radius_on_domains` rebuilds its actual immutable recipe and only issues an owner-bound certificate after complete coverage. A prepared annular test matches the authored original parameter orientation, uses original law domains [0.75,0.9], requests 0.0001 mm tolerance, and exports pending areas or a proven violation through `BREP_EXPRESSION_RADIUS_PATH`. Its compilation passed; numerical qualification remains pending. Wall thickness, solid exchange and WASM/UI delivery remain incomplete.

The full owned-section gate fixture subsequently passed in 520.18 s. Actual certificate owner identity and refusal with zero recipe/regularity budgets passed for both sections; saved `expression-owned-section-joins.json`. This run predates the annular radius check.

The focused annular expression radius test passed as a diagnostic in 20.77 s: qualification remains incomplete after 1,000 cells / 34,034 spans, with 27 pending rectangles and no proven violation. A rigorous center-point probe can refute a radius law but cannot admit any rectangle; it found no violation here. This is not a radius certificate. Correlated or derivative-based residual bounds remain necessary to qualify the requested 0.0001 mm tolerance efficiently.

### Mean-value residual refinement

The native radius check now intersects direct squared-residual bounds with a whole-cell mean-value enclosure. Actual corrected-expression first derivatives and original rational law jets bound the residual gradient; source-parameter derivatives are scaled to normalized across coordinates. Discontinuous law/source charts cannot use this refinement. The original direct enclosure remains valid when a derivative query exhausts work. Eleven radius regressions pass, including nonunit law domains, positive/negative refusal cases and rational-cylinder full coverage.

The first annular mean-value run remained incomplete after 764 cells / 100,000 spans, with 12 pending rectangles and no proven violation; its diagnostic test passed in 53.23 s. An extended run is active with 10,000 cells / 1,000,000 spans at the same 0.0001 mm tolerance. It exports accepted-cell error bounds and every pending rectangle via `BREP_FOCUSED_EXPRESSION_RADIUS_PATH`. Radius qualification remains pending, and these changes are not yet packaged into WASM.

### Exact expression face clearance

`inspect_face_clearance` bounds the complete product of two exact face support domains. Native interval distance bounds accept a cell only when its lower bound meets the requested clearance. Subdivision preserves all remaining domains; work or resolution exhaustion returns no global lower bound. Trim supersets make this conservative.

`qualify_face_clearance` recomputes coverage and issues a private `ExpressionFaceClearance` tied to the immutable shell only after complete finite coverage. Caller reports cannot issue certificates. The combined annular fixture requests 13.5 mm between transition face 28 and inner face 3, checks zero-budget refusal and exports `BREP_EXPRESSION_GAP_PATH`. The current API and fixture compile, but the full fixture is still running; the subsequently added certificate-owner assertions require a fresh run. This is selected face separation, not proof of material wall thickness or all-wall coverage. No WASM/UI qualification is claimed.

The running combined fixture has now emitted the clearance result: 304 cells / 10,000 spans, six pending product domains, no global lower bound. Saved `expression-face-clearance-pending.json` preserves these domains and the 13.5 mm request. This is a work-limit refusal, not confirmation or refutation of the requested gap. The remaining combined contact/volume stages and extended radius run are still active.

The extended radius run subsequently finished in 638.14 s: 7,635 cells / 1,000,000 spans, 3,812 accepted cells, 11 pending rectangles and no proven violation. Saved `expression-radius-million-pending.json`; this diagnostic pass does not issue a radius certificate. Native subdivision now chooses the larger whole-cell mean-residual contribution when available, falling back to normalized width otherwise. This changes coverage ordering only, not admission bounds. Both focused radius regressions passed; a fresh annular comparison is running with the same tolerance and budgets. Face clearance now reuses its immutable constrained bridge across cells; compilation passed, numerical equivalence and timing remain to be checked.

Those 11 pending rectangles cover 0.9885711669921875 of the normalized parameter area: the unresolved region remains large despite the small rectangle count. `Report::pending_parameter_fraction` and the JSON export now expose this diagnostic area alongside the native domain; it is not physical surface area or a certificate. The combined pre-cache fixture subsequently passed in 611.65 s, retaining the incomplete radius and gap statuses. A fresh combined run now checks cached clearance and certificate owner/refusal assertions.

Initial radius refutation now probes a 3-by-3 interior grid using exact point enclosures and original law intervals. All work is charged, and a point can only refute the law; acceptance still requires whole-cell interval coverage. A regression uses a quadratic radius law equal to the cylindrical radius at the center but differing at quarter parameters, exercising an off-center violation that the previous single probe missed. The active annular comparison predates this additional probe set.

The strengthened focused regressions passed in 23.12 s. The mismatch is proven in the first cell at a degenerate point domain whose across coordinate differs from 0.5. Zero-work refusals retain parameter fraction 1; complete cylindrical coverage reports fraction 0. These tests qualify the diagnostic/refutation behavior, not the annular radius or all-wall thickness.

Clearance now caches successful exact support enclosures in two per-run maps, one for each fixed face, keyed by all four parameter endpoint bit patterns. Each map is capped at 8,192 entries; refusals are not cached. Native span work counts fresh queries, and `cacheHits` records reuse. Compilation passed. The active combined run predates this domain cache, so no timing, clearance completion or runtime equivalence claim is made for it yet.

The original-source material chord code cannot accept an expression shell. As a prerequisite for expression boundary/material analysis, native `ray_surface::finite_line_enclosure_parameters` now intersects outward parameter bounds from every position-box coordinate with the finite interval [0,1]. None proves exclusion; Some preserves possible contact without claiming a root or material. A focused test passed for forward/reverse crossings, a constant-coordinate miss, incompatible coordinate intervals, endpoint tangency and invalid zero direction. Expression integration, root isolation and outside-seeded material parity remain to be implemented.

`ExpressionShell::inspect_segment_supports` now adaptively covers every exact support face using that finite-line exclusion. Work exhaustion preserves all remaining face domains; unresolved contact cells retain their UV and line-parameter intervals. Only complete absence of candidates and pending cells reports boundary exclusion. Candidates are not isolated roots, and boundary exclusion is not material containment. The API compiled. Combined fixture assertions for a distant segment, zero work and invalid zero direction are prepared but not yet run; the active combined run predates this integration. Root isolation, trim classification and outside-seeded parity remain open.

The prior combined bridge-reuse run subsequently passed in 581.32 s, including gap certificate refusal and unchanged contact/volume qualifications. Its gap JSON is byte-identical to `expression-face-clearance-pending.json`: six pending domains, no certified lower bound. This establishes numerical equivalence for bridge reuse on this fixture, not performance or a complete clearance. A fresh combined run now includes domain caching and segment assertions, including retention of possible contact for [0,0,3]+t*[30,0,0].

The dominant-contribution radius comparison finished in 712.39 s. At the same 1,000,000-span budget it accepted 3,811 cells and retained 13 rectangles covering 0.9869785308837891 of parameter area, versus 0.9885711669921875 before. Saved `expression-radius-adaptive-million-pending.json`. This small coverage improvement does not resolve the interval dependency bottleneck or qualify the radius; wall thickness and material crossings remain unproven. The run predates the nine-point refutation probes and area export fields.

The exact authored annular source baseline `plane_cylinder_transition(20,6,0,1.25,0,pi/8)` qualifies its complete original support radius at 1e-6 mm in 0.03 s, with error upper bound 8.406836743732612e-7 mm and 10,236 coefficient operations. The test checks original surface and center ownership. This uses correlated homogeneous polynomial coefficients, unlike independent position/jet boxes. It does not transfer proof to the factored profile or corrected bridge; a complete exact coefficient representation of that expression is the next route to tighter radius bounds. Pole regularity and wall material remain separate.

Native private `bernstein_interpolation` now encloses Lagrange-to-Bernstein coefficients on exact binary64 nodes with outward arithmetic and bounded coefficient work. The caller must prove a polynomial degree; this helper cannot certify an arbitrary sampled expression. Two tests passed: a quadratic with known Bernstein coefficients and interval uncertainty/refusal cases, and an independently evaluated degree-eight polynomial using interval de Casteljau. Composition with the exact corrected bridge and its common denominator remains to be implemented and qualified.

The segment/domain-cache combined fixture passed in 370.80 s. A distant finite segment is excluded in exactly 29 face cells; the touching segment retains eight candidate contact cells and zero pending work domains after 404 spans. These are not isolated material crossings. Clearance reached its 1,000-cell limit after 1,368 fresh spans with 1,841 cache hits and nine pending product domains; saved `expression-face-clearance-cache-pending.json`. The certificate remains absent. A fresh run raises clearance budgets to 10,000 cells / 100,000 spans.

Polynomial reconstruction now supports original nonunit node domains without rounded normalization. Three interpolation tests passed, including the exact polynomial x^2 over [2,4] with original binary64 node 2.7. This is a prerequisite for exact expression coefficients; the corrected bridge representation remains pending.

Private `profile_expression_bernstein` now builds interval homogeneous coefficients of an immutable rail-corrected bridge on a single original knot rectangle. Its common denominator is the exact factored source denominator: section positions/across jets have the free weight denominator, fixed-free source rails/free jets have the across denominator, and cubic Hermite corrections give numerator degree bounds [free_degree+3,across_degree+3]. Multiple-span and excessive-degree recipes refuse. Reconstruction uses actual node coordinates, charges native enclosure spans and coefficient work separately, and does not create a rounded replacement Surface. The rational-cylinder regression passed in 0.03 s, including independent position checks, coefficient dimensions and zero-budget refusal. Annular coefficient qualification and correlated radius residual integration remain pending; this is not a radius, tangency or material certificate.

The extended clearance fixture passed in 344.29 s. Complete support-product coverage between transition face 28 and inner face 3 establishes lower bound 13.50001179091153 mm, with 4,993 cells, 4,701 fresh spans, 9,656 cache hits and zero pending domains. The fresh owner-bound certificate and its shell/face identity assertions passed. Saved `expression-face-clearance-qualified.json`; the fixture now requires completion. This is full separation of these two selected support faces, not all-wall material thickness.

`moving_radius::bound_coefficient_nets` now shares the native correlated homogeneous residual algebra between source patches and corrected expression nets, with law curves on either parameter axis and explicit degree/work refusal. Three moving-radius regressions and the corrected-cylinder coefficient test passed. The latter bounds whole-patch radius error below 1e-6 mm, proves a strictly negative squared residual for radius 2 instead of 1, and refuses zero coefficient work. Actual annular law coefficient construction and complete corrected annular radius admission remain pending.

Original law curves now receive interval homogeneous coefficients on the actual across domain, without rounded law trims. The nonunit/restricted law-domain cylinder test passed. The annular full-cell coefficient run passed in 3.10 s: squared residual [-1.6075818376465343e-9,5.67515920721969e-8], radius error upper 1.6214740592058623e-6 mm, 986 enclosure spans and 60,072 coefficient operations. Saved `expression-radius-whole-cell-coefficients.json`. This bounds the complete corrected chart inside the requested 1e-4 mm tolerance. `TransitionFace::qualify_radius_coefficients` now rebuilds the owned recipe and only issues a private certificate from fresh whole-face bounds. Owner identity and zero-budget refusal checks are running in the combined fixture; their completion is not yet claimed. Neither this radius relation nor the face clearance establishes pole G1 smoothness or full material-wall thickness.

### Fresh owner radius and corrected-chart line roots

The full `restricted_profile_seams_match_owned_neighbor_supports` fixture passed in
323.06 s with a fresh owner-bound coefficient radius certificate. The persisted
`expression-owned-radius-coefficients-qualified.json` records an upper radius
error of 1.6214740592058623e-6 mm against 1e-4 mm, 986 spans and 60072 coefficient
operations. Owner identity, original law domains and zero-budget refusal were
checked in the fixture.

The coefficient line solver uses outward homogeneous Bernstein coefficients and
Krawczyk isolation without substituting a rounded surface. Plane and rational
cylinder tests cover isolated roots, misses, original parameter domains and
unresolved coincident/tangent cases. The corrected annular chart test passed in
5.50 s: one isolated root, no unresolved cells, 41 cells. Its UV enclosure was
[[0.49999428468685353,0.5000038195155316],
[0.5000006481064496,0.5000009630070706]] and its line parameter enclosure was
[0.9916472811330479,1.0083577671561548]. Evaluated points only author the test line;
the coefficient solver supplies the root evidence.

This is a single-chart diagnostic. Trim membership, all-face crossings, material
parity and complete wall thickness remain unqualified. Pole smoothness and
general NURBS fillet admission are not implied by either result.

### Shell line audit implementation (acceptance pending)

`inspect_line_support_roots` now dispatches original surfaces, exact
separable-weight retained profiles and the rail-corrected transition. It retains
unsupported coefficient domains and exhausted work as pending. Retained profile
isolation passed the restricted rational cylinder test (0.10 s), including a
zero span budget. The nine existing ray isolation regressions also passed.

`inspect_trimmed_line_roots` freshly classifies owned contours, requires a strict
interval sign of (Su cross Sv) dot direction and retains overlapping line
parameter intervals as pending. The transverse-sign unit test passed, covering
both crossing directions, tangency and uncertain normal sign. Shell acceptance
of this newly combined route is pending; its report is diagnostic and cannot
construct a material or thickness certificate.

### Multi-span retained roots and material-line route

Retained separable profiles now visit each intersected original knot rectangle,
using exact factorized weights. The rational cylinder test includes two source
U spans and one isolated interior root; it passed in 0.08 s. Zero span work now
returns explicit pending rectangles in a report. Roots on knot boundaries are
not silently merged or admitted.

The previous full shell fixture passed in 404.68 s with the zero-cell and
exterior-line exclusion checks. The newer trimmed/transverse fixture has a
separate running acceptance and is not covered by that earlier success.

`EmbeddedExpressionGeometry::inspect_material_line` freshly isolates and
classifies crossings, checks a strict exterior whole-support hull seed and
pairs the positive ordered crossings into material intervals. It preserves the
full root audit and refuses pending boundary, seed bands and odd crossing
counts. The combined material fixture is added but has not yet passed; no
minimum thickness or general fillet qualification is claimed.

The seam-coincident shell line [0,0,3] + t [30,0,0] produced zero admitted
crossings and eight pending rectangles (473 cells, 1036 spans, 3756 coefficient
operations, 484 trim cells). This explicitly preserves seam uncertainty; it is
not a material-line qualification. Multi-span retained tests also passed the
interior-knot-root refusal and the one-span budget retention checks. General
infinite-line enclosure exclusion passed all nine ray regressions.

### Fresh shell material intervals passed

The full trimmed-line/material fixture passed in 320.63 s. The outside-seeded
line [-30,2,3] + t [60,0,0] had four distinct transverse trimmed crossings,
zero pending domains and two material intervals; total support work was 1104
spans. Zero-cell refusal also passed. This qualifies only that line on the
embedded annular expression shell, not minimum thickness over the whole wall.

A further fixture run checks the private owner-bound material-line certificate
and exports complete root/interval diagnostics. That owner/serialized-artifact
acceptance is pending until its own terminal result is observed.

### Local endpoint normal and length implementation

The immutable material-line owner exposes outward endpoint-distance bounds.
The length regression passed uncertain endpoint bands, direction scale and
reversed direction. The shared normal-enclosure algebra preserves the prior
parallel/antiparallel/oblique/collapsed normal test. Endpoint inspection rebuilds
whole-root derivative enclosures on the same expression geometry, with explicit
normal budget exhaustion. Its annular strict-angle/refusal acceptance has been
added and compiled but is not yet run to completion. Local line intervals and
endpoint normals do not certify the minimum over a complete wall.

### Owned material-line acceptance passed

The fresh owner/certificate and diagnostic-artifact fixture passed in 331.25 s.
`expression-owned-material-line-qualified.json` records four transverse roots,
two material intervals (faces 16–17 and 8–2), zero pending rectangles, 155 cells,
1104 spans, 3756 coefficient operations and 36 trim cells. Certificate geometry,
origin, direction and interval count were checked against the actual immutable
owner. This is local material-line qualification, not a minimum wall certificate.
The subsequent endpoint-normal/length integration remains a separate acceptance.

### Whole-wall support coverage and budget repair (acceptance pending)

The expression coverage route enumerates all n(n+1)/2 products, including every
same-face product. A FIFO traversal visits original products before refinement.
Exact original-control planarity excludes positive coplanar chords; complete
position distance and endpoint-normal cone tests can exclude other cells.
Paid position/normal rectangles are reused only inside the same fresh shell
invocation. Remaining products are retained explicitly; no whole-wall minimum
has been qualified.

Endpoint acceptance found a real zero-budget derivative bug after 432.04 s:
original surface dispatch called a positive-budget normal routine with zero.
Dispatch now refuses exhausted derivative work, including exhaustion between
axes. Its focused regression passed budgets 0/1/2 and invalid zero-budget input.
The full radial/material/coverage acceptance is running separately. Later generic
zero-budget dispatch validation was added for all face kinds; it remains subject
to the consolidated full fixture acceptance.

### Radial line and first wall pass accepted

The combined fixture passed in 318.68 s. Both radial material intervals are
approximately 15 mm long; whole-root endpoint normal squared-sine upper bounds
are at most 1.2240019928961575e-11, below 1e-4. The oblique line failed the strict
normal test as expected. Zero and partial normal budgets were refused.
`expression-radial-material-lengths-qualified.json` preserves the native values.

All 435 initial products were visited. Distance excluded 148 cells; 490 child
rectangles remained pending after subdivision (435 cells, 175 spans).
`expression-wall-first-pass-pending.json` explicitly records incomplete coverage.
Planar exclusions and per-owner cache reuse were checked in the fixture.
Original curved same-face products now also invoke the existing source-owned
intrinsic chord proof on the complete union rectangle. Its combined acceptance
is a separate running test; retained/corrected expressions cannot inherit that
original-source certificate. Minimum whole-wall thickness is still unproven.

### Original same-face intrinsic coverage verified

The full `restricted_profile_seams_match_owned_neighbor_supports` test passed in 316.73 s. All 435 initial support pairs were visited. Original continuous-chart self-chord exclusion reduces pending child domains from 490 to 486 at the same 435-cell limit; span work is 189 (previously 175). Retained profiles and corrected transitions do not inherit original-surface certificates. Whole-wall qualification remains incomplete. The radial material intervals, endpoint normals, owned line intervals and volume checks also passed. The zero/partial derivative-budget regression passed separately. See `expression-wall-intrinsic-first-pass-pending.json`.

### Expression Jacobian self-chord algebra

Added `surface_self_chord::inspect_jacobian_self_chords` to apply the existing conservative mean-Jacobian angular bound to native derivative enclosures. It returns a diagnostic bound only, with no source certificate. Expression callers still need a proof of continuity over the convex endpoint rectangle before admitting exclusions. Invalid intervals, nonfinite entries and invalid angular tolerances are rejected; a singular zero Jacobian cannot qualify. All 33 self-chord and coverage regressions passed, including short normal-chord refusal and exhausted-budget coverage. Retained/transition continuity admission is still pending.

### Retained profile intrinsic coverage integration

Added exact `WeightProfile::inspect_self_chords`: natural-domain and angle validation, explicit continuity screening, fresh recipe derivative bounds, and conservative Jacobian chord exclusion. Wired retained same-face products into the owner-bound wall audit and local qualified-rectangle cache. Focused plane/budget/invalid-domain/disconnected-source regression passed; B-rep cargo check passed. Full control-fixture integration test is running (`/tmp/expression-retained-intrinsic-wall-full.log`); no new full-fixture result or reduction count is claimed yet. Corrected transition admission remains pending.

### Corrected transition intrinsic coverage integration

Added `RailConstrainedBridge::inspect_self_chords` on the exact transverse-corrected derivatives. It conservatively refuses source knots permitting derivative discontinuities, validates the expression rectangle, and preserves zero-budget uncertainty. Integrated transition same-face products into the wall audit without borrowing original-surface certificates. The corrected rational cylinder coefficient regression passed (0.09 s), including self-chord query, zero budget and invalid domain; B-rep cargo check passed. The running retained-only full fixture predates this transition integration; full transition fixture verification remains pending.

### Combined coverage report capture

All four `profile_expression` regressions passed (4.50 s). Added optional native-fixture JSON export via `BREP_EXPRESSION_WALL_REPORT_PATH`, including every pending face pair and both UV rectangles, budgets and exclusion counters. Combined retained/corrected-transition full fixture started with output `/tmp/expression-combined-intrinsic-wall-full.log` and report `/tmp/expression-combined-wall-report.json`. This is a distinct verification of the changed integration, not a restart of the still-running retained-only test. Neither new full-fixture completion nor whole-wall qualification is claimed yet.

### Retained-profile full fixture and counterexample verified

The retained-profile full control test passed in 320.65 s: 435/435 initial pairs, 486 pending child domains, 435 cells, 191 spans, 148 distance exclusions and 0 angular exclusions. Thus retained integration preserves the previous incomplete coverage and does not establish a whole-wall minimum. Both profile self-chord regressions passed, including an exact rational half-cylinder diameter that must remain a normal-aligned candidate and one-span exhaustion returning `(None, 1)`. The combined corrected-transition full run is still live.

### Adaptive wall verification launched

Added optional `BREP_EXPRESSION_WALL_CELLS` to exercise refinement beyond the 435 initial products, bounded by the native 100000-cell limit. Coverage assertions now require exact consistency of complete/pending status and reserve the expected incompleteness assertion for the initial-pair budget. Zero/partial derivative regression passed after fixture changes. An extended 10000-cell combined-expression run is live, with full pending-domain export to `/tmp/expression-adaptive-wall-report.json`; the independent default-budget combined run remains live. No completion result is claimed for either.

### Owner-bound aligned-chord admission

Added `qualify_minimum_aligned_chords` and private-construction `ExpressionAlignedChordMinimum`. Admission recomputes fresh support-product coverage and requires complete/no-pending evidence; it cannot consume caller-mutated reports. The certificate retains geometry identity, requested lower bound and angular scope. It certifies aligned chords only, not unconstrained thickness or an attained material witness. B-rep cargo check passed. Added full-fixture zero-budget certificate refusal assertion; its compilation/regression is running. Both preceding full runs are still live and predate this wrapper.

### Combined retained and corrected-transition fixture verified

The combined full control test passed in 331.30 s. Default wall audit: 435/435 initial products, 486 pending domains, 252 spans, 148 distance exclusions, 42 planar exclusions, 2 intrinsic exclusions and 1210 cache hits. There are 243 pending source pairs, including 58 domains involving transition face 28 and 30 same-face domains. These are unresolved candidate regions, not detected defects. Full native domain data and pair distribution are saved in `expression-combined-wall-first-pass-pending.json` and `expression-combined-wall-pending-summary.json`. The independent 10000-cell refinement test remains live. Whole-wall completion is still unproven.

### Complete intrinsic-decision cache

Unified original/retained/corrected same-face exclusion dispatch. The invocation-local cache now stores both successful exclusions and complete insufficient-exclusion results for exact owner/rectangle keys. Insufficient results never exclude a pair and incomplete calculations are never cached. This avoids repeating derivative work when child products have the same convex union. Cargo check and diff check passed. The uncached 10000-cell run remains live; a distinct cached 10000-cell run started for comparison and owner-wrapper verification (`/tmp/expression-cached-intrinsic-wall-full.log`). No measured speedup is claimed yet.

### 10000-cell wall refinement verified

The uncached full fixture passed in 333.70 s. All 435 initial products were visited; 10000 cells used 14403 spans, with 2968 distance, 151 angular, 42 planar and 34 intrinsic exclusions. The report retains 4045 pending domains belonging to 210 source pairs (initial audit: 243 pairs). Subdivision increases domain counts, so these counts cannot be interpreted as defect counts or compared directly with initial rectangles. Full native report: `expression-adaptive-wall-pending.json`. Whole-wall completion remains false.

### Same-face product symmetry

Added invocation-local product deduplication under exact endpoint exchange on the same face; cross-face products retain their ordered domains. Distance and endpoint-normal alignment are symmetric. A pending representative remains pending, and work-exhausted products are retained before registration, so symmetry reuse does not fabricate coverage. Added `symmetry_reused` report counter. The exact-key regression passed. A distinct 10000-cell symmetry/owner-wrapper full verification is running, alongside the prior cached run; no improved qualification count is claimed yet.

### Line-query validation before support limits

Moved retained-profile line degree-limit refusal after input/natural-domain validation. A valid degree-17 query still returns explicit unsupported-degree diagnostics, while zero direction and an out-of-domain rectangle now return input errors rather than being hidden by the degree limit. The regression passed. Cached and symmetric 10000-cell full runs remain live; neither has a new terminal result yet.

### Common-chord endpoint normal cone exclusion

Enhanced native `endpoint_chord_excluded` with the necessary two-normal angular condition: for squared sine s < 0.5, common-line alignment requires endpoint normal squared sine <= outward 4*s*(1-s). Fresh owner-proven normal boxes can therefore exclude incompatible cones even when coordinate chord boxes overlap. Unknown/degenerate normal cones and the 45-degree boundary do not qualify. Normal-alignment regressions: 23 passed, 1 ignored (0.01 s), including new overlapping-box, endpoint-exchange, uncertain-cone and angular-boundary controls. Current cached/symmetric full runs predate this addition and remain live; full integration verification is pending.

### Complete intrinsic cache verified

The cached 10000-cell full fixture passed in 512.08 s. Span work fell from 14403 to 7036 (51.15 percent reduction) with identical ordered pending UV rectangles and identical exclusion counters: 4045 domains, 2968 distance, 151 angular, 42 planar and 34 intrinsic exclusions. The comparison establishes reduced span work, not wall-clock speedup (runs overlapped). Saved native report: `expression-cached-wall-pending.json`. All 33 self-chord/coverage regressions also passed after the endpoint-normal cone change. The symmetry run remains live and predates the latter change.

### Same-face symmetry full fixture verified

The 10000-cell symmetry fixture passed in 447.00 s. It retains 3757 pending domains, uses 6551 spans, and records 3055 distance, 172 angular, 42 planar and 26 intrinsic exclusions. Full native report: `expression-symmetric-wall-pending.json`. Different queue allocation means individual exclusion counters need not match the preceding run. Completion remains false. Added per-knot-span original/exact-profile normal queries to avoid crossing independently hulled derivatives; cargo check passed. Full zero-normal-budget fixture assertion added; integration qualification is pending.

### Owned trim exclusion integration

Added fresh `ExpressionShell::inspect_support_trim_outside` using original fragment contours and root-valued endpoints. Mixed source/transition loops and exhausted work remain candidates. Added `inspect_trimmed_minimum_aligned_chords` with a separate bounded contour budget, cached only proven outside rectangles, and explicit domain-cell/trim-exclusion counters. Existing support-only API retains a zero trim budget. Full fixture accepts `BREP_EXPRESSION_WALL_TRIM_CELLS` and exports trim counters. Cargo check passed; contour regressions are being verified. Current normal-cone and per-span-normal full runs remain live and predate this optional trim integration. No new wall certificate is claimed.

### Endpoint cone full fixture verified

The 10000-cell common-normal-cone run passed in 316.43 s. It records 4425 pending domains belonging to 70 source pairs, 11672 spans, 1997 distance exclusions, 704 angular exclusions, 42 planar and 100 intrinsic exclusions. Different refinement allocation changes counts; no monotone improvement claim is made. Native report: `expression-normal-cones-wall-pending.json`. Whole-wall completion remains false. Contour winding regression passed after trim resource guards; unsupported contour counts remain candidates rather than input failures. Per-span-normal run remains live.

### Per-span normal full fixture verified

The full per-span-normal fixture passed in 314.72 s. Pending domains: 4425; pending source pairs: 70; spans: 11133. Full native report saved as `expression-span-normal-wall-pending.json`. Completion remains false. Resource-limited trim contours are preserved as candidates; contour regression passed. The independent trimmed-domain full run remains live.

Per-span normal comparison: the same 70 pending source pairs and 4425 domains are retained while span work decreases from 11672 to 11133. This is a computational reduction, not expanded qualification. Endpoint-specific normal/mean-path bounds remain a possible next refinement after trimmed-domain results.

### Endpoint-specific mean-Jacobian algebra

Added diagnostic `inspect_jacobian_endpoint_chords`, using the existing conservative endpoint/mean-matrix cosine bound. It does not create a source certificate; callers must establish expression continuity and endpoint normal ownership before using it for coverage. Plane-normal exclusion, tangent-direction refusal, singular matrix, uncertain normal and invalid numeric inputs are tested. All 34 self-chord/coverage regressions passed (0.43 s). Expression admission/integration of this new bound remains pending; the trimmed-domain full fixture is still live.

### Exact retained-profile endpoint mean-path integration

Added `WeightProfile::inspect_self_chord_pair`: validate both endpoint rectangles and angular/work inputs, require a continuous exact separable profile chart over their convex union, recompute whole-path derivatives and endpoint-specific normals, and apply native mean-Jacobian exclusion. Exhaustion remains `(None, spent)`. The exact plane test qualifies; half-cylinder diameter remains a candidate and one-span work refuses. Both profile regressions passed. Integrated retained same-face pair checks after ordinary endpoint/chord exclusion; cargo check passed. Full integration verification remains pending, and the preceding trim run is still live.

Trim full fixture passed in 323.00 s: 59 pending pairs, 4429 pending domains, 293 trim exclusions and 100000 contour cells. Saved `expression-trimmed-wall-pending.json`; complete remains false.

### Corrected expression endpoint mean-path integration

Added `RailConstrainedBridge::inspect_self_chord_pair`, sharing conservative C1 source-field continuity screening with the whole-union inspector. Fresh corrected derivatives over the connecting rectangle and corrected endpoint normals feed the native mean-path bound. Zero/exhausted work stays unproven. Integrated transition same-face pairs after ordinary exclusion; no original-surface certificate is borrowed. All six profile-expression regressions passed (4.54 s), and cargo check passed. Retained-only full endpoint-pair test remains live and predates this transition addition; combined verification is pending.

### Endpoint mean-path scale normalization

Normalized each mean-Jacobian parameter column by a fixed positive enclosure scale and endpoint normals by their maximum component before computing the cosine bound. Both transformations preserve the geometric angle/path theorem while reducing overflow risk. Algebraic plane controls at scales 1e-90, 1 and 1e90 pass; all 34 self-chord/coverage regressions passed (0.45 s). This qualifies the bound computation, not arbitrary full models at those scales. Existing retained/corrected full runs remain live and predate normalization; profile regressions are running.

### Trim-aware owner admission

Added `qualify_trimmed_minimum_aligned_chords`, recomputing fresh complete coverage including owned outside-trim exclusions before issuing the private geometry-bound aligned-chord certificate. Support-only qualification delegates with a zero contour budget. The full fixture now asserts certificate/complete consistency and checks geometry identity, lower bound and angular tolerance if admitted. Fixture compilation and the derivative-budget regression passed. Neither an actual whole-wall certificate nor completion of the new full assertion is claimed; earlier retained/corrected pair full runs remain live and predate the wrapper/scale normalization.

### Retained endpoint-pair full fixture verified

The retained-only endpoint-pair run passed in 361.22 s: 59 pending source pairs and 4429 domains, using 13711 spans. It did not improve the prior trimmed fixture coverage; the extra pair-specific bound remains available for broader geometry. Native report: `expression-retained-endpoint-pair-wall-pending.json`. Added complete inside/outside trim decisions with invocation-local caching; unresolved/boundary/resource cases remain `None` and are not cached. Combined corrected-pair run remains live and predates scale/owner/cache changes.

### Corrected endpoint pair full fixture and rank precondition

The corrected-pair full fixture passed in 441.47 s, retaining 59 pairs and 4429 domains with 68794 spans. No coverage improvement over the preceding fixture was observed. Saved `expression-corrected-endpoint-pair-wall-pending.json`. Added a balanced interval mean-matrix rank precondition before endpoint-normal queries in exact profiles/corrected bridges: failure skips only this insufficient theorem, never the candidate product. Six profile regressions and 34 self-chord regressions passed. Full rank-precondition comparison remains pending; complete trim-cache run remains live.

Additional rank-precondition regression passed: independently scaled columns at 1e-90 and 1e90 retain the sufficient rank bound; an enclosure admitting coincident columns refuses both rank qualification and endpoint exclusion. Reversed and infinite intervals are rejected. Both endpoint-Jacobian tests passed. The new full comparison uses 10000 wall cells and 100000 trim cells; it remains running, so no reduced-work or complete-wall claim is made.

### Complete trim-decision cache verified

The complete inside/outside trim-cache fixture passed in 412.20 s. Its ordered pending rectangles are identical to the corrected endpoint-pair report: 59 source pairs, 4429 domains, 68794 spans and 293 trim exclusions. Cache hits increased to 36828; neither coverage improvement nor wall-clock acceleration is claimed. Saved `expression-complete-trim-cache-wall-pending.json`. The rank-precondition full run is separate and remains pending.

### Original NURBS endpoint-pair integration

Added `inspect_original_endpoint_pair`: validate both original rectangles and angle/work bounds, recompute the continuous convex-union mean matrix, then query source-bound endpoint normals within the remaining budget. False remains unproven. Zero work still validates inputs before refusal. Connected original same-face products to wall coverage, alongside retained and corrected expressions. The planar varying-tangent regression passed, including zero/one-span refusal and two-span qualification; B-rep cargo check passed. Full model comparison is running and no additional resolved pairs are claimed yet.

The independent parabola endpoint-pair regression also passed: analytic chords and endpoint normals for `S(u,v)=(u,v,0.01u²)` agree with the exclusion over separated endpoint rectangles. Invalid angular tolerance, excessive work and NaN rectangles reject even at zero budget. These analytic samples are regression controls, not substitutes for the native interval coverage. The earlier 35 self-chord regressions passed before this additional test.

### Rank-precondition full comparison verified

Full fixture passed in 388.87 s. Ordered pending rectangles and every exclusion counter are identical to the complete trim-cache report: 59 pairs, 4429 domains. Spans decreased from 68794 to 30666 (55.42% less); overlapping run timings do not establish a wall-clock speedup. Saved `expression-rank-precondition-wall-pending.json`. This run predates original endpoint-pair integration and the pending-reason counters.

Added separate pending counters for global budget exhaustion, incomplete position enclosures and UV resolution limits. Their sum is asserted against the entire pending list in the full fixture and exported in its native report. Fixture compilation and the exact-product symmetry regression passed; full counter verification remains pending.

### Exact-profile coordinate-strip fallback

When the convex-union Jacobian cannot provide a rank bound, exact-profile endpoint inspection now bounds a two-leg coordinate path: U across A's V strip, then V across B's U strip. The existing source continuity screening covers both paths; derivatives are freshly recomputed from the exact profile and charged to the same budget. Rank or angular uncertainty still refuses exclusion. Three profile regressions passed and B-rep cargo check passed. The planar chart `S=(uv,u(1-v),0)` qualifies using the regular B endpoint while its collapsed A boundary receives no regular normal. The spatial chart `S=(uv,u(1-v),v)` remains unproven; a half-cylinder diameter is preserved. Existing full runs predate this fallback, so no full-fixture coverage gain is claimed.

### Original endpoint-pair full fixture verified

Passed in 347.57 s. The pending source-pair set is unchanged at 59 (zero newly pending or resolved pairs); pending subdivisions changed from 4429 to 4443 under the fixed 10000-cell allocation. Spans are 34702. Saved `expression-original-endpoint-wall-pending.json`. This fixture predates the coordinate-strip fallback and pending-reason counters.

The corrected transition endpoint inspector now also uses the two-leg fallback, with each derivative column freshly bounded from the corrected expression and its continuous source derivative fields. All seven profile-expression tests passed in 4.54 s. Combined full comparison, including pending-reason accounting, is running separately; no new whole-body qualification is claimed.

Pending-reason accounting assertion now covers both zero-budget and refined reports. The first insertion matched only the zero-budget report; the refined assertion was added explicitly after reviewing the fixture. Compilation and the exact-product regression passed. The two running full binaries predate the refined assertion, so their reason sums must also be checked directly from native JSON when terminal results arrive.

### Pending-reason full result

The reasons fixture passed in 344.47 s. Direct native JSON accounting confirms all 4443 pending domains are global-budget pending; enclosure and resolution counters are both zero. All 435 initial pairs were visited. Saved `expression-wall-reasons-pending.json`. Thus this 10000-cell run does not establish an irreducible 59-pair geometric limitation. Started a distinct 50000-cell run of the current coordinate-strip implementation, retaining the same tolerance and 100000 trim-cell budget; results are pending.

### Original NURBS coordinate-path fallback

Original endpoint inspection now recomputes U/V strip mean matrices when the full convex-union rank bound is insufficient, and uses the existing source-identity/domain-checked coordinate-path construction. Every strip and normal query pays the same span budget. The collapsed-boundary planar control qualifies with five spans; budgets zero through three preserve uncertainty. All 37 self-chord/coverage regressions passed in 0.40 s, and B-rep cargo check passed. The live 10000/50000-cell binaries predate this original-chart fallback; no whole-model improvement is claimed.

### Profile/transition coordinate-strip full fixture

Passed in 369.30 s: 59 pairs, 4443 domains, all budget pending; no new coverage compared with the original endpoint-pair run. Spans increased to 72856. Saved `expression-coordinate-strips-wall-pending.json`; full native reason accounting matches. This run predates the original-chart fallback and per-domain reasons.

Each pending product now carries an explicit budget/enclosure/resolution reason alongside owned face indices and UV rectangles. Global span exhaustion during position enclosure is classified as budget rather than geometric uncertainty. Reasons are exported in the native JSON; refined fixture counts are checked against the actual per-domain tags. The enum is re-exported through `source_profile_body`. Fixture compilation and the exact-product regression passed. No WASM/UI transport or full per-domain integration qualification is claimed yet.

### Runtime transport boundary inspected

`geometry-bridge/src/cad_whole_wall.rs` decodes an ordinary `brep_core::Model` and calls `material_wall_coverage::inspect`; its stated scope is all original face pairs. `src/services/solidWholeWall.ts` validates that same original-face scope. This route does not reconstruct the source-bound retained/corrected expression shell used here. Integrating expression-wall evidence requires native reconstruction of its exact recipe/owners before transport of pending rectangles and reasons. Existing original-model whole-wall UI evidence does not qualify this expression fixture.

### Per-domain reasons full fixture verified

Passed in 365.91 s with the original coordinate-path fallback: 59 source pairs, 4443 domains, 74056 spans. All pending domains carry `budget`, matching every native aggregate counter; enclosure/resolution counts are zero. Saved `expression-domain-reasons-wall-pending.json`. No new resolved pairs were observed under the fixed 10000-cell budget.

A one-second process sample of the still-running 50000-cell fixture captured two test-thread stacks in corrected transition endpoint derivative evaluation, including profile homogeneous-net enclosure/interpolation. This confirms its active wall-computation path, not a statistically meaningful hotspot percentage or speedup. Its complete report remains pending.

### 50000-cell result verified

Passed in 772.66 s: the same 59 source pairs remain pending, with 28881 subdivided domains, all budget pending. Spans reached 931312 of 1000000; 4983 angular and 4477 distance exclusions were obtained. No enclosure/resolution pending domains occurred. Saved `expression-50000-cells-wall-pending.json`. Increasing cell allocation alone did not close source pairs; this remains incomplete whole-support coverage, not detected thin-wall defects. The run predates the original coordinate-path fallback and per-domain tags.

Prepared the immutable corrected transition once per wall inspection and reused it for intrinsic/pair checks. Zero cell/span budgets skip preparation; a preparation refusal remains None. B-rep cargo check passed. A separate 10000-cell comparison is running; no changed geometric conclusions or speedup are asserted before it finishes.

### Endpoint-domain chord enclosure

Added native `inspect_jacobian_endpoint_pair_chords`, bounding `M*(B-A)` with outward interval arithmetic and testing its unoriented angle against an owned endpoint normal. It issues no source certificate; original/profile/corrected callers bind the continuous matrix, domains and normal. Original callers use the unscaled parameter matrix for this enclosure, preserving parameter differences. Existing zero/aligned chord candidates remain unproven. The previously unresolved spatial ruled control `S=(uv,u(1-v),v)` now qualifies over the tested separated rectangles, with independent analytic corner chords/normals as regression controls. All 38 self-chord/coverage tests and seven profile-expression tests passed; B-rep cargo check passed. Full comparison is running; arbitrary geometry and whole-wall admission remain unqualified.

Endpoint-domain enclosure now uses common positive scales for the whole matrix and all endpoint coordinates before multiplying. Unlike independent column scaling, these preserve the actual chord direction without changing relative parameter differences. Algebraic controls at scales 1e-200, 1 and 1e200, including exchanged endpoints, preserve exclusions and aligned candidates; all 38 self-chord and seven profile-expression regressions passed. These scale controls do not qualify extreme-scale source models. B-rep cargo check passed. Both live full binaries predate this normalization.

### Prepared transition reuse full result

Passed in 450.27 s. The entire native report is identical to `expression-domain-reasons-wall-pending.json`, including ordered pending rectangles/reasons and 74056 spans. Saved `expression-prepared-transition-wall-pending.json`. Concurrent timings do not establish acceleration; preparation reuse preserves the tested geometry conclusions.

### Shared original corner paths

Added `inspect_shared_corner_chords`, considering only exactly equal authored clamped corners, then delegating to the existing source-bound continuous corner-path cone proof. Integrated distinct original faces after ordinary chord exclusion; midpoint-box direction is only a finite projection proposal. Each product has at most 64 additional charged spans; refusals stay candidates. Eight original corner regressions passed, including approximately coincident corner refusal and reflected/exchanged source domains. B-rep cargo check passed. A separate full comparison with scale normalization and this cross-face path is running; no new resolved fixture pairs are claimed yet.

Added a curved-chart regression for `S=(sign*u,sign*v,0.01*u*v)`: the two nonplanar charts share their authored corner and qualify through fresh original paths. Zero projection and zero work remain unproven; out-of-domain endpoints reject even at zero work. All nine original corner regressions passed (0.02 s). This extends selected controls, not arbitrary seam or whole-wall qualification; both full comparisons remain pending.

The shared-corner inspection wrapper now explicitly requires finite positive-width endpoint rectangles before any zero-budget refusal. Degenerate/out-of-domain inputs reject at the API boundary instead of reaching a later positive-width path query. All nine corner regressions passed again. Existing full fixture rectangles already satisfy this contract; full cross-face coverage remains pending.

### Native diagnostic wire contract

Added `value_codec::Serialize` for pending reasons, cells and the report, with explicit expression support-product method/scope. Public report types are re-exported through `source_profile_body`. The geometry-owned certificate has no wire decoder or serialization admission path. JSON round-trip preserves domains, reasons and counters and carries no certificate field; the focused contract test and B-rep cargo check passed. Replaced fixture-only hand-built JSON with this same serializer plus request parameters; fixture compilation passed. Runtime recipe reconstruction, WASM/worker transport and UI remain open.

### Endpoint-domain full fixture result

Passed in 658.19 s. Its pending pair set remains 59 with 4443 budget domains and 74056 spans; zero pairs were newly pending or resolved. Saved `expression-endpoint-domain-chord-wall-pending.json`. This full run predates common scale normalization, shared-corner integration and diagnostic serialization. Its runtime overlaps other work and does not establish a performance regression or speedup.

The native report now stores the validated minimum aligned-chord length, squared-sine threshold and UV tolerance, and serializes those exact request values. The fixture no longer inserts hard-coded query metadata. The wire round-trip test verifies all three fields; fixture compilation passed. Runtime fillet dispatch still returns ordinary Models, whereas expression qualification requires source/profile/transition owners and a reproducible native recipe. Preserving/reconstructing that operation recipe remains required before expression-wall WASM integration.

### Shared-corner full fixture result

The full fixture passed in 654.45 s. Saved `expression-shared-corner-wall-pending.json`: 59 pending source pairs, 4443 budget domains, 80688 spans. No source pair was resolved or newly pending compared with endpoint-domain coverage. Shared-corner integration increased charged span work for this fixture without closing coverage. This run predates interior factored-direction handling and native query serialization. Overlapping host work prevents a wall-clock performance conclusion.

### Interior profile normal directions

Added a separate direction-only enclosure API, retaining ordinary normals on rectangles touching either U endpoint. Strictly interior rectangles may use exact factored pole normals when ordinary component intervals all contain zero and a factored component separates from zero. This does not certify pole immersion or a unique limiting plane. The wall angle gate consumes directions; the existing normal-magnitude API remains unchanged. A focused regression passed for both endpoint orientations, boundary equality, zero budgets and invalid domains. B-rep cargo check passed; full coverage qualification remains unverified for this change.

The rational regression now exercises the actual factoring branch on `S=(u,u^2*4v/(1+3v),0)`, U=[0.01,0.99], V=0.5: ordinary normals contain zero, factored Z encloses the analytic value 0.64 and separates from zero. Exhausting the additional work retains ordinary uncertainty; widening V to [0,1] remains uncertain. All 26 profile tests passed (6.87 s); diff check passed. The full body comparison is running and is not yet evidence of improved coverage.

Extended the rational direction regression to reversed U orientation (analytic factored Z=-0.64), every span budget from 0 through 20, and rejection above the public 100000-span limit. Both direction regressions passed (0.01 s). Added expression-shell assertions for zero direction budget on every face and equality with ordinary normals on pole-touching retained domains; fixture compilation passed. A pre-existing ambiguous test sum in `miter_seams.rs` was annotated as `usize` to compile with the current dependency set. Those new shell assertions are not yet executed by the already-running full fixture.

Added a rational quadratic noncollapsed-end control with an uncertain ordinary normal box. Neither endpoint admits factor removal: direction bounds and spent spans exactly match ordinary bounds. All three direction regressions passed (0.01 s); diff check passed. Full comparison remains live.

### Interior direction full fixture result

Full fixture passed in 360.55 s. Saved `expression-interior-direction-wall-pending.json`: 59 pending pairs, 4337 budget domains (previously 4443), 81990 spans (previously 80688), 744 angular exclusions. Pending source-pair sets are identical: zero newly pending or resolved pairs. The report uses native serialization and exact validated query parameters. This run predates the subsequent profile endpoint-pair direction integration and newly compiled shell assertions. Completion remains false; runtime differences do not establish a speedup.

The endpoint-pair angle inspector now also requests interior direction enclosures; its mean Jacobian still uses ordinary derivatives and its rank gate remains mandatory. Rational planar interior pairs qualify in both U orientations. A wider near-pole pair qualifies in the low orientation but remains unproven after U reversal, exposing coordinate-path rank limitations rather than assuming symmetric proof strength. The cylinder diameter control remains pending.

### Direct endpoint chord versus rank admission

Exact profile and corrected-expression endpoint inspectors no longer refuse all angular checks solely because the mean-Jacobian rank bound remains unresolved. Direct outward `M*(B-A)` enclosure requires its own nonzero chord and incompatible endpoint normal; rank admission still guards the mean-matrix fallback. A wide rational interior pair now excludes in both U orientations. Six profile/expression regressions passed (5.41 s), including preservation of the cylinder diameter candidate. A separate uncertain-rank matrix regression verifies direct exclusion while overlapping endpoints and zero normals remain candidates. The currently running full pair-direction comparison predates this rank-gate change.

Original source-bound `MeanMatrix::exclude_pair` now follows the same direct-versus-rank distinction. A conservatively widened source derivative enclosure includes rank-deficient matrices but proves nonzero planar chord direction through separated X. The degenerate zero surface still refuses, now charging two fresh endpoint-normal queries. All 40 self-chord/coverage regressions passed (0.45 s); diff check passed. Full pair-direction run remains live and predates these latest rank changes.

Validated the direct-rank distinction against aligned chord candidates in both endpoint orders: they remain unexcluded while the incompatible Z direction excludes. Focused regression passed (0.01 s); B-rep cargo check passed (1.92 s); diff check passed. This verifies algebraic admission boundaries, not whole-body coverage.

Saved `expression-interior-direction-wall-comparison.json` with per-pair domain changes and exact query metadata. Reductions mainly involve retained faces 0 and 27; corrected transition pairs 27/28 (512 domains) and 0/28 (462) are unchanged. Some original self-pairs gain subdivisions as work is redistributed. Domain-count deltas therefore do not measure removed physical wall defects or prove increased covered parameter volume. Cross-chart transition/retained chord exclusion remains a major next target.

### Endpoint-pair direction full comparison

Full fixture passed in 351.77 s. Saved `expression-pair-direction-wall-pending.json`. The entire native diagnostic report is identical to the preceding interior-direction run: 59 pending pairs, 4337 budget domains, 81990 spans. No coverage gain from endpoint direction integration on this fixture. This run executes the added expression-shell zero-budget/pole-boundary assertions but predates the direct-chord rank separation. The next comparison isolates those latest original/profile/corrected rank changes.

### Native profile-to-corrected chord path

Added `RailConstrainedBridge::inspect_profile_endpoint_chord`, binding the source profile through the bridge own section. Its three-leg path integrates profile transverse and free derivatives to the exact section, then corrected across derivatives to the target. Fresh endpoint normals and outward chord bounds supply angular exclusion; no foreign profile or decoded proof is admitted. All seven profile/expression tests passed (4.64 s): both source axes and both sections, planar nonzero chords, overlap candidate preservation, exhausted work and invalid domains. The overlap control needs a larger budget for both endpoint normals and remains a candidate. This helper is not yet connected to expression-wall cross-face dispatch; curved qualification and full coverage remain open. The current direct-rank full run predates this helper.

Added rational-cylinder cross-chart controls: sufficiently narrow rectangles near both owned sections exclude through fresh corrected derivatives; wider source/transition rectangles remain unproven. Both cases are retained as tests. All seven profile/expression regressions passed (6.66 s). Connected retained/transition products in expression-wall dispatch with side/order handling, retained end selecting its owned section, and at most 256 additional spans per product charged to the global budget. B-rep cargo check passed; diff check passed. Full cross-chart integration has not yet run; the existing direct-rank full fixture remains live and predates this dispatch. No whole-wall coverage gain claimed.

### Direct-chord rank full comparison

Full fixture passed in 388.81 s. Saved `expression-direct-rank-wall-pending.json`: 59 pending pairs, 4321 domains, 125622 spans. Resolved pair set: []; newly pending: []. This run predates cross-profile transition dispatch. Full coverage remains False. Additional cross-path budget regressions (1 through 8), invalid angle, oversized work and zero-width domains passed (0.03 s); diff check passed.

Added an aligned-diameter control using a single-span positive-weight rational cubic with opposite endpoints and vertical endpoint tangents. With equal axial coordinates the source/transition chord is normal at both endpoints. Neither owned section excludes this candidate at budgets 0, 1, 64 or 1000. Focused regression passed (0.09 s); diff check passed. The interior-section rail constructor refused this recipe, so the control uses exactly admitted natural endpoint sections and explicitly qualifies only that scope. The full cross-profile wall run remains live.

Cross-profile inspection now encloses both source coordinate-path orders, shares the already paid corrected leg, and intersects complete outward chord enclosures. Missing/exhausted alternate bounds do not authorize an exclusion. All eight profile/expression regressions passed (7.03 s), including diameter/overlap candidates and wide-cylinder refusal. Thus this local improvement is not yet evidence of wider curved coverage. Full wall run remains live and predates the alternate path intersection.

Added cross-profile controls on U=[2,4], V=[-3,1], both transverse axes and both sections. Exact dyadic section widths preserve native rail admission; the chord path excludes planar nonzero chords with correctly scaled profile/free derivatives. Zero-work refusal remains validated. Focused regression passed (0.02 s); diff check passed. This expands parameter-domain controls, not arbitrary curved qualification. Full cross-wall run remains live.

Checked source-seam admission: an internal degree-one derivative seam is refused by native original-rail construction before it can supply a corrected cross-chart owner. A disconnected multiplicity-two degree-one source is rejected by Surface validation. The regression passed (0.00 s); diff check passed. This tests constructor/validation refusal, not support of piecewise charts or execution of the cross-chord helper on an admitted seam. Full cross-wall run remains live.

Added native diagnostic counters for retained/transition path attempts, exclusions and charged spans, including JSON round-trip checks. Focused wire regression passed (0.00 s). Fixture checks require attempts <= visited cells, exclusions <= attempts/total angular exclusions, and path spans <= total spans. These counters will separate extra cost from coverage gain; the currently running first cross-wall fixture predates instrumentation and alternate-path intersection. No live runtime or coverage claim from serialization alone.

### First cross-profile full fixture result

Full fixture passed in 646.06 s. Saved `expression-cross-profile-wall-pending.json`: 59 pending pairs, 4291 budget domains, 264263 spans. Compared with direct-rank coverage, domain count falls by 30 while span work more than doubles; no source pair resolved or became newly pending. No complete wall certificate. This run predates alternate path intersection and method-specific counters. Runtime overlaps other host work and is not a controlled benchmark. Fixture with new counter invariants compiles; diff check passed.

### Immutable cross-chord workspace

Added `ProfileChordWorkspace` bound by reference to one immutable native `RailConstrainedBridge`. Private exact-UV caches store only complete profile derivative hulls, cap entries at 100000, and expose only hit counts. Wall dispatch reuses one workspace per owner-bound invocation and charges actual fresh spans. Repeat queries preserve results and spend fewer spans; independent workspaces reproduce the cold cost; warmed zero-budget queries still refuse and invalid domains reject. Ten profile/expression tests passed (4.57 s), B-rep cargo check passed, and an additional two-knot-span regression verifies incomplete hulls never enter the cache (0.00 s). Full body savings are not yet measured; running two-path comparison predates workspace caching. Corrected derivative/normal reuse remains a next performance target.

Extended the same immutable workspace to complete corrected derivative jets and exact-profile direction enclosures (each cache bounded to 100000 entries). Corrected normals are recomputed algebraically from cached native jets with the same outward cross product as the uncached API. Tests verify exact enclosure equality and zero fresh spans on repeat normal queries; warmed diameter controls remain unexcluded at budgets 0, 1, 64 and 1000. All 11 profile/expression tests passed (4.57 s), B-rep cargo check passed, diff check passed. Full geometry/cache savings remain unmeasured: the currently running two-path fixture predates both caches.

Direction-cache retry correction: do not cache boxes containing zero in every component, because they can be complete ordinary normal bounds while optional pole-factor tightening exhausted its budget. A small-budget rational query stays uncertain; a later larger-budget query freshly finds separated factored Z, which may then be reused. Focused regression passed (0.00 s); diff check passed. Latest cached fixture compiled before this small correction; full two-path uncached comparison remains live.

### Two-path full fixture result

Full fixture passed in 463.48 s. Saved `expression-cross-two-paths-wall-pending.json`: 59 pending pairs, 4287 budget domains, 266321 spans. No source pair resolved or became newly pending compared with the first cross-profile run. The method-specific counters show 1090 attempts, 20 exclusions and 140661 charged spans. This run predates workspace caching. All 12 latest profile/expression regressions passed (5.38 s); diff check passed. A same-query cached comparison is next; no complete wall certificate or runtime performance conclusion.

Extended corrected-cache tests: a one-span exhausted corrected derivative query leaves no stored hull, a sufficient fresh budget supplies complete jets, and a repeated small-budget query reuses those complete jets with zero fresh spans. The regression covers both transverse axes and owned sections and passed (0.08 s); diff check passed. Full cached wall comparison remains live.

Shared workspace now also covers corrected-expression same-face endpoint inspections, caching only complete union/coordinate-strip jets and deriving normals from the same native cached jets. Wall self-transition dispatch shares the invocation-owned workspace with cross-face queries and charges actual cache hits. Twelve profile/expression tests passed (9.58 s); additional cold/warm planar equality and zero-budget controls passed (0.26 s), and the aligned rational diameter remains a candidate after self-cache warming at all tested budgets (0.10 s). B-rep cargo check and diff check passed. Running full cached comparison predates this same-face integration, so its full savings remain unmeasured.

### Cached cross-profile full fixture result

Full fixture passed in 399.24 s. Saved `expression-cross-cached-wall-pending.json` and its comparison. Pending domains are exactly identical to uncached two-path coverage: 59 pairs, 4287 budget domains, no complete certificate. Overall span work falls 266321 -> 129039 (51.55%); cross-profile method work falls 140661 -> 3379, preserving all 1090 attempts and 20 exclusions. Cache hits total 44138. This is span-work evidence on one fixture, not a controlled runtime benchmark. Running code predates same-face corrected-cache integration; that comparison is next. Latest same-face fixture compiles; diff check passed.

### Independent endpoint-position chord refinement

The native cross-profile endpoint inspector now intersects its two derivative-path
chord enclosures with the independently enclosed corrected endpoint minus the
exact weight-profile endpoint. This refinement runs only after path/normal checks
remain unresolved. Every fresh position span is charged; incomplete position
hulls return pending and are never cached or used for exclusion. Position caches
are private to the immutable transition workspace and accept no caller bounds.

Focused `profile_expression_radius` qualification: 12 tests passed (5.92 s).
The rational-cylinder case with axial separation at least 0.49 now excludes
alignment: cylinder normals have zero axial component, while both endpoints lie
inside a unit cube, so the axial chord fraction exceeds 0.49/sqrt(3), well above
the requested sine threshold 0.01. Exact normal diameters and overlapping domains
remain non-excluded; zero budgets still refuse. A two-span position cache test
checks incomplete-budget refusal followed by complete retry and reuse.
The full shell run with this refinement has not yet been qualified. These
endpoint-aligned support-chord checks are not a whole-wall thickness certificate.

Additional position-cache controls passed: corrected position enclosure with one
span returns no hull and leaves its cache empty; a larger-budget retry completes
and the exact same owned rectangle can subsequently reuse the paid hull.
Relevant native regressions also passed: 40 self-chord tests and 23 normal-alignment
tests (one pre-existing ignored test). No WASM/UI or whole-model admission is
implied by these focused native checks.

### Shared corrected-jet cache: full fixture result

The complete restricted-profile shell fixture passed with the corrected same-face
inspector sharing the immutable transition workspace. Compared with the previous
cross-profile-only cache, all 4287 pending domain records are exactly identical;
the report remains incomplete. Total paid spans decreased from 129039 to 40711,
cache hits increased from 44138 to 45586, and cross-profile paid spans decreased
from 3379 to 1305 through shared corrected jets. These are work-counter results
for this fixture, not a controlled latency benchmark or additional coverage.
Artifacts: `expression-shared-corrected-cache-wall-pending.json` and
`expression-shared-corrected-cache-wall-comparison.json`.
A separate full fixture run for endpoint-position chord refinement is in progress.

### Translation/scale limit regression

A new native cylinder regression passed at scales 0.001, 1 and 1000, checking
fresh/warm agreement, bounded work and zero-budget refusal. At the origin, all
three analytically non-aligned chord cases exclude. Translating the cylinder by
(1000000, -2000000, 3000000) leaves the 0.001 and 1 scale cases unresolved
(`Some(false)`, not an exclusion). This is a numerical enclosure limitation:
large translated homogeneous coordinates widen derivative/correction bounds.
The translated case is retained as a qualification limit; coordinate-centered
native derivative/correction evaluation remains needed. It is not evidence of
translation-invariant complete wall qualification.

### Center coordinates before weight multiplication

`WeightProfile::derivative_bounds` now builds the extracted homogeneous net from
interval-enclosed original control differences relative to a local original
control point, before multiplying by exact separable weights. The quotient
Jacobian then uses zero origin. This is the same exact translated expression:
its derivatives are unchanged, and subtraction remains outward interval arithmetic.
The existing absolute-position net API retains its original behavior.

All 13 profile-expression tests passed. The translated-scale regression now also
requires the exact extrusion derivative dX/du to remain inside (0.5*scale,
2*scale) at scales 0.001, 1 and 1000, including translation by coordinates of
order millions; it passed. Full corrected-transition chord qualification at the
far translation remains unresolved for scales 0.001 and 1, so additional
centering in corrected transition evaluation is still required. The running full
position-chord shell comparison predates this derivative centering change.

### Endpoint-position chord intersection: full fixture result

The full restricted-profile shell fixture passed in 337.02 s. With the same
10000-cell budget, pending domains decreased from 4287 to 4257; cross-profile
attempts changed from 1090 to 1087 and exclusions increased from 20 to 35.
Paid spans increased from 40711 to 41796 (cross-profile spans 1305 to 2374).
This run predates all early-centering derivative changes. Domain counts reflect
adaptive subdivision, not a measured UV-volume fraction or physical defects.
The whole-support report remains incomplete. Saved artifacts:
`expression-position-chord-wall-pending.json` and
`expression-position-chord-wall-comparison.json`.

Early local centering has additionally been applied to second and higher exact
profile quotient derivatives, using the same outward-centered original-control
net. The 13 profile-expression tests passed after this extension (4.57 s).
A separate full shell run with all centered derivatives is in progress.

Higher-derivative translated-cylinder controls passed: the exactly zero mixed
axial partial and order-(2,1) axial partial both enclose zero with interval width
less than the model scale, at all tested scales and both origin/far translations.
Each query covers one complete span. The centered-net helper now skips subtraction
for an exactly zero origin component, preserving the original absolute net's
arithmetic rather than adding unnecessary rounding inflation. The full corrected
transition at far translation is still not qualified by this derivative check.

The profile normal path now also constructs its quotient Jacobian from the early
centered original-control net, instead of independently reintroducing an absolute
homogeneous network. The translated-cylinder regression passed with an explicit
normal control: its exactly zero axial component contains zero and has enclosure
width below 1e-6*scale^2 at all tested scales and translations. This isolates
normal accuracy from the still-unresolved full corrected-chord query. The live
centered-derivative shell run predates this separate normal-path change.

### Centered derivatives: complete fixture comparison

The full shell fixture passed (344.62 s). Its pending-domain records and all
reported work are identical to the preceding endpoint-position run: 4257 pending
domains, 41796 paid spans, 35 cross-profile exclusions. This run predates the
separate centered-normal path and quadratic transverse-derivative refinement.
Artifacts: `expression-centered-derivative-wall-pending.json` and comparison JSON.

### Preserve transverse Hermite cancellation

The bridge transverse derivative is now also enclosed as its exact quadratic
Bezier polynomial, restricted by blossom evaluations to the requested interval.
Its controls are scaled endpoint tangents and three times the endpoint difference
minus both tangents. The complete restricted-control hull intersects the existing
Hermite derivative enclosure; no source geometry or work-span count is changed.
This avoids treating cancelling Hermite basis contributions as independent over
wide intervals. Interval endpoint differences and all control arithmetic remain
outward; exact common-source differences are used only with existing ownership.

All 13 profile-expression and 9 section-bridge tests passed. The formerly unresolved
translated-cylinder regression now excludes at every tested scale (0.001, 1, 1000),
both section ends and both zero/far translations. This supersedes the earlier
recorded translation limitation for these selected cases only. Exact normal
candidates, overlap, cache retry and zero-budget controls remain in the passing
suite. General NURBS and whole-wall qualification remain open.

### Both bridge first derivatives in restricted Bezier form

An independent curved cubic Hermite regression passed for endpoint tangent scales
0.75 and 1.25, zero/far translation, whole/partial/point transverse ranges, and
ordinary/common-source difference modes. Its native derivative hull contains all
65 independently evaluated polynomial derivative samples per case; sampling is a
regression check, while the restricted-control hull supplies the continuous bound.
A one-span insufficient budget still returns no complete jet.

The free derivative now also intersects a restricted cubic Bezier-control hull
of the same exact Hermite field. Its four controls use the endpoint free tangents
and scaled mixed partials. This preserves their common variation in the normalized
transverse parameter. All 10 section-bridge tests passed (1.73 s); the curved
regression additionally encloses the exact extrusion tangent (0,1,2) with width
below 1e-8 per component, including far translation. The live full shell fixture
predates this cubic free-derivative addition.

### Restricted mixed Hermite derivative

The bridge mixed derivative now intersects its existing enclosure with the
restricted quadratic Bezier hull of the exact differentiated free Hermite field.
This uses endpoint free-tangent differences and scaled mixed partials; no sampled
values enter admission. All 11 section-bridge tests passed (1.36 s), including a
new varying free-tangent field whose independent analytic mixed derivatives are
inside the native bound at 65 points on whole/partial/point ranges. Insufficient
work still returns no complete bound. These samples check the implementation;
the interval control-hull argument covers the continuous interval. The active
full shell run predates both cubic free and quadratic mixed refinements.

### Transverse Bezier derivative: complete shell result

The full restricted-profile shell fixture passed in 296.30 s, with 4165 pending
domains instead of 4257, 87 cross-profile exclusions instead of 35, and 42019
paid spans instead of 41796. All 59 source face pairs still have some pending
coverage; the report is incomplete. The largest pending groups are retained
profile 27/transition 28 (448 domains), retained profile 0/self (402), retained
profile 0/transition 28 (336), and corrected transition 28/self (272).
These counts measure adaptive leaves, not physical defects or area coverage.
The saved report/comparison are `expression-bernstein-transverse-wall-pending.json`
and `expression-bernstein-transverse-wall-comparison.json`.
This full result includes centered normals and quadratic transverse refinement;
it predates cubic free and quadratic mixed refinement. A separate complete
fixture run with all current jet refinements has been started.

### Corrected self-pair chords: two exact coordinate paths

Unresolved corrected-transition self-pairs now also inspect the reversed two-leg
coordinate path. Complete native corrected derivatives on both connecting strips
produce a second enclosure of the exact endpoint chord. The intersection is
checked against both already computed endpoint normals. No incomplete strip may
supply a chord exclusion; all fresh strip spans share the existing query budget
and the immutable native corrected-jet workspace. Earlier direct/rank exclusions
still return before this additional work.
All 13 profile-expression tests passed (1.79 s). The exact rational normal-diameter
control additionally exchanges endpoints under fresh/warm budgets 0, 1, 64 and
1000; both orientations must remain non-excluded and every call stays in budget.
The running full fixture predates this self-pair path extension.

### All Bezier jets: full shell comparison

The full fixture passed (210.66 s), with 4167 pending domains versus 4165 before,
42019 paid spans in both reports and 87 cross-profile exclusions in both. All 59
source face pairs remain pending. This is not a coverage improvement; adaptive
leaf counts do not measure geometric coverage. Artifacts:
`expression-bernstein-all-jets-wall-pending.json` and comparison JSON.
This run predates corrected self-pair path/position intersections.

Corrected self-pairs now additionally intersect both path chord enclosures with
the independent difference of complete native corrected endpoint-position hulls.
No partial position hull supplies exclusion. The 13 profile-expression tests passed
(1.90 s), including exact normal diameter, swapped endpoints, budgets and cache
retry. Unresolved pairs now inspect both coordinate paths even when the wider
union Jacobian has full rank: rank alone does not imply the union is the tighter
chord enclosure. Earlier successful rank/direct exclusions still return first.

### Retained exact-profile self-pair chord intersections

The exact separable-weight profile self-pair inspector now also intersects both
complete coordinate-path chord bounds and, only if still unresolved, the independent
complete endpoint-position difference. All fresh derivative/position spans share
its original work budget. Continuity and input validation remain required; incomplete
paths/hulls never exclude. Earlier rank/direct exclusions return first.
All 4 dedicated profile self-chord tests passed, including a two-span rational
normal diameter and discontinuity/budget controls. The broader 13 profile-expression
tests passed too. The diameter regression now additionally exchanges endpoints
under budgets 0, 1, 4, 8 and 100. The active full corrected-transition shell run
predates this retained-profile extension.

### Corrected self-path/position: full shell result

The full fixture passed (236.67 s). Pending count remains 4167 across 59 face pairs,
with 42019 paid spans and 87 cross-profile exclusions, equal to the preceding
all-jet run. No additional complete coverage is claimed. Saved artifacts:
`expression-corrected-two-path-position-wall-pending.json` and comparison JSON.
This run predates retained-profile and original-surface two-path additions.

Original rational surfaces now also inspect both source-bound coordinate paths
when previous endpoint checks remain unresolved. Their exact chord enclosures
are intersected, with the reversed-path chord sign corrected before intersection.
Every connecting strip must complete under the same span budget; fresh original
endpoint normals supply the angle test. All 40 existing self-chord tests passed
(0.43 s). No unbudgeted surface-position query or caller-supplied bounds are used.

### Early centering for original rational Jacobians and normals

Original `jacobian_on` now centers original controls before multiplying by their
original binary64 weights and extracting the homogeneous chart, rather than
subtracting the origin after extraction. The finite evaluated reference point
is only a translation choice; outward control-coordinate subtraction encloses
the exact translated original surface, with unchanged derivatives. No sampled
normal or different weight recipe replaces the native source.

All 23 existing normal-alignment tests passed (one existing ignored). A new
original quarter-cylinder test passed at scales 0.001, 1 and 1000 with zero/far
translation: the exactly zero axial normal is enclosed with width below
1e-6*scale^2, while both transverse normal components retain their strict signs.
The running all-source two-path shell fixture predates this original-Jacobian
centering change.

### Center original second-derivative jets before extraction

Original second jets now extract the centered original-control homogeneous net
before quotient recurrence, then restore the original translation only to the
returned position enclosure. First/second derivatives retain their actual source
meaning. All 24 normal-alignment tests passed (one existing ignored), including
the scale/translation cylinder control. Its exact zero axial second partials
contain zero with width below 0.1*scale; ordinary rational interval dependency
still gives about 0.03*scale width even at the origin, so no exact-zero or tighter
curvature certificate is claimed. The test also checks that world-position hulls
contain an independently evaluated source point after centering. The active full
shell run predates this original second-jet change.

### Original/retained two-path extension: full shell result

The complete restricted-profile fixture passed (322.55 s). Pending domains decreased
from 4167 to 4085, with paid spans increasing from 42019 to 57479; cross-profile
exclusions remain 87. All 59 source face pairs still have pending domains, so the
report does not qualify the whole support. Retained profile 0/self now has 374
pending domains versus 402 in the earlier transverse-only run. Adaptive leaves
are not area coverage or defect counts. Saved artifacts:
`expression-all-source-two-path-wall-pending.json` and comparison JSON.
This result predates early centering of original first/second jets. The entire
current NURBS library test suite has been started to check the wider impact of
those shared derivative/normal changes before further full shell qualification.

The full current NURBS library suite completed successfully: 823 passed, 0 failed,
2 existing ignored, no filtered tests (21.88 s). This is broad native regression
evidence for the current source, including centered original first/second jets;
it does not qualify generated WASM, the UI, STEP exchange or the full plan.

### Current native-to-WASM delivery work

The standard `scripts/build-geometry-kernels.mjs` build has been started after the
823-test native NURBS suite passed. It rebuilds the geometry WASM from the current
worktree and repackages the browser artifact. Verification must wait for successful
build completion: the original-body whole-wall WASM script and worker/client
regressions cover ordinary native models, cancellation, stale replies and Retry.
They do not reconstruct or admit the retained/corrected expression-shell fixture;
that native operation recipe and ownership-preserving runtime integration remain
open. The centered-original shell run is separately still active.

WASM delivery progress: the release Rust compilation for wasm32 completed
successfully (1m47s); the standard optimizer/packager is still running, so the
new browser artifact is not yet qualified. The existing whole-wall client test
for cancellation, ignored stale replies, forged source rejection and Retry passed
(1 selected test, 3 skipped). It uses a controlled fake worker port and is protocol
regression evidence, not real WASM/UI completion. Fresh artifact tests remain
pending successful packaging.

### All centered original jets: complete native shell result

The full restricted-profile shell fixture passed (216.52 s). It retains 4085
pending domains across 59 source pairs, 57479 paid spans and 87 cross-profile
exclusions, equal to the preceding all-source two-path result. Centering improves
the separately tested large-coordinate cases but adds no full-fixture coverage
here. Artifacts: `expression-all-centered-source-wall-pending.json` and comparison
JSON. The original-body WASM optimizer remains active; packaged-artifact and live
worker checks have not yet been run against its new output.

### Prepared actual worker cancellation/Retry qualification

New `scripts/check-cad-whole-wall-worker.mts` plus its `.mjs` worker entrypoint
exercise production handler/client through actual Node worker threads. The check
requires a warm native result, cancellation after request-dispatch observation,
termination and joined worker exit, then a qualified matching result through a
fresh worker realm. It records the stable public WASM hash and joins both realms
on cleanup. Dispatch observation deliberately does not claim interruption inside
Rust, and Node worker evidence does not substitute for browser UI qualification.
Command after successful packaging:
`node --import tsx scripts/check-cad-whole-wall-worker.mts <fresh-output-directory>`.
The entrypoint syntax check passed; actual execution awaits the active packager.

### Updated packaged WASM and actual worker checks passed

The standard geometry-WASM build/optimizer/packager completed successfully:
12203679 bytes, SHA-256
`b19235bbd01860c519c18054e462e229ce31caf3e5959ceb867c909fdec280aa`.
The packed generated literal decodes byte-identically to the public WASM.
The whole-wall native-WASM script passed all 7 control cases and 7 production
handler calls, including qualified automatic box/enclosure searches. All 4
whole-wall protocol tests passed. The actual Node-worker cancellation/Retry
check passed with 2 started and joined realms, warm qualified result, cancellation
before response admission and matching fresh-realm result. Its initially named
baseline output hash matches this newly built artifact, so it qualifies the new
WASM. The worker entrypoint has a distinct filename to avoid TS-loader `.mjs` to
`.mts` sibling resolution. Saved reports/fixtures: `wasm-current-2026-10-09/`.

These checks cover ordinary original-body whole-wall dispatch and transport;
the exact retained/corrected expression-shell recipe is still not reconstructed
by that runtime API. Browser highlights, actual UI cancellation/Retry, general
fillets, STEP/history acceptance and the remaining P0/P2/P3 scope stay open.

### Material diagnostic cancellation and browser check — 2026-10-09

Added an explicit Cancel calculation button. Cancellation clears the job, candidate, wall result, overlay and progress; the existing current-job guard prevents late result admission. Three component regressions cover pending whole-wall cancellation and retry, worker failure and retry, and active-body replacement. The selected UI/worker suite passed 11 tests; vue-tsc --noEmit and git diff --check passed.

CUA browser inspection on separate origin 5196 preserved the existing document at 5195. The exact cube primitive returned 20 … 20 mm with 21/21 face pairs. A sphere returned volume-unproven with 36/36 pairs and eight unchecked faces, displayed as orange markers. Extended sphere calculation was cancelled in the browser, then retried and returned its explicit unresolved status. Escape removed the panel. No console errors were captured. Evidence: wasm-current-2026-10-09/ui-material-cancellation/report.json and screenshots. These checks qualify the ordinary original-face pipeline only; corrected expression shell transport and general geometry completion remain open.

### Native exact profile and section-bridge recipe replay — 2026-10-09

source_profile_body::restore binds the persisted profile definition to a freshly admitted original Body, validates its version/kind/tolerance and face-anchor recipe, and reruns native preparation with caller work limits. restore_section_bridge reconstructs the exact two-section Hermite expression from owned proposal/face/axis/parameter/scales; stored certificates do not admit geometry. The annular retained_profile_recipe_replays_native_admission regression passed: definition round trip, identical interval enclosures, original-rail constraints, zero-budget refusal, invalid source/recipe rejection and ignored forged certificate. git diff --check passed. This is native reconstruction plumbing only: rooted trims, closed expression-shell assembly and new WASM/worker transport are still pending, and the currently served WASM predates these reconstruction functions.

### Native root-ended seam recipe replay — 2026-10-09

RootEndedSeam::definition persists the owned proposal and two authored section-end recipes. restore_cut_endpoint freshly encloses the original pcurve crossing; restore_root_ended_seam reruns both root splits, private carrier-point ownership and tangency qualification. Endpoint enclosure and mapping budgets are shared across both ends; no saved world bounds or certificate admits geometry. The extended retained_profile_recipe_replays_native_admission regression passed: canonical interval/owner round trip, zero cut/mapping/carrier budget refusal, invalid owner/version/recipe rejection, duplicate and incompatible ends refusal, and forged world bounds ignored. This preserves common trimmed-edge roots after reconstruction; closed expression-shell recipe replay and WASM transport remain pending.

### Incident contour and common carrier replay — 2026-10-09

restore_incident_contours rebuilds the owner-bound root seam, splits both neighboring contours at the same private crossing roots, verifies contour joins and requalifies the retained common 3D edge. It returns the boundary assembly only after all three gates succeed; it does not admit a closed shell or volume. Extended native recipe regression passed in 10.79 seconds: fragment definitions, face/middle addresses, contour cyclic joins and shared-edge definition match the fresh original construction; zero fragment and insufficient exact/driver work return no assembly. git diff --check passed. Closed transition patch/outer-edge shell replay and runtime transport remain open.

### Transition boundary replay and fresh face admission — 2026-10-09

TransitionBoundary::definition records its two exact owner-bound seam recipes. with_restored_transition_boundary restores both root seams, rebuilds the exact implicit section-width bridge, reruns original rail constraints and full corrected side regularity, and invokes inspection while native owners remain alive. A higher-ranked callback prevents returning borrowed proof attached to local replay owners. Budgets for seam replay are explicitly allocated per side.

The expanded retained_profile_recipe_replays_native_admission test passed in 27.92 seconds: all four boundary enclosures and normalized recipe match the original; duplicate sides reject; zero controls never invokes inspection. Inside replay, fresh full-domain interval injectivity from original rails admits the selected transition face (2000 cells / 100000 spans allocation); zero face work refuses. Candidate midpoint derivatives choose a projection only, not proof. geometry-bridge cargo check and git diff --check passed. This establishes native replay for one qualified annular transition fixture, not general fillet completion; retained exterior faces, outer-edge assembly, closed-shell replay and WASM transport remain pending.

### Section-contour replay through closed shell incidence — 2026-10-09

TransitionBoundary::replay_section_contours freshly reconstructs both cut endpoints and root splits for an entry/terminal section, sharing bounded endpoint/mapping work and binding the resulting complementary contours to the already replayed owner boundary. Zero endpoint, mapping and fragment budgets refuse.

The expanded retained_profile_recipe_replays_native_admission regression passed in 27.76 seconds. Inside the replay callback, both section joins and retained exterior faces requalified, including the terminal factored-pole/quotient case. Fresh transition rail incidence, section incidence and four outer shared-edge checks produced a closed 29-face, single-component expression shell. Insufficient outer-edge work and zero/one-less-than-required shell-use budgets refuse. geometry-bridge cargo check and git diff --check passed. This is restored closed incidence for the annular fixture; fresh shell embedding/self-intersection and volume qualification, general geometry, and WASM/worker exposure remain pending. The public WASM still predates these native restoration changes.

### Restored contact pipeline and oriented volume — 2026-10-09

ExpressionShell::inspect_reconstructed_contacts runs the existing source, retained-profile and transition contact qualifiers in one bounded native pipeline, sharing cumulative audit work and retaining every unresolved pair. The restored annular fixture requalified all 406 cross-face pairs: zero pending, 7228 cells, 41318 spans, 38855 domain cells, 15108246 exact-work units and four driver cells. Fresh owner-bound embedding then admitted the shell.

Fresh oriented volume qualification returned positive bounds [7049.541261407235, 7069.534459645989] mm³, with 5835 cells and 31993 spans; interval width is below the requested 20 mm³. Zero volume budget refused. The expanded restoration test passed in 57.74 seconds; geometry-bridge cargo check and git diff --check passed. Reports: restored-contact-pipeline-qualified-2026-10-09.json and restored-volume-qualified-2026-10-09.json. This is one restored annular fixture, not arbitrary NURBS/mixed-radius fillets, global wall thickness, STEP acceptance or complete P0. WASM/worker/UI exposure of expression reconstruction remains open.

### One-call native shell replay entry point — 2026-10-09

source_profile_shell_replay.rs adds with_restored_expression_shell with explicit per-stage work limits. It replays the boundary, both entry/terminal contours, section joins, retained exterior faces, transition injectivity, both rail incidences, section incidences, four outer shared edges and closed manifold incidence. A scoped higher-ranked callback keeps the full owner graph alive and permits only owned inspection outputs to escape. Incomplete stages return no shell result with the native reason. Excessive stage allocations reject before reconstruction. Embedding, oriented volume and wall admission remain separate fresh gates.

The expanded recipe regression passed in 75.27 seconds: the new single-call entry returns 29 faces and one component; no boundary controls never invokes the shell callback; excessive shell-use allocation rejects. Existing restored contact/volume qualification also passed in the same test. This is a native API prerequisite for WASM transport; no new WASM action or UI capability is claimed.


### Native expression shell dispatcher — 2026-10-09

`cad_expression_shell_restore` reconstructs the original source body, retained profiles, root-ended boundary and closed shell from recipes and explicit work limits. The saved request is `expression-shell-replay-request-2026-10-09.json`.

The geometry-bridge dispatcher regression passed: 29 faces, one connected component, closed incidence qualified. A zero profile control budget refuses reconstruction even with forged input qualification flags. The response explicitly leaves embedding and material qualification false. This qualifies the annular fixture only; it does not establish general fillets, wall thickness, or solid admission. The command has not yet been rebuilt into the served WASM or connected to the worker/UI.

Validation: native retained-profile replay regression passed in 76.05 seconds; geometry-bridge dispatcher regression passed in 32.73 seconds.


### Expression shell worker transport — 2026-10-09

Added `expressionShellRestore` to the shared worker registry, executor, client and response validation. The native response echoes the full request so recipe and budget identity can be checked before accepting metadata. Embedding/material flags must remain false; closed incidence does not authorize insertion as a solid. A zero-budget refusal carries no shell.

`vue-tsc --noEmit` passed. The expression transport and existing main-solid worker client suites passed 33 tests total. Transport tests cover changed recipes/budgets, object/array identity collisions and forged solid admission; worker client tests cover its existing cancellation/stale-reply lifecycle, and an expression replay test confirms cancellation, late-reply rejection and Retry for the new job. Actual WASM execution of the new job and its UI remain unverified.


### JavaScript numeric recipe round-trip — 2026-10-09

The first actual expression worker check failed on artifact `b304b35c8dbdfaacf5b5cc5c5e68cde1063a9ccd9035264b638bf2c8d2a18930`: JavaScript converts integral JSON floats to integer transport values, while native structural recipe equality distinguished numeric variants. No successful worker qualification report was published.

Native recipe comparison now permits exactly representable safe integers to match their identical floating value. It uses no geometric tolerance and rejects rounding beyond the safe integer range; finite floating values preserve their bit identity, including signed zero. The geometry-bridge annular replay regression with simulated JavaScript integer transport passed in 20.72 seconds. Dedicated numeric boundary tests also passed. A final WASM rebuild from these sources and actual-worker validation remain required.


The native dispatcher regression additionally rejects a transition boundary bound to a proposal with a different position tolerance. Exhausting boundary controls returns no shell. The extended JavaScript numeric round-trip regression passed in 25.92 seconds. Transport suites now pass 34 tests, including bounded topology metadata. Actual-worker qualification remains open following the first numeric variant failure.

The full retained-profile recipe replay regression passed after exact numeric identity changes in 74.36 seconds, including fresh shell contact and volume gates on the annular fixture. General fillets and global thickness remain outside this qualification.


### Actual expression replay worker and native inspection dispatcher — 2026-10-09

Actual Node worker replay passed on WASM `06acde521eb641a1efc8b5efa339f97bf4422e4160818618a48c16c8ee4a8cbf`: matching published/packed bytes, zero-profile-budget refusal, dispatch cancellation, terminated/joined realm, fresh-worker Retry, 29 faces and one closed component. See `expression-wasm-replay-2026-10-09/expression-worker-report.json`. This checks replay only; browser preview and global material/thickness are not qualified.

The new native `cad_expression_shell_inspect` dispatcher replays the shell and freshly checks projected-region authority, all cross-face contacts and signed volume on its current owners. The annular native regression and projected-region zero-budget refusal passed. Its result remains `materialQualified:false`: a volume certificate does not prove radius, wall thickness or construct an admitted solid Body. The native request/result are saved in `expression-shell-inspection-native-2026-10-09.json`. This inspection command and the final signed-zero comparison require a further WASM build and transport/UI integration.


### Expression inspection worker transport — 2026-10-09

Added `expressionShellInspect` to the worker registry, executor and client. Response validation binds full source/recipe/budget identity, checks unique complete face-pair coverage and pending histograms, finite UV products, native work limits and coherent oriented volume bounds. `materialQualified` remains false because these diagnostics do not admit a solid Body or certify radius/thickness.

The actual native request/result/refusal fixtures pass transport validation. Malformed pair coverage, nonfinite geometry, forged pending counts, oversized work and altered volume are rejected. Three transport/client suites pass 38 tests, and `vue-tsc --noEmit` passes. Actual new WASM inspection worker execution and browser integration remain unverified while the current build completes. `scripts/check-expression-inspection-worker.mts` is prepared for artifact-identity, refusal, cancellation and Retry qualification.


### Unresolved inspection and UI controller — 2026-10-09

With contact cells and exact-work budgets set to zero, fresh native replay keeps closed incidence but reports 391 pending cross-face pairs, no embedding and no volume. The expanded native inspection regression passed in 83.68 seconds. The saved pending response is `expression-shell-inspection-pending-2026-10-09.json`; transport validation accepts its incomplete status and rejects a forged embedding flag.

`useExpressionInspection` now owns a separate worker lifecycle for UI consumers: run/cancel/Retry, immediate invalidation on active/recipe changes, disposal, and full request-identity comparison before publishing a result. Its tests cover late replies after cancel, retry, failure, close and raw recipe mutation. Four controller/transport/client suites passed 40 tests; after adding the actual native pending fixture, the two focused controller/inspection suites passed 7 tests. Typecheck and diff checks passed. This controller is not yet attached to a visible panel or expression viewport; actual inspection WASM execution remains unqualified until the build and worker check complete.


### Actual WASM inspection and visible panel integration — 2026-10-09

Actual worker inspection passed on WASM `24ea250aac4203ee070e2a58e8bec3beac6b2afd58d3db81700a80b725d941d7` (12,669,436 bytes). Published and packed artifacts match. The test covers refusal, dispatch cancellation, worker termination/join, fresh-worker Retry, 29 faces, full 406-pair contact coverage with zero pending pairs and qualified signed-volume bounds. See `expression-inspection-wasm-2026-10-09/expression-inspection-worker-report.json`. This is one annular recipe and does not prove general fillets, wall thickness or a solid Body.

Source records can now retain an optional independent MGV1 inspection recipe, strictly bound to the original body. Original source admission is preserved; saved diagnostic flags are not authority. `ExpressionShellCheck.vue` is attached to selected source bodies carrying this recipe in the existing file/source controls. It displays incidence, pending contacts and volume, provides cancel/Esc and Retry, and maps selected pending pairs to highlighted original source faces. The panel explicitly states that the scene still shows the original body; expression viewport rendering remains open.

Archive/controller/inspection transport suites pass 9 tests and typecheck passes. `scripts/prepare-expression-inspection-ui.mts` generates a native document fixture for the upcoming isolated browser check. Visual panel behavior is not yet qualified.


### Browser inspection panel qualification — 2026-10-09

On isolated origin 5197, the actual browser imported the native expression archive, selected its source edge by keyboard and ran the inspection panel. Manual cancel cleared the result; Retry qualified 29 faces, 406 cross-face pairs with zero pending and the native volume interval. The limited-budget recipe instead showed 391 pending pairs and unqualified volume. Selecting pair 0↔1 highlights both original faces, including unresolved display boxes. Esc cancels, and reload restores the saved recipe for a fresh native rerun. Inspection-tab console error capture was empty. The main user document at 5195 was preserved; the temporary tab/server were closed.

Visual review prompted two UI corrections: a compact selector for all pending pairs replaces the long button list, and unresolved source-face boxes participate in highlighting. Display volume is explicitly rounded (≈); exact native endpoints remain in Details. Five selected suites passed 43 tests; final typecheck and diff checks passed. Screenshots and report are in `expression-inspection-ui-2026-10-09/`. This qualifies one recipe and this panel only. The viewport still shows the original body; the transition-shell preview and the full roadmap remain open.


### Native display enclosure groundwork — 2026-10-09

Added bounded support-domain display enclosures to the native expression shell and optional `displayLimits` on replay/inspection commands. All original, retained and transition faces are covered; work exhaustion preserves complete unsampled face domains instead of silently dropping them. Output explicitly carries `trimQualified:false`: support rectangles may include trimmed-out material and are not a mesh or a solid certificate.

On the annular dispatcher fixture, a 2×2 partition of all 29 faces produced 116 enclosed cells. Zero cell budget preserved all 29 faces with null bounds; oversized divisions were rejected, full request identity was echoed and display did not promote embedding admission. The expanded dispatcher regression passed in 101.55 seconds. The grid endpoint/shared-coordinate regression passed. Diff checks passed. This new native API is not in the current published WASM yet; bounded transport validation and scene rendering remain open, as does native trim-aware triangulation.


### Native trim classification for display — 2026-10-09

Display cells now carry inside/outside/unresolved classification from current root-ended source contours, with a separate bounded winding budget. Only an equivalent original face with admitted whole-chart material, or the admitted rectangular transition chart, bypasses contour work. Retained support identity alone does not authorize a whole-chart trim. Exhausted faces preserve unresolved coverage; display remains `trimQualified:false` because no boundary triangulation or solid admission is produced.

The expanded native annular dispatcher regression passed in 78.28 seconds, checking 116 enclosures, finite ordered bounds, positive classification work, unresolved boundary cells, aggregate coverage, explicit zero-budget face coverage and oversized-budget refusal. Five transport/archive/controller suites passed 19 tests, including malformed/nonfinite intervals, duplicate or missing faces, forged counts, work limits, unsolicited display responses and stale display-budget identity. Typecheck and diff checks passed. Native display work is ready for the new WASM build; scene rendering and trim-aware boundary tessellation remain open.


### Display UI wiring awaiting packaged runtime — 2026-10-09

The inspection panel requests a bounded 4×4 native display partition and can emit validated display data to the scene. A switch shows interval boxes, filters native outside cells and marks unresolved trim in orange. Cancellation and invalidated results clear the overlay through the existing independent controller lifecycle. This is diagnostic region display, not trim-aware triangles; the original body remains visible. Vue typecheck passed. The real-worker checker has an optional `--display` mode for the 464-cell annular fixture. The WASM build is still running its optimizer; actual new worker execution and this overlay's visual behavior remain unqualified.


### Actual display WASM, worker and browser qualification — 2026-10-09

The new published and packed WASM is `36057c96090d7d6d923d49c117290070519cdc36c69ec5d32347602b65315101` (12,676,396 bytes). The production Node 24.19.0 worker handler/client passed refusal, dispatch cancellation, terminated/joined workers and fresh-worker Retry with optional display work. On the annular recipe, all 464 cell position enclosures were returned: 248 inside, 120 outside and 96 unresolved trim, spending 960 support spans and 2,839 winding cells. Native contact coverage remained 406 pairs with no pending pairs and volume qualified. Unresolved trim is display classification, not a detected self-intersection. See `expression-display-wasm-2026-10-09/expression-inspection-worker-report.json`.

The actual in-app browser loaded this artifact on isolated origin 5197 and showed 344 interval boxes, omitting the 120 native outside cells. Turning the switch off, starting another calculation, pressing Esc and closing the panel each clear the overlay. A fresh calculation restores the same boxes and qualified contact/volume result. Inspection-tab console capture was empty. Screenshots and scope report are in `expression-display-ui-2026-10-09/`. The user document/server on 5195 was preserved. This qualifies diagnostic box rendering and its lifecycle for one recipe, not a transition mesh. Boundary triangulation, mesh deviation control, general fillets, global thickness and the broader roadmap remain open.


### Adaptive native display mesh with bounded deviation — 2026-10-09

Added adaptive breadth-first subdivision on every owned expression face. Only entire rectangles classified as material produce two display triangles. Native singleton position enclosures provide rounded corners, shared within each face. An outward interval L1 diameter bounds the pointwise Euclidean distance between the exact support and affine triangles, including rounded vertices. Unresolved trim, vertex/support enclosure failure, depth limits and exhausted work preserve pending domains. This is not a watertight mesh or a replacement for exact topology; cross-face welding and boundary clipping remain open.

Three native algorithm regressions passed: curved polynomial patches with pointwise samples and shared vertex identity, complete domain coverage after cell/span/control/depth limits, and explicit unresolved trim bands. The actual annular dispatcher regression passed in 60.58 seconds. With an intentionally coarse 8 mm target and 4,096 cells, it produced 304 tiles (608 triangles), including 40 transition tiles, with maximum reported deviation 7.94598576681575 mm. It preserved 3,120 pending domains and excluded 168 outside domains. The 3,592 leaves equal 29 original face domains plus 3,563 binary splits. Work was 7,977 support spans and the full 30,000 winding-cell budget. Zero mesh-cell budget preserves all 29 faces pending and ignores forged qualification flags. Saved request/result: `expression-mesh-native-2026-10-09.json`.

Six selected transport/archive/controller suites passed 23 tests, including native partial mesh acceptance, missing leaves, duplicate domains, coordinate/deviation corruption, work counters, stale request identity and unsolicited triangles. Typecheck and diff checks passed. The new mesh API is not in the currently published WASM (which still qualifies diagnostic boxes only). Practical mesh precision, tighter error bounds, boundary triangulation, cross-face welding, new WASM/worker/viewport integration and the full CAD roadmap remain unfinished.


### Derivative residual precision qualification — 2026-10-09

Native mesh error now uses outward enclosures of the actual support derivatives minus each affine triangle gradient, including the anchor enclosure and rounded vertex error. Continuous nonperiodic validated NURBS supports enable this tighter bound; periodic supports retain the support-box fallback. Seven focused native algorithm tests passed. The annular dispatcher regression passed in 55.91 seconds with a 0.05 mm target: 881 tiles (1,762 triangles), including 179 transition tiles, maximum error upper bound 0.049955340660326926 mm. There are 6,123 pending work-limit domains and 168 outside domains; winding work exhausted its 30,000-cell budget. Pending domains are unverified coverage, not detected defects. Exact boundary clipping, welding and solid admission remain unfinished.

The saved precision request/result is `expression-mesh-precision-native-2026-10-09.json`. All five mesh transport tests passed, including acceptance of this actual native result, rejection of the stale coarse response and an error exceeding 0.05 mm. The production worker checker now accepts `--mesh` with these same limits; execution awaits a new packaged WASM. The current published artifact still provides diagnostic boxes only. No new worker or browser mesh qualification is claimed.


### Mesh preview UI wiring awaiting packaged execution — 2026-10-09

The inspection panel can request native 0.05 mm mesh certificates with standard (8,192 support cells / 30,000 winding cells / depth 14) or extended (32,768 / 100,000 / depth 18) work budgets. Both retain a 100,000-span cap. Changing budgets invalidates the current result through the existing controller. A switch renders only validated native tile corners; it does not compute geometry or certificates in TypeScript. A separate selector can highlight one pending domain's native position enclosure in orange. Missing native coordinates disable highlighting. Cancellation, recipe changes and closed inspection panels clear emitted mesh data.

Six transport/archive/controller suites passed 25 tests, including a partial mesh response, cancellation and a late answer with stale precision limits. Direct Vue and MCP TypeScript checks passed, including the budget controls and pending-region selector. The new WASM is still optimizing, so actual mesh worker execution and browser rendering remain unqualified. Partial preview does not close trimmed boundaries, weld neighboring faces or admit a material Body.


### Actual mesh WASM, worker and browser — 2026-10-09

The packaged/published WASM is 12,701,819 bytes, SHA256 `1e4099284758502a394d2bbed7a67e183c7c71e2a5fa789cefbe608d4bce55fd`. Production Node worker qualification with `--display --mesh` passed, including dispatch cancellation, joined workers and fresh Retry. Timings were 969.877 ms refusal, 31.455 ms cancellation including worker exit, and 12,595.440 ms Retry. It reproduced 881 native tiles (1,762 triangles), 6,123 pending domains, 168 outside domains and maximum error 0.049955340660326926 mm. These timings cover the client/worker operation, not individual Rust stages.

The actual in-app browser on isolated origin 5197 rendered 1,762 triangles and one selected pending enclosure. Hiding the mesh clears triangles; starting another inspection clears triangles and pending highlights. A fresh result restores 1,762 triangles. Escape on the focused cancel button cancels the pending request and leaves zero triangles with inspection enabled. Inspection console capture was empty. The compact panel was corrected to wrap long bounds and keep selects within its width. Screenshots and scope report are in `expression-mesh-ui-2026-10-09/`; worker report is in `expression-mesh-wasm-2026-10-09/`. Temporary tab and server were closed; user origin 5195 was preserved. Exact boundary clipping, cross-face welding, complete material-domain coverage and Body admission remain open.


### Extended preview budget qualification — 2026-10-09

The same production WASM/worker passed `--display --mesh --extended-mesh`. Fresh Retry took 13,629.490 ms, producing 6,619 tiles (13,238 triangles), 8,252 pending work-limit domains and 404 outside domains. Maximum error upper bound was 0.04998378468685881 mm. It spent 22,269 of 32,768 support cells, all 100,000 support spans and all 100,000 winding cells; 15,275 leaves equal 29 original face domains plus 15,246 splits. Both standard and extended runs cover the entire rectangular transition chart (face 28) with accepted tiles. Retained and original trims remain partial.

Pending leaf counts grow with subdivision and must not be interpreted as defect counts or remaining physical surface area. Summing UV rectangle areas per face and averaging normalized fractions across the 29 charts gives accepted parameter-domain coverage 0.27875808189655166 (standard) and 0.5425141433189655 (extended). This is a parameter-domain reporting metric, not a physical surface-area measurement or trim/solid certificate. Current exhausted budgets obscure whether each pending domain is an actual trim-boundary crossing. Exact trimming and shared boundary vertices remain the next geometry work, along with better native work accounting.

Report: `expression-mesh-extended-wasm-2026-10-09/expression-inspection-worker-report.json`. A transport regression consumes that actual production response and rejects mismatched winding limits.


### Canonical shared boundary sampling foundation — 2026-10-09

Added Rust `SharedEdge::sample_interior`: a sample retains a borrow of its privately admitted shared edge, one normalized canonical world parameter, inverse parameter enclosures and UV enclosures on both original uses, and one canonical world enclosure. The entire inverse interval must lie strictly inside both actual retained fragments. Uncertain root caps and endpoints are refused rather than rounded. Span exhaustion returns a pending report with accounted work. Unsupported incomplete original source mappings are refused. This is positional ownership for future shared mesh vertices, not triangulation or Body admission.

All 11 shared-edge native tests passed. New checks cover opposite traversal, fixed trimmed ranges, endpoints, out-of-range samples, zero/insufficient work and nonfinite input. Existing polynomial and rational curved-support fixtures with independently qualified root ends now also check a shared interior sample and refusal past the retained root. The initial test used the wrong qualifier orientation flag; it was corrected to the forward-pcurve convention before the passing run. This API is not yet consumed by display meshing or the published WASM. Shared boundary subdivision, root endpoint vertices, clipping and cross-face welding remain unfinished.


### Shared interior boundary chain — 2026-10-09

Added bounded `SharedEdge::sample_interior_chain`. Strictly increasing canonical parameters identify one shared ordered point list for both incident faces. The operation preserves every requested point, including retained-domain refusal and span exhaustion; no unverified point silently disappears. Duplicate, reversed, nonfinite and out-of-domain requests are rejected before work. The chain is discrete positional evidence, not a complete curve deviation or topology certificate.

All 11 shared-edge tests passed after the addition. A five-point trimmed-line request with six span units returns two confirmed interior points and three explicit pending points; a zero-work request preserves all three points pending. Root endpoint sampling, adaptive chord error certification, mesh boundary clipping and using these chains in the shell mesher remain open. Published WASM remains the previously qualified mesh artifact; this new chain API is native-only until integration and rebuild.


### Shared root endpoint enclosures — 2026-10-09

Added Rust `SharedEdge::sample_endpoint`. The first use's directed endpoint is paired with the opposite endpoint of the second use through the privately admitted shared edge. Both original root/parameter definitions remain owned by that edge. Canonical parameter intervals are mapped from both uses and intersected only to tighten enclosures; overlap is not the identity proof. Native evaluations return UV intervals on both charts and one canonical world interval. No Cartesian vertex, midpoint root or rounded welding is manufactured. Unsupported original maps and exhausted span work retain explicit refusal.

All 11 shared-edge tests passed. Fixed trimmed endpoints check directed pairing and world bounds, invalid endpoint indices and exhausted work. Polynomial and rational curved-sheet fixtures check the paired irrational root endpoint, retention of each original parameter interval and a nonzero canonical root interval. These source qualifiers still reject wrong world curves and ambiguous endpoint roots. The new endpoint/chain APIs are not yet connected to shell triangulation or packaged WASM. Chord error certification, shared boundary vertex emission, exact trim clipping and closed mesh admission remain open.


### Native curve-to-affine error bound — 2026-10-09

Added `interval_eval::curve_affine_error_upper` in nurbs-core. For every nonempty knot interval it restricts the original homogeneous Bernstein controls, elevates numerator/weight to degree p+1, subtracts the weight multiplied by the prescribed affine segment, and bounds the resulting positive-weight rational residual. The outward L1 residual norm bounds pointwise Euclidean deviation and includes supplied vertex rounding. Exhausted span work returns no bound. It does not infer curve/segment equality from sampled values.

Seven interval-evaluation tests passed, including the three new error-bound regressions: a 1,000-unit straight line has a near-roundoff bound, a displaced endpoint includes its perturbation, a rational bend and restricted subinterval are bounded, and all spans of a piecewise line must be covered. Invalid/nonfinite domains and insufficient budgets are refused. Numerical point samples are regression checks only; the certificate comes from native interval controls. Adaptive shared-edge subdivision and uncertain root endpoint cap handling are still pending, and this API is not yet packaged or connected to display meshing.

### Shared adaptive boundary partition — 2026-10-09

Native `SharedEdge::prepare_boundary_mesh` now creates one vertex list for both incident uses. Original root endpoint parameter intervals are retained. Polynomial and rational curved support fixtures subdivide to a 0.01 mm conservative chord deviation; internal vertex indices have exactly two segment incidences and both end indices have one. A restricted straight edge needs one segment. Zero cells, zero spans, and an exhausted cell budget preserve explicit pending coverage.

Validation: `cargo check` succeeded; `cargo test --manifest-path crates/brep-core/Cargo.toml source_shared_edge:: --lib` passed all 11 tests, including the new assertions; `git diff --check` passed. These are native fixture checks. Face clipping, cross-edge root vertex links, conforming face triangulation, shell mesh consumption, and publication in WASM remain unfinished. The current published WASM does not contain this boundary partition.

### Shell boundary vertex identities and source partitions

Shell assembly retains dense endpoint IDs from its qualified union classes. `boundary_vertex(address,end)` rejects invalid addresses/ends and never compares Cartesian positions. The annular native recipe replay passed (1 test, 74.70 s), checking contour closure, paired opposite endpoints, pole collapse, and the complete vertex count.

`prepare_source_boundary_mesh` connects original/outer shared carriers to those endpoint IDs under one global cell/span budget. Every shell pair remains in the report; rail/section expression carriers explicitly have no partition yet. Cargo check passed. The extended annular replay covering source partitions is running; face clipping and shared face triangulation remain unfinished.

The extended native source-boundary replay passed (1 test, 75.69 s), checking all shell pair entries, endpoint IDs and aggregate budgets. Rail carriers are now partitioned using their qualified shared source curves; the report records native UV chart side indices and carrier endpoint orientation explicitly. The transition chart itself remains unqualified for this partition. Section carriers remain explicit pending entries. Rail changes compile; their extended replay is running separately.

Rail partition replay passed (1 test, 83.50 s). Each rail now also carries interval UV coordinates in shell-use order: the selected original chart maps its section-axis coordinate to normalized across parameter, while the free coordinate stays at its owned low/high range endpoint. This uses the original-rail-constrained bridge and the existing qualified rail ownership, without fitting or inversion by sample proximity. Cargo check passed. A separate replay checks transition enclosure overlap at every shared vertex; it is still running. These checks do not yet prove a closed face triangulation.

The rail UV replay passed (1 test, 76.73 s), including every partition vertex's transition enclosure overlap. A new native section partition now uses the admitted profile section range and actual retained traversal direction. It supplies retained/transition UV intervals, preserves both shell endpoint IDs, and bounds bidirectional chord deviation using an outward box diameter including rounded vertices. Cell/span/depth exhaustion preserves pending segments. Cargo check passed; the extended section replay is running. This conservative section bound is not yet an optimized residual bound or a closed triangulation certificate.

The section replay passed (1 test, 75.42 s), checking both section partitions, actual shell-use UV arrays, and transition enclosure overlap. A native boundary graph now merges endpoint positions only by qualified shell vertex IDs, intersects their proven enclosures, and leaves any conflict pending. Interior vertices remain pair-local. Rounding shifts introduced by the common endpoint position are added conservatively to each segment's deviation; segments exceeding the target remain pending. The graph carries UV intervals in both shell charts. Its annular replay is running; this graph does not yet contain face triangles.

The shared boundary graph replay passed (1 test, 79.35 s). Every shell vertex had a common position enclosure; no shared-endpoint conflict was reported. Face contour ordering now reuses graph segment indices, validates endpoint incidence, and checks loop closure. A focused chain regression passed for forward/reverse traversal and rejection of missing, branched and detached constraints. The annular contour replay is running. Closed single-edge cycles remain explicitly unqualified; these ordered constraints are preparation for triangulation, not face triangles or a solid admission.

The annular ordered-contour replay passed (1 test, 81.40 s). Boundary-preserving face triangulation proposals now use the existing native planar triangulator, retain shared graph IDs, and reject omitted boundary segments, ambiguous UV vertex identity, or nonmanifold edge counts. Holes are passed to the same triangulator. Every face is represented by proposed triangles or a pending reason. Cargo check passed; the annular candidate replay is running. Surface deviation, exact trim correspondence, oriented global incidence, and solid admission remain separate unfinished gates.

Face triangulation candidate replay passed (1 test, 78.94 s), checking per-face coverage/pending accounting and every proposed boundary edge. Native triangle residual error now uses interval affine slopes in arbitrary skew UV coordinates, support derivatives over the enclosing rectangle, and the original support anchor including rounded graph positions. Degenerate determinants and insufficient work refuse the bound. Two focused tests passed: affine skew triangles, rounding error, degenerate UV, and an interior bow invisible at the three vertices. The annular numerical-bound replay is running. This numerical bound neither classifies exact trim nor admits material topology.

The first annular triangle-error replay failed on its zero-budget regression: original NURBS support evaluation rejected zero spans. The numerical triangle gate now returns explicit unqualified/zero work after checking the owned domain, rather than issuing the invalid query. A native centroid refinement stage preserves boundary edges, checks interval orientation of each split, and admits only triangles whose exact trim classification is Inside and whose numerical error meets the target. Work/depth/trim failures preserve pending triangles. Refinement compiles; corrected annular replay is running. This stage still needs better interior-edge refinement and boundary trim-band admission before full mesh closure.

Corrected numerical error and interior refinement replay passed (1 test, 77.55 s). Candidate face orientation now follows actual ordered shell constraints, with opposite internal edge directions. A native triangle incidence audit counts open/nonmanifold edges, reversed neighbors, duplicate/degenerate triangles, and disconnected vertex links. The tetrahedron regression passed and correctly rejected an open tetrahedron, reversed face, duplicate face, and two closed fans pinched at one vertex. The annular orientation replay is running. This is an incidence audit only; it does not certify geometric embedding, material or a complete mesh while faces remain pending.

The annular oriented candidate replay passed (1 test, 76.23 s): 2564 triangles, zero pending faces, zero open edges, zero nonmanifold edges, zero orientation conflicts and zero invalid vertex links. All original shared boundary constraints are retained. This proves closed oriented combinatorial incidence for this fixture, not the final surface tolerance or exact curved trim correspondence. A separate extended 0.05 mm refinement run now uses 8192 cells, 100000 spans and 100000 trim cells and emits a native JSON report with per-face/per-pair pending reasons. The extended run is active.

The extended refinement initially failed after 78.99 s with `NURBS_NUMERIC_ERROR: Distance denominator is not separated from zero`. It did not establish full geometric qualification. Refinement now preserves that triangle with an explicit numeric-unproven reason. Because the lower-level exception lacks spent-work accounting, the entire issued span allowance is conservatively charged; other leaves remain pending when exhausted. Refinement also checks the leaf count identity (initial triangles + two per split) and audits the incidence of all accepted/pending leaves. The corrected extended run is active. Removing the denominator ambiguity and improving low-level spent-work reporting remain necessary for full coverage.

Correction to the denominator diagnosis: the triangle affine-slope code used positive-only interval division for a signed UV determinant. Reversing a face made that determinant negative, so this was an implementation error rather than proof of a singular surface. Both affine slope quotients now use `div_signed`; the existing determinant separation test still rejects true degeneracy. Both triangle error tests passed, including the added reverse-orientation regression. The older active extended replay was compiled before this fix; its numeric-pending results must not be used as current geometry limits. A fresh extended run follows its completion.

Signed-determinant extended refinement passed in 127.67 s: 2564 initial triangles, 2540 splits, 2541 evaluated cells, 100000 spans, 9657 trim cells; all 7644 leaves remain work-limit pending and zero are geometrically admitted. The complete leaf incidence remains closed/oriented. This is preserved topology, not successful 0.05 mm surface qualification. Breadth-first centroid refinement exhausted the span budget before evaluating children; long inherited interior edges also require conforming edge bisection. The native report is `boundary-triangle-refinement-2026-10-09.json`.

The geometry bridge now accepts optional `boundaryMeshLimits`, binds them into the echoed request, and returns the native graph, both UV charts, candidate triangles and pending reasons. Closed oriented incidence is reported separately from material and surface-tolerance qualification (both false). Geometry bridge cargo check passed. A fresh packaged WASM, worker transport validation, and UI consumption are still pending.

Native conforming interior-edge bisection now chooses long interior edges, verifies both incident uses have the same face and UV endpoint identities, evaluates one original support midpoint, and splits both neighbors using one graph vertex. Shared CAD boundary constraints are excluded. The focused midpoint/edge-incidence regression passed. Numerical refinement can now consume the subdivided candidate set directly. The annular replay requests 32 interior splits and verifies closed oriented incidence before checking the resulting candidates; that replay is active. Surface/trim qualification remains separate.

The annular 32-edge bisection and downstream refinement replay passed (1 test, 84.74 s), preserving closed oriented incidence. Bisection now additionally requires interval-separated UV orientation for every child matching its parent; the focused test passed for forward/reversed and degenerate UV configurations. A new packaged geometry WASM build has started to expose the native boundary graph through the optional bridge request. WASM/worker/UI verification is still pending.

Boundary mesh transport now validates native request identity, limits, finite coordinates, indexed references, per-edge tolerance, explicit unqualified material/surface flags and incidence/pending consistency. Vue typecheck passed and nine selected transport tests passed. The UI can request the shared boundary graph and display its preliminary triangles separately from tolerance-qualified display tiles, with explicit surface/trim limitations. Pending/cancel/replacement clears the emitted boundary preview. The UI typecheck passed; fresh packaged WASM, actual worker response validation and browser visual checks remain pending while the build runs.

Seven controller/transport tests passed, including an explicit shared-boundary response: cancel removes the result and a late reply with stale boundary cell limits is rejected. This uses a synthetic transport fixture, not geometric evidence. The production worker checker now requests the native boundary graph and independently checks all 29 face entries, zero incidence defects, and separate false material/surface-tolerance flags. Its actual run awaits the current wasm-opt process.


### Packaged shared boundary mesh, 2026-10-09

WASM packaging completed: SHA256 `e6bdada2d184c4d9546b0b5723057d0e11de0e921fc11fb2fd5a72f19beb4140`, 12,769,971 bytes. The actual worker checker with `--boundary-mesh` passed, including cancellation/join and Retry in a fresh worker. Its report is in `boundary-mesh-wasm-2026-10-09/expression-inspection-worker-report.json`. All 29 fixture faces are represented; boundary and face pending lists and incidence defects are empty. These are fixture and combinatorial checks, not surface tolerance or material admission.

In the localhost UI, requesting and showing the preliminary boundary mesh produced 2,564 SVG triangles. The UI explicitly states that surface tolerance and trimming remain unconfirmed. Starting a new request immediately cleared the triangles; Escape on the cancel control left zero triangles. MCP TypeScript check and diff whitespace check passed.

The UI retry after Escape completed again with 2,564 candidate triangles. Screenshot: `boundary-mesh-wasm-2026-10-09/ui-preview.png`. No full surface/trim qualification is claimed.


### Triangle directional error bound, 2026-10-09

The native mean-value bound now takes the hull of the three vertex directional residuals instead of multiplying gradient intervals by an independent enclosing UV rectangle. For every enclosed gradient, a linear directional residual has extrema at triangle vertices; outward interval operations retain anchor/mesh rounding and either orientation. A skew affine regression with rounded mesh vertices retains the 0.1 discrepancy without inheriting unrelated box corners. Three error-bound tests and all five triangle-related tests passed.

Extended annular replay passed in 128.63 seconds; report `triangle-directional-refinement-2026-10-09.json`. With the existing 32 paired interior bisections, 2,628 initial triangles, 2,540 centroid splits, 2,541 examined cells and 100,000 spans produced zero accepted and 7,708 pending leaves, all work-limit pending. Leaf incidence remains closed and oriented. This does not qualify surface tolerance or trim; the scheduling and subdivision bottleneck remains. The published WASM predates this native bound update.


### Per-query native refinement allowance, 2026-10-09

Added `spans_per_query` independently of total spans. Support error and centroid queries receive at most that issued allowance. Numeric exceptions conservatively charge only their issued allowance, keep the leaf pending and allow subsequent leaves to proceed; centroid numeric errors now follow the same policy rather than aborting the report. A zero-query native replay asserts zero cells/spans/domain work/splits, no graph mutation, full pending coverage and closed oriented incidence.

Native check and all five triangle tests passed. Extended replay passed in 129.62 seconds with a 64-span query cap and unchanged total work: 0 accepted, 7708 pending, 100000 spans. Leaf incidence remains closed. Report: `triangle-bounded-query-refinement-2026-10-09.json`. The cap is resource isolation, not proof that refinement converges; surface and trim qualification remains incomplete. This change has not been packaged into WASM.


### Incremental conforming interior bisection, 2026-10-09

Interior bisection now maintains triangle-edge adjacency locally and selects candidate lengths from a heap rather than rebuilding and rescanning the whole adjacency table after each split. Length is a subdivision proposal only; it never certifies approximation error. Each split preserves both original triangle orientations, inserts one original-support midpoint shared by both neighbors, and excludes every admitted CAD boundary edge. Qualification split count can be selected with `CAD_BOUNDARY_INTERIOR_SPLITS`; default remains 32.

The paired-neighbor regression passed. Extended annular replay with 512 requested splits passed in 139.83 seconds: all 512 completed, 512 support spans, zero refused edges. Subsequent 100,000-span refinement of 3,588 candidates accepted one triangle at the 0.05 mm target and exact trim gate; 8,665 leaves remain work-limit pending after 2,539 centroid splits. Closed oriented leaf incidence is preserved. The pending count increased with the larger initial mesh and is not an improvement claim. Report: `triangle-conforming-512-refinement-2026-10-09.json`. This supplies first fixture-level joint trim/tolerance acceptance, not a qualified full shell. Full convergence and geometric admission remain incomplete; WASM has not been rebuilt for these changes.


### Adaptive numerical interior bisection, 2026-10-09

Added `bisect_candidate_deviation_edges`: candidate support-error bounds are queried lazily and cached per unchanged triangle. An interior edge whose two uses already satisfy the numerical target is skipped. Replacing either triangle invalidates its cached bound; new children remain unexamined until selected. Every query has its own issued span cap, and numeric exceptions charge only that cap. This gate intentionally proves no material-trim coverage. Native zero-query replay asserts no queries, spans, splits or graph mutation and unchanged incidence.

Paired-neighbor and all five triangle tests passed, as did geometry-bridge check. Extended 512-split adaptive annular replay passed in 167.42 seconds: 512 splits, 1,236 numerical queries, zero numerically unproven queries, zero refused edges, 4,220 bisection spans. Subsequent refinement retained closed oriented leaf incidence but accepted zero triangles; 8,668 leaves remained pending at the 100,000-span work limit. Report: `triangle-adaptive-512-refinement-2026-10-09.json`. This is weaker acceptance than the length-only run (one accepted) and does not improve full qualification. Adaptive selection also needs trim uncertainty; numerical tolerance alone must not stop material-region refinement. The native API has not been packaged into WASM.


### Joint trim/deviation adaptive bisection, 2026-10-09

Added `AdaptiveInteriorLimits` and `bisect_candidate_trimmed_edges`, exported through the source-profile facade. A candidate edge is skipped only when both uses satisfy the numerical target and their complete UV rectangles classify Inside the current material. Outside/Unresolved regions are kept and remain eligible. Trim and numerical caches invalidate on triangle replacement. Trim classification has separate total/per-query budgets; numeric exceptions charge the issued trim allowance and stay Unresolved. Zero-query fixture checks preserve the graph, full candidate coverage and incidence with zero numerical/trim calls.

Neighbor regression, all five triangle tests, bridge check and whitespace check passed. Extended 512-split joint replay passed in 149.06 seconds: 512 splits, zero refusals, 1,024 numerical queries (none unproven), 3,584 support spans; 1,024 trim queries, 26,195 trim cells, 644 unresolved query results. Subsequent refinement accepted one triangle and left 8,665 work-limit pending leaves with closed oriented incidence. Report: `triangle-trim-adaptive-512-refinement-2026-10-09.json`. Acceptance matches the earlier length-only fixture and remains incomplete. Next scheduling work must examine all current candidates before spending the remaining support budget on centroid subdivisions. These native changes are not in the published WASM.


### Inspect-first triangle refinement, 2026-10-09

Refinement now keeps a separate deferred-subdivision queue. All current candidates receive their trim/tolerance inspection before midpoint queries are issued; failed candidates remain queued, and accepted triangles keep their shared edges unchanged. `initial_checked` reports actual inspected input triangles. Native replay asserts that no splits occur if resource exhaustion prevents a complete initial pass; zero-query checks include zero initial checks. All leaf identities and coverage accounting remain intact.

All five triangle tests, bridge check and whitespace check passed. The extended trim-adaptive 512-split replay passed in 144.00 seconds: all 3,588 original candidates were inspected; 239 leaves passed both the 0.05 mm numerical bound and exact trim gate. After 918 centroid splits, 5,185 leaves remain pending (5,183 work-limit, two numerical-unproven); the domain budget reached 100,000 cells, using 78,349 support spans and 6,340 examined cells. Closed oriented leaf incidence remains true. Report: `triangle-two-phase-512-refinement-2026-10-09.json`. This improves fixture-level coverage over the prior one accepted leaf, but does not qualify the full shell, radius, tangency, wall thickness or all requested geometry classes. Published WASM predates this native scheduling change.


### Inherited material rectangles and numeric localization, 2026-10-09

A descendant inherits Inside only when its complete rounded UV rectangle lies inside its proved parent rectangle on the same immutable face. Centroids are clamped to their parent box; interval orientation still refuses degenerate children. Proven Outside triangles are retained as explicit pending leaves rather than subdivided or dropped. Exhausted trim work does not block already proved Inside descendants. Reports now include inherited classification count and numeric failure face/indices/UV/stage/code/message. Replay checks every numerical pending leaf has a matching failure record.

Five triangle tests, one contour-winding test, bridge check and whitespace check passed. Extended replay passed in 144.66 seconds: all 3,588 initial triangles inspected, 1,086 classifications inherited, 255 accepted; 5,475 pending after 1,071 splits, with closed oriented incidence. Both total trim and support budgets reached 100,000. Report: `triangle-trim-inheritance-512-refinement-2026-10-09.json`. Two numeric failures were localized on face 0 in support-bound evaluation, each including UV `2e-323` and `Distance interval exceeded numeric range`. Subsequent arithmetic repair is being qualified separately; no full-shell or packaged-WASM qualification is claimed here.


### Subnormal quotient repair, 2026-10-09

Positive-denominator interval division preserves its previous reciprocal/multiplication path when the outward reciprocal is finite. If that reciprocal overflows, it computes the four endpoint quotients directly and rounds their extrema outward. This admits finite subnormal/subnormal ratios while retaining refusals for a denominator crossing zero or a genuinely unrepresentable quotient. Signed division shares the same repair. Regression cases cover exact positive/negative ratios, signed denominators, zero numerator, actual overflow refusal, and a planar surface derivative queried at parameter `f64::from_bits(4)` (the observed near-zero scale).

The actual extended annular replay passed in 127.54 seconds. Both previously captured support-bound failures disappeared: `numericFailures` is empty in `triangle-subnormal-quotient-512-refinement-2026-10-09.json`. All 3,588 original triangles were examined; 255 accepted, 5,477 pending (5,474 overall-work limit and three trim-work limit), 1,072 splits and 1,087 inherited trim proofs. Closed oriented incidence is preserved. The larger pending count follows additional subdivisions after numeric refusals were removed and is not full-shell qualification. Native WASM packaging and UI verification of these changes remain outstanding.

Final post-regression NURBS library suite: 828 passed, two ignored, 22.03 seconds. Geometry-bridge check and whitespace check passed. A fresh WASM build was started after these native checks; packaging and real worker/UI qualification are still pending.


### Fresh WASM and inspection disposal qualification, 2026-10-09

Packaging completed successfully: 12,770,564 bytes, SHA256 `5f6001640ee46ec3640d980ed4292c58419fc6b1f8100b4b909ccc4901fbc9df`. The actual worker checker passed using two worker realms: cancellation and fresh-worker retry, 29 inspected faces, contact and candidate-boundary checks. Its report is `subnormal-quotient-wasm-2026-10-09/expression-inspection-worker-report.json`. Native triangle refinement acceptance is not exposed by this boundary-mesh bridge.

The isolated localhost:5197 UI restored the saved annular fixture, accepted source-edge keyboard selection, canceled an active calculation with Escape, and completed a second calculation. Enabling boundary display produced exactly 2,564 DOM triangles. A subsequent request immediately cleared all displayed triangles; Escape restored the run button. The UI explicitly retained the warning that surface tolerance and trimming are unqualified. Screenshot: `subnormal-quotient-wasm-2026-10-09/ui-preview.png`. These checks qualify this fixture and inspection path, not the complete command matrix or all geometry classes.

A controller regression reproduced a stale Retry starting another worker request after Vue scope disposal. The permanent disposed guard now prevents it. Late success and late rejection cases both leave result/error/pending cleared and start no additional request. Controller and mesh transport suites: 15 passed across three files; Vue type checking and whitespace checks passed. This does not establish the cause of the reported browser renderer crash. Reopening localhost:5195 restored the model and displayed 60 FPS without captured console errors, but crash prevention remains unproven.


### Exact rectangle proof reuse, 2026-10-09

Triangle refinement now caches only proved Inside/Outside results for bit-identical UV rectangles on the same immutable face, within one invocation and one tolerance. Unresolved results are not cached; different faces and domains cannot share proofs. Cache hits charge no new domain work and are reported separately from parent inheritance. The zero-query replay also asserts zero cache hits.

Bridge compilation, whitespace check and full retained-recipe replay passed (214.95 seconds). Report: `triangle-trim-proof-cache-2026-10-09.json`. There were 203 exact proof reuses, all 3,588 initial triangles inspected, 255 accepted, 5,479 pending after 1,073 splits, no numerical failures, and closed oriented leaf incidence. Both work budgets still reached 100,000. Acceptance did not improve over the preceding quotient-repair fixture; no speed improvement is established from this run. The next geometry work must address unresolved material bands and support bounds rather than rely on exact-domain reuse alone. This native change has not been repackaged into WASM.


### Spatial triangle error norm, 2026-10-09

The mean-value bound now combines coordinate residual bounds with an outward Euclidean norm, scaled before squaring to avoid range loss on tiny or large models. The preceding L1 upper bound is retained as a second upper bound; their minimum remains conservative. Four native regressions passed, including affine/skew triangles, interior bowing and known 3-4-5 residuals at scales 1e-150, 1 and 1e150. Bridge compilation and whitespace checks passed.

The extended annular replay passed. Report: `triangle-euclidean-error-2026-10-09.json`. Results: {'accepted': 309, 'cells': 6805, 'domainCells': 100000, 'initialChecked': 3588, 'initialTriangles': 3588, 'leafIncidenceClosed': True, 'spans': 100000, 'splits': 1073, 'trimCacheHits': 203, 'trimInherited': 1087}; pending leaves 5425, numeric failures 0. These finite-fixture results do not establish full material/surface, radius, tangency, thickness or general fillet qualification. This native change is not repackaged into WASM.


### Triangle-shaped exact trim queries, 2026-10-09

Added outward separating-axis exclusion of source arc enclosures against a tolerance-padded triangle. The homotopy ray witness additionally requires its reference point outside the axis-aligned enclosure before a contribution is counted. Native rational-contour regression covers triangles inside/outside where the bounding rectangle overlaps the contour, and retains boundary-band refusal. Nonfinite triangle coordinates are rejected. Rectangle classification remains unchanged.

Refinement uses this fallback only after an unresolved rectangle query, charges its actual domain work, and never caches or inherits a triangle proof as a rectangle proof. Bridge check, contour test, full retained-recipe replay and whitespace checks passed. Report: `triangle-shaped-trim-2026-10-09.json`; {'accepted': 244, 'cells': 4755, 'domainCells': 100000, 'initialChecked': 3588, 'initialTriangles': 3588, 'leafIncidenceClosed': True, 'spans': 100000, 'splits': 389, 'trimCacheHits': 203, 'trimInherited': 1167}; pending 4122, numeric failures 0. Full shell qualification remains incomplete. No new WASM package is claimed.


### Deviation-gated triangle trim fallback, 2026-10-09

Triangle-shaped fallback work is now issued only when a numerical support bound already meets the target. Exact rectangle caching/inheritance remains separate; a triangle proof is never promoted to a rectangle proof. Counters track fallback queries and proved responses; zero-work replay asserts both remain zero and the replay checks counter bounds. Bridge compilation, whitespace check and full retained-recipe replay passed.

`triangle-gated-trim-2026-10-09.json`: 3,588 initial triangles checked; 271 accepted, 4,615 pending after 649 splits; closed oriented incidence, zero numeric failures. Both support and domain budgets reached 100,000. There were 819 triangle fallback queries and zero proved triangle responses on this fixture. The gated scheduling improves acceptance over the unconditional fallback (244), but remains below the rectangle-only spatial-norm run (309). The measured fallback currently consumes work without proving additional material. Next work should use one shape-aware traversal with explicit rectangle-versus-triangle proof provenance, rather than two contour traversals per candidate. No full geometry or WASM completion is claimed.


### Single shape-aware contour traversal, 2026-10-09

Refinement now issues one shape-aware query instead of sequential rectangle and triangle queries. The winding report separately records whether every contributing enclosure also excludes the full padded rectangle. Only such resolved proofs enter the rectangle cache or parent inheritance; triangle-only proofs are not promoted. A reference-point orientation enclosure must prove the chosen winding reference lies strictly inside the triangle before tracing; otherwise classification remains unresolved. Contour regression checks both provenance classes.

Native contour tests, bridge check, whitespace check and extended retained-recipe replay passed. Report: `triangle-single-pass-trim-2026-10-09.json`: all 3,588 initial triangles checked, 271 accepted, 4,349 pending after 516 splits; zero numeric failures and closed oriented leaf incidence. Domain/support budgets both reached 100,000. There were 3,781 shape-aware query calls with 1,670 resolved responses, including full-rectangle responses; these are not 1,670 newly admitted triangles. Accepted coverage is unchanged from deviation-gated double traversal and remains below the prior rectangle-only spatial-norm run (309). Pending count follows a different number of subdivisions and cannot establish increased global coverage. Additional domain preparation and boundary-aware admission remain necessary. No new WASM package is claimed.


### Borrowed source contours, 2026-10-09

Contour winding accepts either owned or borrowed exact Fragments via Borrow. Expression display/triangle queries now collect fragment references instead of cloning full source curves, surfaces and rooted endpoints for every query. The geometry remains immutable and retains original identity. Native regression compares complete classification outcomes, work counters, endpoint query counts, reasons, uncertain addresses and rectangle-proof provenance for rectangle/triangle inside/outside/boundary fixtures at budgets 1, 1,000 and 10,000.

Contour regression, bridge compilation, whitespace checks and extended retained-recipe replay passed (204.04 seconds). `triangle-borrowed-contours-2026-10-09.json` has an entirely identical refinement report to the preceding single-pass report, including 271 accepted, 4,349 pending, zero numerical failures and closed oriented leaf incidence. Deep-copy removal is confirmed from source; no runtime speedup is claimed from these uncontrolled replay timings. The full-shell and broader CAD roadmap remain incomplete; this change is not in the packaged WASM.


### Prepared immutable contour enclosures, 2026-10-09

Prepared contour queries own shallow references to one immutable inventory and a bounded cache of outward UV curve enclosures. Keys include loop/fragment indexes and exact parameter endpoint bits. Root endpoint contributions are hulled explicitly on each use. Classification and winding checks still run per query; visited-cell charging is unchanged. Refinement constructs one inventory per face, with total enclosure-cache capacity bounded by 100,000 entries across its faces. No global or cross-body cache is used.

Regressions compare cached versus fresh geometric outcomes, work counts and provenance for multiple shapes and capacities 0/1/8/100,000, assert capacity bounds, reject oversized capacity, and verify a repeated query performs zero new enclosure evaluations with actual cache hits. Zero-refinement work asserts zero enclosure evaluations/hits. Contour regression, bridge compilation, whitespace check and full retained-recipe replay passed (126.22 seconds).

`triangle-prepared-contours-2026-10-09.json`: 17,570 fresh curve enclosures and 74,545 reused enclosures (about 81 percent of those accesses). Excluding the two new cache counters, the entire refinement report is identical to `triangle-borrowed-contours-2026-10-09.json`: 271 accepted, 4,349 pending, zero numeric failures, closed oriented leaf incidence. This establishes eliminated repeated enclosure evaluations, not a controlled wall-clock speedup or increased geometric coverage. Remaining material-boundary and surface admission work is unchanged. Native changes are not yet packaged into WASM.


### Reserved initial material-query budget, 2026-10-09

Each original candidate receives at most the remaining domain budget divided by the number of original candidates still to inspect, capped by the per-query allowance. Zero-cost whole-chart/cached proofs leave the reserved work available for later candidates. Descendants continue under the existing per-query/global caps. The report distinguishes initial domain work; zero-query replay asserts zero, and replay bounds it by total domain work.

Bridge compilation, whitespace checks and extended retained-recipe replay passed (121.64 seconds). `triangle-fair-initial-trim-2026-10-09.json`: all 3,588 originals inspected using 46,211 domain cells, leaving 53,789 cells for descendants; 271 accepted and 4,615 pending after 649 splits, closed oriented incidence and no numerical failures. Total domain/support budgets still reached 100,000. Acceptance is unchanged from the previous prepared single-pass run; the greater pending count follows more subdivisions, not established loss or gain of geometric coverage. There were 24,410 fresh enclosure evaluations and 66,049 cache hits. Initial scheduling no longer consumes the full trim budget, but full boundary/material/surface qualification remains open. Native changes are not yet packaged into WASM.


### Centered mean-value anchor qualification and restoration, 2026-10-09

Added an outward mean-value bound supporting an arbitrary anchor UV point, including interval evaluation of the affine mesh plane at that anchor. Five regressions cover flat/skew/degenerate triangles, spatial norms across scale, interior bowing, and orientation reversal. The centered anchor improves the explicit bow fixture without an additional support query.

On the actual extended annular replay, however, `triangle-centered-anchor-2026-10-09.json` accepted only 209 leaves versus 271 with the vertex anchor (637 splits, 4,653 pending; no numeric failures and closed oriented incidence). Center anchoring was therefore removed from the production candidate path. The generalized helper is retained for further qualification, while the original vertex special case preserves its preceding arithmetic.

Five tests, bridge compilation and whitespace checks passed. A second full replay, `triangle-vertex-anchor-restored-2026-10-09.json`, has an entirely identical refinement report to `triangle-fair-initial-trim-2026-10-09.json`, restoring 271 accepted, 4,615 pending and no numeric failures. This experiment establishes that universal centroid anchoring is not a useful default for the current fixture; no coverage improvement, full geometry qualification or new WASM package is claimed.

## Addressed unresolved triangles, 2026-10-09

Native retained-profile replay passes in 125.50 seconds. The new diagnostic export adds vertex IDs and UV triangles to every unresolved leaf. Removing these additional fields produces exact equality with triangle-vertex-anchor-restored-2026-10-09.json, so no geometry result or work accounting changed: 271 accepted, 4615 unresolved, closed oriented leaf incidence, zero numeric failures. Solid qualification remains false.

The summary script validates finite addresses, retains all UV triangle addresses and records the input SHA256. Largest unresolved faces: 28 (1193), 0 (1078), 27 (508), then 16/20/24 (226 each). Face 28 spans the full [0,1]² chart; face 0 spans [0,0.75] × [0,1]; face 27 spans [0.9,1] × [0,1]. UV area sums are numerical diagnostic quantities, not physical material areas or certificates. This now supports targeted investigation of surface and trim bounds without silently treating budget exhaustion as a clean result.

## Fair face scheduling

Interleaving initial candidates across all faces preserves within-face order and all candidates. The retained-profile replay passes at the same 0.05 mm tolerance, initial candidate set, bisection inputs and work limits. Accepted leaves increase from 271 to 371; unresolved leaves increase from 4615 to 5105 because refinement creates additional leaves. Numerical unresolved UV area decreases on 12 faces and increases on none above 1e-10 UV units. Closed oriented leaf incidence is preserved, numeric failures remain zero, and solidQualified remains false. Eight focused native triangle/error/incidence/scheduling checks pass. See triangle-face-round-robin-comparison-2026-10-09.json. This is bounded coverage improvement, not full surface, tangency, thickness or self-intersection qualification. Packaged WASM has not been rebuilt for this native scheduling change.

## Descendant trim sharing experiment

Dividing remaining trim work among unproven queued/deferred descendants increases accepted leaves 371 to 512, with 5516 unresolved leaves, closed oriented incidence and no numeric failures. Numerical unresolved UV area decreases on six faces but increases by 1.349163857 UV units on face 1. The change was reverted because the count gain masks a per-face coverage regression; the previously validated face round-robin scheduling remains. Experimental result and per-face comparison are retained. Surface qualification remains incomplete.

## Vertex spatial error bound

The support-minus-affine-plane mean-value residual uses the same enclosed gradients and anchor as before. For each admissible residual gradient and anchor, its spatial norm is convex in triangle displacement, so its maximum is attained at a vertex. Each vertex residual is enclosed independently with outward interval arithmetic, its norm is bounded with scaled outward arithmetic, and the maximum is intersected with the previous valid upper bound. This avoids mixing coordinate extrema from different vertices and issues no new native support queries. A fixture with disjoint X/Y extrema improves the upper bound from 5 to approximately 4. Nine focused checks pass; an additional finite adversarial control covers all 512 corners of anchor/gradient interval inputs on 66 barycentric points each. These finite controls supplement the convexity argument; the whole-body replay is still pending at the time of this entry.

## Vertex spatial bound whole-body result

The completed native replay passes in 133.32 seconds. At the same candidate inputs, bisection, tolerance and work limits, the result is 371 accepted and 5105 unresolved leaves, exactly matching the face round-robin baseline in every per-face numerical unresolved UV area. Closed oriented leaf incidence is preserved and numeric failures remain zero. The tighter bound improves the isolated disjoint-extrema control but does not increase coverage on this annular fixture. Full solid qualification remains false. See triangle-vertex-spatial-norm-comparison-2026-10-09.json. The report additionally identifies original, retained and transition faces; face 28 is a transition and retains 1159 unresolved leaves spanning its full chart. This result directs subsequent work toward transition trim/support bounds rather than claiming a coverage gain from the new norm. No updated packaged WASM or full thickness, tangency or self-intersection qualification is claimed.

## Resource-specific unresolved diagnostics

Native refinement now distinguishes exhausted cell, support-span and exact-trim budgets, retaining simultaneous exhaustion explicitly. Disabled queries also receive a separate reason. All unresolved triangles and the existing work accounting are preserved; this changes diagnosis, not admission. Two scheduling/diagnostic tests pass and whitespace checks pass. The full annular replay producing triangle-resource-diagnostics-2026-10-09.json is running; its result has not yet been accepted or compared.

Resource-specific replay completed successfully. Coverage remains 371 accepted / 5105 unresolved, with identical per-face numerical UV areas to the preceding run, no numeric failures and closed oriented incidence. Unresolved reasons: {'triangle-trim-work-limit': 390, 'triangle-span-trim-work-limit': 1971, 'triangle-span-work-limit': 2744}. Face 28: {'triangle-span-work-limit': 1159}. The transition is stopped by support-span work, so subsequent coverage work must address support evaluation/refinement rather than exact trim for this face. Full solid qualification remains false.

## Immutable-call support anchor cache

Triangle refinement now reuses successful native support point enclosures for identical face, exact UV bits and control-limit keys. The cache is scoped to one immutable shell/refinement call, capped at 100000 entries, and never stores unresolved answers or boundary graph estimates. Exact native centroid enclosures populate the same cache. Cache hits charge zero new span work; derivative queries keep their previous admission and remaining-budget checks. Public uncached calls retain an empty per-call cache. The full annular replay is running into triangle-anchor-cache-2026-10-09.json; coverage improvement and packaged WASM integration remain unverified.

Anchor-cache replay completed successfully. Comparison: {'acceptedBefore': 371, 'acceptedAfter': 371, 'pendingBefore': 5105, 'pendingAfter': 5281, 'solidQualified': False}. Per-face UV changes: []. Leaf incidence closed: True; numeric failures: 0. UV sums are diagnostic, not physical areas or solid qualification.

The anchor-cache experiment was removed from the production refinement path: it creates more unresolved leaves without increasing accepted count or reducing any per-face numerical unresolved UV area. The baseline resource diagnostics remain. The experiment is retained as evidence, not a shipped coverage improvement.

## Rotation-independent interior-edge priority

Interior edge bisection now ranks spatial Euclidean length using scaled hypot rather than the largest coordinate difference. The preceding scheduling metric changes under rotation and can rank a shorter axis-aligned edge above a longer diagonal. The new metric remains a scheduling choice, never an error certificate. Two regressions pass: rotation and extreme-scale length controls, and shared midpoint/orientation/boundary preservation across both adjacent triangles. Full annular replay is running into triangle-euclidean-edge-priority-2026-10-09.json. Candidate subdivision may differ, so geometry coverage comparisons must identify changed bisection inputs; no coverage gain, general rotation equivalence of the entire pipeline, or packaged WASM change is claimed.

Euclidean edge-priority replay completed successfully. Result: {'acceptedBefore': 371, 'acceptedAfter': 215, 'pendingBefore': 5105, 'pendingAfter': 5261, 'subdivisionChanged': True}. Per-face numerical unresolved UV changes: [(6, 1.0538158899762209), (9, -18.62116535311793), (15, 29.09514951221732), (18, 32.91459154998597), (19, 25.725894993348533), (22, 34.348733165073725), (23, 26.032562304676958), (26, 34.348733165073725)]. Closed incidence: True; numeric failures: 0. This comparison explicitly allows and labels changed subdivision. Full solid qualification remains false.

The Euclidean priority experiment was reverted in the production scheduler: it lowers accepted leaves from 371 to 215, increasing unresolved UV area on seven faces while improving one. Its rotation-independent metric is mathematically preferable, but bounded scheduling needs face-aware work distribution before adoption. Baseline scheduling is restored; changed-subdivision comparison support and diagnostic evidence are retained.

## Current packaged WASM inspection worker qualification

Actual production handler/client in Node worker passes with published/packed WASM SHA256 3afcab4a6f41e5db19fecd9ce108245372c1595e1dbfdc6e2d8a14a5a1b4c4b7. The refused budget remains explicit; dispatch cancellation terminates and joins the first worker, and Retry succeeds in a fresh worker. Two workers started and joined. Timings: refusal 935.32 ms, dispatch cancellation 24.53 ms, Retry 8546.12 ms. The annular response contains 29 faces, 406 contact pairs with zero pending contacts, positive bounded signed volume and 2564 boundary-preserving candidate triangles with closed oriented incidence. Material and surface-tolerance qualification remain false. Evidence: current-inspection-worker-2026-10-09/expression-inspection-worker-report.json. This confirms the current package transport/cancellation path; it does not expose the subsequent native refined leaves, qualify a full fillet solid, or prove browser preview behavior.

Current browser boundary preview control passed in isolated localhost:5198 tab: imported source recipe, selected shared-boundary computation, started, cancelled by Escape, reran and displayed the result. DOM contains 2564 rendered polygon elements across 29 faces. UI explicitly states surface tolerance and trim remain unqualified, and radius/tangency/thickness require separate checks. Evidence and screenshot are in current-inspection-worker-2026-10-09/browser-preview.json and browser-preview.png. The temporary tab was closed; user localhost:5195 tabs were preserved. No forced delayed browser worker reply or full solid qualification is claimed.

## Current trimmed shared-edge root replay controls

Eleven native source_shared_edge tests and the shell root-partition test pass. The curved-support regression now explicitly rejects endpoint sampling at zero span budget for both ends and restores the admitted edge from original definitions with fresh native qualification. On both polynomial and rational curved carrier controls, replay preserves the original definition, retained parameter bounds and the irrational root enclosure around 1/sqrt(2), while complete-original-edge coverage remains false. The strengthened control passes separately. Existing tests cover opposite charts, reflected charts, distinct parameter domains, mixed fixed/root ends, unique-root requirements and rejection of two world-plane roots. This validates these authored classes; arbitrary general NURBS/mixed corners and a full admitted fillet Body remain open.

## Explicit refinement work-limit evidence

Native TriangleRefinementReport now retains the exact limits used by the call; export reads this retained value rather than reconstructing it from environment flags. Reports include deviation tolerance, cell/span/query/control limits, trim budgets, UV tolerance and maximum depth. Comparison rejects unequal explicit refinement limits even when changed subdivision is allowed. Legacy reports remain readable but receive refinementLimitsVerified=false and an explicit unavailable status; this field covers refinement only, not the entire geometry pipeline. Three comparison controls and two native scheduling/diagnostic tests pass. A new explicit-limits annular replay is running; no coverage improvement is claimed by this metadata change.

Explicit-limit replay completed in 129.15 seconds: 371 accepted / 5105 unresolved, unchanged coverage and closed incidence. New optional transitionBudgetProbes evaluate three original transition candidates at four per-query budgets (64,128,256,512), retaining addresses, actual spent spans and optional deviation bound. These probes are independent of production budgets/refinement and do not change admission. Probe compilation and two native controls pass; the full probe run is active and its numerical results remain pending.

Transition-budget probes complete. All three transition candidates return the same bound at budgets 64/128/256/512, spending 62 spans each. Bounds are 0.09200271400391716, 0.09200271400391845 and 0.09187832959843595 mm, all above the 0.05 mm target. Their UV triangles span the full transverse [0,1] direction despite narrow free-parameter width. Therefore insufficient per-query allowance is not the cause for these controls; increasing it cannot improve their bound. Full replay remains 371 accepted / 5105 unresolved, with explicit refinement limits matched against the baseline. Next work must reduce transverse mean-value overestimation or provide conforming refinement in that direction. These probes do not establish all transition candidates behave identically.

## Rectangle-center anchor diagnostic

Added a diagnostic candidate bound anchored at the center of its derivative UV rectangle, preserving the production vertex-anchor path. Each anchor-to-triangle segment remains inside the derivative rectangle; the existing affine-plane-at-anchor residual and outward mean-value bound therefore apply. A finite quadratic residual control covers an anchor outside the triangle but inside the owned derivative rectangle. Native transition probes now compare vertex and rectangle-center anchors at the same four per-query budgets; the full replay is running. No center-anchor adoption, coverage gain or updated WASM package is claimed before the results are examined.

Rectangle-center probe replay completed successfully. At 64 spans per query, measured vertex/center results: [{'anchor': 'vertex', 'deviationUpperMm': 0.09200271400391716, 'spent': 62, 'vertices': [774, 10, 0]}, {'anchor': 'rectangle-center', 'deviationUpperMm': None, 'spent': 64, 'vertices': [774, 10, 0]}, {'anchor': 'vertex', 'deviationUpperMm': 0.09200271400391845, 'spent': 62, 'vertices': [774, 1164, 10]}, {'anchor': 'rectangle-center', 'deviationUpperMm': None, 'spent': 64, 'vertices': [774, 1164, 10]}, {'anchor': 'vertex', 'deviationUpperMm': 0.09187832959843595, 'spent': 62, 'vertices': [774, 1000, 1164]}, {'anchor': 'rectangle-center', 'deviationUpperMm': None, 'spent': 64, 'vertices': [774, 1000, 1164]}]. Production refinement remains unchanged; this diagnostic result alone does not qualify the whole surface or justify default anchor replacement.

All rectangle-center budgets: [([774, 10, 0], 64, 64, None), ([774, 10, 0], 128, 93, 0.061701186311430053), ([774, 10, 0], 256, 93, 0.061701186311430053), ([774, 10, 0], 512, 93, 0.061701186311430053), ([774, 1164, 10], 64, 64, None), ([774, 1164, 10], 128, 93, 0.061622928371964814), ([774, 1164, 10], 256, 93, 0.061622928371964814), ([774, 1164, 10], 512, 93, 0.061622928371964814), ([774, 1000, 1164], 64, 64, None), ([774, 1000, 1164], 128, 93, 0.06162595362584684), ([774, 1000, 1164], 256, 93, 0.06162595362584684), ([774, 1000, 1164], 512, 93, 0.06162595362584684)].

## Regional error against the original mesh plane

The native mean-value helper now supports a local parameter triangle while retaining the original mesh triangle and its affine plane. Existing whole-triangle entry points delegate with the original region, preserving their arithmetic. A quadratic control checks that a local bound becomes tighter but still encloses the 0.25 mm residual against the parent plane, which would be hidden by refitting a subtriangle to the surface. Nine triangle-error tests pass. Whole-triangle admission still requires independently proved coverage of all regional derivative domains; this helper alone grants no surface certificate and has not changed production scheduling or the packaged WASM. A future rectangular partition can preserve exact common floating split coordinates and cover the parent derivative rectangle without changing shared mesh boundaries.

## Complete rectangular regional support diagnostic

Added an optional diagnostic bound that partitions the complete derivative rectangle into four rectangles sharing identical split coordinates. Each rectangle evaluates its own native anchor and derivative enclosure, then bounds both triangles of that rectangle against the original parent mesh plane. Taking their maximum covers the parent rectangle conservatively, including parts outside the candidate triangle. Any unresolved anchor, derivative or residual returns an unresolved result with actual spent work; no partial certificate is admitted. The production vertex-anchor path remains unchanged. Extreme and subnormal partition controls and transition probes are running; results remain pending before any integration or coverage claim.

Partitioned diagnostic succeeds for all three controlled transition candidates: bounds [0.029804155355868278, 0.02976139562298375, 0.029826479388759538] mm, below the 0.05 mm target, each spending 372 spans. Budgets 64/128/256 remain explicitly unresolved. This provides complete supporting-surface error coverage of these candidate rectangles against their original mesh planes; material trim, the rest of the surface and full solid qualification remain separate. Production refinement still has 371 accepted / 5105 unresolved because this diagnostic is not yet integrated. Newly added zero-budget entry-point assertions compile but were added after this run started and have not yet executed.

## Explicit regional transition admission stage

Added a native second stage for pending transition leaves. It independently asks exact trim classification, evaluates complete regional support coverage against the unchanged mesh plane, and plans all new admissions before committing changes to the report. It preserves all vertices, leaves and prior accepted results; failures cannot partially commit new admissions. Additional span work is bounded at 100000 and each query at 512, recorded separately from the baseline refinement. Comparison rejects changed additional budgets unless explicitly requested and then labels them; no same-total-budget gain is implied. Zero-budget controls for all diagnostic paths are included in the next full run. The regional native replay is running; full-body improvement remains unverified until its report is examined.

Regional transition replay passes in 177.10 seconds. Stage work: {'accepted': 0, 'maxSpans': 100000, 'queries': 269, 'spans': 100000}. Accepted leaves 371 -> 371; unresolved 5105 -> 5105. Per-face numerical unresolved UV changes: []. No per-face numerical area regression, closed oriented incidence and zero numeric failures. The additional 100000-span budget produces no new admissions; this stage is not enabled by default. All zero-budget diagnostic and regional controls included in this replay pass. Full solid qualification remains false; no new WASM package or UI refinement bridge is claimed.

### Face-aware interior edge scheduling — 2026-10-09

A per-face round robin heap experiment preserved shared midpoint and boundary controls. The full retained shell replay passed in 138.02 s. At matching refinement limits, accepted triangles rose from 371 to 429, while pending leaves rose from 5105 to 5417. Numerical pending UV area improved on 17 faces but worsened on six (15, 18, 19, 22, 23, 26); the transition face 28 did not improve. This is not a coverage improvement suitable for the default pipeline. The scheduler was reverted.

Evidence: `triangle-face-edge-queue-2026-10-09.json` and `triangle-face-edge-queue-comparison-2026-10-09.json`. Leaf incidence remained closed and oriented; no numeric failures were reported. Solid qualification remains false. UV area is a numerical parameter-space diagnostic, not physical area or a solid certificate.

### Native point deviation lower bounds — 2026-10-09

Added an outward interval lower bound for correspondence error between a native support point and the original affine triangle plane. The conservative coordinate bound avoids inward-rounded norms; degenerate UV triangles return no result. Ten triangle error tests pass, including an interior bow at tiny and large scales, reversed orientation, overlapping intervals and degeneracy. The retained-shell replay passed in 81.80 s; 16 pending transition point queries and their zero-budget controls are recorded in `triangle-point-witness-controls-2026-10-09.json`.

This diagnostic does not change admission, mesh refinement, or packaged WASM. A small point lower bound cannot establish whole-triangle tolerance. Material membership, parameter correspondence and closest-surface distance must be qualified separately before using a point result as a rejection gate.

### Recursive regional bounds - 2026-10-09

The native diagnostic supports bounded recursive rectangle partitions through depth 4. Shared floating split coordinates cover the parent box. Two partition tests pass, including recursive seams, extreme and subnormal domains. The original affine mesh plane is retained in every bound.

triangle-point-depth-controls-2026-10-09.json (134.94 s replay) records 16 original pending transition leaves. Upper bounds at depths 1/2/3 are respectively 0.029826-0.047534, 0.020147-0.026955 and 0.017275-0.020943 mm.

triangle-pending-depth-controls-2026-10-09.json (181.20 s replay) examines 16 leaves remaining after the full 512-edge adaptive split and extended refinement. Depth 1 bounds are 0.083574-0.091769 mm, depth 2 bounds are 0.043281-0.045928 mm and depth 3 bounds are 0.032561-0.034259 mm. All 16 meet the 0.05 mm support-deviation target at depth 2. Point lower bounds are 0.018131-0.018830 mm. Depths 1/2/3 spend 5952/23808/95232 spans across these controls; this is additional diagnostic work. Point correspondence and surface error do not establish material membership or solid qualification.

The depth-2 regional admission replay passed in 303.22 s. At an explicit additional 100000-span budget it issued 68 queries and admitted 30 formerly pending transition triangles. Baseline accepted/pending counts change from 371/5105 to 401/5075. Existing mesh vertices, prior admitted leaves and oriented incidence are preserved. No numeric failures were reported; full solid qualification remains false. Evidence: triangle-regional-depth2-2026-10-09.json and triangle-regional-depth2-comparison-2026-10-09.json.

The raw exporter in this compiled replay still labelled the old per-query constant (512). Its original bytes and hash are preserved in triangle-regional-depth2-raw-export-2026-10-09.json. The corrected report records the actual native depth-2 limit (2048) and a metadataCorrection trace; geometry and results were not changed. The exporter source is fixed for subsequent runs. This additional native stage is not yet packaged in WASM or exposed in the UI.

Comparison against the previous regional stage also uses the same total additional 100000-span budget: depth 1 admitted zero leaves, depth 2 admits 30. The per-query partition depth and allowance differ and are labelled explicitly. The subdivision remains unchanged; only transition face 28 improves in numerical pending UV area (delta -0.0185546875); no other face regresses. See triangle-regional-depth2-equal-total-budget-2026-10-09.json.

### Early unproven regional evaluation - 2026-10-09

The additional native stage now stops a query when any regional upper bound exceeds its target. It returns no whole-triangle certificate and keeps the triangle pending; this is not a proof of a geometric defect. Only complete coverage meeting the target admits a triangle, after independent material classification.

At the same 100000-span regional budget, the full replay passed in 198.93 s: 425 queries, 42 additional admissions (previous depth-2 stage: 68 queries, 30 admissions). Accepted/pending counts are now 413/5063. The exact remaining face/vertex/UV addresses are a subset of the preceding result, with 12 removed pending addresses and zero new ones. Only transition face 28 improves (numerical pending UV delta -0.005859375). Oriented incidence remains closed, numeric failures are empty, and solid qualification remains false. Evidence: triangle-regional-early-unproven-2026-10-09.json and its comparison report. Timings are individual runs, not a matched performance benchmark.

triangle-target-stop-controls-2026-10-09.json (107.49 s replay) verifies three native transition triangles at depths 1/2/3: successful targeted bounds and work counts equal complete evaluation; an intentionally tight target returns no certificate using strictly less work. Zero budget returns no result, and nonfinite tolerance or zero partition depth is refused. These controls do not change mesh or production admission. Packaged WASM and UI integration remain outstanding.

### Refined boundary transport and UI integration - 2026-10-09

The geometry bridge accepts an optional boundaryMeshLimits.refinement request and returns native accepted/pending triangles, per-leaf upper errors or refusal reasons, numerical failures, work counters and exact requested limits. The initial shared graph and its incidence remain separate; solid and global material/surface qualification are not inflated. Interior vertices generated by refinement are serialized with native point enclosures.

The transport verifies the requested tolerance/work limits, original triangle count plus two leaves per centroid split, valid point references, unique leaf addresses and bounded accepted errors. A refined response is required only when requested. Eighteen transport/controller tests pass, including cancellation, Retry and a late response after a regional-budget change. The native bridge replay passed in 39.62 s and its actual result passes the TypeScript transport validator: expression-triangle-refinement-bridge-2026-10-09.json. Zero-budget replay retains every initial triangle as pending.

The new UI includes an optional refinement toggle and green qualified/orange unproven/red numerical-error triangle rendering. Reason text is localized. All 321 direct modeler UI tests pass (70.35 s). Direct Vue type checks pass. The optimized WASM build is still active at this checkpoint; real packaged worker and rendered browser verification are pending. No claim of completed WASM/UI runtime qualification is made here.

### Packaged refinement runtime - 2026-10-09

The full WASM build, npm typecheck and Cargo check completed successfully. Current package: 12838695 bytes, SHA256 dd8599c34e8311b0818d4b4b21ac12c63c914b05a52ea5fb5662240c9ae8faa8. Nineteen transport/controller tests pass after adding malformed optional-request controls.

The production Node worker/client replay passes against this package, with two workers started and joined, native budget refusal, dispatch cancellation, a terminated worker realm and a fresh-worker Retry. Actual timing samples are 1003.33 ms refusal, 24.49 ms dispatch cancellation and 11948.80 ms Retry. At the UI request budgets (512 cells, 20000 spans, 10000 regional spans) the native reply retains all 2564 initial triangles: 6 accepted and 2558 pending, without numeric failures. This differs from the larger native 413/5063 qualification run; budgets and subdivision are not interchangeable.

The browser replay on port 5198 uses the same code/package. It loads the saved transition recipe, requests refinement, and shows 6 green and 2558 orange polygons. Esc during a second request clears every preview polygon; rerunning restores the same counts. Evidence: refined-inspection-worker-2026-10-09/expression-inspection-worker-report.json, browser-preview.json and browser-preview.png. Red numeric-error rendering is implemented, but this fixture contains no numeric errors. Late-response protection and error Retry are additionally covered by controller tests; no browser fault injection is claimed. The pre-existing Rush background build error and WebGPU unavailable warning remain outside this SVG CAD replay. Solid qualification, radius, tangency and thickness are still incomplete.

### Regional support cache experiment - 2026-10-09

The bounded per-call native support cache experiment was not retained. The 171.89 s full replay recorded zero cache hits at the current 100000-span additional budget, identical 413 accepted / 5063 pending leaves, exact unchanged pending addresses and per-face numerical UV coverage. It preserved closed incidence and reported no numeric failures, but did not reduce actual enclosure work or improve admission. Source changes were reverted. Evidence: triangle-regional-support-cache-2026-10-09.json and its comparison report.

### Strict interval triangle separation - 2026-10-09

Regional bounds now omit only rectangles proved strictly separated from the original UV triangle by outward interval half-plane tests. Touching, degenerate and unresolved cases remain in the support evaluation. Rectangle partitions still cover the original triangle, and every retained region evaluates the original affine mesh plane. This is not sample-based trimming. Three partition/separation tests pass, covering both orientations, boundary contact, degenerate UV triangles and scales from 1e-150 to 1e150.

The 176.96 s full replay uses the same subdivision and the same additional 100000-span regional budget. It issues 441 queries and adds 69 admissions to the 371 baseline leaves, giving 440 accepted / 5036 pending (previous early-unproven stage: 413 / 5063). All 27 newly removed pending addresses become admissions; no new pending addresses appear. Only transition face 28 improves in numerical pending UV area (delta -0.0185546875), with no per-face regression. Oriented incidence remains closed, numeric failures are empty, and solid qualification is false. Evidence: triangle-regional-separated-cells-2026-10-09.json and its comparison report. The packaged dd8599 kernel predates this optimization; a fresh build is required before runtime claims.

### Separation transport validation, 2026-10-09

The native geometry bridge replay after strict triangle separation passed in 41.23 s. The exported `expression-triangle-refinement-separated-bridge-2026-10-09.json` passes the actual `validBoundaryMesh` UI transport validator: 1 accepted and 2563 pending triangles under its smaller bridge-test budget. All 19 tests in expressionBoundaryMeshTransport, expressionShellInspectionTransport and expressionInspectionController passed. These checks validate bounded transport and cancellation/stale-result handling; they do not establish general fillet admission or runtime qualification of the rebuilt WASM. The geometry WASM release compilation finished; package optimization was still running when these checks completed.

### Native radius and source seam checks, 2026-10-09

Six source_seam_tangency tests pass (0.06 s), covering a complete authored curved seam, reversed/partial carriers, rational root endpoints and refusal for creases, singular endpoints and insufficient work budgets. The rotating-plane case bounds sine squared by 1e-3; this is an angular bound for the owned test seam, not assembled-fillet tangency admission. The annular_corrected_expression_coefficient_radius_cell test passes (4.51 s). Ten native triangle-error tests also pass. General mixed-radius corners, whole-shell radius/tangency, wall thickness and self-intersection qualification remain open.

### Separated regional bounds packaged runtime, 2026-10-09

Geometry WASM build completed: 12840942 bytes, SHA256 aa42d375984159f2da456eb5c4606468a32a9b4b1ac8787763c653622b8d15fd. The real worker replay passed, with both workers joined: refusal 1277.16 ms, dispatch cancellation 30.99 ms, fresh-worker Retry 13232.97 ms. The browser preview at the same UI budgets contains 10 accepted and 2554 pending triangles, versus 6/2558 before separation; numeric errors are zero. Esc during calculation clears all preview polygons, and rerunning restores 10/2554. Evidence is in separated-refined-inspection-worker-2026-10-09. This is bounded preview admission, not whole-solid/radius/tangency/thickness qualification. The pre-existing background Rush invalid-surface and WebGPU warning remain visible; this verification uses the Solid SVG viewport.

### Strict-separation refusal controls, 2026-10-09

All four partition tests pass after adding one-ULP diagonal contact, contained point rectangles in both triangle orientations at scales 1e-160 through 1e150, and overflow refusal. These are regression controls for conservative interval separation, not sampled surface admission. Only test code changed, so the packaged aa42d375 runtime evidence remains applicable.

### Coefficient radius law mutation control, 2026-10-09

The annular coefficient-radius replay passes with an added law mutation control. The original whole-domain upper bound is 1.595684089104902e-6 mm, within the 1e-4 mm tolerance. Increasing every radius-law control value by 0.25 mm produces an upper bound of 0.40019526485580814 mm, which cannot admit that tolerance; this upper bound is not a certified lower-bound violation. Both attempts use 986 spans and 60072 coefficient-work units. The exported coefficient-radius-refusal-2026-10-09.json contains both outcomes. No production geometry code changed. The separate authored-law interval replay is still running and has not established complete coverage.

### Certified coefficient radial mismatch lower bound, 2026-10-09

The native CoefficientRadiusBound now includes an optional error_lower. For positive Bernstein weight enclosures, the signed squared residual gives its minimum absolute magnitude; division by the outward upper bound 2*r_upper+error_upper yields a whole-cell radial error lower bound. Overlapping residual intervals yield zero (inconclusive), and incomplete coefficient work yields no lower bound. The mutated annular radius law has error_lower 0.06197609549178115 mm and error_upper 0.40019526485580814 mm, proving a mismatch above 1e-4 mm. Evidence: coefficient-radius-lower-bound-2026-10-09.json. Two native coefficient module tests pass. This native API addition is not yet rebuilt into the packaged aa42d375 WASM or exposed in UI. The separate authored-radius test completed successfully in 615.32 s, but its assertions allow pending coverage and its captured passing output does not establish completeness.

### Owned expression radius bridge, 2026-10-09

The expression-shell restore/inspect request accepts optional radiusCheck with centers, radius, lawDomains, toleranceMm, controls, spans and coefficientWork. It rebuilds the actual owned transition face and issues fresh coefficient radius qualification. The result distinguishes qualified and violationQualified with lower/upper error bounds; exhausted work remains unproven, and solidQualified/materialQualified remain false. The native dispatch test passes in 92.99 s for original, changed-law and zero-span requests, including exact request echo. Evidence: expression-radius-dispatch-2026-10-09.json. Cargo check passes. WASM/TS transport/UI integration of this new request remains pending; the packaged aa42d375 predates it.

### Radius transport validation, 2026-10-09

ExpressionShellOptions now carries optional radiusCheck, and the base shell validator requires the requested native report with bounded work, finite nonnegative error bounds, consistent admission/violation flags and explicit solidQualified=false. Request identity includes the entire radius and center laws. No radius geometry runs in TypeScript. All 11 radius/inspection transport tests pass against actual native dispatch fixtures, covering original/violation/unproven states, changed law identities, malformed requests, missing reports, budget overruns and contradictory qualification flags. Vue typecheck passes. Packaged WASM and UI exposure of radiusCheck remain pending.

### Radius UI and stale law protection, 2026-10-09

The inspection panel displays the original-law tolerance and native radius admission, certified violation or unproven state, including numeric radial error bounds. Radius input comes from the saved inspection recipe; no synthetic law is inferred in TypeScript. The new controller regression ignores a late response after law replacement and accepts the current violation reply. Fourteen radius/controller tests and Vue typecheck pass. UI preparation and real-worker scripts accept --radius, --radius-violation and --radius-unproven, using actual native fixture laws. Three UI fixtures were prepared successfully. The radius WASM build is running; browser and packaged-worker validation remain pending.

### Radius law archive preservation, 2026-10-09

Seventeen archive/radius/controller tests pass. The archive test round-trips native admitted, violated and unproven radius requests including exact center/radius control data, law domains, tolerance and work limits. Original source bytes remain unchanged; mutating decoded radius data cannot mutate a saved archive. The running geometry WASM rebuild is still compiling release code. No new packaged-runtime result is claimed.

### Radius mismatch region localization, 2026-10-09

The radius panel now offers region highlighting for a certified mismatch. It emits only transition face IDs from the current native reply; pending calculation, cancellation and source-body replacement clear the selection. The scene draws available support enclosure boxes in red with data-radius-violation, retaining trim state separately. This is support-region localization rather than watertight material admission. Three SSR panel tests pass for qualified/violated/unproven labels, and Vue typecheck passes. Actual browser highlighting awaits the rebuilt package; no visual/runtime claim is made yet.

### Disabled radius work transport guard, 2026-10-09

The radius report validator rejects any lower/upper error bound when controls, spans or coefficientWork is zero. Regression checks reject fabricated admission and fabricated violation even with zero reported work, while accepting a bound-free unproven report. All 21 radius panel/archive/transport/controller tests pass. Geometry WASM optimization is still running; packaged radius runtime remains unverified.

### Independent radial-bound scale controls, 2026-10-09

A native coefficient-radius regression passes nine cylinder-law combinations: scales 1e-50, 1 and 1e8, with requested radius factors 0, 1 and 1.25. Lower and upper bounds enclose the analytical cylinder reference error; mismatched laws have strictly positive lower bounds. This includes a zero radius law. The initial 1e50 fixture was refused by the native input rule limiting coordinates to 1e9, so no above-limit support is claimed. The test completes in 0.34 s. Production code remains unchanged by this test; WASM optimization is still running.

### Packaged radius diagnostics and browser cancellation, 2026-10-09

The WASM rebuild completed: 12,873,854 bytes, SHA256 `0f1f7848bb72556b46587a186f7817b6a118f43db9a3b872c925c008bc7b5de0`. The qualified, violation and unproven real-worker reports each pass with two workers started and joined. Retry took 8.40–8.45 s; dispatch cancellation took 22.3–23.2 ms. These measurements concern the controlled annular recipe, not all commands.

Browser evidence `radius-ui-cancel-retry-2026-10-09.json` and its PNG confirm source-body selection by mouse, a native radial-error interval of 0.06197609549178115–0.40019526485580814 mm, and 16 highlighted transition support boxes. Esc clears the result and all 16 boxes; a fresh run restores the same 16. The earlier inability to select via an automated SVG polyline center click is not sufficient evidence of a production pointer bug. Arbitrary edge-stroke picking still needs a targeted hit-location test.

The display remains a support enclosure preview. Tangency, wall thickness, full material admission, general NURBS fillets and the full P0 matrix remain incomplete. Prior entries describing the radius build/runtime as pending are superseded by these current reports.

### Interior crease with matching endpoint planes, 2026-10-09

A new native regression uses an exactly shared straight carrier between a plane and the polynomial patch z=2*a*u*(1-u)*v, for a=0.1, 1 and 10. The native normal bounds confirm matching tangent planes at both ends and oblique planes at the midpoint. Whole-seam qualification refuses admission in all three cases. This prevents endpoint-only evidence from being treated as full G1 qualification. All seven source seam tests pass in 0.05 s, including curved seams, root endpoints, reversed partial carriers and singular endpoints. Production geometry is unchanged; no WASM rebuild is required for this test-only addition. General assembled fillet seam qualification remains open.

### Owned full support tangency gate, 2026-10-09 (qualification running)

`TransitionFace::qualify_support_tangency` rebuilds the owned exact full-Hermite recipe and covers section-low, section-high, rail-low and rail-high joins with a shared cell/span budget. It creates a face-bound certificate only after all four complete covers; zero work, exhausted work and incomplete covers cannot admit. This proves first-jet agreement with the original profile support only. Neighbor body incidence, all assembled seams, wall thickness and material admission remain separate. Cargo check and diff check pass. The existing `restricted_profile_seams_match_owned_neighbor_supports` native test now checks complete four-boundary admission, exact face identity, total work bounds and zero/short-budget refusals; its current process is still running, so test success is not yet claimed. Log: `/tmp/owned-support-tangency.log`. The packaged WASM predates this new gate.

### Support tangency bridge and transport verified, 2026-10-09

The owned native support tangency test passes in 195.18 s, including four complete boundary covers, exact face identity and zero/short-budget refusals. The optional `supportTangencyCheck` bridge request is echoed exactly; all four complete covers are required. The dispatch fixture `expression-support-tangency-dispatch-2026-10-09.json` contains admitted, radius-mismatch and exhausted-work responses. Support tangency takes 32 cells/32 spans in the controlled recipe; changing the independent radius law preserves support tangency, while zero spans produces no certificate or visited boundaries. The bridge test passes; the first run reached the export step but failed on a relative output path, then the corrected absolute-path run passed.

The TypeScript transport validates request identity, boundary order/coverage flags, budget totals, disabled work and explicit false neighbor-body/solid flags. All 21 tests across support tangency, radius transport, inspection controller and archive pass; Vue typecheck passes. This is original profile support G1, not qualification against all neighboring body faces. UI presentation, rebuilt WASM and real-worker qualification of this new gate remain pending. The previous radius WASM package remains unchanged.

### Support tangency UI integration, 2026-10-09

The panel has an optional original-support tangency request with bounded native work, displays qualified/unproven states, four-boundary coverage and cell/span counts, and explicitly retains neighbor-body/thickness limits. Five SSR panel tests pass, including both new states; Vue typecheck passes. Worker and UI fixture scripts accept `--support-tangency` and `--support-tangency-unproven`. Prepared UI inputs are `/tmp/cad-support-tangency-qualified-20261009.json` and `/tmp/cad-support-tangency-unproven-20261009.json`. The geometry WASM rebuild is running (`/tmp/support-tangency-wasm-build.log`); no new packaged-worker or browser qualification is claimed yet.

### Support tangency recovery controls, 2026-10-10

Archive regression preserves support work limits and original source geometry across reload, including disabled spans; decoded mutations cannot alter saved limits. Controller regression ignores a late support certificate after the request budget changes and accepts the current unproven response. All 21 tests in the archive/controller/transport/panel batch pass. UI now offers saved, standard (10000 cells/100000 spans) and extended (100000/1000000) support budgets, enabling an explicit retry after exhausted work. Vue typecheck and all five panel tests pass. Native geometry stays Rust-owned. The current build process remains live in WASM optimization, so packaged-worker/browser evidence is still pending.

### Support tangency packaged runtime, 2026-10-10

WASM rebuild completed: 12,880,978 bytes, SHA256 `3caaa4ad6559488d4544e22473d5d3551b4f1882f0714a1eaee42d93c76e2fa9`. Real-worker reports in `support-tangency-qualified-worker-2026-10-10` and `support-tangency-unproven-worker-2026-10-10` both pass; published bytes and packed identity agree. Each run starts and joins two workers, checks dispatch cancellation and fresh-worker retry. Qualified/unproven retries take 8983.99/8988.75 ms, cancellation 27.36/24.89 ms. The controlled request admits four original-support joins at normal budget and refuses with zero spans. Neighbor-body/solid flags remain false. Browser qualification is being recorded separately.

### Browser budget recovery confirmed, 2026-10-10

Browser evidence `support-tangency-browser-2026-10-10.json` and PNG confirms the actual new WASM path: imported zero-span request displays unproven 0/4 with zero work; changing the UI budget to standard clears the prior result. Esc on the running request clears the result; a new run admits 4/4 original-support boundaries using 32 cells/32 spans. Neighbor-body and wall thickness remain explicitly separate. Source-edge selection used keyboard Enter after import; immediate source-body mouse selection did not stick during this run, so the full pointer path still needs targeted qualification. Final five panel tests pass and diff check passes.

### Import menu overlay recovery, 2026-10-10

The open File popover overlays the source body scene row. After it is explicitly closed, body selection by mouse succeeds. A successful Solid/ModelGraph JSON import now closes its enclosing details menu after the history transaction commits, preserving error/cancellation paths. Actual browser evidence `source-import-menu-recovery-2026-10-10.json` and PNG confirm the menu is closed and the first scene-row click selects source edge 0 after importing the controlled recipe. Vue typecheck and diff check pass. No WASM rebuild is needed for this UI-only change.

`source-pointer-selection-2026-10-10.json` records a separate stroke experiment: a native scene click selected source edge 1, while the preceding DOM hit check identified edge 0. This does not qualify exact intended-edge selection in a dense projection. The full mouse/key command matrix remains open.

### Current UI regression suite, 2026-10-10

All 321 DirectModeler component scenarios pass in 75.76 s against the current packaged geometry (SHA256 3caaa4ad6559488d4544e22473d5d3551b4f1882f0714a1eaee42d93c76e2fa9), after support-tangency UI and import-menu changes. This is regression evidence for the harness scenarios, not full mouse/key qualification of all 95 commands, crash recovery or multi-tab acceptance. Summary: `cad-ui-current-2026-10-10.json`.

### Actual assembled-neighbor tangency, 2026-10-10

The native gate binds its certificate to the exact assembled ExpressionShell and verifies its four transition/retained/original incidences. It recomputes both root-ended rail angle covers against the actual original neighboring surfaces, including when their profiles also occur in the proposal. Full regular Hermite support joins and these angle covers share one cell/span budget. Exhausted work, invalid neighbor incidence and nonfinite tolerance issue no certificate. The dedicated proposed-neighbor independence regression passes in 18.55 s: full-edge original-neighbor bounds and work counts match with and without the proposed neighboring profile. The assembled-shell regression, including small/zero budgets and exact owner identity, passes in 267.06 s.

Bridge qualification passes in 68.43 s. Actual native replies are preserved in `expression-neighbor-tangency-dispatch-2026-10-10.json`. Transport validates exact angular/work request identity, finite four-join bounds and false solid admission. Archive checks retain tolerance/budgets after reload and prevent detached changes from mutating the saved recipe. Controller tests reject late authority after budget or tolerance changes. The panel displays the requested tolerance and qualified/unproven joins, while keeping thickness, other seams and self-intersections separate.

The rebuilt WASM is 12,885,509 bytes, SHA256 `4cf2545b10c2d9f53534fc46f052a94a9a2b42a6767bf434bd9c369fa3843696`. Both real-worker scenarios pass with two workers started/joined, package identity verification, dispatch cancellation and fresh-worker retry. Cancellation samples are 24.66/24.78 ms; qualified/unproven retries are 10758.95/10433.13 ms. The qualified controlled recipe uses 2298 cells/9096 spans, with squared-angle-sine upper bounds `[0,0,2.17e-321,9.981695228808158e-7]` at requested tolerance `1e-6`. These are finite tolerance bounds, not exact zero-angle evidence.

`neighbor-tangency-browser-2026-10-10.json` and PNG record the actual UI path: keyboard edge selection, native zero-budget refusal, budget change clearing the prior result, Esc cancellation clearing results, and a fresh standard-budget calculation qualifying all four joins. This qualifies the saved annular transition only; general NURBS/complex corners, full body thickness, remaining shell seams and whole-body acceptance stay open. `neighbor-tangency-native-2026-10-10.json` summarizes the evidence. All 321 existing CAD component tests pass after the controls were added; that run used the preceding WASM and does not independently qualify the new gate or the complete 95-command browser matrix.

### Intrinsic cache, packaged parity and extended coverage, 2026-10-10

The owner-bound corrected-jet cache is shared by whole-rectangle and endpoint-pair chord queries. Standard native and packaged worker reports match exactly, including all pending domains and work counters: 10,000 cells, 48,488 spans, 4,085 pending regions. WASM SHA256 is `7c77219ae1ee8e8899ef563a8124b67c93f896784c2fd6fbd00848055164183d`. Both zero-budget and bounded worker runs pass cancellation/fresh-worker Retry. The browser displays the same standard counters; see `intrinsic-cache-ui-2026-10-10.json` and PNG. Standard span work is 15.63% below the earlier 57,471-span baseline; this is not a wall-clock performance claim.

The extended native run passes in 1,394.10 s with the unchanged 100,000-cell / 1,000,000-span / 1,000,000-trim-cell limits. It processes 92,368 cells and leaves 40,791 pending regions across 45 face pairs, compared with 30,952 regions across 46 pairs before intrinsic-cache reuse. Pair [0,27] has no remaining pending regions. Exact endpoint containment confirms every new pending product is inside an earlier pending product, allowing endpoint exchange only on the same face. More regions reflect finer subdivision; raw parameter volumes are not physical wall volume or thickness. Detailed accounting and containment are in `expression-wall-intrinsic-extended-summary-2026-10-10.json` and `expression-wall-intrinsic-extended-containment-2026-10-10.json`. Whole-body thickness remains unqualified.

A subsequent first-path short circuit now tests a complete coordinate-path chord before computing the alternate path. Thirteen expression-radius tests and 41 self-chord theorem tests pass. Its standard full-body comparison is still running and this subsequent change is not in the packaged WASM above.

The inspection controller also rejects stale worker errors after raw recipe mutations that bypass Vue notifications. Valid and invalid mutation regressions both permit a corrected Retry; 74 related controller/transport/focus/panel tests and Vue typecheck pass. This is bounded inspection coverage, not the full 96-command P0 matrix.


### Original surface chord cache: native standard qualification (2026-10-10)

The pair-only cache and shared intrinsic/pair cache both preserve every standard-budget coverage field and all 4,085 pending parameter products exactly. The regression comparator permits only reduced span work and its accounting fields. Pair-only work is 38,894 spans; shared work is 38,179, compared with 48,488 before caching (10,309 fewer, 21.26%). These are operation counts, not a measured latency speedup. The tests passed in 403.77 s and 374.06 s under concurrent load. Geometry remains unqualified. Reports and comparison certificates are saved alongside this document. The existing packaged WASM does not yet contain this original-surface cache; a new build is required.


### Browser cancellation and tangency acceptance (2026-10-10)

Packaged first-path WASM bc8bb52d0e1ea23a5614d7989628d0e83590235b9a55928f25fb2405a3f7ab98 was exercised in the new UI at port 5198. Selecting source edge 0, standard inspection, Escape while pending and a fresh successful run produced 4,085 explicit pending areas / 48,488 spans. The candidate source-face action was invoked. A separate run with actual-neighbor and original-support tangency enabled qualified four joins and four support boundaries; maximum displayed sin-squared bound 9.981695228808158e-7 is below 1e-6. Console error list was empty. These are controlled-recipe checks, not full P0 or global thickness/solid admission. JSON and screenshots are saved alongside this document. The original-cache WASM build is still running and is not covered by this evidence.


### Extended first-path result and input focus (2026-10-10)

The completed first-path extended run used 92,376 cells and exhausted both 1,000,000-span and trim budgets. All 435 initial products were checked; 40,797 pending regions remain across 45 face pairs. Exact comparison confirms every pending product is contained in a previous pending product. Only transition self-pair [28,28] changes: 15,061 to 15,067 finer regions, with raw parameter-product sum 0.32354736328125 to 0.323516845703125. This is a tiny bounded coverage improvement, not global geometry admission or physical volume. Native test passed in 1,429.65 s.

Invalid distance-check numeric inputs now cancel old diagnostics and focus the first invalid field before dispatch. Invalid saved parameters focus the budget selector because saved inputs are locked. The controller/panel suite passed 22 tests; actual browser invalid-angle focus, correction, Enter start and Escape cancellation were observed. Diagnostic selections are transient component state; durable model/recipe restoration does not currently promise persistence of these temporary UI overrides.


### Independent-profile Hermite local chord diagnostic (2026-10-10)

`ProfileSectionBridge::inspect_self_chords` now supports independently owned source profiles using this exact bridge's interval derivatives and the existing mean-Jacobian theorem. The complete requested local rectangle must have positive widths, valid source parameters, bounded work and a conservative continuous derivative field. Insufficient bounds remain explicitly unresolved. Twelve bridge tests passed in 2.72 s: 256 planar local rectangles, actual rational normal-diameter preservation at five budgets, folded-candidate refusal and invalid inputs. This does not sew independent rails, check cross-rectangle chords, establish a radius law, or admit a closed body. It is a native API and is not exported through the currently building WASM.


### Profile-owner hardening native rerun (2026-10-10)

The standard-budget shell test passed in 341.51 s after owner keys were added. Its complete report is byte-identical to the prior shared-original-cache report (SHA256 02e7a4779241f8d03b4cdc750d2546737ecde17d13173578aef4ecad3dcea5d5): 10,000 cells, 38,179 spans, 4,085 pending regions. This controlled-case rerun supports the public-constructor reachability audit; no change to whole-body admission is claimed.


The independent bridge now also exposes `inspect_self_chord_pair`: complete owned mean-derivative and endpoint-normal enclosures are followed by two exact coordinate-path bounds for the same chord. Both endpoint orders, genuine rational normal diameters, conservative C1 refusal and invalid/zero-budget requests are covered. The expanded native chord regression suite passed 102 tests in 1.33 s. No cache, wire certificate or B-rep admission is introduced by this API; native-to-WASM/UI integration and independent-profile shell assembly remain open.

### Independent-profile JSON and typed worker transport (2026-10-10)

The native `profile_section_bridge_chords` operation reconstructs both profile owners, validates source domains before budget refusal, and echoes the complete request. Its native fixture is saved in `profile-bridge-native-transport-2026-10-10.json`. Reports identify a proposed expression, include profile deviation bounds when qualified, and keep whole-thickness and solid admission false. The TS validator binds all input fields and work limits; three transport tests cover native replay, malformed admission/bounds, zero-budget refusal, cancellation and late replies after Retry. The existing worker regression suite passed 70 tests and vue-tsc passed. These are native and transport checks, not actual deployment of this new operation through WASM.

`scripts/check-profile-bridge-chords-worker.mts` is the prepared real-WASM gate: it checks packed identity, full native-report equality through the actual worker, zero-budget refusal, cancellation and Retry. It has not yet passed against a build containing this operation. The owner-hardening WASM build already running predates this API; it must not be used as evidence that the new operation is deployed. Independent-profile shell assembly, UI controls and global body qualification remain open.

Transport qualification was extended with a nonseparable rational source: its reported profile-weight and position deviation bounds are nonzero, while proposed-expression and non-admission flags remain explicit. A separate rational cylinder case contains a genuine normal diameter; native transport preserves it as unresolved rather than excluding it. The native test also rejects out-of-domain rectangles, section parameters and pivots even at zero work budgets. Both additional native reports are included in the replay fixture and the prepared real-worker gate.


### Whole independent-bridge endpoint coverage and packaged worker parity (2026-10-10)

`inspect_self_chord_coverage` adaptively covers the complete symmetric endpoint product, using LL/LR/RR on diagonal products and binary subdivision elsewhere. Unsupported or exhausted obligations remain explicit. The native regression passed 103 tests in 1.06 s, including conserved unresolved parameter-product mass, complete planar coverage and preservation of a genuine rational normal diameter. This excludes normal-aligned chords only; it does not prove absence of zero-length self-intersections, minimum thickness or closed-body validity.

Owner-hardening WASM 9dd27d1c48ca260b5dfc953eaeef6b2ebc7f85f958217e2ee4025bd4fb3dd829 passed the actual worker gate, including exact complete native wall-report equality, termination and fresh Retry. Two workers were started and joined; dispatch cancellation took 92.39 ms and Retry 21,644.40 ms in this controlled run. The wall report still has 4,085 pending regions. This build predates the independent-profile transport operation; a subsequent build containing that operation has started and remains unqualified.


### Independent-profile injectivity negative control (2026-10-10)

The existing fixed-projection P-matrix injectivity path was exercised with distinct profile owners. The planar bridge qualifies. A genuinely overlapping bridge, x(t)=4t−9t²+6t³, maps three distinct parameters to x=1/2 and remains unqualified at all 16 tested cell/span budget combinations. Each run retains the central fold, conserves the entire parameter area and respects both work limits. The native test passed in 0.06 s. This is separate evidence from normal-chord exclusion; it does not qualify arbitrary B-rep self-intersections or a closed solid.


The whole-bridge chord test additionally qualifies a reparameterized planar source with u=[10,12], v=[-2,6]. Five increasing cell budgets preserve unresolved cylinder products inside previous pending products, allowing endpoint exchange. Invalid domains, angles and excessive budgets reject even with zero work. The expanded chord regression remains 103 passing tests (1.98 s). This is native evidence; the current WASM build began before these additional test and comment changes and is still compiling. The prepared real-worker gate now also requires recovery after native validation failure before reporting success.

### Whole bridge coverage transport (2026-10-10)

`profile_section_bridge_chords` accepts optional `coverage: {domain,cells,spans}`. Its independent coverage budget is separate from the local pair budget. Native chart validation occurs before profile-budget refusal; complete coverage, work counts and all unresolved symmetric endpoint products are returned separately from the local pair result. The native fixture includes a complete planar product and a zero-cell product where the local pair is excluded but the whole product remains unresolved. Six TS transport tests passed (1.82 s), including absent/forged coverage, invalid product bounds and impossible complete claims. Whole-thickness and solid flags remain false.

The real-worker script has an explicit `--whole-coverage` gate. That gate is not yet qualified: the running WASM optimization includes the earlier pair API but predates this optional transport field. The default pair gate can be exercised on that build; optional whole-coverage deployment needs a subsequent build. Native-to-UI whole coverage, body assembly and full B-rep qualification remain open.

### Shared reactive inspection controller (2026-10-10)

The expression-shell controller and independent-profile controller now share `useBoundNativeInspection`. Both bind success and failure to the complete request identity, invalidate on reactive edits or deactivation, detect raw mutations before accepting a response, cancel their disposable worker, and clear reports on Cancel/Retry. Existing panel/controller regression passed 22 tests (9.86 s); the new profile controller test passed (3.23 s), covering whole-product reports, raw coverage-budget mutation followed by a late error, changed profile scales, Retry and deactivation. vue-tsc passed. This supplies the reactive controller for UI integration; no new visible profile-selection panel or actual browser acceptance is claimed.

### Profile transition panel in the new UI (2026-10-10)

Selecting two NURBS surfaces now exposes `ProfileBridgeCheck` beside existing surface diagnostics. Section axes/parameters, tangent scales, angular tolerance and optional whole-product coverage are editable. The shared controller owns cancellation, Retry and stale-response guards. Source-profile modification bounds and normal-chord scope are displayed separately from thickness/self-intersection/solid admission. Invalid numeric parameters show a correction message; Enter focuses the first invalid field before any worker dispatch.

Actual browser acceptance at port 5198 created two temporary NURBS surfaces, selected them with Shift, opened the panel, entered sin²=1 and pressed Enter in section A. Focus moved to the angle input with aria-invalid=true; Escape closed the panel. Screenshot `profile-bridge-panel-2026-10-10.png` records the visible panel. vue-tsc passed. No successful native calculation in this panel is claimed yet: the current optimization predates optional whole-product transport, and the packaged kernel still predates the pair API. Existing user model at port 5195 was not edited.

### Profile panel focus and P0 regression (2026-10-10)

Cancel now restores focus to Inspect; completion/failure restores it to Inspect/Retry, and Escape closing restores it to the panel toggle. A panel test using the real reactive controller with a controlled worker covers invalid-input focus, pending cancellation, late response suppression, error/Retry, successful report display, source replacement invalidation, closing and disposal. The combined panel/controller suite passed 24 tests (22.67 s); the final panel rerun passed (6.27 s).

Actual browser verification observed Retry focused after a quick worker failure and the toggle focused after Escape closing. The published pre-API kernel reports `NURBS_INVALID_INPUT: Invalid surface: Expected object` for this unknown operation because its old fallback attempts surface parsing. That run is only error/focus acceptance, not geometry validation or a confirmed defect in the selected surfaces. Native errors are available under Error details. Screenshot `profile-bridge-retry-focus-2026-10-10.png` records this state. The newer pair-API kernel is still optimizing; whole-coverage transport will require a subsequent build.

### Direct compiled WASM ABI replay (2026-10-10)

The compiled pre-Binaryen pair-API module (SHA256 506509c0df54c8fc8f9e70f17d16115e49872dcdaf7b793d24026307c4b98fa0, 14,654,750 bytes) passed direct binary ABI replay using production valueBinaryCodec/wasmHost helpers. Plane, nonseparable rational-profile and genuine normal-diameter reports exactly match native Rust. Zero-budget refusal, invalid-domain refusal at zero work and recovery after a validation error also pass. `profile-bridge-unoptimized-abi-2026-10-10.json` records the artifact identity and scope. This does not qualify the packaged optimized worker or UI calculation.

Binaryen optimization consumes a private immutable input snapshot, so a cargo-only build of the subsequent whole-coverage API was started while that optimizer continues. It does not publish generated JS or public WASM. The last panel delivery typecheck completed successfully.

### Optimized pair worker and whole-product ABI replay (2026-10-10)

The published optimized pair-API kernel passed the actual MainSolidWorkerClient gate (`profile-bridge-worker-2026-10-10.json`). Plane, rational proposed-profile and normal-diameter reports match the native fixtures. Zero-budget refusal, invalid-domain refusal, recovery, cancellation and a fresh Retry pass. This qualifies the tested transport cases; it does not qualify arbitrary fillet bodies or UI geometry acceptance.

The subsequent whole-product module passed direct binary ABI replay for all five native fixture cases, zero-budget refusal, invalid-domain refusal and recovery. `profile-bridge-whole-unoptimized-abi-2026-10-10.json` records SHA256 cd9861840f85255f5dafdedb71e7ee9164a9e23dc90e2d721091ea6db52958d6 and 14,660,836 bytes. Its optimized publication build is running; whole-product packaged-worker acceptance remains open.

Actual browser acceptance on port 5198 reloaded the optimized pair kernel, selected the two temporary default NURBS surfaces with Shift, opened the panel and dispatched Inspect. The native report rendered as “Предварительная проверка не подтверждена”, with source-profile deviation details and the explicit separate thickness/self-intersection/closed-body scope. This is a successful request/render path with an unqualified geometry result, not a successful exclusion claim. Screenshot: `profile-bridge-native-ui-2026-10-10.png`.

### Profile command palette reference correction (2026-10-10)

The profile diagnostic lives under the pane loop. Its string template ref therefore became an array in Vue, whereas the command expected a single exposed component. A function ref now assigns the actual mounted component. vue-tsc passes. Actual browser acceptance closes the panel, opens Commands, searches “переход профилей” and presses Enter: the panel expands and the active element is “Параметр сечения A”. Screenshot: `profile-bridge-palette-focus-2026-10-10.png`. This qualifies that command route only; the complete P0 command matrix remains open.

### Whole-product curved negative control (2026-10-10)

The transport fixture now also includes `curvedWholeCoverage`: a rational arch bridge with a genuine normal diameter at the chart endpoints, 31 cells and 10,000 spans. Native acceptance requires incomplete exclusion and explicitly checks that an unresolved endpoint product contains both diameter ends (allowing endpoint exchange). The native transport test passes in 0.20 s. The six-case direct WASM ABI replay passes with exact native report equality. The optimized-worker script now requires this curved whole-product fixture when `--whole-coverage` is enabled. Packaged whole-product acceptance remains pending optimization. This is a negative control for normal chords; it does not prove self-intersection or qualify thickness.

### Endpoint enclosures on partial affine source coverage (2026-10-10)

`SharedEdge::sample_endpoint` now uses the privately admitted exact affine maps for retained endpoints even when an original pcurve covers only part of the canonical source. The previous full-canonical-coverage guard unnecessarily refused these endpoints. Pair admission remains the authority for endpoint identity; intersecting outward enclosures does not create identity or weld rounded vertices. Complete retained coverage is still reported false.

Five mapped-edge unit tests pass (0.02 s), including differently covered source charts, a fixed/root mixed end and insufficient span budget. Eleven shared-edge unit tests pass (0.02 s). The current optimized whole-profile build predates this change; endpoint deployment and partial-map boundary meshing remain open.

### Partial affine boundary partition and replay (2026-10-10)

Interior sampling now uses privately admitted partial affine maps. An inverse outside either original chart returns an unresolved retained-interior result; no extrapolation is authorized. On the differently covered fixed-end control edge the shared boundary mesh has two vertices and one qualified segment. Zero cells retains that segment as pending, and the six-span chain preserves all five requests with three pending. Definition restoration reproduces canonical, UV, parameter and world intervals exactly. Reversed carrier sampling also passes. Five mapped-edge and eleven shared-edge unit tests pass (0.02 s each); `partial-affine-boundary-2026-10-10.json` records source identities and scope. Curved partial-map mesh qualification and deployment remain open.

### Curved partial affine boundary control (2026-10-10)

A quadratic world edge is now qualified between noncoplanar source surfaces whose original pcurves cover different canonical portions and have different parameter domains. Its common boundary mesh adaptively uses multiple segments, each with deviation upper bound at most 0.001 mm. With zero subdivision depth the complete segment remains pending. A mixed fixed/root endpoint variant independently qualifies the common plane root, keeps its interval-valued endpoint, and also meets the mesh tolerance. Six mapped-edge and eleven shared-edge tests pass (0.02 s each). `curved-partial-affine-boundary-2026-10-10.json` records scope/source identity. This control does not qualify arbitrary rational NURBS, vertex links, clipped face meshes or closed-solid assembly.

### Rational partial-map root endpoint control (2026-10-10)

The curved boundary control now exercises both polynomial weights [1,1,1] and rational weights [1,0.5,1]. Both use differently covered original charts and produce shared adaptive partitions within 0.001 mm, with fixed endpoints and an independently qualified mixed root endpoint. The rational root is at the exact symmetry plane x=1/2; no rounded plane-coordinate approximation is used. The six mapped-edge tests pass (0.02 s). `rational-partial-root-boundary-2026-10-10.json` records the bounded control scope. The full-map guard in shared-chord direction preparation is intentionally retained: its free-endpoint formula needs a separate partial-map derivation before support can expand.

### Published whole-product worker and browser acceptance (2026-10-10)

Optimized module SHA256 a3def4970533d7d4f30cfb7b90a775d7dec35cd5165d908b34f1db50efe50114 (13,000,641 bytes) finished packaging. `profile-bridge-whole-worker-2026-10-10.json` passes exact native parity for whole planar coverage, stopped coverage and curved normal-diameter coverage, together with error recovery, zero-budget refusal, cancellation and fresh Retry. The gate accepts `--expected-sha256` to prevent testing an earlier still-published package while compression runs. An earlier premature run correctly refused the previous pair-only report; no Binaryen semantic failure was found.

Actual browser acceptance reloaded the new kernel on port 5198, Shift-selected the temporary surfaces, enabled whole coverage and dispatched Inspect. The whole product excludes normal chords after 172 cells while the unsubdivided pair remains unproven. UI wording now states these distinct scopes explicitly. Screenshot `profile-bridge-whole-ui-2026-10-10.png` records the result. The calculation completed before the attempted Escape action, so that browser run does not qualify pending cancellation; the real-worker gate and controlled panel tests cover cancellation separately. Seven panel/transport tests pass (4.21 s). This is normal-chord exclusion for the proposed expression, not thickness, self-intersection or closed-solid admission.

A full native B-rep library regression (1,029 tests) is running separately in `/tmp/brep-current-all-lib-20261010.log`; no aggregate pass is claimed until its terminal result. The deployed kernel predates partial-affine edge sampling changes.

### Partial affine charts for chord diagnostics (2026-10-10)

`source_shared_chord_chart` retains exact rational canonical endpoint pairs and outward direct/inverse maps. The tangent scale is d(free)/d(canonical), including sign and the partial canonical span. Unit controls exercise reversed [1/3,2/3] maps, inverse midpoint containment, derivative scale ±24 and disjoint-domain refusal. The arithmetic test passes.

Wall-pair chord preparation now has a separate partial-map route; regular seam preparation retains its previous full-source contract. Partial chord anchors must stay in the conservative common canonical domain of both original charts. Tangent intervals are mapped back through the first original chart, and uncertain/outside coverage refuses rather than extrapolates. A current-source annular diagnostic regression is running in `/tmp/partial-chord-annular-regression-20261010.log`. No end-to-end partial-map Body fixture, general wall qualification or deployed WASM support is claimed yet. The previously launched full B-rep regression predates this chart change.

### Partial canonical carrier in an admitted Body (2026-10-10)

A cuboid source shell is serialized with one line carrier extended by a factor of three. Exact pcurve map endpoints become [1/3,2/3], while original region and fragment definitions stay unchanged. Restoration rechecks shell incidence, embedded geometry and volume. The admitted volume still encloses 6,000 mm³. Chord-only preparation selects that partial carrier; full-source seam preparation refuses it. Both midpoint chart maps enclose canonical 1/2, and tangent bounds enclose three times the original carrier vector. The current native test passes (0.08 s); `partial-carrier-body-native-2026-10-10.json` contains the restored body definition and diagnostic values.

The expanded test exposed a zero-budget tangent bug: the public diagnostic forwarded zero to a positive-budget derivative primitive. It now validates both endpoint charts first and returns (None,0) for stopped work. Outside-chart queries remain errors. The rational chart regression also passes. Annular diagnostic regression on the partial-map implementation passed in 82.62 s. A new WASM publication build includes the partial endpoint/interior/mesh and chord-map changes; deployment is not yet qualified.

### Partial carrier cut endpoint and restricted seam (2026-10-10)

Inverse canonical conversion now has an unclipped form. Partial-map retained-interval admission checks that outward inverse intervals lie inside both retained uses before chart clipping can remove unsupported tails. The source-profile cut endpoint and restricted angular path use this sufficient check. Full-source contracts elsewhere are unchanged.

The restored partial-carrier cuboid now exercises a proposed face: the midpoint cut owns canonical 1/2; its zero-span request remains unproved; angular inspection on [0.4,0.6] detects the actual sharp angle and issues no seam. Requesting [0,1] on the extended carrier remains `profile-seam-carrier-unproven`. The expanded native test passes (0.07 s); `partial-carrier-body-cut-native-2026-10-10.json` records the body and diagnostic values. Six mapped-edge and eleven shared-edge regressions also pass (0.02 s each). This does not qualify a smooth partial seam, general fillet assembly or deployed support: the running WASM build began before these cut/seam changes.

The full B-rep library run launched before the partial-chord chart changes completed successfully: 1,026 passed, 3 ignored, 0 failed in 864.82 s. Log: `/tmp/brep-current-all-lib-20261010.log`. This establishes the earlier source baseline; current partial-map changes are covered by the targeted regressions above rather than attributed to that older full binary.

### Smooth partial-carrier contact control (2026-10-10)

The contact certificate now separates complete original-pcurve coverage from canonical-carrier coverage. For an admitted partial affine line, literal clamped natural-boundary surface corners provide the owned support segment; canonical extension tails no longer enter the planar contact hull. Unsupported corner charts remain unresolved. No evaluated endpoint weld is introduced.

The body regression passes for both a sharp cuboid and a prism with two coplanar adjacent side faces (0.18 s). The smooth prism retains volume 6000, admits a proposal-owned seam on [0.4,0.6], and refuses [0,1] on the extended carrier. The native body and diagnostic report is `partial-carrier-smooth-body-native-2026-10-10.json`. Four root-partition regressions pass (2.37 s), including complete mapped chains and refusal of incomplete fragments. This is a straight coplanar seam control, not qualification of arbitrary curved fillets.

A separate cylinder control fails the original, unmodified shell geometry gate with `source-shell-different-face-contacts-unproven` for faces [0,1]. Cylinder contact support remains open; the failure is not evidence of a partial-map regression. The publication artifact c4ececb5133303d19b19ca7df08b664fd2f0d8f865781d1600381294c879739e (13004529 bytes) predates the latest cut/seam/contact changes.

### Cylinder straight-edge contact correction (2026-10-10)

Follow-up diagnosis localized the preceding cylinder refusal: the paired-fiber certificate proposed a separator for a two-control shared line only from collapsed poles. An ordinary cylinder has no such poles. The candidate search now also proposes finite coordinate-transverse planes through the original line. Every candidate still requires exact, nondegenerate plane-fiber proofs on both unchanged source surfaces, opposite material sides and owned boundary contacts; a candidate alone authorizes nothing.

`cylinder_straight_shared_edge_has_owned_paired_contact` passes (0.08 s): both face orders return certificates bound to their shared edges, budget counters remain bounded, and the complete original cylinder passes fresh shell-geometry admission. This supersedes the original-cylinder refusal above, without claiming a curved fillet or partial-carrier cylinder. The current running WASM build started before this latest candidate-search change.

### Partial canonical carrier on curved cylinder faces (2026-10-10)

The restored-body regression now also extends a cylinder's straight shared seam to a canonical carrier whose retained source range is [1/3,2/3]. Fresh shell geometry, volume, proposal-owned tangency on [0.4,0.6], midpoint cut ownership, zero-budget refusal and full-carrier refusal all pass. The recorded volume interval [9424.77771346221,9424.778130090697] encloses 3000*pi. The cut interval encloses canonical 1/2, and restricted tangency returns `profile-path-tangent-planes-qualified`; [0,1] remains `profile-seam-carrier-unproven`.

`partial-carrier-cylinder-native-2026-10-10.json` records unchanged source definitions and diagnostic enclosures. The expanded three-body test passed in 8.05 s. This qualifies a straight seam between adjacent curved cylinder charts, not a new rolling-ball fillet surface, arbitrary NURBS seam, mixed-radius corner, or complete wall-thickness proof. The full native run and WASM build already in progress predate the paired-fiber candidate correction and must not be credited with this latest change.

### Transition-shell contact transfer and remaining rational layout gate (2026-10-10)

The retained/original paired-contact transfer no longer requires complete canonical-carrier coverage. It still requires both current source-fragment definitions to equal the source edge uses, every current pair to be an original edge, complete coverage of the freshly recomputed source certificate's exact edge definitions, and a fresh plane-image certificate. The existing full transition-shell regression is running; transfer through an actual rational partial carrier is not yet qualified.

The attempted annular transfer fixture has a quadratic rational common edge. A three-span carrier can preserve its middle Bezier span literally while adding constant outer spans, avoiding rounded homogeneous quotient construction. Restoration refuses this layout with `source-affine-layout-unproven`: `curve_surface_affine::verify_algebraic` currently requires the canonical curve itself to be a single Bezier segment. `partial_rational_multispan_carrier_requires_affine_layout_proof` verifies this explicit refusal (0.68 s), not successful transition assembly. The next implementation needs exact span-local affine composition while preserving the original world curve and rational endpoint maps.

### Exact affine composition on an original B-spline knot span (2026-10-10)

`rational_bspline_composition_affine_identity` now computes the canonical curve's homogeneous polynomial on one knot span with exact de Boor arithmetic. Each recurrence retains polynomial numerators and a shared scalar denominator; Euclidean cross products cancel only that common factor. Original knot values, poles, weights and endpoint fractions remain authored leaves. Both mapped endpoints must lie inside the selected span by exact sign checks. The approximate midpoint in `curve_surface_affine` only proposes a span and cannot authorize containment.

Three `curve_surface_affine` tests pass (0.01 s), including a quadratic rational span with repeated knots and a non-Bezier quadratic span with simple internal knots. Both orientations qualify; changed weights are different; zero work and a range crossing spans are indeterminate. Input curve definitions remain unchanged. This removes the single-Bezier canonical-layout restriction for a single owned knot span. Multi-span pcurves/surface charts and a mapped interval crossing several canonical spans remain outside this predicate's current qualification.

The annular transition control now attempts positive restoration and contact transfer with a literal unchanged rational middle span and constant outer spans. Candidate contact planes use distinct original poles so repeated outer controls cannot hide the middle span's separator. The full integration regression and predicate library tests are running; successful complete transition-body admission is not yet established by this entry.

The complete existing `restricted_profile_seams_match_owned_neighbor_supports` regression passed in 417.91 s with the redundant canonical-coverage guard removed. The cad-predicates library suite passed: 41 tests, 0 failures (0.01 s). These do not substitute for the still-running partial rational transition integration test or qualify the latest sources in WASM.

### Exact fraction boundary refusal (2026-10-10)

The spline-span test now perturbs each numerator outward by one binary64 step at the owned interval boundaries. Exact containment refuses both excursions, for both rational repeated-knot and simple-knot controls, even though floating midpoint selection still proposes the middle span. All three affine-composition tests pass (0.03 s). This verifies that candidate-span selection cannot clip an unsupported tail into the identity proof.

### Piecewise canonical affine identity and transition integration (2026-10-10)

The exact predicate now also partitions the mapped canonical range across all intersected knot spans. Each positive-width intersection uses exact fraction comparisons; its source-pcurve interval is obtained by cross-multiplied affine inversion. Both homogeneous canonical and pcurve pullbacks are compared against the unchanged surface. Every nonempty span must be equal under one shared work budget. Zero work remains indeterminate; a difference on a later span cannot be hidden by earlier equal spans. The wrapper no longer uses a floating midpoint candidate.

Four affine-composition regressions pass on the piecewise implementation (0.01 s before the final wrapper simplification), including polynomial and nonuniform rational curves spanning an internal simple knot, both traversal directions, and a changed control on the last span. Outward one-step endpoint excursions cannot be clipped into the original span; they now may return an exact difference after inspecting the adjacent span. The predicate library suite passes 41 tests (0.02 s). Multi-span pcurves and multi-span surface charts remain unsupported by this composition path.

`partial_rational_multispan_carrier_transfers_owned_transition_contact` passed in 335.50 s using the preceding single-span composition implementation: the three-span carrier's unchanged rational middle span restored, the expression transition shell assembled, and the retained/original paired contact transferred in both face orders. The test also preserves existing bounded refusal and ownership checks. It does not establish complete contact absence or universal fillet-body admission; unresolved diagnostics remain relevant. This successful integration supersedes the earlier layout refusal for that fixture. The latest piecewise change still needs B-rep integration regression and a fresh WASM publication.

The earlier full B-rep run completed: 1,028 passed, 3 ignored, 0 failed in 1,033.69 s (`/tmp/brep-partial-contact-all-lib-20261010.log`). Its binary predates the later paired-fiber candidates and spline-composition work; it verifies that source snapshot only. The final piecewise wrapper's four affine tests pass (0.01 s). A fresh mapped-edge regression and partial rational transition integration run now target the latest piecewise source, with a requested contact-audit JSON artifact.

### Partial piecewise parameter domains and late resource exhaustion (2026-10-10)

Five affine-composition tests pass (0.02 s). The new control maps canonical [1/4,3/4] across an internal knot, with world domain [-5,3] and pcurve domain [10,18], for both polynomial and nonuniform rational curves. Exact source coefficients qualify in both traversal directions; a changed control on the later span is different. A budget one unit below the complete proof's measured work produces `Indeterminate(ResourceLimit)` with bounded charged work, never partial equality. Knot-order verification now propagates resource/cancellation reasons instead of replacing them with MissingProof. The predicate suite passes 41 tests after this correction (0.02 s).

A B-rep regression additionally attempts two independently owned crossing endpoints with distinct pcurve domains on a canonical line split at an internal knot, then replays the complete shared-edge definition and tests a perturbed root. That regression is running; its result is not established by the native composition tests above.

The extended `linear_roots_on_different_source_coverage_keep_exact_canonical_parameters` B-rep regression passes (0.02 s), including both the original one-span line and a two-span canonical line. Both crossing endpoints retain their independent root ownership; fresh shared-edge replay preserves exact definitions; the displaced root is refused. This establishes that the new piecewise correspondence reaches B-rep endpoint admission rather than only a standalone composition predicate.

The preceding WASM publication completed successfully with SHA-256 `284b888d0d0939d8fecdd394c08331fea63a2ff71ebc87d8295bdd6f19b387d9`, 13007632 bytes. It includes the earlier partial cut/seam/contact implementation but predates the later paired-fiber and spline-composition changes. A worker gate pinned to that hash is running; no claim is made that the newly added spline path is deployed by that binary.

### Require complete cross-face contact acceptance (2026-10-10)

Review found that the large transition regression previously allowed all geometry/material/volume checks to be skipped when the final contact audit was incomplete. The control now explicitly requires `complete_cross_face_contacts()` after its bounded contact passes. Diagnostic JSON is written at that point, before expensive material/volume work, so unresolved pairs remain inspectable even if acceptance fails. A new run of the partial rational fixture targets this stronger condition and the current piecewise source; its result remains pending. The already-running earlier binary is preserved and is not credited with the new assertion.

The command inventory remains incomplete P0 evidence: the saved coverage file lists 96 commands and zero commands fully qualified across the whole acceptance matrix. Geometry progress must not be interpreted as completion of that interface matrix.

The preceding piecewise integration run completed successfully in 415.36 s. Its `partial-rational-piecewise-contact-audit-2026-10-10.json` contains 29 faces and all 406 unordered cross-face pairs, zero pending pairs and no pair labelled pending. Pair [0,5] carries `profile-paired-contact` after the rational partial-carrier change. Native work: 52824 spans, 15115426 exact-work units and 4 driver cells. The complete audit triggers the test's owner-bound embedding and subsequent bounded material/volume checks; it is not a universal geometry or whole-wall thickness certificate. The separately launched run with the strengthened unconditional acceptance requirement remains active.

### Require an owned oriented-volume certificate (2026-10-10)

A dedicated `partial_rational_transition_has_owned_oriented_volume` control now runs the same rational partial-carrier reconstruction, transition assembly and complete contact acceptance, then explicitly requires an `OrientedExpressionVolume` certificate from the current embedded geometry. Its positive bounds must exclude zero and have width at most 20 cubic model units. An incomplete report cannot pass this control. The optional output `CAD_PARTIAL_RATIONAL_VOLUME_REPORT_PATH` records actual bounds, orientation, work and pair count, with `wholeWallQualified:false`. The new test is running; no volume bounds are claimed for this fixture yet. The older expression-volume artifact is separate evidence and must not substitute for it.

The strengthened partial rational transition regression passed in 293.25 s. Its required-contact audit again contains all 406 pairs and zero pending pairs. Thus the unconditional contact acceptance requirement is now tested, rather than inferred from the earlier conditional branch.

### Cross-piece retained-profile chord exclusion (2026-10-10)

The wall checker now recognizes that both `Retained` faces in this expression assembly restrict the same immutable selected profile in unchanged source coordinates. Its endpoint-pair self-chord proof is therefore applicable across those two pieces as well as within one face. This additional sufficient proof runs after distance and general normal exclusion, avoiding a new whole-profile derivative query before cheap distance separation. Other face-kind combinations are unchanged. No minimum-thickness certificate follows unless every product is excluded.

Five native profile self-chord controls pass (0.03 s), including preservation of a real cylindrical diameter, transformations and reversed endpoints. The full B-rep fixture is running with the same 10000-cell/1000000-span budget as `expression-wall-original-shared-standard-2026-10-10.json`; the new report is `expression-retained-cross-pair-wall-2026-10-10.json`. Pending-count and work improvements are not yet claimed.

### Partial rational transition: owned volume accepted (2026-10-10)

The dedicated `partial_rational_transition_has_owned_oriented_volume` control passed (283.41 s). The current artifact `partial-rational-oriented-volume-2026-10-10.json` records all 406 cross-face pairs, zero pending pairs, and an owner-bound oriented-volume certificate with positive bounds [7049.541261407235, 7069.534459645989] cubic model units. The interval width is 19.993198238754, below the requested 20; orientation is not reversed. Work: 5835 cells, 31993 spans and 63670 domain cells. This completes the volume requirement for this fixture only. `wholeWallQualified:false` remains explicit: volume and contact acceptance do not prove minimum wall thickness or general fillet admission.

Current inspection UI regression run: `profileBridgeInspectionController`, `profileBridgePanel`, `expressionInspectionController` and `expressionInspectionFocus` passed all 24 tests in 4 files (6.31 s). This is controller/panel evidence for bounded inspection behavior; it does not replace the pinned worker gate for the pending WASM publication, browser interaction acceptance or the full P0 command matrix.

### Correct the retained-piece wall comparison budget (2026-10-10)

The cross-piece retained-profile run passed its bounded regression (307.41 s), but its report has `domain_cells:0`, `trim_excluded:0` and 4213 pending products, whereas the baseline consumed 100000 contour-domain cells and excluded 271 products. Its 35111 spans cannot be credited as an improvement over the baseline's 38179 spans / 4085 pending products: the contour budgets differ. A matched rerun explicitly sets `BREP_EXPRESSION_WALL_TRIM_CELLS=100000`, 10000 wall cells and 1000000 spans, writing `expression-retained-cross-pair-trimmed-wall-2026-10-10.json`. Whole-wall admission remains unproven.

### Preserve completed Retry against an older generation (2026-10-10)

Two profile-bridge controller regressions now settle the cancelled request only after a newer Retry has already published its complete coverage. Both an obsolete success and an obsolete `CAD_TIMEOUT` failure must preserve the exact current result reference, cleared error/code and non-pending state. `profileBridgeInspectionController` and `expressionInspectionController` pass all 24 tests in two files (4.21 s). These exercise delayed callback ordering in the UI controller; actual worker/publication and full command-matrix acceptance remain separate requirements.

### Piecewise kernel published and worker checked (2026-10-10)

The piecewise affine-composition build completed: SHA-256 `271c328a0a95dc1af811cfdd646be015a1764c4b00eed22a577398147ac62ba9`, 13031120 bytes. The pinned `piecewise-current-publication-worker-2026-10-10.json` gate passes native parity, rational profile parity, whole coverage parity, preservation of a real normal diameter, zero-budget refusal, recovery from native validation failure and cancel/Retry via the actual worker client. This publication predates the new cross-retained chord proof and exclusion cache. A separate browser tab loaded the current app on port 5195, recovered the saved source body, rendered its diagnostic regions and selected an original edge with interval endpoints. No console errors were captured. This is a restoration/selection smoke check, not full fillet-command or WebGPU qualification (the other scene displayed a WebGPU-unavailable notice).

### Reuse proven retained-profile endpoint exclusions (2026-10-10)

The wall inspector now retains only successful endpoint-pair exclusions on the immutable selected profile. Reuse requires exact containment of both endpoint rectangles, allowing their exchange; union-box containment is insufficient. Failed and incomplete proofs are never stored. The cache is local to the fresh inspection and its angular policy; other face kinds cannot use it. Four wall product regressions pass (0.02 s), including a gap between endpoint regions, a one-step outward escape and non-finite input refusal. The matched 10000-cell / 1000000-span / 100000-trim-cell regression is running with output `expression-retained-pair-cache-trimmed-wall-2026-10-10.json`; no performance or whole-wall improvement is claimed yet.

### Matched cross-retained wall result (2026-10-10)

The explicit 10000-cell / 1000000-span / 100000-trim-cell run passed its bounded integration regression (353.09 s). `expression-retained-cross-pair-trimmed-wall-2026-10-10.json` remains incomplete: 4081 pending products in 58 face pairs, versus 4085 / 59 in the baseline. Span work increased from 38179 to 38693. The exact containment comparison in `expression-retained-cross-pair-trimmed-containment-2026-10-10.json` passes: no new pending product escapes its previous obligation, allowing only same-face endpoint exchange. Face pair [0,27] drops from 16 unresolved products to zero; pair [23,28] loses one product; additional subdivisions on [24,24] shrink their unresolved raw parameter volume despite increasing their count. Parameter volume is not physical thickness. This is a bounded coverage improvement, not a speedup or whole-wall certificate. The separate positive-exclusion-cache run remains live and is not credited with these numbers.

Native report output now additionally writes `<report-path>.request.json` with all actual wall/span/trim/control budgets, angular and UV tolerances, carrier mode and aligned-chord scope. This avoids inferring request budgets from consumed work; the transport report remains unchanged. `cargo check --tests` passes for this instrumentation (16.95 s). The already-running measurements predate the sidecar change and their settings are recorded explicitly above.

### Retained exclusion-cache experiment: no benefit, removed (2026-10-10)

The matched cache regression passed (303.22 s), but `expression-retained-pair-cache-comparison-2026-10-10.json` proves identical coverage and zero saved span work. Both reports have 4081 pending products, 38693 spans and 59339 cache hits. The cache, containment helper and its dedicated test were removed rather than retaining unsupported performance complexity. The cross-retained native chord proof remains: its separate matched result did close face pair [0,27]. The recorded cache artifact is experimental history, not the current implementation.

### Exact multi-span UV and canonical correspondence (2026-10-10)

The native affine correspondence no longer requires the original UV pcurve to be one Bézier segment. `rational_bspline_pair_composition_piecewise_identity` intersects the canonical and UV knot partitions with exact fractions, including reversal and partial canonical coverage. Homogeneous de Boor pullbacks retain common scalar denominators; no rounded split control or resampled knot is introduced. Every nonempty span pair must match the unchanged surface composition under one shared work/cancellation budget. The de Boor helper now supports both 3D and UV homogeneous coordinates.

Six affine-composition regressions pass (0.02 s), including rational weights, independent authored domains, noncoincident internal knots, partial coverage crossing knots, reverse traversal, a changed last UV span, changed knot and zero/late budget exhaustion. The predicate library passes all 41 tests (0.02 s). Seven B-rep mapped-edge regressions pass (0.04 s): the new independently partitioned UV uses bind to one partial canonical edge, fresh restoration preserves the exact definition, and a displaced fixed endpoint is refused. Existing root-ended and curved-edge regressions remain passing. This establishes native source correspondence; general root isolation on newly supported multi-span pcurves is not established by the fixed-end fixture. Surface charts still require one Bézier tensor patch; periodic charts and general non-affine parameter maps remain unsupported here.

All five current expression bridge regressions also passed (152.29 s), covering dispatch/closed incidence, mesh trim/deviation, inspection contacts/volume, radius admission/violation/unproven and pending triangle coverage. That run predates the new multi-span UV implementation. A fresh WASM build for the latest implementation is running (`/tmp/spline-pair-current-wasm-20261010.log`); the earlier SHA `271c328a...` does not contain this new path.

### Root candidates on independent multi-span UV uses (2026-10-10)

`curve_point_identity::verify` now admits exact point comparisons on original nonperiodic rational B-splines, using the new native `rational_bspline_uv_point_identity`. Scalar homogeneous de Boor evaluation retains a shared exact denominator at every interpolation stage. Span selection is based on exact authored knot/parameter comparisons. At internal knots all touching positive-width spans must agree; no rounded Cartesian evaluation or caller-supplied span can establish equality. Invalid/disconnected source curves refuse before identity.

Both point-identity regressions pass (0.00 s): rational multi-span curves with independent domains, interior knots, reversed traversal, changed poles, outside parameters and zero/late exhaustion. A disconnected-curve negative control is rejected by native curve validation. The expanded B-rep independent-partition fixture passed (0.02 s): four privately certified crossing endpoints retain their original expressions, fresh exact parameter candidates identify each root, both uses bind to one partial canonical edge, all four root checks run, and fresh definition replay preserves ownership. A candidate displaced by 1e-12 is refused. This qualifies exact authored candidate roots for this fixture; automatic discovery of all irrational roots and arbitrary multi-span root relations remain open.

The live spline-composition WASM build started before this root-point extension, so its eventual artifact must not be credited with the new root path without an additional build and pinned gate.

The complete mapped-edge regression selection passes all seven tests on the root-point extension (0.02 s), and the predicate library passes all 41 tests (0.01 s). These results qualify native regressions only; WASM publication and worker/UI acceptance of this latest extension remain pending.

### Irrational root ends without numerical parameter candidates (2026-10-10)

The multi-span mapped-edge regression now additionally covers x(q)=(q+q²)/2 and two independent UV knot partitions. Roots at x=1/4 and x=1/2 are irrational. Neither root is replaced by a binary64 parameter candidate. Original `SourcePoint` crossing expressions on both faces, exact source-cutter plane composition and a fresh monotone canonical-coordinate proof establish one shared endpoint per plane. Both one-span and two-span original canonical definitions pass fresh shared-edge replay. Missing plane evidence, zero driver work and a plane displaced by 1e-12 refuse.

All eight mapped-edge regressions pass (0.05 s). This supersedes the earlier statement that no irrational-root case had been qualified for multi-span UV uses. The support scope is still bounded: the canonical coordinate must admit the existing monotonicity/unique-plane proof; nonmonotone curves with several plane intersections and arbitrary root relations remain open. No universal root or fillet admission follows. The full native B-rep regression is being rerun against the source snapshot in `root-point-native-suite-inputs-2026-10-10.json`.

### Local plane uniqueness on nonmonotone canonical curves (2026-10-10)

Mapped root/root endpoint admission now maps both privately owned root enclosures outward to canonical parameter intervals and proves plane uniqueness on their full contiguous union. The new `curve_axis_driver::certify_domain` covers every original span intersecting that contained interval and requires consistent strict derivative sign across all admitted cells. It creates no rounded subcurve and does not use evaluated endpoint coordinates as proof. Zero work and invalid domains cannot qualify. The existing global driver remains unchanged for other callers.

The new two-branch control x(q)=(2q−1)² has two global intersections for each chosen plane. Roots on the same left branch qualify and fresh restoration preserves the complete definition. A mate on the opposite branch is refused, even though it meets the same physical plane and point. Missing plane evidence and zero driver work also refuse. All nine mapped-edge regressions pass (0.03 s); all three axis-driver regressions pass (0.02 s), including a domain spanning a turn. This extends the preceding globally monotone support scope to locally monotone root neighborhoods. General oblique curves with varying coordinates, tangencies and root relations without an owned plane remain open.

The full native B-rep run still active in `/tmp/spline-root-brep-all-20261010.log` was compiled before this local-uniqueness change. Its source snapshot remains the preceding one and must not be credited with current local-driver coverage. The live WASM build also predates this change.

### Local oblique-plane root uniqueness — 2026-10-10

The original-curve restricted driver now bounds the derivative of signed plane
position directly using outward interval arithmetic on authored plane anchors.
It does not require all but one world coordinate to be constant. Both owned
root parameter bounds form the covered canonical interval; strict sign over
that interval proves at most one plane intersection there. The original UV
root ownership and exact cutter-plane membership remain required.

Evidence: `/tmp/oblique-plane-driver-native-20261010.log` contains four passing
driver tests (0.01 s), including reversed normals, turning intervals, tangent
and degenerate planes, invalid anchors and zero work. The mapped-edge suite
`/tmp/oblique-plane-mapped-all-20261010.log` passes nine tests (0.06 s). The
nonmonotone quadratic fixture runs both axis-aligned and exact affine-sheared
surfaces/canonical curves: independently isolated irrational roots on the
same branch agree and survive fresh definition restore; opposite-branch roots,
missing planes and exhausted driver work refuse admission.

Scope: root/root pairs with a common owned plane and a strictly monotone signed
plane position on their enclosing canonical interval. This is not arbitrary
root equivalence, mixed fixed/root generalization, complete fillet-body
qualification, or current WASM/UI evidence. The concurrently running WASM
build started before this change and does not include it.

### Mixed fixed/root local uniqueness — 2026-10-10

Mapped mixed endpoints now use the enclosing outward canonical interval from
both immutable mapped uses for signed-plane derivative coverage. Exact fixed
parameter plane membership and original crossing cutter membership remain
mandatory. A failed strict local derivative proof can use the existing global
coordinate proof with only the remaining work budget; this preserves the
qualified zero-endpoint-derivative case.

`/tmp/mixed-local-plane-mapped-20261010.log`: nine mapped-edge tests pass
(0.07 s). The axis-aligned and sheared quadratic fixtures additionally match a
fixed q=1/4 with its independently owned root on the left branch, restore the
full edge definition, and refuse a fixed q=3/4 on the other branch despite
identical plane position. An initial run exposed the zero-endpoint-derivative
regression; the final run includes and passes that existing control. Direct
unmapped mixed endpoints still use their original global proof. No WASM/UI
qualification is inferred from these native tests.

### Published multi-span composition kernel worker gate — 2026-10-10

The completed build published SHA256
`ec4828e66d4bfb072d5153d85fb12d9a7974ddb84ffb0712add3a4d648bc015b`
(13,052,541 bytes). `spline-pair-publication-worker-2026-10-10.json` passes
native parity, whole-coverage transport parity, rational profile parity,
normal-diameter preservation, native error recovery, zero-budget refusal,
and cancel/Retry. A first invocation refused the old published SHA while
packing was still live; the successful invocation occurred after that build
process terminated. These are the bridge-chord worker gate cases, not direct
qualification of every new mapped-edge path or whole-wall certification.
This binary includes multi-span composition but predates root-point and local
plane uniqueness changes. A subsequent build is in progress, with selected
source hashes in `local-oblique-wasm-inputs-2026-10-10.json` and build log
`/tmp/local-oblique-root-current-wasm-20261010.log`; its success is not yet claimed.

### Current native integration controls — 2026-10-10

After the mixed fixed/root extension, the existing shared-edge suite passes
11 tests (0.02 s; `/tmp/local-root-shared-restore-20261010.log`). The mapped
root partition closed-body chain passes (one test, 2.36 s;
`/tmp/local-root-shell-chain-20261010.log`). This checks its existing fixture's
complete chain and does not qualify arbitrary fillet bodies.

The driver suite now passes five tests (0.01 s;
`/tmp/oblique-rational-driver-native-20261010.log`). The added rational
multi-span curve uses nonuniform positive weights and a nonunit authored
domain: outward coverage spans the internal knot without gaps, confirms each
local direction, refuses their union after an interior reversal, and refuses
insufficient work. The added source change is test-only and postdates the
selected input hashes of the in-progress WASM build; production driver code
is unchanged from that snapshot. The older full B-rep suite remains live and
predates local-plane changes, so it is not current full-suite evidence.

### Profile transition failure recovery messages — 2026-10-10

`ProfileBridgeCheck.vue` now distinguishes request validation, timeout,
worker startup/crash, disposal, transport and protocol failures with Russian
and English next-action hints. The existing keyboard lifecycle panel test now
injects all seven codes, checks each English recovery hint, retains Retry
focus, and finally recovers a native coverage result. Three panel/controller
suites pass 25 tests (13.11 s; `/tmp/profile-bridge-p0-errors-20261010.log`).
These are component/controller checks, not real browser crash injection,
Russian rendered-layout qualification, or the full P0 command matrix.
`npx vue-tsc --noEmit` also completed successfully after the panel changes
(`/tmp/profile-bridge-p0-typecheck-20261010.log`).

### Retained-piece union proof experiment — in progress 2026-10-10

Whole-rectangle intrinsic profile chord exclusion is now attempted for two
retained pieces of the same immutable selected profile, using their enclosing
original UV rectangle. The existing paired-endpoint retained proof remains
available. Only a complete positive native self-chord certificate excludes the
product; incomplete or negative calculations preserve its obligation. Cache
keys remain local to this immutable owner and angular policy.

A matched native integration run is live, with explicit settings
`BREP_EXPRESSION_WALL_CELLS=10000`, `BREP_EXPRESSION_WALL_SPANS=1000000`,
`BREP_EXPRESSION_WALL_TRIM_CELLS=100000`. Its eventual report is
`expression-retained-union-trimmed-wall-2026-10-10.json` and log is
`/tmp/retained-union-trimmed-wall-20261010.log`. Compare against the retained
cross-pair trimmed baseline (4081 pending products / 38693 spans), including
containment of pending domains. No reduction, speedup or full thickness
certificate is claimed before that comparison. The currently optimizing WASM
build predates this union-proof change.

### Global signed-plane fallback — 2026-10-10

The direct/global plane-uniqueness helper now also attempts outward signed
plane derivative coverage over the original canonical domain when the existing
exact coordinate proof is insufficient and driver work remains. It preserves
the existing endpoint-zero coordinate proof and all original cutter/point
membership requirements. The fallback supports multiple varying coordinates;
it never admits solely from sampled monotonicity or a proposed plane normal.

`/tmp/global-oblique-shared-edge-20261010.log`: 12 tests pass (0.02 s).
The additional quadratic control has x-y=q despite a reversal in y; the
oblique-plane proof succeeds, while a turning signed-plane function,
degenerate plane and zero driver budget refuse. Existing different-chart,
owned irrational root, mixed endpoint and two-root refusal cases remain green.
The currently optimizing WASM predates this fallback. The pending-region
comparison tool's four controls also pass, including rejection of expanded
regions and forbidden different-face endpoint exchange.

### Rendered Russian/English recovery hint controls — 2026-10-10

The Vue panel test now switches the live component between English and Russian
for each of seven worker/request failure codes. It checks the Russian next
action and preserved code, prevents raw opaque native failure text from
replacing the localized hint, verifies the localized Retry button, and confirms
that changing locale does not dispatch an additional calculation. The final
Retry still recovers the native result. Panel plus both controller suites pass
25 tests (6.99 s; `/tmp/profile-bridge-p0-locales-20261010.log`). This uses the
Vue test renderer, not a browser screenshot or the entire P0 matrix.

### Retained union experiment rejected — 2026-10-10

The run reached the native wall summary: 4081 pending products, 10000 cells,
100000 trim-domain cells and 38701 spans, versus the prior 4081 / 38693 spans.
It then FAILED while writing the request sidecar because Cargo ran the test
with the crate directory as cwd and the relative output directory did not
exist. No complete comparison report was produced and no pending-domain
containment claim follows from the printed counts. Preserve the failed log
`/tmp/retained-union-trimmed-wall-20261010.log` (363.89 s) as experimental evidence.
The extra cross-piece intrinsic union attempt was removed: no measured pending
count benefit, and eight extra spans. The earlier cross-piece paired-endpoint
proof remains. Report export now creates its parent directory; future runs
should still pass an absolute output path to make placement explicit.

### Worker artifact identity across Cancel/Retry — 2026-10-10

The bridge worker qualification script now sends the expected SHA/length to
each worker. The worker checks the identity module imported by its kernel
before accepting work, including replacements after cancellation. A publication
change refuses qualification rather than assigning results to the initial SHA.
`spline-pair-pinned-workers-2026-10-10.json` passes the existing parity/recovery
cases and records two checked worker artifacts for SHA
`ec4828e66d4bfb072d5153d85fb12d9a7974ddb84ffb0712add3a4d648bc015b`.
`tests/workerArtifactPin.test.mjs` passes both matching and mismatching startup
controls (0.58 s; `/tmp/worker-artifact-pin-controls-20261010.log`). This is a
qualification harness improvement; it does not broaden geometry qualification.

### Continuous knot-seam crossing admission — 2026-10-10

The independently partitioned complete root-chain fixture initially failed
native restoration with `NURBS_CROSSING_BOX`: the original selector crossed
multiple knot spans, including the root at a continuous seam. This limitation
was addressed in `uv_curve_crossings::certify_box`. The complete selector now
uses interval position/derivative hulls over all touched original spans. Each
span's locally normalized derivative is converted outward to the selector's
normalized coordinates before hulling. Validated curves are continuous, so
strict inclusion plus contraction bounds the continuous piecewise map, even
at C0 seams. Disconnected curves remain invalid; selectors touching more than
256 positive-width spans conservatively remain unresolved.

Seven crossing tests pass (0.35 s;
`/tmp/piecewise-crossing-native-20261010.log`), including a root at a C0 knot,
independent nonunit parameter partitions, two-root refusal and discontinuity
refusal. `prepare-multispan-root-body.py` authors only exact binary64 dyadic
fixture data and grants no admission. Its 57 independently partitioned source
curves are in `multispan-root-body-request-2026-10-10.json`.
Native bridge restoration now admits the whole four-face/seven-edge body,
preserving its original definition (1478.30 ms;
`multispan-root-body-native-2026-10-10.json`). The first failed attempt is kept
in `/tmp/multispan-root-body-native-20261010.log`; the successful run is in
`/tmp/multispan-root-body-native-piecewise-20261010.log`.

The older full B-rep run completed: 1033 passed, three ignored, 1856.57 s
(`/tmp/spline-root-brep-all-20261010.log`). It predates local/global signed-plane
and continuous crossing changes; it is not their full-suite qualification.
The published kernel SHA
`4d5d6ed2b990585ab6cee9ee2eb2538c48d5d34119f2e637c236e8f0b4ad8676`
(13,077,409 bytes) passes the pinned bridge-chord worker gate, with both
initial and replacement worker identities checked
(`local-oblique-root-publication-worker-2026-10-10.json`). It includes the
local-plane/root-point changes but predates global-plane and knot-seam changes.
The next full kernel build is live in
`/tmp/piecewise-root-seam-current-wasm-20261010.log`; no new binary result is
claimed yet.

### Independently partitioned knot-root body: native STEP acceptance

The current native bridge prepares STEP for the same four-face/seven-edge
body without replacing its crossing recipes with fixed points. The result is
`multispan-root-body-step-native-2026-10-10.json`; its endpoint error upper
bound is 3.858025010572423e-15 mm. The exported artifact is
`multispan-root-body-native-2026-10-10.step`.

Independent OpenCascade verification passes (`multispan-root-step-occt-2026-10-10.json`):
one valid solid, volume 0.16666666666666663 mm³ versus 1/6 mm³,
and maximum unit-cube bounds error 1.0000000005838672e-7 mm.
This qualifies this supplied fixture and exchange path, not general fillets,
whole-wall thickness, or the three-part browser acceptance matrix.
The new real-worker STEP/history/altered-candidate regression awaits the
kernel containing continuous knot-seam crossing certification.

Current-source rerun: all seven `uv_curve_crossings` tests pass (0.26 s),
and all nine `source_mapped_edge` tests pass (0.11 s), with zero failures.
Logs: `/tmp/piecewise-crossing-current-20261010.log` and
`/tmp/knot-seam-current-mapped-20261010.log`.
TypeScript checking completed without diagnostics
(`/tmp/multispan-root-worker-typecheck-20261010.log`).
The full NURBS library run is separate and its completion is not implied by
these selected regressions.

The full current NURBS library run passes: 844 passed, two ignored, zero
failures, 35.34 s (`/tmp/knot-seam-current-nurbs-all-20261010.log`).
The three UI/controller regression suites also pass: 25 tests in 4.86 s
(`/tmp/knot-seam-p0-controls-20261010.log`). These cover localized recovery,
request cancellation and stale-response handling at controller level, not the
full command matrix or browser crash qualification.
The pending knot-root real-worker regression now also requires cancellation
and successful Retry in a replacement worker, reproducing the native STEP
result. TypeScript checking passes after this test change.
A fresh full B-rep library run is active under
`/tmp/knot-seam-current-brep-all-20261010.log`; its result is not yet claimed.

### Rational C0 knot-seam controls

`rational_quadratic_and_cubic_c0_seams_keep_original_root_boxes` passes
(`/tmp/rational-knot-seam-current-20261010.log`). It checks degree-two and
degree-three rational curves with nonuniform positive weights, repeated
interior knots and nonunit original domains. Their supplied selectors cross
C0 seams; the certified boxes contain the original seam parameter and the
corresponding parameter on the other original curve. Swapped inputs also
qualify. Coincident curves remain unresolved and separated boxes are excluded.
This expands supplied-selector qualification; automatic isolation at seam
boundaries, arbitrary tangency and general fillet construction are separate.
The artifact pin controls were run separately with Node's test runner:
four passed, zero failed (`/tmp/knot-seam-artifact-pin-controls-20261010.log`).

### Preserve derivative correlation across original seams

An exact affine shear plus a translation of 1048576 exposed a conservative
failure in the new rational degree-two/three seam control: the original code
returned `Unresolved` (`/tmp/affine-rational-knot-seam-20261010.log`). Hulling
axis derivatives across C0 spans before preconditioning erased the relation
between tangent components.

The Krawczyk check now applies its fixed preconditioner to each original
span derivative enclosure before taking their hull. Each transformed bound
still contains its complete original span, and their hull covers the entire
continuous selector. This avoids replacing correlated span data with a larger
coordinatewise hull before the linear operation. All eight crossing controls
pass, including the affine rational case and the exclusion/coincidence/
multiple-root controls (`/tmp/correlated-knot-seam-native-20261010.log`, 0.26 s).

The already-running `piecewise-root-seam-current-wasm` build predates this
production change. Its successful publication must not be reported as WASM
qualification of derivative-correlation preservation. The current full B-rep
run also predates this change; a source-matched follow-up is still needed.

After the correlation change, complete native restoration still admits the
four-face/seven-edge body and prepares STEP. The entire returned result is
exactly equal to the previous native STEP result, including the STEP text
(`multispan-root-body-correlated-native-2026-10-10.json`). Thus this change
preserves that exchange artifact rather than silently changing its geometry.

### Automatic global knot-root isolation

Before decomposing a multi-span curve pair, isolation now attempts a complete
original-domain certificate, charged as one visited cell. A unique result
covers the entire product and is refined with the same continuous-span check.
If unresolved and work remains, the original span-product search continues
using the remaining budget. On exhaustion the unresolved complete domain is
retained, so no unvisited region disappears.

All eight crossing regressions pass (0.40 s;
`/tmp/global-knot-root-isolation-20261010.log`), including discovery of the
continuous seam root as one globally unique result and refinement to 1e-8
original parameter widths. This does not solve all multiple-root or tangency
cases at partition boundaries. The full current NURBS run is separate.

The preceding derivative-correlation source snapshot passed the full NURBS
suite: 845 passed, two ignored, zero failures, 58.66 s
(`/tmp/correlated-knot-seam-nurbs-all-20261010.log`), and all nine mapped-edge
regressions (0.06 s; `/tmp/correlated-knot-mapped-native-20261010.log`).
These runs predate the global isolation prepass.

The full NURBS library run including the global isolation prepass and the
one-cell exhaustion control passes: 845 passed, two ignored, zero failures,
87.08 s (`/tmp/global-knot-root-nurbs-all-20261010.log`). The one-cell
multiple-root stop retains the complete original parameter product and never
claims uniqueness or completion. The longer elapsed time is not a matched
performance comparison; other native builds/tests were running concurrently.

All 12 current-source shared-edge regressions pass (0.02 s;
`/tmp/global-knot-root-shared-native-20261010.log`), including original
root-ended edges on curved support charts and oblique signed-plane drivers.

### Scene-row layout recovery

Browser inspection found both direct row buttons sharing equal flex width:
the isolate button consumed the same space as the source name, and the long
name wrapped beyond the virtual row's 32 px height. The scene dock also grew
to its content minimum. Names now have a single-line ellipsis with the full
name in a title; action buttons keep intrinsic width and the dock permits
shrinking. Measured on the running 5195 UI: row 285×32 px, selection button
207.47 px, isolate 23.53 px, visibility/lock 26 px each; the name is 126.51×17
px with 263 px original text width. The model and selection were not edited.
Screenshot: `/tmp/cad-scene-row-layout-20261010.png`.
TypeScript and `git diff --check` pass. This fixes the visible overlap;
it does not establish a cause or fix for the reported browser renderer crash.

### Published knot-seam WASM: complete worker path

The piecewise crossing kernel build completed and published SHA
`1a90900b3c779322463cbb70f2a09f49473a59dcaa9d7496357a0a7189fb8113`
(13,080,430 bytes). The pinned real-worker regression passes: complete body
restoration equals the native result, STEP prepares, rename/Undo/Redo and
archive serialize/reparse preserve original recipes and edge vertex IDs,
altered endpoint candidates are refused, cancellation is observed and Retry
in a replacement worker reproduces the native result. Artifact:
`multispan-root-worker-2026-10-10.json`; log:
`/tmp/multispan-root-worker-fixed-20261010.log` (two selected tests, 5.42 s).
The separate bridge-chord publication gate also passes, checking initial and
replacement worker artifact identities and whole-coverage transport parity
(`piecewise-root-publication-worker-2026-10-10.json`).

The initial worker regression exposed a transport bug: a valid native denial
`mapped-source-endpoints-unproven` was rejected as `CAD_PROTOCOL` because
only `source-` refusal labels were recognized. The validator now recognizes
three explicit native mapped-edge refusal codes. Denial responses must still
contain no source body, edges, STEP or diagnostic qualification payloads.
New controls reject geometry-bearing denials and success/unknown labels.
The original failure remains in `/tmp/multispan-root-worker-20261010.log`;
the captured denial is `/tmp/multispan-root-worker-raw-20261010.json`.

This published binary predates derivative-correlation preservation and global
isolation. Their next build is running from the recorded inputs in
`correlated-global-root-wasm-inputs-2026-10-10.json`, log
`/tmp/correlated-global-root-current-wasm-20261010.log`.
No qualification of those later changes in WASM is claimed yet.

### Localized source-display refusal locations

Display restoration now preserves the native diagnostic in a structured
`SourceBodyDisplayError`. A validated native incidence pair index is mapped
back to that exact source definition's two face uses. The UI presents the
body name, edge/face address, an RU/EN next action and expandable raw reason.
Invalid, missing or out-of-range addresses never produce invented locations.
The three mapped refusal causes and work-limit stops have specific guidance;
other errors retain the general recovery message.

Three source-display controls pass (1.88 s;
`/tmp/source-display-error-location-tests-20261010.log`): denial is never
published as geometry, original addresses/locales survive, and a canceled
request cannot publish a late denial or its address. TypeScript and diff
whitespace checks pass. Error-state browser rendering is not yet qualified.

The full B-rep run started before derivative-correlation/global isolation
changes completed: 1035 passed, three ignored, zero failures, 1000.30 s
(`/tmp/knot-seam-current-brep-all-20261010.log`). A fresh source-matched run
is now active in `/tmp/correlated-global-root-brep-all-20261010.log`.

### Refusal notice browser acceptance

`SourceBodyFailureNotice.vue` is now used by the production source-display
error path and provides a local Retry button bound to refresh preparation.
The same component was rendered in a dedicated qualification page with the
saved real-worker denial (`source-refusal-notice-browser-2026-10-10.json`).
RU/EN guidance, edge 6 / faces 1 and 2, expanded native reason and Retry by
mouse and Enter were verified in the browser. Event count advanced exactly
0→1→2; console errors were empty. Screenshots are saved beside the report.
The page clearly records Retry events only; it does not impersonate native
recomputation or the complete modeler command flow. Real worker cancellation
and replacement-worker Retry remain covered by the separate worker artifact.
TypeScript and diff checks pass after extracting the production component.
The final source-display unit suite passes all four controls in 2.62 s
(`/tmp/source-display-error-locales-final-20261010.log`).

### Current partial-rational whole-product qualification run

The old extended runs already reached one million spans with unresolved
products: the both-paths run checked 92376 cells and retained 40797 budget
pending regions. Thus raising the old cell budget alone is not evidence of
complete thickness. Its largest costs were intrinsic (33.76%), corrected
pairs (24.26%) and retained pairs (21.79%); the dominant pending face was
transition self-products. These historical figures are not current timings.

A fresh bounded native run is active for the current *partial rational*
carrier fixture: 100000 cells, 1000000 spans, 1000000 trim cells, unchanged
1 mm aligned-chord threshold, 1e-4 sine-squared bound and 0.05 UV tolerance.
Log: `/tmp/partial-rational-wall-max-work-20261010.log`.
Expected report: `partial-rational-wall-max-work-2026-10-10.json`, with an
explicit request sidecar. No complete thickness or runtime improvement is
claimed before the run returns. Even a complete aligned-chord certificate
would retain its stated scope; unconstrained material thickness is separate.

### Current global crossing isolation: native body replay (2026-10-10)

`multispan-root-body-global-native-2026-10-10.json` repeats the original
independently partitioned root-ended body request against the current native
source, including derivative correlation and the global isolation prepass.
The complete result is exactly equal to
`multispan-root-body-correlated-native-2026-10-10.json`, including STEP text:
admitted, 4 faces, 7 edges, 5 vertices, STEP prepared, endpoint error upper bound
3.858025010572423e-15 mm. Native dispatch took 2807.42 ms under concurrent work;
this is not a standalone performance benchmark. This supplied-root request
checks compatibility, not coverage of automatic isolation's new global path.
Log: `/tmp/multispan-root-body-global-native-20261010.log`.
The current WASM publication and full BRep/partial-wall runs were still live at
this observation; their results are not inferred from this native replay.

### Correlated/global-root WASM publication and real worker gates (2026-10-10)

Build completed successfully. Public and packed runtime artifact:
SHA256 `7525335b37e56aa8548e38b12206c175ac2e0a74a441fb6d5b478269498090b9`,
13,082,826 bytes. All 1,811 Rust source inputs still match
`correlated-global-root-wasm-inputs-2026-10-10.json` after publication.
Earlier gate attempts refused the old public binary during packaging; no result
from those attempts is credited to this publication.

- `multispan-global-root-worker-2026-10-10.json`: actual worker, pinned hash,
  native complete-body parity, prepared STEP, identity-preserving history replay,
  altered-candidate denial, cancellation and replacement-worker Retry.
  Two selected tests pass, 6.59 seconds.
  Log: `/tmp/multispan-global-root-worker-published-20261010.log`.
- `global-root-publication-worker-2026-10-10.json`: public file, packed bytes,
  runtime identity and both worker instances checked against the same hash;
  native and rational-profile parity, whole-coverage report parity, preserved
  normal diameter, native failure recovery, zero-budget refusal, cancel/Retry.
  Command exited zero. Log:
  `/tmp/global-root-publication-worker-published-20261010.log`.

Whole-coverage parity means equal native and WASM reports, including pending or
refused regions; it is not a claim of complete physical wall thickness coverage.
The root body remains a bounded original-curve fixture. General NURBS fillets,
complex corners and the full UI command matrix are still unfinished.

Current matched full BRep library suite also completed: 1,035 passed, zero
failed, three ignored; 1,269.54 seconds under concurrent work. Log:
`/tmp/correlated-global-root-brep-all-20261010.log`. This suite includes the
latest derivative-correlation and global-crossing-isolation source snapshot;
ignored tests and unsupported geometry are not qualified by its green result.
