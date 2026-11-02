# Checked retained NURBS regions and ordinary-Model Boolean — 2026-11-02

Status: the complete original transverse graph now feeds a checked TWO-dimensional
face partition, original-solid material classification and ordinary `Model`
author. Standard `operations::boolean` tries this path after its existing exact
families and nonintersecting proof. This is an admitted, bounded exact path;
it is NOT universal NURBS/NURBS Boolean or a checked approximate author.

## Public Rust API

`minkowski3d::curved::{author_retained_nurbs_boolean,
certify_retained_nurbs_boolean, verify_retained_nurbs_boolean}`.
The automatic author constructs proposals; the explicit certifier also accepts
complete `NurbsFacePartition` proposals for exact representable arrangements.
Both return a private `CertifiedRetainedNurbsBoolean` with immutable model and
receipt accessors and `into_model()`. The verifier rebuilds from ORIGINAL inputs,
partitions and limits, then compares the full receipt and result Model.

## Admission proof

1. The fresh complete original graph binds both original Models, all face pairs,
   root ranks, actual trims, original seams and every active physical arc.
   No numerical trace or caller label substitutes for that coverage proof.
2. Every proposed descendant uses the LITERAL unchanged original surface and
   orientation. Every saved 3D curve and pcurve has a fresh EXACT coefficient
   identity on that surface. Literal world/UV endpoints and positive natural-
   domain pcurve hulls prevent tolerance closure, snapping or extrapolation.
3. Every original rim is partitioned into exactly covering, strictly ordered
   original parameter intervals. Exact composition through a constant-V strip
   checks restrictions against the ORIGINAL B-spline coefficients, including
   reversed face-local clocks; rounded knot insertion is only a proposal.
4. Every sector is an independently certified positive connected UV region
   with material-left holes. Each internal cut has two opposite same-carrier
   uses of the exact same full pcurve. These oriented chains cancel EXACTLY.
   Their sum equals the original rim; hence the sum of their 0/1 windings is
   the original 0/1 indicator at ALL off-boundary points. This proves full 2D
   coverage and absence of overlapping sectors, not just seed coverage.
5. EACH physical graph arc has exactly one saved cut edge per operand. Both
   edges are the same exact 3D curve; both source lifts are exact. Endpoints
   carry the original algebraic root keys, and the native free clock has a
   strictly monotone polynomial Bernstein proof on its owning operand. Global
   source injectivity and complete proper physical branches identify the whole
   exact arc. All graph arcs are accounted for, so no region interior can cross
   an omitted piece of the other ORIGINAL boundary.
6. For one strict seed in EACH connected UV region, an exact rational source
   evaluation is separated from EVERY patch over the ENTIRE checked NURBS-to-PL
   source homotopy. Exact oriented PL winding therefore equals ORIGINAL-solid
   membership. An unseparated point is inconclusive, never Outside. Complete
   graph coverage makes this membership constant throughout that open region.
   The operation truth table chooses keep/reverse; supplied booleans are unused.
7. Retained faces preserve original surfaces and exact restrictions. Edges and
   vertices merge by original source interval IDs and original graph-root keys,
   never nearest coordinates. Literal saved points must coincide; even a tiny
   rounding mismatch refuses. Every selected edge must have two opposite uses.
   Connected closed shells become ordinary bodies, each proved outward by its
   own source atlas and exact nominal signed volume. A fresh global result atlas
   checks embedding of ALL components. Parent/child lineage records face splits
   and generated boundaries; no mesh-only or sidecar geometry is returned.

The material oracle permits a single connected outward original shell per input;
genus is unrestricted, but nested cavity shells/multiple original bodies are not
admitted by this new path. Existing nonintersecting/cavity paths remain available.

## Automatic construction and limits

- Proposes polynomial 3D curves and both pcurves through degree 4, using bounded
  dyadic endpoint/root proposals and native original-fiber stations. Interpolation
  residuals and root-box proximity grant NO authority: fresh exact source lifts
  and the complete arrangement checker decide admission.
- Splits original shared edges once per ordered original root, updating every
  incident coedge. Per-face chains join graph arcs across OTHER operand seams;
  chains can be open through original rim vertices or closed inside a face.
  All sectors survive until fresh material classification. Closed proposals may
  be reversed, but the complete positive trim proof is always rerun.
