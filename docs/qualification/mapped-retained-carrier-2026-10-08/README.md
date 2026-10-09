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
