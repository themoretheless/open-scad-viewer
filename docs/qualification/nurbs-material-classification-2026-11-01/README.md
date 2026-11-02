# Whole-domain NURBS exclusion and retained-material classification

Status: exact, source-bound proof for **nonintersecting connected source boundaries**. Transverse/tangent/overlapping general NURBS intersections remain unresolved. This is not universal NURBS/NURBS Boolean completion.

## Geometry and completeness contract

`certify_nonintersecting_nurbs_boolean` independently proves each operand's complete original trimmed NURBS atlas and connected, outward single-shell boundary. There is no convexity, graph, common projection, common deformation or shared parameterization requirement.

Every pair of cross-operand chart homotopies is checked by the existing exact rational swept-hull separator, including bounded exact subdivision. Both time endpoints and the entire intervening rational homotopy are covered. Only complete successful exclusion of **all** cross pairs authorizes the zero-intersection conclusion. A failed separator, touch, intersection, resource limit or unproved source atlas refuses; it is never counted as an empty pair. Existing degree/bit/work/chart budgets remain unchanged. Public calls expose the existing depth 0..3; standard dispatch uses depth 0 without raising it on failure.

After complete exclusion, each connected boundary has constant membership in the other solid. Two exact generic-ray winding predicates on the proven PL endpoints determine that membership. They are sufficient only because source embedding, positive orientation, connectedness and the joint disjoint homotopy have already been proved. Membership transfers to the original rational surfaces. This is not centroid/AABB classification or sampled ray casting on original NURBS. The PL volume is used only to prove source orientation, not as the original curved solid's volume.

For [A-boundary in B, B-boundary in A], the only accepted winding pairs are [0,0], [0,1], [1,0]. Mutual containment or other winding numbers refuse. Boolean truth tables select entire original unchanged shells:

| Relation | Union | Difference A-B | Intersection | Xor |
|---|---|---|---|---|
| Disjoint material | A + B | A | Empty | A + B |
| A contains B | A | A with reversed B cavity | B | A with reversed B cavity |
| B contains A | B | Empty | A | B with reversed A cavity |

Before source-atlas derivation, the actual 3D edge curves of both input Models receive an independent whole-interval boundary agreement check at their declared tolerances (`verify_with_exact`, 100000 interval cells and 1000000 exact-identity work units per input). The certificate records each source's use/cell ledger. A surface atlas alone is not evidence for a separate authored 3D edge curve. One regression creates a degree-9 perturbation vanishing at all nine sampled-validator positions: ordinary sampled validation passes, while Boolean certification must refuse. No source edge budget exhaustion is accepted. Result construction only copies/reindexes the proved original curve/pcurve/carrier relations; shell orientation changes do not change their clocks or geometry. The positive result tests also independently call continuous Model validation.

The result is an ordinary editable Model with retained original NURBS carriers, curves and pcurves. New cavity orientation and index offsets are authored explicitly. A fresh global result-atlas proof checks every nonempty assembled result. No fitting, coordinate rounding or invented intersection curves are introduced.

The certificate binds both sources, operation, complete source atlases, cross-pair count, winding pair, relation, full result identity and result atlas. `verify_nonintersecting_nurbs_boolean` reconstructs the proofs and full Model; caller-supplied classification flags do not supply authority.

## Standard Boolean integration

Existing common-quadratic and specialized exact curved families keep priority. Their dispatcher is isolated in `curved_boolean_exact`; a recognized proof refusal (including an early specialized-family refusal) then invokes the independent complete-material prover. Invalid-input and explicit resource-limit codes retain their hard-error behavior. Supported analytic primitives do not pay for an unnecessary general atlas proof. A failed sufficient proof may be handled by another exact family. The final automatic numerical fallback was removed: if the exact families do not prove the intersection/material obligations, standard Boolean returns `BREP_NURBS_BOOLEAN_UNPROVED`. This is an intentional behavior change: previously some such calls returned a numerically traced candidate with sampled residuals.

Explicit numerical candidate APIs, including `nurbs_boolean_continuous_candidate`, remain available with their documented boundary-only scope. The direct numerical torus/cylinder/sphere tests are retained on that API; standard dispatch now has a regression that refuses an unproved off-axis crossing. Existing exact analytic cases must continue to pass. This change does not claim a new proof for every old simplifier or specialized family.

## Scope and open work

Inputs currently require one connected outward outer shell per operand, with no pre-existing cavity shells or multiple bodies. Every coedge must use the full forward authored domains of its stored 3D curve and pcurve (or have no explicit range metadata). Partial/directed subrange materialization is not yet covered by this source-atlas constructor and refuses explicitly. A complete diagnostic relation over only a requested subrange is not whole-boundary authority. Genus/concavity is not prescribed. Results can have a cavity or disconnected bodies. Some valid separated inputs may still be unproved by the bounded sufficient atlas/separation checker.

