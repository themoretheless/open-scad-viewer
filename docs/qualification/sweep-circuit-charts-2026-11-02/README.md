# Exact arithmetic-geometric mean circuit certificates for sweep transitions

Qualified: 355 tests passed (116 predicates, 65 continuity, 25 world traces, 119 sweep/BRep, 30 native protocol tests), plus a no-codec build. All geometry and proof code is Rust. No UI/focus interaction, no WASM publication and no increased numerical/input/work limits.

New sign proof

After the existing Bernstein and weighted-square routes, the algebraic continuation owner can now decompose a cofactor into exact two- or three-positive-monomial arithmetic-geometric mean circuits. Positive outer monomials must have even exponents. Their exponent average must equal the negative monomial exponent. For arity n=2 or 3, the proposed negative coefficient delta is accepted only after the ORIGINAL exact rational inequality delta^n <= n^n product(a_i) is checked. No square/cube root is evaluated.

A successful step removes that nonnegative circuit, with exact coefficient conservation. The negative coefficient may be allocated across multiple circuits by source-derived rational fractions; all allocations are checked before use. Remaining positive even monomials are nonnegative. There is no sampled sign acceptance: a few exact evaluations only reject impossible global proposals and grant no positive evidence. Search failure remains MissingProof. All work, including failures, spends the caller's existing PredicateContext.

Strictly monotone coordinate covers and independent original regularity remain mandatory. A sign-definite cofactor's zeros on the coordinate graph belong to the finite critical-pair set; continuity extends the exact common-factor/normal-jet identities across those nodes. The existing original-join, orientation and full parameter-cover checks remain in force. Local evidence does not promote global embedding, continuousBound, full-shell G2 or a Solid.

Genuinely beyond SOS

The regression polynomial M=s^4 t^2+s^2 t^4+1-3s^2 t^2 is certified by the three-term circuit. Its one-ULP negative constant perturbation refuses. M is a nonnegative polynomial that is not a sum of polynomial squares; this is not just a failure of the older greedy constructor. The primary mathematical reference is Blekherman, Kozhasov and Reznick, On odd powers of nonnegative polynomials that are not sums of squares, introduction and section 3.4: [arXiv:2407.21779, sections 1 and 3.4](https://arxiv.org/html/2407.21779v1). Setting X3=1 in its Motzkin form gives this test polynomial.

Actual rational transition

R(q)=(q^3+9q)/(1+q^2), Y=q^2 has R'=(q^2-3)^2/(1+q^2)^2 and a regular physical trace through the stationary X projection. Its divided-difference cofactor K=s^2 t^2+s^2+t^2-8st+9 is certified by two allocated two-term circuits. The full-rectangle owner and the complete exact continuation are tested under the original work limit, including one-short refusal. K also has an alternative SOS identity, K=(st-3)^2+(s-t)^2; it is outside the previous constructor, not mathematically outside all SOS. The Motzkin regression establishes the genuinely non-SOS extension.

The Surface fixture uses degree-elevated positive integer weights and an exact common XYZ scale to make every Cartesian pole integer. Its target uses a positive rational Mobius clock with exact integer weight scaling. No floating-point fitting, parameter-root approximation or rounded knot insertion is used. The public API is free to use an earlier established exact map proof for this fixture; the lower-level full-box test explicitly requires the new circuit route.

Scope

This is a sufficient exact circuit class: only the discovered two/three-term exponent-average decompositions and checked rational allocations are accepted. Arbitrary nonnegative polynomials, all possible barycentric weights, unresolved components or singularities without admissible regularity are not claimed. MAX_WORK=1,000,000, expansion terms=4096, continuation cells=4096, depth=12 and existing deadlines/tolerances remain unchanged. Full dirty-main/WASM/Rush/viewport delivery and the complete independent STEP/UI matrices remain separate gates.

The native affine-tilted arc and arc-clock G2 tests freshly export STEP, import it and recompute certificates. These are Rust round-trip checks; no new independent OpenCascade matrix is claimed.

Final snapshot: 3295 source files and six changed main paths, all hash-verified. The source archive and incremental patch preserve the exact native qualification input. No fresh external STEP/UI matrix or WASM publication is included.
