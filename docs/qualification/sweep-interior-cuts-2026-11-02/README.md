# Ordered algebraic branches with independent interior cuts

Qualified: 183 tests passed (109 predicates, 62 continuity, 12 native seam tests), plus a no-codec check. Rust implementation only; no WASM publication or UI interaction.

The new exact route groups consecutive original spans at common Cartesian chart endpoints. Inside a group, original knot counts and cut positions may differ. Strict same-direction monotonicity in one locally selected Cartesian coordinate proves a unique cover. For every overlapping pair of original coordinate ranges, exact bivariate divisibility proves position and the existing normal-jet G1/G2 relations. No inverse root is approximated, no knot or trim is rounded, no tolerance is introduced. The coordinate may change at consecutive chart groups. The caller's PredicateContext owns all work, including unsuccessful proposals.

The positive regression uses the existing four-branch closed contour with incompatible clocks qA=(s+s²)/2 and qB=(t+t³)/2. One original branch is cut at s=1/4 and independently at t=1/2. The latter source preimage solves s+s²=5/8 and is irrational. Both original B-spline surfaces remain below the unchanged 33-row admission bound. A lower-level regression also checks different original span counts (4 vs 8).

Limits and proof boundaries

- No global monotone Cartesian coordinate is required; each group still needs a proved local monotone coordinate and exact common chart endpoints.
- Arbitrary internal knots of these proved chart groups need no matching inverse knots. This is not a universal algebraic-curve decomposition algorithm.
- Unsupported branch graphs, chart transitions with no proved common endpoints, nondivisible residual classes, and failures of intrinsic regularity still refuse.
- Local G1/G2 does not grant continuousBound, global embedding, full-shell smoothness or a Solid. The native protocol must reject caller-forged global flags.
- MAX_WORK=1,000,000, 4096 expansion terms, the 33-row admission bound, existing cell/depth/deadline limits and tolerances remain unchanged.
- No fresh STEP/UI/WASM or entire dirty shared-main qualification is claimed by this focused change.

Evidence

`source-r4.json` identifies the frozen source; `changed-paths.json` binds the five changed main paths. `qualify-r4.py` runs the predicate suite, surface continuity suite, three native seam suites and a no-codec check. `result.json` is the final authority when its state becomes complete. The previous r7 archive is the baseline; unrelated dirty main changes are preserved and excluded from this snapshot.

The initial high-level fixture split all four branches and correctly refused before proof work because it exceeded 33 rows. r1 remains preserved; r2 limits the fixture to one independently cut branch. No limit was raised to make it pass.

An r2 attempt exhausted the unchanged 1,000,000 work budget. r3 fixed redundant divisibility proofs by proving equality of all original polynomial normal jets before reusing a result. Original interval coverage and monotonicity checks are never skipped. The final r4 adds a targeted normal-jet perturbation to ensure same seam positions cannot borrow that reuse.

Proof outline

For each admitted group, let cA(s) and cB(t) be the selected rational Cartesian coordinate on the two original piecewise domains. Exact endpoint/range checks and strictly signed derivative numerators show that both are bijections onto the same oriented scalar interval. Thus phi=cA^-1 o cB is a unique increasing cover even when the inverse values at interior knots are irrational. Every original pair whose scalar ranges overlap is checked. The relation f=NA(s) WB(t)-WA(s) NB(t) vanishes on that graph; exact divisibility of the Cartesian and normal-jet residuals by f makes those residuals vanish there. Complete original range coverage, adjacency and common group endpoints sew these covers in original parameter order. The independent intrinsic-knot and normal-regularity admission still applies. Polynomial reuse requires exact equality of every original normal jet coefficient; coordinate-only equality is insufficient and is covered by a targeted refusal regression.

The final source snapshot contains 3287 files. All five changed main paths match it. The whole dirty shared main is not qualified. `qualified-source-r4.tar.gz` preserves the exact isolated source.
