# Closed material ownership through internal NURBS knots

The complete classified closed-body matrix passes **12 tests, 0 failures** (17.66 seconds). Three new cases merge consecutive exact Bezier spans into one NURBS profile and corresponding material UV trim. Internal knots have multiplicity equal to degree; raw world domains [-4,4] and UV domains [7,11] differ. No carrier, world image, rational weight, requested geometric class or error tolerance changes. The class list addresses flattened Bezier spans, not input curve count.

New qualified cases:

- Hollow spatial RMF body: eight input curves, ten spans; actual 320 faces and 640 edges, including 256 C0 and 384 G2 edges.
- Hollow nonplanar material with varying authored affine frame: the same internal G2 joins and counts; complete boundary error 0.04016661902915116 under unchanged 0.05.
- Rational profile with a genuine internal G1 curvature jump: three input curves, four spans; actual 128 faces and 256 edges, including 64 G1 and 192 G2 edges. Both original and actual G1-to-G2 upgrades refuse.

Each case traverses original material ownership, source classes, actual coefficient binding, every edge role/class, periodic station seam, regularity, global filled-volume embedding/topology and orientation. Measured actual proof budgets replay and one-less budgets refuse; joint role/census budgets behave likewise. Original inputs remain unchanged. The rational case also verifies all eight tensor gauges with face permutation and rejects a sub-tolerance coefficient mutation.

Only the Rust test file differs from the preceding classified-closed-solid packet. All 6,619 recorded source/Cargo/toolchain inputs were hashed during and after this matrix and match. Production implementation is unchanged. The preceding full BRep result (2103 passed, 0 failed, 5 ignored) is historical evidence for that unchanged implementation, **not a fresh full-suite result**. This run executes all 12 classified closed matrix tests; other library tests are filtered. Full logs, command, exit code and source hashes are retained here. No geometry moved to TypeScript; no proof ceiling or tolerance was increased.

This expands qualified internal-knot coverage. It does not complete the broader objective for arbitrary loft/miter-law correspondence or every frame/cap/seam combination. Packaged WASM, browser, independent STEP, remote CI and publication are outside this native run. Older receipts and unrelated drafts are unchanged.
