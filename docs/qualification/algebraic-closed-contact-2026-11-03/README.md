> Follow-up qualification, 2026-11-03: the flat quartic loop described below
> as unproved now has a positive original-source certificate. Polynomial contact
> certificates are connected to standard Boolean for checked graph-separated
> materials. See [the new evidence](../algebraic-boolean-2026-11-03/README.md).
> This document and its older refusal logs remain the historical milestone.

# Original algebraic closed branches and isolated contacts

This is a bounded native Rust extension, not qualification of arbitrary NURBS
Boolean operations or arbitrary singular algebraic sets.

`ss_intersection::construct_qualified_closed_ss_branches` proposes local original
root charts numerically, selecting a well-conditioned free coordinate locally.
Every accepted chart is an original-source uniform-root proof. The unchanged
complete mixed-product coverage, ALL-component transition census, original C1,
rank/immersion, strict interior root enclosures and whole-support injection gates
must then admit the result. Numerical convergence, proximity, sampled points and
fitted curves grant no authority. The returned sealed owner can replay its proof
and select original roots on individual charts. It does not require a common
free coordinate or an exact rational parametrization of the intersection.

`ss_intersection::qualify_isolated_graph_contact` checks an exact rational
Bernstein identity for two single-span degree 1–8 polynomial graph carriers
with identical affine XY projection. Their height gap must equal
`c0*(u-a)^even + c1*(v-b)^even`, with an interior normalized center and nonzero
same-sign coefficients. The entire original intersection is exactly one point;
all first gap derivatives vanish there. Both carriers may be nonplanar. Positive
constant weights may differ between the carriers. The coefficient identity is
checked against every original control pole using BigRational. This is a special
singular class, not a general singularity classifier.

Mixed coverage split proposals now decline chart-boundary cuts whose normalized
balance is less than one eighth of the widest cell width. Such near-wall cuts
can spend the depth budget on ulp-sized slabs. Every closed child still requires
the same independently replayed original exclusion or root-containment proof;
no evidence condition or work limit is weakened.

These APIs do not split trimmed Model faces or publish Boolean material regions.
There is no new implicit/algebraic edge representation in ordinary Model, and no
new standard Boolean dispatch fallback in this change. Open mixed-clock arcs,
nonisolated singularities, arbitrary coincident carriers, arbitrary algebraic
endpoints and arbitrary trimmed face boundaries remain separate obligations.

Qualification results and exact source hashes are recorded alongside this file.

## Qualification, 2026-11-03

70 native tests passed, zero failed: eight new tests, 61 existing integration
regressions and one mixed-coverage unit test (including corrupt-receipt replay
and a near-wall subdivision regression). The eight new tests also passed in the
current shared checkout after rebuilding the predicate dependency. These eight
replays are not eight additional distinct tests. API-map checking and scoped
Rust formatting/diff whitespace checks passed. WASM/dist, browser, STEP and the
full workspace suite were not built or qualified.

The smooth nonconic quartic loop needs 32 proposal stations and coverage depth
32 in this fixture. Its original supports are both nonplanar. An isolated
fourth-order contact is checked exactly; wrong sign/center/power, altered XY
projection, a single perturbed height pole, a shifted height gap, incomplete charts
and exhausted/invalid work controls are negative tests.

The flatter superellipse `(u-.5)^4+(v-.5)^4=1/256` is a KNOWN UNPROVED valid
closed loop: even 32 stations, eight repair attempts, 64 charts and depth 32 did
not establish full mixed coverage. `development-flat-loop-refusal.log` records
that failed qualification attempt. The final negative test asserts refusal
when complete coverage has not been established; it is not a positive geometry
qualification. General singularities/coincidence, arbitrary open mixed-clock
branches and propagation through original face trims remain unfinished.

Numerical proposal work is shared across the initial grid and repairs. Coverage
allows up to `1 + max_repairs` complete attempts, each with the explicitly
supplied coverage allowance; `max_repairs <= 8`. Component/embedding proofs
retain their own native bounds. No unlimited retry or missed-root acceptance
is introduced. Default budgets are not a promise that all admitted geometric
families can be constructed automatically.

The frozen native run uses `/private/tmp/osv-closed-source-20261103`: the previous
immutable qualification baseline plus exactly the six task files recorded in
`task-source.sha256`. `frozen-source.sha256` records all frozen input files.
Concurrent unrelated work is excluded from this frozen claim; the live replay
checks integration separately. The test profile retains debug assertions,
uses opt-level 0 for nurbs-core and opt-level 1 for dependencies. This is not
performance evidence. Logs and machine-readable counts are adjacent.

```
CARGO_TARGET_DIR=/private/tmp/osv-nurbs-next-target-20261102 \
CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 \
cargo test --locked \
  --manifest-path /private/tmp/osv-closed-source-20261103/crates/Cargo.toml \
  --config 'profile.test.package.nurbs-core.opt-level=0' \
  -p nurbs-core --test ss_closed_automatic
```

Existing regression targets: `ss_closed_fiber_branches`, `ss_closed_degenerate`,
`ss_mixed_fiber_coverage`, `ss_automatic_fiber_transition`, `ss_fiber_coverage`,
`ss_discover_atlas`, `ss_mixed_quadratic_graph`; unit filter
`fiber_coverage::mixed::tests`. Live replay adds
`--config 'profile.test.package.cad-predicates.opt-level=0'` to force a fresh
predicate build rather than reuse artifacts from the other source snapshot.
