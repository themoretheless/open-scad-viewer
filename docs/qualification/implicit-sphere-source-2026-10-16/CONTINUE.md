# Original implicit sphere checkpoint

Frozen Rust code: `6476dc11c7091e42577541d46fe2dfe8f356177d`.
Native input guard: 3781 inputs,
`cee4257be5c7064cfc84dba9c7233038618d21faba9c8d05f6d47a711b9b2aa3`.

The full predecessor run terminated with 1114 BRep successes (3 ignored),
1600 NURBS successes and a single stale JSON test expectation failure. Its
source guard is honestly false because the exact reviewed test-only correction
was applied after the bridge gate had terminated while the dependency-disjoint
BRep binary was finishing. This predecessor is NOT a passed checkpoint.
The successor in `../implicit-sphere-test-contract-2026-10-16/` verifies that
literal test-only diff, every remaining source byte and the disjoint package
manifests, reruns the full bridge and JSON/STEP gates, and preserves original
successful dependency-disjoint receipts. The original failed log is retained.
The earlier attempt was stopped after detecting a stale fixed-work assertion
(968 versus improved 136). Its evidence is retained at
`/private/tmp/osv-implicit-sphere-failed-qualification-20261016`; the production
proof was unchanged by the subsequent test-only correction.

Completed independent focused evidence: four typed body tests (including all
seven signed tensor gauges for regular and pole spheres), two JSON tests, four
STEP tests, 106 `cad-step` tests, four independent OCCT cases. The external
checker compares the same original source; it uses converged span-aware
Gauss-Kronrod volume integration and verifies ownership of every OCCT point
edge, rather than expecting OCCT to retain native point-edge IDs.

Completed private release WASM ABI evidence: four sphere cases, six prior
rational-frame/annular-cap cases, each with native construction/audit/STEP
re-audit/refusal, plus preservation of a genuine sharp C0 case. The final-source
build is byte-identical to the tested binary: 18,073,723 bytes,
`bf079682200cccd37f3cd9b0ea77147e6d25e1b07f3da8eef5bf78139e822a6a`.
The private binary is preserved at
`/private/tmp/osv-implicit-sphere-artifact-20261016/geometry_wasm.wasm`.
The preceding qualified cap binary is separately preserved, not overwritten.

The public package/fingerprints, browser/Rush/viewport/Solid UI, full semantic
G1 matrix, fresh main integration and CI are separate unfinished gates.
This does not complete the goal for arbitrary frames, caps or seams.
Publication remains subject to the prior automatic approval rejection and needs
fresh human authorization for the exact destination and source/evidence payload.
Local branches are already preserved in the primary repository.

## Next native source obligation

A useful next sufficient theorem is an open source with ordered transverse
fibers, including smooth flat end caps whose volume parameterization is singular
at its ends. For a literal source

`F(q,t) = (P_t(q), h(t))`,

strict monotonicity of continuous `h` and whole-domain injectivity of every
`P_t` imply global injectivity; compactness gives the original material
homeomorphism. Strict interior `h'` supplies interior regularity. This must use
a separate stratified report: it cannot set the existing strict whole-cube
Jacobian regularity flag when `h'(0)` or `h'(1)` vanishes. Exact independence of
height from material parameters and all source-law correlations require native
proof. Neither retained-body validity nor sampled height/section ordering is a
substitute.

Boundary regularity and actual/original Gk still need independent complete
premises. For example, exact quintic height controls `[0,0,0,1,1,1]` and radial
scale controls `[1,1.25,1.5,1.5,1.25,1]` give an instructive disk case: the
height has vanishing first/second derivatives at the ends and the wall has a
nonzero radial first derivative. A hollow region is **not** automatically a
smooth positive case: uniform radial expansion can put an inner wall on the
same side as the planar filled cap, creating a cusp with opposite oriented
normals. Hole topology, cap interiors, orientation and the full vertex links
must remain owned; strict G1/G2 must refuse this counterexample.

General spatial frames, arbitrary cap atlases, mixed C0/G1/G2 ownership and
non-spherical pole fans still require their own original-law/boundary proof and
qualification. Keep the verified sphere and rational-cap checkpoints frozen
when developing these successors.
