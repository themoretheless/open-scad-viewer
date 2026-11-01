# Varying rational affine source material

Regression implementation: `59509526c40669f243c21501911ffc1f24c730d6` (Rust).
The production spatial-frame theorem remains the one qualified at
`641c030d789b6c35c22245d1afc1d8fe2930bebe`.

Two source fixtures combine nonconstant rational uniform/axis scales and center
laws on distinct raw domains with a spatial rational closed path, periodic twist,
nonplanar filled carrier and opposite-winding hole. Authored and guided frames
are tested separately. Each requires all global source guarantees, C2 periodic
motion, mixed G2/C0 joins, exact-budget replay, source identity and unchanged
original laws. A broken affine-law seam refuses even with all profile classes C0.
An independent closed-form regression checks raw-clock derivative normalization
and the complete product rule. See [THEOREM.md](THEOREM.md).

Frozen validation passed: 318 unique native tests in 31 focused groups and
10 Node infrastructure tests. Full nurbs-core: 2058 passed, 0 failed, 1 ignored.
Full brep-core: 2061 passed, 0 failed, 5 ignored. Focused and full coverage overlaps
and must not be summed. Ignored tests provide no evidence. All 4928 frozen source
files remained unchanged. [verification.json](verification.json) records commands,
logs, source hashes, catalogs, environment and backup hashes.

[completion-audit.md](completion-audit.md) maps the full objective to the evidence
and remaining gaps. This packet does not establish whole-product completion,
retained Solid, WASM, browser, STEP or remote CI results. Architecture remains
at 68 diagnostics with no baseline edits, so publication is held.
