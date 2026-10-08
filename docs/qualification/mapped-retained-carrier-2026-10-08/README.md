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
