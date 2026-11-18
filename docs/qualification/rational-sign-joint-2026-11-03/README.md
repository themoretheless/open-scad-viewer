# Original rational derivative signs beyond Mobius clocks

Qualification completed on the frozen 4668-file source snapshot. Native Rust only; no WASM/UI publication in this run.

## New sufficient proof

For a validated, clamped, nonperiodic rational Bezier coordinate `N/W`, all
original weights must be positive. This proves `W>0` on the full interval.
The new Rust owner builds the exact Bernstein coefficients of `N'W-NW'`
from the original binary64 poles and weights using bounded rational arithmetic.
Exact dyadic subdivision proves a common weak sign and a nonzero polynomial
on every leaf. Thus the coordinate is strictly monotone, including isolated
critical points at leaf joins. There is no fitted map, sampled sign, numerical
root approximation or replacement of the original coordinate at common time.

For tensor surfaces, exact identities `w_ij*w_00=w_i0*w_0j` prove independent
positive denominator factors. Literal pole-coordinate independence and the
two scalar proofs then prove original whole-chart injectivity. The remaining
coordinate is unrestricted. Arbitrary separable weights are permitted; they
need not form geometric sequences. Rank, Jacobian metrics, curvature, trims,
global contacts and shell orientation remain separate proof obligations.

The previous Mobius path retains its order and successful work reports.
The new fallback spends the caller's remaining ledger. Degree 32, dyadic
subdivision depth 16, rational bit bound 8192 and all qualification budgets
are unchanged. Exhaustion or unsupported signs refuse certification.

## Joint geometry

The new degree-five profile has weights `[1,2,2,2,2,1]` and original
homogeneous numerators `10q^5`, `10(q^2+q/4)`, `q=2u-1`, with
`W(q)=(31-10q^2-5q^4)/16`. The X derivative vanishes at `u=1/2`, while
the Y derivative is exactly `80/31`. The denominator is genuinely quartic
and the weights are not geometric. The old Mobius route refuses this fixture.

Three profile spans at z=0,1,2 form an eight-face body with three actual
internal station seams. A second case replaces each linear station map with
`z(v)=(2v-v^2)/(1+2v-2v^2)`. This is also outside Mobius, with positive
derivative `2(1-v+v^2)/(1+2v-2v^2)^2`. At a station join its first
derivatives match, but second derivatives are +10 and -10: literal C2 fails,
so geometric G2 must be independently proven. Caps/profile corners retain C0.

The joint test requires the whole retained-boundary continuous bound, global
regularity/injectivity/trim/contact/nesting/orientation and actual station
G1/G2, then fresh STEP import and transport. Low-budget, folded-coordinate,
nonseparable-weight, missing-topology and genuine rank-loss controls refuse.

## Independent oracle

An exact rational oracle binds every original carrier, edge and cap trim to
the exported body hash. Its independent Green volume integral has a rational
enclosure from the expansion of `(1-a)^-3`, where
`a=(10q^2+5q^4)/31 <= 15/31`. A fixed 129-term sum plus a closed exact tail
bound encloses the volume to width below `1e-30`. No external Python numerical
dependency or sampled quadrature supplies this bound. OpenCascade acceptance
uses the former actual `1e-7` tolerance against this independently enclosed volume.
Both Gauss and Gauss-Kronrod error estimates remain diagnostics: the exact
source oracle, rather than a kernel estimate, controls the volume comparison.
Requested integration-Eps misses are reported explicitly, not called converged.

## Scope

This is a sufficient exact sign-cover class, not completeness for arbitrary
rational/algebraic maps. Nonseparable weights, unresolved derivative signs,
critical roots not resolved within the existing cover, general periodic maps
and universal combinations of source laws, frames and caps remain open.
Neither chart injection nor local station G2 certifies a whole smooth shell
or the ideal motion law. Parallel unrelated main changes are excluded from
the frozen native qualification.

## Final results

| Gate | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| nurbs-full | 2273 | 0 | 1 |
| brep-full | 2322 | 0 | 5 |
| algebraic-maps | 41 | 0 | 0 |
| native-matrix | 47 | 0 | 0 |

Counts are executions per gate. Existing ignored measurement cases remain named in `audit.json`; no new exclusions were introduced. The focused five new joint tests are also included in the full BRep gate.

Both native bodies have fresh original, imported and transported certificates: six complete continuous-bound/global-volume certificates and six low-global-budget refusals. Every positive boundary independently proves regularity, injection, trim, contact, nesting and orientation. Each body has three actual station G1/G2 seams before and after STEP. The two-clock case has a real transverse C2 jump (+10/-10) while exact geometric G2 holds. Full-shell sharp smoothness remains false. Source chart work is {'body': 674, 'body-both': 990} under the unchanged shared 1000-unit limit.

Both exact rational volume enclosures have width below 1e-30 around 141.91438726444202655. Independent OCCT accepts two valid 1-solid/8-face/15-edge/9-vertex STEP bodies, with zero interference pairs and 56 exact-parameter boundary checks each. The adaptive Gauss-Kronrod kernel owner uses the former Eps=1e-10 and acceptance tolerance 1e-7. The standard Gauss owner underestimates its error on the new rational trim; its disagreement is preserved in diagnostic fields, together with the initial failed external run. The two-clock GK run does not meet its requested internal Eps estimate; this remains explicitly reported and is not called convergence. The actual mass comparison uses the exact source-volume enclosure and the unchanged absolute 1e-7 tolerance. No acceptance tolerance was raised. Closest-point diagnostics do not control acceptance.

The initial naive derivative-product proof exhausted the shared face ledger, although local maps were proved. Its refusal is preserved in `development-body/`. Antisymmetric coefficient pairs and exact positive-factor cancellation close the budget instead of raising it. The intermediate two-clock budget refusal is preserved in `development-both/`. Source weights/caps were not simplified to obtain these final certificates.

The orchestration checkpoint after NURBS was refreshed when independent Python diagnostics were updated. Its hash guard stopped before BRep, then resumed using the same verified Rust snapshot and retained NURBS log; no Rust sources, input bodies or proof budgets changed.

Frozen source, selected owned main files, probe inputs and every gate log were hash-verified after qualification. `audited-source.tar.gz` includes the baseline literal compile-time inputs plus this overlay. `source.json`, `owned.json`, `inputs.json`, `audit.json` and `evidence.tar.gz` record reproducibility. Concurrent unrelated main edits are outside this source qualification.