Still open: complete general transverse/tangent/coincident intersection strata, owned branch/trim arrangements through face seams, classification and assembly of the resulting split regions, and arbitrary algebraic edge authorship or topology-certified approximation. Unsupported cases refuse rather than acquire topology authority from a numerical seed graph. WASM/browser/dist are not qualified here.

## Regressions

The source is a nonconvex L-shaped solid independently deformed by z += x^2/64. The other operand is independently deformed by y += z^2/64; their common quadratic dispatch is explicitly unavailable. Tests cover all four operations for strict containment and for disjoint material with overlapping overall AABBs, reversed operand containment, cavity orientations, certificate forgery, inward shells and actual crossings. Standard Boolean is compared with the exact certified result for a nested cut.

Final qualification uses a separate immutable copy of the Rust workspace and its literal compile-time fixtures. The original checkout continues to contain unrelated concurrent work. The archive binds the copied source tree; external dependencies are pinned by the copied Cargo.lock. Compiling unrelated modules in that snapshot does not qualify their behavior.


## Retained diagnostics

`fixture-domain-diagnostic.log` records the initial malformed test fixture: its degree elevation had incorrectly replaced original active parameter ranges by [0,1]. The fixture was corrected to preserve every original surface and edge domain before any geometry-proof qualification. No proof predicate or arithmetic budget was relaxed.

`early-prover-order-diagnostic.log` records an intentionally interrupted qualification run after observing unnecessary general atlas work before supported analytic families. Dispatch was reordered to preserve the specialized exact paths' priority; the final qualification uses that order.

The numerical integration suite keeps its original volume/Euler/invariance/sequence oracles on the explicitly selected numerical candidate API. Exact-family cross-validation still uses standard Boolean for the exact result. A new integration regression requires standard Boolean to refuse the unproved off-axis crossing for union, intersection and difference while the explicit candidate remains available.


## Final qualification

Source bundle: `source-snapshot.tar.gz` (archive SHA-256 in `archive-sha256.txt`). It contains the Rust workspace, pinned toolchain, Cargo.lock, 76 additional compile-time fixtures and `source-sha256.txt`. All copied source hashes were unchanged after build and tests. The nine scoped implementation/test files also match the delivered working tree. The copied unit/integration executable hashes are recorded in `binary-sha256.txt`; compiler provenance is in `rustc.txt`.

The isolated build used:

```sh
CARGO_TARGET_DIR=/private/tmp/osv-nurbs-material-frozen-target CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 cargo test --manifest-path crates/Cargo.toml   --config 'profile.test.package.brep-core.opt-level=0'   -p brep-core --lib --test tolerant_ssi --no-run
```

This is functional native qualification: brep-core uses opt-level 0, its dependencies opt-level 1. It is not a release-performance benchmark. Suites ran on copied executables with `--test-threads=2`:

| Filter | Passed | Failed | Seconds |
|---|---:|---:|---:|
| `minkowski3d::curved::atlas::boolean::` | 6 | 0 | 82.09 |
| `tolerant_boolean::` | 20 | 0 | 60.55 |
| `operations::` | 25 | 0 | 6.78 |
| `nurbs_ss_g6::` | 46 | 0 | 2.67 |
| `boundary_agreement::` | 14 | 0 | 3.28 |
| External `tolerant_ssi` integration harness | 15 | 0 | 311.50 |

**126 distinct tests passed; 0 failed.** Source/binary hashes, scoped formatting and diff-whitespace checks passed. `earlier-structured-boolean.log` contains six additional passing structured-family checks from a pre-final optimized artifact and is not included in the final total.

The six new proof regressions include the specialized-graph-family refusal followed by a successful independent material proof through standard Boolean, the sample-invisible degree-9 edge defect, and partial-use evidence refusal. Each remaining positive test checks full source/cross-pair coverage or continuous result agreement. Verification rejects forged winding data, inward source orientation, true crossings and incompatible ranges.

The shared-checkout build diagnostics (`shared-body-*`) record transient Body-schema/import failures while an unrelated module was being edited; final qualification is insulated from that churn by the archived snapshot. No shared diagnostic was accepted as a passing build.

Open goal remains unchanged: general **nonempty** NURBS/NURBS intersections, their complete branch/trim graph, split-region classification and resulting editable B-rep authorship are not proved by this path. Such calls may succeed through an existing separately admitted exact family; otherwise standard Boolean refuses. Explicit numerical candidates remain separate. No WASM, browser or dist qualification is claimed.
