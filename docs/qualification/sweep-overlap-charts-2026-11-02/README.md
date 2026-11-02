# Algebraic continuation through overlapping charts

Qualified: 345 tests passed (110 predicates, 63 continuity, 25 world traces, 119 sweep/BRep, 28 native protocol tests), plus a no-codec build. Rust computation and proof only. No changes to authored geometry, WASM distribution, or UI.

The new continuation route does not require corresponding internal chart endpoints or a global monotone Cartesian coordinate. It computes a primitive common polynomial factor of the ORIGINAL Cartesian position residuals by exact pseudo-remainder sequences over Q[source]. Every candidate is independently rechecked by exact divisibility of the ORIGINAL position and existing G1/G2 normal-jet residuals.

For each local source/target rectangle, a Cartesian coordinate must have strictly same-signed derivatives on both intervals. The quotient of its position residual by the common factor must have a strictly nonzero tensor Bernstein enclosure on that entire rectangle. Thus scalar coordinate agreement owns a unique local correspondence and cannot select an unrelated real algebraic branch. Exact dyadic Bernstein subdivision is cached from original coefficients; it never creates rounded source knots.

An ordered frontier walk advances whichever original/subdivision interval ends first in the selected coordinate. The next Cartesian point is exact even if its inverse parameter on the other side is irrational. Existing point membership and original adjacency carry that same root into the next overlapping chart. Only advancing both full original parameter covers grants equality. A second lap, reversal, uncovered tail, missing branch exclusion, work exhaustion or a failed normal-jet identity refuses.

Two exact conditioning steps reduce work without proof relaxation: adjacent intervals may share a proof only after equality of ALL polynomial normal jets, and one common positive diagonal change of world coordinates applies to ALL jet orders on BOTH sides. These are private derived proof coordinates; original sources, intrinsic-knot regularity checks and geometry remain unchanged. Cofactor and derivative constant gauges are also removed exactly.

Limits remain MAX_WORK=1,000,000 and 4096 expansion terms. Continuation is additionally bounded by 4096 cumulative cells and depth 12, with failed refinements spending the same work ledger. No tolerance, time limit, admission bound or existing cell bound is increased. No UI/focus operation is performed.

Regression geometry uses qA=s+s²-1 and qB=t+t³-1, with C(q)=(q-q³,q²). This regular Cartesian trace has no globally monotone XYZ coordinate. The source's interior cut s=1/2 and the target's t=1/4 have different Cartesian images. Polynomial coefficients are authored as exact dyadics using integer Bernstein conversion; no numerical root, fitting or approximate knot insertion is involved. The high-level case uses degrees 6 and 9. Original intrinsic seams and normal regularity remain mandatory.

The low-level refusal starts a reversed traversal at the SAME Cartesian closure point, exercising real-branch ownership. API regressions additionally cover one-ULP coefficient perturbation, insufficient exact work, reverse traversal, G1 without G2 after curvature perturbation, source immutability and native rejection of forged Solid/global certificates.

Scope remains explicit: a common exact component factor, strict local derivative/cofactor enclosures and the existing normal-jet class are required. Branch singularities or ambiguous components without these proofs refuse. Local G1/G2 is not a continuousBound, global embedding or full-shell certificate. This focused native qualification does not publish WASM or claim complete STEP/UI qualification.

Qualification and development history

The exact source snapshot and changed main paths are bound by `source-r5.json` and `changed-r5.json`. `qualify-r5.py` runs predicates, all surface continuity tests, world traces, sweep/BRep audit, seven native protocol suites and a no-codec build. `qualification-r5.json` records phase completion and source guards; final `result.json` is written only after successful verification. The full dirty shared main is not claimed qualified.

Initial kernel and API attempts reached the unchanged exact-work ceiling. Primitive remainder normalization, exact jet reuse, cached Bernstein subdivision, lower-degree chart priority and common world-coordinate conditioning remove repeated work. r1/r2/r3 failed API snapshots, diagnostic logs and the r4 regression are preserved. The r4 full suite exposed a previous proper quintic rational-map regression; r5 restores established inverse proof priority for open traces, keeping the new route early only for closed traces and as a fallback otherwise. No failed run is counted as final qualification.

The native affine-tilted arc and arc-clock G2 tests freshly export STEP, import it and recompute certificates. These are Rust round-trip checks; no new independent OpenCascade matrix is claimed.

Final snapshot: 3291 source files and nine changed main paths, all hash-verified. The source archive and incremental patch preserve the exact native qualification input. No fresh external STEP/UI matrix or WASM publication is included.
