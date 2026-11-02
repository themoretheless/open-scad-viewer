# Source-referenced end-runout law and sphere-band checks

The full seven-stage CAD goal remains active. This proves a distinct source-input runout law on the same declared transverse core band for ALL twenty native panels, including the four longitudinal ends. It does not replace the constant-R1 test, enlarge its tolerance, qualify the transverse outer runout strips or complete fillet/application admission. Earlier receipts remain unchanged.

## Reference comes from original inputs

`SourceContactCurvatureLaw::new` snapshots the original source support and original width curve BEFORE building the output profile. It reads no output surface coefficients or transverse shape. Binding requires exact equality of source/width snapshots with the native Patch. Native volume inspection requires exactly one matching reference per transition face, without duplicates, omissions or cross-assigned clocks. The descriptor alone is not a geometry certificate; it applies only when bound to the qualified native graph family.

For the original graph z=q(U)-h(U)*V, contact half-width w(U), a0=q'-h'*w and D0=1+h^2+a0^2, the reference inverse-radius magnitude is |h|/(w*sqrt(D0)). Its finite-h source-normal intersection sphere has center

`C=(U+a0*w/h, 0, q-h*w-w/h)` and `R=w*sqrt(D0)/|h|`.

At the flat G2 ends h=0 the reference is a plane limit with inverse radius ZERO, not a fabricated finite R1 sphere. The expressions used in the proof eliminate division by h, so the complete endpoint-containing domain can be checked. The same fixed reference rejects the old quartic and accepts the rebuilt profile; it is not inferred by fitting the result.

## Three separately dimensioned requirements

The native inspector compares the actual declared-Sv normal curvature to the source inverse-radius law over the COMPLETE explicit normalized domain. It uses the exact correlated graph factors from the preceding radius proof; a positive-denominator Bernstein ratio bounds h^2/(w^2*D0), including zero numerator at flat ends. The relative actual/reference curvature multiplier is |phi''|/(4*E*sqrt(D/D0)). A whole-domain error bound, not a passing sample, must meet the supplied inverse-mm tolerance.

Radial position is checked separately. With x=2V-1, the squared sphere residual simplifies exactly to `w^2*(x^2+1-2*phi+h^2*(1-phi)^2)`. Dividing by a lower bound on distance+R gives the conservative absolute radial-distance error bound `w*|h|*|x^2+1-2*phi+h^2*(1-phi)^2|`. It extends continuously to zero at the flat limit. Position alone is insufficient: the old quartic can pass this loose radial-distance criterion and still fails the inverse-curvature law.

Source-consistent radial-normal alignment is also checked. The homogeneous radial vector is `(-a0,h*x,Z)` with `Z=1+h^2*(1-phi)`, and the actual upward graph normal is proportional to `(-a0-B,h*phi'/2,1)`. After dividing the first vector by positive Z, both have final coordinate one. Twice the Euclidean difference of their first two components bounds the chord distance of the UNIT normal vectors. This is a dimensionless chord tolerance, not radians or the source G2 tangent tolerance. Products h'*w and h*w', and the polynomial x-phi'/2, are combined exactly before enclosure to avoid false uncertainty from independent interval bounds. Incomplete, stale or numerically inconclusive work remains unqualified.

## Terminal tests

32 native regressions, two profile tests, three new reference-law tests and fifteen B-rep tests pass: 52 focused tests. The paired old/new test uses one unchanged source reference, accepts the rebuilt profile, rejects the old quartic, and demonstrates that radial position alone would miss the wrong curvature. A source h=U^4/1024 test includes the zero-curvature endpoint and remains finite. Zero radial tolerance and zero normal tolerance are not silently passed as exact-circle claims. Zero work stays unresolved; a changed source-width reference refuses. The B-rep test creates references before shape assembly, checks complete source-law coverage, rejects missing/cross-assigned face references and preserves incomplete-work status, while retaining independent exact volume and all native geometry checks.

## Expanded control result and explicit scope

All twenty panels pass the source-reference law on local U=[0,1], V=[0.25,0.75]. Control tolerances are 0.001/mm inverse-curvature error, 0.001 mm radial-distance error and 0.001 unit-normal chord error. The respective maximum certified upper bounds are 8.952134138240365e-5/mm, 9.605502950197557e-6 mm and 1.048307484703911e-4. Every per-face bound is in source-law-reports.json; none is unresolved.

The SEPARATE constant R1 +/-0.001 mm test still passes the sixteen middle panels and still rejects faces 0,1,18,19. That result is preserved in the final runtime log. The end law is explicitly VARIABLE and derived from the original support/width inputs; it is not the same requirement as constant R1 on all panels. These are configured control specifications, not newly supplied user manufacturing tolerances.

The same admitted native body remains 27 faces, 82 edges, 57 vertices, one body and zero open edges. All 351 face pairs remain accounted for with no unresolved geometry; nineteen internal G2 joins and whole-support curvature upper 9.867331764661289/mm remain below the unchanged 10/mm cap. The geometry/volume was not changed in this step. Total shared charged coefficient work is 9,620,257 out of 10,000,000, with no reset. The immutable volume's already-owned geometry proof is reused safely for the additional source-law query; no rounded preview inherits admission.

## Reproduce

Use `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo test --offline --manifest-path crates/Cargo.toml -p nurbs-core --test source_contact_curvature_law -- --nocapture`, plus the existing `original_graph_transition --lib` and `--test quadratic_core_g2` targets. For B-rep add `--config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0` and run `test -p brep-core --test exact_graph_strip` or `run -p brep-core --example expanded_contact_runout -- --exact-source-brep-split --quadratic-core-g2`.

## Remaining full goal

The proved band excludes the TRANSVERSE V runout strips [0,0.25] and [0.75,1]; no source-law radius acceptance is claimed there. Those need an explicitly designed transverse law, not silent cropping of a full-surface requirement. Full rolling-ball/characteristic and circular-section qualification beyond this toleranced source sphere band, actual wall thickness and part-specific acceptance remain open. General rational/multi-span edges, unequal-radius complex nodes and 5+ corners remain open. The new shape is still a native opt-in prototype; WASM/worker/UI, STEP/independent measurements/history/reload, three-part acceptance, P0 and P2-P3 remain open. Hashes describe selected files in the shared dirty checkout, not an isolated delivery revision.
