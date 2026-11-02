# Original critical-quintic body — global Rust proof

This closes the previously unproved full geometric boundary certificate for
the original profile C(q)=10(q^5,q²+q/4), its closing chord, and extrusion through
z ∈ [0,2]. It does not close the complete arbitrary-frame/cap/closed-seam goal.
All production geometry and proof changes are Rust. No work, tolerance or public
source-degree ceiling was raised; there are no new ignore/skip exceptions.

Two missing original-source obligations are addressed. Independent strictly
monotone source coordinates prove complete chart injectivity even when a fixed
projection has an isolated critical point. The opposite-row support-plane owner
proves that the two wall contacts consist only of two complete, exactly bound,
shared authored edges. Its complete proof spends 85 cells from the existing
pair ledger. It refuses missing edge ownership, a side-changing pole, coplanar
overlap, nonfinite weights, zero work and one-cell-short work.

The complete certificate separately recomputes exact original lifts, valid
material trims, all-face injection, all pair classifications, full regularity,
shell nesting/roles and material orientation. Positional correspondence is not
substituted for these stages. Full-body G1/G2 stay false at sharp cap/wall joins;
reported seam continuity is C0. The Native STEP test now requires complete
certification for the original body, imported body, and their source-owned UV
transport, with error bound zero. A one-ULP UV edit still refuses.

## Rank contract audit

The new `certify_chart` route gives no rank guarantee. BOTH existing APIs
`certify_contraction` and general `certify` keep their previous strict-proof
behavior. An audit found that the fibered-volume owner relies on this stronger
legacy contract. The earlier experimental wiring through general `certify` was
removed before publication. The regression Q=(10(2u−1)^5,v,0) proves injection
through the explicit new entry, but refuses both legacy rank routes and every
fibered-volume global flag. An independent BRep case C(q)=10(q^5,q^4) has genuine
interior rank loss and refuses the full certificate despite injection and an
exact zero positional error.

The source after this audit is `source-after-rank-audit.json`. The earlier
`source-frozen.json`, `native-binary.json` and `development-*` logs are historical
pre-audit evidence. The initial full BRep run was intentionally stopped for the
verified caller-contract correction; its partial passes are NOT qualification.
The final frozen executable is identified in `native-binary-r2.json`.

## Native and independent evidence

Final-rank `nurbs-core`: 2245 passed, 0 failed, 1 pre-existing ignored measurement.
Final-rank focused BRep: all 6 passed, including the formerly failing cavity,
owned-contact and resumable-report regressions. Their tests now use actual
subdivided/full chains and explicit edge-only/vertex-only source fixtures,
independent of Boolean's coplanar face decomposition. Frozen legacy report
comparison remains; new joint traversal compares exact work reports across
uninterrupted and resumed execution, verifies cumulative ledger sums and limits,
and requires complete classification for the fully qualified unit-cube control.
The deliberately invalid translated-plane fixture makes no geometry claim.

`exact_oracle.py` independently expands Bernstein coefficients of the FRESH
native body using rational arithmetic, checks both source polynomial identities,
strict chord support coefficients and exact analytic volume 800/7. The external
OpenCascade checker independently validates one solid, four faces, six edges and
four vertices, 28 exact-parameter boundary points, 15 material classifications,
no interference pairs, and analytic volume error below the unchanged 1e−7
absolute tolerance. Closest-point diagnostics do not control acceptance.

Final-rank full BRep, STEP and no-codec phase completion are recorded separately
when their specific native processes terminate. No full-suite success or full
mathematical coverage is inferred from focused runs. WASM publication and UI/CI
qualification are not claimed here.

## Terminal final-rank STEP and no-codec phases

Fresh native STEP tests: **2 passed, 0 failed**, 0.45 seconds. All three complete
boundary audits (original, imported, transported) certify with error bound zero;
full-body G1/G2 remain false and cap/wall continuity remains C0. Native library
checks without default features pass. The independent OCCT result is bound to
the exact fresh STEP SHA-256; identical bytes reuse the same checked geometry.
It reports one solid / four faces / six edges / four vertices, volume error
1.4210854715202004e-14, zero exact-parameter point error, all 15 classifications
passed, and no interference pairs. The rational oracle is rerun on the fresh
native model. All 4411 final-rank source hashes still match.

The full default BRep run is still a separate active phase; this checkpoint
makes no claim that it has passed. Its five pre-existing ignored cases are
measurement/benchmark diagnostics, not new exclusions introduced here.
