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
