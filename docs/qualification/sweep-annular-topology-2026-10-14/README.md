# Native retained annular cap — 2026-10-14

The unchanged original spatial hollow request now has a coherent retained cap and a positive **native global volume certificate**. Its original wall surfaces, boundary curves and vertices are retained. Replacing the final planar cap by four radial rational patches gives29faces; every coedge and trim join is exact, all29faces are injective, all406distinct face pairs are classified, and shell orientation is certified. No proof or tolerance limit was increased.

Replacing both ends gives32faces and a positive global certificate covering496pairs through the generic algorithm, with no planar-cap optimization. Thus the new cap patches are included in the geometric audit, not excluded from it.

## Representation and proof

`retained_annular_cap::construct` retains the two authored corresponding Bezier rings and their positive weights. `replace` shares their existing wall edges, adds exact radial seams, preserves other surfaces, and returns an explicit old-to-new face map. Construction alone always remains unqualified for global embedding; a fresh whole-model volume audit is mandatory.

The shared-boundary proof aligns a reversed Bezier boundary by permuting its original poles/weights only. A new sufficient method glues the two natural parameter half-rectangles continuously along the exact common curve. One fixed integer projection must have positive symmetric Jacobian on every interval cell of **both complete faces**, using the existing native monotonicity checker. The resulting map is strictly monotone on the convex glued rectangle, excluding any additional contact. Sampled derivatives choose parameter scales and a projection only; they never certify the result.

The existing exact polynomial station split is also used through an axis permutation. For weights independent of U, it proves `S=R+P(U)` by exact pole subtraction, enclosing `Su=Ru+Pu` and `Sv=Rv`. Unrepresentable differences still refuse this split. This removes the long stationary drift from rational derivative bounds without changing geometry or dropping a derivative term.

## Validation

Nine annular-cap tests and43existing geometry regressions passed inside the current Rust crates. Coverage includes original source retention, both tube ends, full406/496pair coverage, orientation, independent rational ring weights, real shared-work exhaustion, overlapping surfaces, and a one-bit boundary perturbation that invalidates the global certificate. See `native-receipt.json` for source and log bindings.

## Filled retained-reference comparison

The subsequent `retained_annular_cap::reference_filling::inspect` proves the symmetric set-distance between the entire new annular filling and the old bilinear trimmed cap. Its premises are recomputed from the actual source and replacement models. Supplied work counters, face maps and embedding flags are not accepted as geometric evidence.

The old UV region must have a simple outer loop and one disjoint, oppositely oriented contained hole. A native rational UV annular sheet retains those exact boundary curves. Every complete patch must be injective and every distinct patch pair must exclude additional contacts. Together with the generated annular topology, these checks establish coverage of the full Jordan region between the loops. Boundary proximity by itself cannot set the coverage flag.

Matching world/UV rational bases give the same radial mixing coefficient throughout each parameter rectangle, including independent positive weights on the two rings. The full filling error is bounded by the maximum whole-curve lift error plus an outward enclosure of the bilinear interior defect. The latter uses `beta*(1-beta) <= 1/4` and the mixed surface coefficient; it is not discarded merely because the boundary lifts are exact. All boundary and interior pole products share one product budget, while cap construction and UV construction share one exact-work budget.

The final run passed **13 annular-cap tests**. Both original end caps have complete UV coverage and a filled-reference error below `1e-10`. For the final cap the measured certificate is `2.0670502964490726e-14`, using3436exact-work units and108products. The same run recomputed the global native volume certificate covering all406face pairs. Source hashes were stable across the two checks. `filled-reference-receipt.json` binds the sources and saved logs.

Negative controls cover missing patch ownership, a one-bit surface change, zero error tolerance, exhausted exact/product budgets, unresolved region winding, incomplete injectivity/pair coverage, and geometrically equivalent boundary curves with mismatched stored homogeneous bases. An independent dyadic bilinear example has exact boundary lifts but a nonzero interior defect, preventing a boundary-only shortcut.

## Remaining

The subsequent native source-owned annular mode closes the original spatial hollow's source-to-filling link; see [the source qualification](../sweep-annular-source-2026-10-14/README.md). Its delivery and smoothness scope are recorded separately. The receipts above remain snapshots of the earlier retained-reference stage.

A global geometry certificate and a bound relative to the **old retained cap** do **not** by themselves establish deviation from the original authored sweep. This retained-reference report cannot populate the original-source `continuousBound` or authorize Solid on its own; the subsequent source-owned proof is a separate obligation.

Still required: constructor-owned source/error integration, logical end-cap grouping across multiple faces, native command/WASM/Rush/viewport/Solid delivery, and fresh independent STEP/UI qualification for the resulting artifact. The earlier207STEP/468UI results apply to the previous isolated artifact and are not reused for this native change. The full library goal remains active.
