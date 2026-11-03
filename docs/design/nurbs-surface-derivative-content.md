# Exact surface partial admission through global axis-content cancellation

The existing NURBS-0243 constructor used to refuse some exactly polynomial
fields hidden by a positive rational denominator. A degree6 source with weights
`[5,2.5,1,.5,1,2.5,5]`, x poles `[0,0,20,30,0,20,60]` and linear V extrusion
is exactly `S=(60u²,v,0)`. Its U partial is `(120u,0,0)`, yet the uncancelled
representation had mixed-sign Bernstein denominator coefficients and returned
`UnprovedDenominator`. The new typed regressions reproduced that refusal before
the fix; the source coordinates, geometry limits and request budgets are unchanged.

Before degree/storage conversion, one common U-only factor is computed over
every bivariate coefficient slice in all three numerators and the denominator.
The same operation follows in V. Each quotient is reconstructed exactly; the
actual stored output is still proved against the ORIGINAL uncancelled target.
Per-row or per-column factors would change a general tensor function and are
not admissible. This is separable axis content, not a general bivariate GCD.
The shared1m-work/4096-bit ledger includes extraction, GCD, division, quotient
reconstruction, storage and the fresh full-polynomial stored-output proof.

The positive fixture, its transpose, and a two-axis rational source now produce
exact degree1×1 fields. Existing periodic fields also lose redundant degrees:
18 prior patch encodings change, while the same63 patches retain exactly the
same original-source partial functions and absolute UV cell coverage. Across
that prior dataset the control count falls from370 to316. This is structural
compression, not an asserted numerical evaluator-bit or speed improvement.

This upgrades existing capability243 without increasing the catalogue count.
No installed artifact, JSON/WASM/Rush/editor/STEP interface or topology/normal
certificate is introduced. Broader mixed-variable factor cancellation and an
exhaustive alternate positive-weight/degree search remain separate work.
