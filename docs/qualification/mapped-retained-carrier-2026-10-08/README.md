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
