# Rebuilt quadratic-core G2 profile: native control acceptance

The full seven-stage CAD goal remains active. This actually changes the transition shape, restoring the declared central normal-radius constraint while preserving native source contacts, G2, closure, global embedding, curvature and bounded positive volume. It does not complete rolling-ball radius, runout-law, thickness or application acceptance. Earlier receipts remain unchanged.

## New exact profile, opt-in with the old baseline preserved

`build_quadratic_core_g2` uses x=2V-1 and the exact degree-24 polynomial phi=(1+x^2)/2 - x^20*(1-x^2)^2/8. At both transverse endpoints phi=1, dphi/dV is -2/+2 and d2phi/dV2=0. Those exact endpoint two-jets preserve the original source contact geometry and G2. The high-order correction is small in the SAME central V=[0.25,0.75] band that failed on the quartic; it is not a smaller post-hoc acceptance band.

Patch construction freshly checks all contact jets with the actual V degree. Exact coefficients, transverse boundary curves and rounded preview definitions now use that actual degree (24 for the new profile, 4 for the legacy baseline). The profile coefficients are immutable construction data. This is not a general automatic R-law/circular-arc constructor: the actual source/width law and actual radius still require qualification. The old `build` quartic remains the default for comparison and existing callers; the full native example selects the candidate with `--quadratic-core-g2`. No UI or WASM caller has silently switched profiles.

## Correlated complete-domain normal-radius proof

For the graph P=(u,w*x,q-h*w*phi), use physical-U derivatives and define a0=q'-h'*w, D0=1+h^2+a0^2, B=h'*w*(1-phi)+h*w'*(x*phi'/2-phi), DeltaD=B*(2*a0+B)+h^2*(phi'^2/4-1), E=1+h^2*phi'^2/4. Then the actual declared Sv normal-radius squared is

`16 * (w^2*D0/h^2) * (1+DeltaD/D0) * E^2 / phi''^2`.

The potentially correlated quotient w^2*D0/h^2 is formed in EXACT Bernstein coefficients before enclosure. Positive denominator coefficients turn the rational curve into a convex combination of coefficient ratios. All other factors use outward intervals on the explicit requested domain. The proof freshly verifies EVERY actual X/Y/Z tensor coefficient against the source graph, width law and immutable profile; a changed interior coefficient cannot inherit it. Unsupported degrees, nonpositive denominator coefficients or an inconclusive range fall back to the existing generic inspector. Exhausted work cannot become a pass.

The declared Sv direction is still not automatically a characteristic rolling-ball direction for arbitrary canal parameterizations. This is the same precisely scoped normal-radius diagnostic from the preceding receipt, not a new claim of exact circular geometry. A passing point never certifies a field. Counterexamples now evaluate exact original derivative nets directly before expensive whole-net cropping; asymmetric U/V evaluation has a regression. Full-domain acceptance still requires a complete correlated bound or complete adaptive coverage.

## Reuse immutable admission evidence, not a larger budget

The example now prints hull/contact/chart results from the immutable admission object that already computed them. Previously it ran each identical nested audit again just for intermediate logging and then recomputed admission, exhausting the shared limit. All actual admission gates still run. The single Budget is never reset and its cap remains 10,000,000. The initial duplicate-audit failure and subsequent exact-work exhaustion were not treated as successful qualification.

## Final terminal evidence

32 native regressions, two new-profile tests and 15 B-rep tests pass: 49 focused tests total. The new independent small-angle test passes R1 +/-0.001 over its COMPLETE requested central band, preserves original exact two-jets, validates degree-24 preview/error and proves curvature <=10/mm; it still rejects applying constant R1 to its entire G2-runout domain. The old quartic comparison retains its independently known 2/3-mm center radius failure. B-rep acceptance sews the new profile and admits an exact independently derived volume: integral(phi)=2683/4025, yielding loss=(2*mean(phi)-1)/18432 for the constant-width fixture. It verifies topology, exact source preservation and the new degree-24 edges. Existing split/ownership/embedding/admission refusal tests remain green.

The expanded candidate finishes with 27 faces, 82 edges, 57 vertices, zero open edges and one admitted native volume. All 351 face pairs are accounted for, with zero unresolved; all 27 support charts are injective and nineteen internal G2 joins pass. Transition curvature upper is 8.413089327286526/mm, and the full-support maximum remains 9.867331764661289/mm under the unchanged 10/mm limit.

The exact volume is in result.json, approximately 34.01287162046368 mm^3, a change of -6.323699172569349e-9 mm^3 from the quartic. It passes the explicit control-fixture [0,40] mm^3 interval derived from the original support box; this is not a user-specified volume tolerance. Source geometry and its identities are preserved.

ALL SIXTEEN middle panels (faces 2..17), corresponding to the preexisting middle width law U=[0.5,1.5], pass the declared normal R1 +/-0.001 constraint on V=[0.25,0.75] and each complete local U interval. This is complete-band proof, not midpoint-only agreement. ALL FOUR preexisting longitudinal runout panels (faces 0,1,18,19) retain certified violations of that CONSTANT R1 constraint. They are not silently omitted or relabeled as successful radius checks. Their variable-radius runout law still needs explicit acceptance. All twenty results are recorded; none remains unresolved.

Total shared charged coefficient work, including source construction, all geometric/material admission and all twenty normal-radius diagnostics, is 9,343,671 out of 10,000,000. No budget reset or tolerance increase was used.

## Reproduce

Use `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo test --offline --manifest-path crates/Cargo.toml -p nurbs-core original_graph_transition --lib` and the same flags with `--test quadratic_core_g2 -- --nocapture` instead of the lib filter. For B-rep add `--config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0`, then use `test -p brep-core --test exact_graph_strip` or `run -p brep-core --example expanded_contact_runout -- --exact-source-brep-split --quadratic-core-g2`. Omit the final flag for the original quartic comparison.

## Remaining full goal

Qualify the explicit varying-radius/end-runout law, the actual characteristic frame/center-circle relationship and wall thickness; the complete fillet is NOT admitted merely because the middle normal-radius condition passes. General rational/multi-span trimming, unequal-radius complex nodes and 5+ corners remain open. Exact source-bound WASM/worker/UI and STEP transport, independent measurements/history/reload and three-part acceptance, P0 and P2-P3 remain open. The candidate is native and opt-in. Source hashes cover selected files in the shared dirty checkout, not an isolated delivery revision.
