# Exact same-carrier seam normalization / Boolean — 2026-11-02

Status: native source-exact removal of artificial original seams, integrated
with standard Boolean. This is NOT a general NURBS/NURBS intersection solver.
Seams between genuinely different carriers still require the unresolved full
intersection/trim/material arrangement proof.

## Implemented

Public Rust APIs:

- `minkowski3d::curved::certify_same_carrier_merge` and
  `verify_same_carrier_merge` return/replay a `SameCarrierMergeCertificate`.
- `operations::certify_seam_normalized_boolean` and
  `verify_seam_normalized_boolean` return/replay a
  `SeamNormalizedBooleanCertificate` with BOTH original source bindings, every
  immediate source/result merge receipt, operation and final Model binding.

Standard Boolean retains its existing successful exact-family precedence.
On a curved-family proof refusal, it can retry on certified equivalent inputs
with artificial same-carrier seams removed. The internal retry flag prevents
recursive normalization. The normalized operands STILL require an existing
admitted Boolean path; neither seam removal nor a numerical SS trace grants
new intersection or material authority. Hard input/resource errors propagate.

Normalization inspects both operands, allowing up to eight merge steps each
and at most 64 original faces per automatic operand. Direct merge construction
allows 2..64 selected faces, at most 256 source faces and atlas depth 0..3.
Different carriers or merely nearby seams are never welded.

## Source-equivalence proof

1. The entire original edge/carrier boundary agreement is checked, including
   full forward authored use intervals. Source atlas and individual original
   UV profile certificates prove the original embedded material-left regions.
2. Selected faces must use the IDENTICAL full NURBS carrier definition and the
   same oriented closed shell owner. Original edge topology supplies incidence.
3. Each internal seam is canceled only if its two uses have opposite edge
   traversal, identical reversed rational UV controls/weights, and a reversed
   knot clock checked with exact rational arithmetic. A rounded knot reflection
   or matching endpoints alone is insufficient. Periodic/pole seams refuse.
4. Remaining original coedges form closed, nonbranching cycles. Their complete
   UV profile is independently certified: one outer boundary with disjoint
   holes. The replacement must be a connected region and use exactly the full
   original exposed boundary.
5. Oriented boundary-chain cancellation implies equality of winding at EVERY
   point off the boundary: new winding = sum of original region windings.
   Source embedding excludes overlapping original material sheets. Thus the
   filled UV union and its image under the unchanged carrier are identical.
   Internal seams are removed without changing material or approximating curves.
6. Existing region replacement preserves actual exposed edge curves, runs
   whole-model trim/continuous closure admission, and records merge lineage.
   A fresh result atlas additionally proves the output embedding. The ordinary
   Model remains editable and serializable.

Verifier reconstruction compares full Models and all certificate fields.
Forged source/region labels, missing seams, changed orientations, omitted proof
steps or modified output geometry cannot inherit the certificate.

## Trim-aware retained-carrier exclusion

A prior conservative bound used the entire carrier of every other source face.
After the first pocket, its floor carrier could span the WHOLE original XY
rectangle, even though its actual trimmed region was only the small floor.
This incorrectly prevented an equal-depth subsequent cut far from that floor.

When whole-carrier exclusion refuses, the new optional bound proves an exact
affine XY projection on the original carrier. A positive-weight clamped
closed outer UV trim supplies a conservative control hull for the WHOLE filled
face's XY projection: extrema of an affine functional on a bounded filled UV
region occur on its boundary; holes only remove material. Exact rational hull
separation can then exclude the actual face. It does not discard an interior
bulge of a non-graph surface: non-affine projections still refuse this bound.
The enclosing operation also requires full source-atlas admission. Touching
bounds do not separate. All new exact arithmetic is charged to the existing
work ledger; budget refusal remains refusal.

This permits the tested equal-depth second pocket after removing a CLOSED
original seam while preserving the first pocket. The extra-material-layer
negative regression remains a refusal.

## Restrictions / unfinished request

- Nonidentical NURBS carriers, affine/nonlinear reparameterization recovery,
  genuine creases and arbitrary spatial seams are not normalized here.
- The downstream Boolean's original restrictions remain in force, including
  graph/box footprint guards and refusal of unproved NURBS intersections.
- The tested touching-component orthogonal XOR is still refused by the existing
  author. Seam normalization preserves that refusal instead of promoting it.
- General complete SS branch arrangements, split regions for independent
  intersecting NURBS carriers, their material labels and ordinary-Model authorship
  are NOT completed by this change.
- Qualification is native only. No WASM/dist/browser/STEP or full-suite claim.

## Validation

```sh
CARGO_TARGET_DIR=/private/tmp/osv-seam-boolean-20261102 \
CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 \
cargo test --manifest-path crates/Cargo.toml \
  --config 'profile.test.package.brep-core.opt-level=0' \
  -p brep-core --test seam_boolean --test graph_box_boolean \
  --test regularized_boolean --test curved_boolean_audit --no-run
```

Tests cover an exact quartic world seam on a biquadratic source, a curved
rational UV seam on a planar source, a closed patch seam with a previous pocket,
a cut through both original source sectors, codec roundtrip, repeated editing,
wrong carrier/orientation/region/output receipts and retained solver refusals.
Final logs, native executable hashes and scoped source checksums accompany this
report. The checkout contains concurrent unrelated work; this is not a frozen
full-workspace qualification.

## Final native results

115 tests passed; zero failures.

- `operations.log`: 25 passed, 0 failed.
- `nurbs-ss.log`: 46 passed, 0 failed.
- `boundary.log`: 14 passed, 0 failed.
- `nonintersecting.log`: 6 passed, 0 failed.
- `pockets.log`: 5 passed, 0 failed.
- `seam-boolean.log`: 5 passed, 0 failed.
- `graph-box.log`: 5 passed, 0 failed.
- `regularized.log`: 5 passed, 0 failed.
- `curved-audit.log`: 4 passed, 0 failed.

The five unit groups use the copied executable in `unit-binary-sha256.txt`.
Filters: `operations::`, `nurbs_ss_g6::`, `boundary_agreement::`,
`minkowski3d::curved::atlas::boolean::`, and
`minkowski3d::curved::atlas::region_brep::model_pocket::ordinary::tests::`.
The pocket group excludes the existing long-running
`circular_pocket_can_be_the_source_of_a_second_face_edit`; no requalification
of that case is claimed. All runs use `--nocapture --test-threads=2`.

The four integration executables are identified in
`integration-binaries-sha256.txt`. Core optimization is 0, dependencies 1;
debug assertions are enabled. This is behavior validation, not a release
performance benchmark. API map, scoped rustfmt and diff whitespace checks passed.

Diagnostic logs contain FIXED fixture endpoint/type errors and expected
pre-change whole-carrier/XOR refusals. They are not counted as passing tests.
Final source and executable checksums are checked after qualification.

`scoped-source.tar.gz` retains the eleven source/test files in the scoped
manifest; it is not a complete Rust workspace or dependency snapshot.
