# Sweep Rust follow-up — 2026-11-02

Qualification completed for the frozen Rust snapshot. No all-mode or full mathematical coverage claim.

## Changes under qualification

- Closed hollow: use the existing exact control-hull separation before expensive source-chart attempts. Hull work debits the same pair ledger. Six focused cases pass, including the four prior failures and two controls.
- Corrected quarter-turn cap: one smooth rational source chart may use a fixed left transform and strict interval positivity of all seven principal Jacobian minors over a complete cover of the original rectangle. Rounded midpoint inverses are proposals only. Original motion coefficients are combined outward before evaluation. Failed work stays in the existing ledger, and the old proof remains the fallback.
- Quintic cap contours: strict coordinate monotonicity from exact binary64 de Casteljau subdivisions. Equal-weight polynomial Bezier charts only. Every arithmetic step consumes the existing shared cell ledger; rounded subdivisions refuse. This proves contour injectivity, not surface immersion or solid validity.
- STEP atlas: exact U reversal of a source proof view, with a unique exact vertex/edge bijection and checked face-owned wire renumbering; face, shell and body order remains fixed. Surface poles, knot vectors, pcurve traversal and UV coordinates must be transformed exactly. Whole-surface distance, original and retained regularity, and both volume gates remain independent. No model mutation and no equality from chart proposals.

## Mathematical source

The smooth rectangular P-matrix route uses Gale and Nikaido, “The Jacobian matrix and global univalence of mappings”, Mathematische Annalen 159 (1965), Theorem 4: https://doi.org/10.1007/BF01360282 . The primary paper is retained as `gale-nikaido-1965.pdf`. All principal minors are required; positive determinant or finitely sampled Jacobians are insufficient. The route excludes periodic quotients and piecewise/C0 source charts.

## Boundaries

Rust owns all new geometry and proofs. Work, tolerance and degree ceilings were not raised. Correct refusals remain required. No browser/UI focus, WASM delivery, git operations or shared-worktree cleanup is part of this qualification. Complete coverage of arbitrary algebraic maps and all joint continuousBound/global/G1/G2 combinations remains open.

The final source ledger and archive are `source-final.json` and `audited-source.tar.gz`. Failed development attempts remain alongside subsequent results and must not be counted as passing qualification.

## Final measured results

- 542 tests passed; 0 failed across the 11 audit phases; no-codec check passed. The five original failing cases are included unchanged in the full sweep gate.
- New fixed-metric complete-cover P-matrix tests include exact-budget replay, one-cell-short refusal and an interior fold; all seven principal minors are checked.
- Both quintic cap contours are proven simple in original and STEP-imported UV charts. Original/retained boundary correspondence is proven with error upper bound 0 and exact U transport of face 2. Regularity is independently proven.
- Fresh independent OpenCascade: valid 1-solid/4-face/6-edge/4-vertex STEP; absolute volume error 1.4210854715202004e-14; 28 exact-parameter point checks, 15 material classifications, no interference pairs at level 5. Absolute tolerance remains 1e-7. The closest-point diagnostic does not control acceptance.
- The quintic strict global certificate still refuses: all-face injectivity, all pair classifications, material nesting and shell orientation are not yet jointly established. Simple contours and valid OCCT geometry do not supply those proofs. Whole sharp-body G1/G2 remains false; the intended joins are C0.
- Complete mathematical coverage of general algebraic maps and all joint continuousBound/global/applicable-G1/G2 combinations remains open. The new volume theorem admits one smooth nonperiodic rational chart, not arbitrary periodic or C0 atlases.
- Frozen-source guard and owned-main hashes passed; unrelated shared work was preserved. WASM was not republished; UI was not exercised and no focus was requested.

`audit.json` is the phase-level source of truth. Failed development attempts are preserved but excluded from these final counts. `evidence.tar.gz` retains the current qualification artifacts.
