# Native algebraic CST checkpoint

This extends NURBS-0220; the catalogue remains 221/1000. `cst_algebraic` constructs a two-span polynomial curve directly, using the original checked profile limits and degree <=25. Integer class exponents use a linear chord chart; half-integer leading exponents use a quadratic chart. Constant Bernstein shape rows collapse before degree selection. No sampled fitting is used in this path.

`source.json` binds the changed implementation/test files to base e57b19fe7. Default public checks cover 89 unique tests; the selected no-codec CST targets pass 17 tests. The final degree-25 fixture covers 51 controls, alternating coefficients and chord scales 1e-200, 1 and 1e9. Exact rational control tables independently test the nominal error enclosure. Shared work exhaustion refuses rather than returning a partial curve. Existing limits are unchanged.

`audit-identity.py` uses exact rational arithmetic to verify the coefficient identities for 2,600 admitted exponent/order classes, 13,182 independent shape-coefficient identities, the chord identity and nonnegative coefficient weights. This finite symbolic audit is not a Rust runtime or global qualification claim. It complements the Rust tests and the outward interval argument documented in `docs/design/nurbs-cst-profiles.md`.

The returned bound is continuous nominal position error for the mathematical curve represented by binary64 controls; evaluator roundoff, derivatives, regularity, topology and transforms are outside that bound. No JSON/WASM/Rush/editor/STEP, full workspace or main-delivery qualification is claimed. The default-feature full library passed 2043 tests, failed zero, with one existing ignored microbenchmark and no filters; see `full-library-check.json`.