- Explicit partitions can represent a broader exact class than the automatic
  polynomial proposer; free-clock proof currently requires one polynomial
  Bernstein span with strictly signed successive free-coordinate differences.
- Automatic author: at most 128 arcs, degree 4 proposals, 40-bit dyadic endpoint
  search. Partition admission: at most 256 faces/1024 edges per operand. Existing
  graph/source/trim stages retain their own bounded native budgets. These are
  stage limits, not a global user-configurable CPU allowance.
- Tangencies, coincidence, singular strata, graph cases lacking a proper common
  clock, closed components entirely within one carrier pair, exact nonrepresentable
  endpoints, general rational/nonpolynomial traces, and cuts ending at hole rims
  may refuse. No coefficient rounding or checked approximation path is added.
- Independently closed negative cavity components require separate containment/
  body-role proof and refuse here. Four-use XOR intersection edges refuse rather
  than being exported as a nonmanifold ordinary solid.

## Independent-carrier regression

A is a unit box deformed by `y += x²/16`, with four extended source carriers and
unchanged actual trims. B spans `x,y = [-1/2,3/2]`, `z = [1/4,3/4]`, independently
deformed by `z += x²/16`. There is no common ambient deformation assumption.
The source graph has two closed four-arc cycles through four original A seams.
It partitions 22 complete source-face regions. The test independently checks the
intersection's eight saved vertices against the analytic equations.

Observed development results (final qualification follows below):

| Operation | Bodies | Faces |
| --- | ---: | ---: |
| Intersection | 1 | 6 |
| A minus B | 2 | 12 |
| Union | 1 | 16 |
| B minus A, through-hole | 1 | 10 |

Full certificate reconstruction and standard dispatcher pass. A sub-tolerance
middle-pole pcurve edit with unchanged endpoints refuses. The earlier fixture
with irrational intersection endpoints and a one-rectangle budget both refuse
without obtaining Model authority.

## Qualification scope

The shared checkout is concurrently edited by other work. Final native checks
use `/private/tmp/osv-retained-source-20261102`, copied from the previously
qualified graph snapshot with ONLY this task's narrow integration changes.
The source archive and SHA-256 manifest retain that complete Rust workspace,
lockfile, toolchain file and literal compile-time fixtures. Dependencies and
compiler downloads are not vendored. Copied test executables have separate hashes.
No full-suite, release performance, WASM/dist, browser or STEP claim is made.

```sh
CARGO_TARGET_DIR=/private/tmp/osv-trim-graph-frozen-target-20261102 \
CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 \
cargo test --locked \
  --manifest-path /private/tmp/osv-retained-source-20261102/crates/Cargo.toml \
  --config 'profile.test.package.brep-core.opt-level=0' \
  -p brep-core --lib --test retained_nurbs_boolean --test nurbs_intersection_graph \
  --test face_crossings --test seam_boolean --test graph_box_boolean --no-run
```

The final source also passes the standard-dispatch integration test in the
CURRENT shared checkout (`live-standard.log`, 1 test, 0 failures). Frozen-source
qualification remains the reproducible record; that live run does not claim
the concurrently changing checkout matches every frozen dependency.

`development-*-diagnostic.log` files retain the initial closed-path orientation
refusal, a corrected Rust comparison typo and the final-pass discovery of
kind-invalid lineage parents. Generated edge lineage now refers to the original
ROOT-OWNING EDGE IDs (not face IDs), and face lineage to original face IDs.
These diagnostic logs are intentionally retained, not passing evidence.

## Final qualification results

117 frozen-source native tests passed; 0 failed. In addition, 1 current-
checkout standard-dispatch integration test passed.

| Group | Passed |
| --- | ---: |
| retained | 4 |
| graph | 4 |
| face-crossings | 7 |
| seam-boolean | 5 |
| graph-box | 5 |
| periodic-bounds | 1 |
| operations | 25 |
| nurbs-ss | 46 |
| boundary | 14 |
| nonintersecting | 6 |

Scoped Rust 2024 formatting, diff whitespace and API catalog checks pass.
All 3,806 frozen inputs match their manifest, with no unmanifested source files;
all 11 complete task-owned files match the current checkout. Six narrow parent-
module/integration edits are present in both sources. The archive, source and
all six copied executables have SHA-256 records. The corrected final binary set
passed all groups; intermediate failed lineage/orientation runs are diagnostics.
