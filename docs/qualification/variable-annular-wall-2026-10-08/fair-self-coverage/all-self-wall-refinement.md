# Native original 27-face self-wall qualification

At 5.99 mm and sine squared 1e-6, ordinary coverage followed by pole extension and independently bounded ordinary refinement certifies all 27 original faces. Prior certificates are retained and the Body definition is unchanged.
Additional regular refinement spends 46,266 cells / 94,762 spans solely on the remaining face 5. Pole extension spends 3,370 cells / 74,396 spans and 40,114,685 exact operations.
723 NURBS tests pass; 2 are ignored. Refinement ownership, threshold, angle and repeated-pass rejection checks pass.
This is near-normal chord coverage within individual faces, not general self-intersection absence, not qualification of distinct-face pairs and not a whole-wall certificate. Native bridge transport for the refinement pass, updated WASM and worker qualification remain pending.

The native bridge packet now confirms all 27 faces, retains all prior pair proofs and leaves exactly 160 distinct-face obligations unproven. Aggregate work: 76,404 cells / 226,388 spans, exactly equal to per-face sums and bounded by the separately requested budgets. Whole-wall qualification remains false. Full B-rep suite: 938 passed, 3 ignored. Updated WASM/worker validation remains pending.
