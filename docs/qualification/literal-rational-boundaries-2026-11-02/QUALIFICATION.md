# Literal rational boundaries and independently refused cap rounding

Implementation: `0c0d8b06173bc8c3d6f422de5d1ab5aa9b8680cb`.

Full native results on the final source state: **nurbs-core 2073 passed, 0 failed, 1 ignored** (56.18 s); **brep-core 2107 passed, 0 failed, 5 ignored** (676.29 s). The full BRep run includes the 12 classified closed original/actual cases, original rational hollow cap matrices and the new mandatory rounded-cap refusal. Four focused tests also passed, including two new decomposition regressions. Ignored tests remain ignored.

## Implemented

`Curve::decompose` now selects literal poles and weights for nonperiodic clamped Bernstein-segmented curves. Internal multiplicity must be at least degree. Each positive original knot interval owns exactly its degree+1 original poles and weights; only endpoint knot replication is constructed. No multiplication/division by weights, insertion, knot ratio or clock normalization occurs on this route. Unsatisfied premises retain the existing trim-based route. Detection is linear in knot count. Unit tests cover a nonuniform rational Bezier whose homogeneous multiply/divide changes a binary64 pole, unchanged raw-domain evaluations and first/second derivatives, and a two-span rational curve with an internal knot and separately retained raw intervals.

The capped-loft refusal now reports the first unresolved/non-equal boundary use, native exact outcome and actual work. Proof allowances and predicates are unchanged. This distinguishes a genuine exact discrepancy from exhausted work.

## New independent counterexample; not a positive Solid qualification

The existing unequal rational cap/hole source fixture independently obtains its original classified material and full filled embedding. A supplied retained upper cap formed by adding a Z translation of 2 to each binary64 pole does **not** retain that source's exact trim trace. Its small nonplanar rational term cancels on material boundaries in the original homogeneous coefficients; separate rounded additions break that cancellation. The native boundary predicate reports `Different` on an upper-cap hole use, with total work 805058 below the unchanged 1000000 ceiling. The preserved exploratory counterexample log shows this exact failed positive attempt.

The final regression requires that independently proven original source, then requires the complete actual constructor to refuse the rounded cap with `BREP_RATIONAL_SWEEP_REFUSED` and the exact `Different` diagnosis. Original input definitions remain unchanged. No failed actual geometry borrows the original source certificate. The decomposition fix does **not** repair this independent cap-rounding defect, and this packet does not claim that it does. No cap was flattened or replaced to turn the attempted case green.

## Evidence and limits

All 6619 recorded Rust/Cargo/toolchain inputs match the during-BRep and after manifests. Only four files differ from the preceding multispan packet, listed in result.json. NURBS production inputs were already final for both focused and full NURBS commands; the later BRep test edit only converted the discovered invalid-retained-cap case into an explicit refusal regression. Compiler, settings, exact commands, terminal exit codes, source hashes and complete logs are retained. Predicate sources are unchanged; their prior 131/0 result is reused, not rerun here. Older receipts and unrelated drafts are untouched.

The full objective remains open. In particular, a constrained rational-cap correction must preserve the complete exact material trim traces while bounding filled-interior displacement; ordinary per-pole translation with rounding is insufficient. General loft/miter-law correspondence and remaining frame/cap/seam combinations also remain open. This run qualifies native Rust work only; packaged WASM, browser, independent STEP, remote CI and publication were not performed.
