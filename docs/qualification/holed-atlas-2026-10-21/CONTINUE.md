# Continuation boundaries

This milestone qualifies native standalone source-atlas certificates only.
New classes: multi-hole curved trims, higher-order monomial stationary poles,
straight-seam rational clocks and curved graph-carrier nonlinear seam clocks.
64 native tests passed; separate Rust process verified 12 proofs and rejected
44 forgeries. See manifest.json and native-replay.json for exact inputs.

Remaining work:
- Non-monomial rational/conical poles require proved limits and punctured-chart embedding; do not admit zero weights or drop singular cells.
- General rational space-curve seam inversion needs rational inverse/branch proofs beyond recoverable linear observables.
- Multiple tensor knot cells and nonmonotone trim boundaries need exact subdivision/composition.
- WASM, browser, host transport and dist require separate build/runtime qualification.

The v4 enclosure recipe is frozen in atlas_legacy. Expanded standalone source
admission must not silently change archived enclosure reports or imply complete
Minkowski-sum topology. Preserve the existing nominal-chain checks.
