# Original SS branches against actual retained B-rep faces

`ss_face_membership::qualify` consumes the private compact-curve owner and two immutable actual model/face references. Each face must retain exactly its corresponding original SS support definition and native parameterization, complete persistent naming and one unambiguous shell use. Original face orientation is recorded without reversing the already-oriented native UV traversal. The returned private borrowed owner cannot outlive or mutate its source definitions. Public membership records do not grant topology authority.

The qualifier freshly replays the source-owned compact curves, validates model topology/definitions, audits actual outer/hole UV loops and continuously verifies every actual world-edge/pcurve lift of both faces. It then covers every original edge's entire clock with outward original-native-UV ranges. The proven global pcurve deviation expands each range to contain both original and actually saved proxy positions. Intersecting this overestimate with the native domain uses the original owner's and saved-pcurve owner's independently proven domain facts; it does not widen a retained trim region.

Actual region classification uses interval winding and original rational boundaries. A bounding rectangle wholly in the proven closure of an authored rectangle can use that exact fact. Other unresolved clocks are subdivided with shared work accounting until both sides are Inside or Outside. Every accepted clock cell retains both side classifications, including branches excluded by a hole on only one face. Vertices, including isolated tangencies, are independently covered. Complete edge coverage is preserved by ordered dyadic partitions. A point sample, a chord or the branch's own interior area never establishes face membership.

A finite crossing or contact with a nonrectangular trim boundary can remain unresolved. No partial qualified owner is returned, and no root is fabricated. An explicit source-bound cut constructor is still required to partition such crossings. Coincident patches require face-overlap region ownership and are explicitly refused here; they cannot appear as an empty intersection simply because they have no curve edges. Empty original intersections and isolated point strata retain their own meanings.

`Limits` bounds the shared trim-audit/classification/boundary-lift/visited-clock cells, trim pairs, original range operations and subdivision depth. Original curve-owner replay, general model definition/topology validation and fixed-size domain preparation are separate input-verification costs, not debited as interval cells. Range envelopes require only fixed work per coordinate; their cost is bounded by the charged visited clocks and vertices. All public limits are validated before processing. Exhaustion or unresolved evidence fails conservatively with no tolerance growth. `reverify` recomputes membership from the immutable original curves and actual faces, then compares complete coverage records, identities, orientation and actual work counts.

## Qualification

The final focused run passed **9/9** tests. The full B-rep library run passed **1893 tests, zero failed, five explicitly ignored**, including the new module. [matrix.json](matrix.json) records actual source/log hashes, the ignored test reasons and the initial fixture failure followed by its corrected full verification. Cases cover complete closed clocks, a hole inside the loop, a branch excluded by a hole, unresolved real trim crossings, unrelated supports, mismatched actual world boundaries, insufficient shared work, exact original-range work exhaustion, report tampering, empty/point/coincident strata, original face orientation, independent native domains and support swapping. Original model snapshots remain unchanged.

Rustfmt, diff whitespace and API mapping pass. The unchanged architecture gate still reports 83 existing diagnostics; none names this new module. Reproduce from the worktree root:

```sh
CARGO_BUILD_JOBS=3 CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_TARGET_DIR=/private/tmp/osv-nurbs-plan-target-20261028 cargo test --manifest-path crates/Cargo.toml -p brep-core --lib --no-fail-fast --offline --locked
node scripts/map-nurbs-api.mjs --check
```

 The native contract is membership and exclusion only. Original-face imprinting, trim-root construction, material-side roles, global embedding, sewing and full Solid validation remain separate obligations. No WASM/editor/STEP qualification is claimed by this native extension.
