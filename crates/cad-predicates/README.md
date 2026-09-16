# Candidate native predicate foundation (production-candidate)

This crate implements the three predicates in the existing G2a inventory:
`orient2d`, `orient3d`, and `compare_squared_distance`. Status: **production-candidate**
for B-rep corroboration via `brep-core::predicate_evidence` (transverse line/plane
Complete and UV orientation checks). It is not yet a closed G2 qualification gate
or a solid certificate by itself. It has no runtime dependencies beyond what
`brep-core` already pulls for evidence bridging. Existing numerical geometry
decisions and their `not_certified` status are unchanged unless an explicit
Complete path invokes these predicates.

## Inputs and identities

`SourceArena::authored(source_id, revision, values)` admits finite binary64 bit
patterns and reduced rational constants with an `i64` numerator and positive
`u64` denominator. Zero is canonical only as `0/1`. A rational denominator stays
explicit throughout exact evaluation; its rounded quotient is never admitted as
a binary64 leaf. Values are immutable after admission.

This constructor is a **trusted authored-input boundary**. The caller remains
responsible for supplying original authored values. It cannot infer the history
of an arbitrary caller-supplied bit pattern and does not authenticate a string
source label. There is no `From<f64>`, geometric-model import, trusted-token
deserializer, or arbitrary constructed-point admission API. The bounded line
construction below retains a recipe and enclosure; other future constructions
must do the same. Relabeling a rounded center as a new authored snapshot is
outside this contract.

`LeafRef` has private fields and is minted only by `arena.leaf(index)`. Every
predicate validates every reference against its source arena before shortcuts.
Each arena admission has a unique in-process identity, even if source labels,
revisions, and bits are equal. This avoids accidental cache aliases or foreign
reference reuse. Source labels and revisions are retained for inspection.

`ToleranceContext` owns an immutable validated `ToleranceSpec`. All ADR fields
are retained: `linear_abs`, `linear_rel`, `on_tol`, `clear_tol`, `angular`,
`param_floor`, `ulp_guard`, `max_entity_error`, and `policy`. Positive/finite
bounds, `on_tol < clear_tol`, and `linear_abs <= max_entity_error` are required.
No tolerance is increased during evaluation. Sign computation is independent
of acceptance tolerance. Angular/parametric policy fields are retained for
identity and are not claims of implemented angular or parametric predicates.

`ContextIdentity` binds source identity, the full immutable tolerance object,
and the implementation version. It is an opaque in-process key, not a portable
certificate or a serialized content digest. Independently constructed equivalent
contexts cannot alias differently admitted contexts. Construction evaluation
uses a bounded per-query cache of exact homogeneous values; no persistent or
serialized cache is implemented.

## Evaluation envelope

The stages are:

1. A binary64 scalar filter with a propagated absolute forward-error bound.
   Every operation on that nonnegative bound rounds upward. Subnormal and
   overflowing arithmetic cannot use a relative-error assumption to publish a
   sign. This stage is skipped for rational constants.
2. Outward intervals, using adjacent representable endpoints after every
   arithmetic operation. Rational conversion is enclosed by explicit division
   of numerator/denominator intervals. Ambiguous infinite endpoint expressions
   widen to the whole interval. A filter never reports exact zero.
3. Bounded exact floating expansions built using error-free sums, split
   products, and repeated insertion of components. Rational numerators are
   multiplied by a common positive denominator product. A common exact power
   of two then normalizes coordinates and radius together, preserving the sign
   of each homogeneous predicate.

The implementation uses IEEE binary64 round-to-nearest-even arithmetic with
gradual underflow and separate ordinary operations. It must not be compiled
with reassociation, fast-math, or contraction that changes those operations.
The paper's arithmetic assumes an unbounded exponent range; this candidate
explicitly checks the finite binary64 limits instead.

The split-product stage requires non-subnormal factors, each leading exponent
at most 970, and a sum of leading exponents in `[-900,900]`. These bounds keep
splitter products, carries, and the least possible split-product bits away from
overflow and underflow. Addition intermediates must remain finite. Exact power
of two scaling checks both the leading exponent and the least nonzero input
bit; it refuses any lost bit. Uniformly tiny or huge matrices can normalize
successfully. Matrices with extreme internal exponent spreads and some large
rational-denominator products may return `PrecisionExhausted`.

Constants are limited to canonical `i64/u64`; larger integer/rational domains
are not represented by this API. Even an admitted constant matrix can exceed
the exact expansion envelope. Such cases return an explicit indeterminate
reason; the API does not promise to resolve every finite matrix.

`Decision` returns `Outcome::Sign(Negative|Zero|Positive)` or
`Outcome::Indeterminate(reason)`, along with the last attempted stage, cumulative
charged work, and context identity. No indeterminate result coerces to a Boolean
or a topology edit. Invalid input and foreign provenance are separate errors.

## Enclosure classification and controls

`distance_enclosure` issues an opaque `ProvenResidual` only from the internal
outward distance recipe and binds it to the source/tolerance context. Its
distance width must not exceed `max_entity_error`; otherwise it refuses. Bounds
are intersected with the mathematically known nonnegativity of squared distance.
There is no public arbitrary residual-bound constructor.

`classify_residual` compares the enclosure against outward squared thresholds.
It returns `Coincident` only strictly below `on_tol`, `Separate` only strictly
above `clear_tol`, and `Indeterminate` in the gray band or without proof. A proof
from another source or tolerance context is rejected. This is evidence for that
distance recipe only, not Hausdorff, derivative, topology, or model validity
evidence. General construction/evidence ledgers remain future work.

`PredicateContext` carries `Limits` and an optional atomic cancellation flag.
Work accumulates across calls, with checked arithmetic; defaults are 1,000,000
charged units and 4096 expansion components. Callers may only lower the fixed
1,000,000 work cap; a larger request refuses. Capacity above 4096 or zero refuses.
Deadline/cancellation checks occur on every charge and again before successful
publication. Addition, multiplication, insertion, and expansion traversal charge
declared work units; these units are not a hardware-instruction counter.
Resource, exponent, cancellation, and deadline refusals are explicit.

## Line construction recipes (candidate version 3)

`intersect_authored_lines2d(ctx, a, b, c, d)` solves the two infinite lines
through authored endpoint pairs. It returns a context-bound report with
`Unique(ConstructedPoint2)`, `Parallel`, `Coincident`, `DegenerateLine(index)`
or an explicit `Indeterminate(reason)`. This is a new candidate API, not a
silent extension of the frozen G2a qualification inventory or a general CC gate.

The implementation retains homogeneous coordinates: points form lines by a
cross product and the two lines form their intersection by another cross
product. Positive common power-of-two normalization preserves the represented
point or line. All sums/products use exact expansions; rational denominators
remain explicit. A nonzero homogeneous weight proves uniqueness for these
infinite lines; exact zero distinguishes parallel/coincident cases. Both line
incidences are checked algebraically before publishing a unique construction.

The opaque point retains original source references, the full context identity,
outward coordinate bounds, outward parameters for `A+t(B-A)` and `C+s(D-C)`,
and an outward L1 box diameter bounded by `max_entity_error`. Parameters are not
clamped to `[0,1]`: this API does not classify finite segment intersections.
No public constructor, field mutation, or deserializer can mint the recipe.
It publishes no topology-preservation, Hausdorff, derivative or solid claim.

`orient2d_points` accepts authored points and these constructed points together.
Issued outward enclosures can prove a strict sign. Otherwise the predicate
re-evaluates the exact recipe and homogeneous determinant; it never treats an
enclosure center as a fresh exact input. For example, the intersection at x=1/3
remains on its exact source line and lies strictly to the right of binary64
`1.0/3.0`. Up to three constructed points can participate in a predicate.

`intersect_lines2d` also accepts constructed endpoints. Each unique result owns
an immutable shared recipe DAG; dropping intermediate handles does not discard
its source dependencies. `intersect_authored_lines2d` remains the authored-only
convenience entry point. `inputs()` exposes the four immediate recipe inputs;
`source_points()` now returns `Some` only when all four are authored, otherwise
`None`. This candidate API change is identified by implementation version 3.

Graphs admit at most 32 construction levels and 256 unique construction nodes.
Every operation first validates all reachable source/context references, then
checks cancellation/resource controls. Exact re-evaluation memoizes each shared
node once within that query, with at most 65,536 retained expansion terms in the
cache. Arc addresses are private per-query lookup keys, never canonical IDs or
decision ordering keys. Debug output does not recursively expand shared nodes.
No graph cycle can be authored through the immutable API. A combined query whose
roots exceed the graph cap refuses even if each individual root was admitted.

To prevent avoidable coefficient growth, homogeneous triples use a lossless
common-factor reduction when their dyadic expansion coefficients fit a checked
120-bit-span i128 path. Exact integer extraction and a positive GCD preserve the
represented point or line; this is not rational reconstruction from a rounded
center. Larger spans retain the original bounded expansion path. The reported
diameter bound is never smaller than an ancestor's bound; the new coordinate box
itself is established by exact recipe re-evaluation, not propagation of uncertain
centers. This is not a general uncertainty-amplification or topology certificate.

3D plane intersections, curve roots and general construction/evidence ledgers
remain open. Exact expansions and their enclosures can refuse due to finite
exponent range, error width, denominator bounds, work/term caps or cancellation.
The crate still has no B-rep/WASM dependency consumer.

Six public-API construction tests use a separate bounded i128 rational oracle
with affine Cramer's rule and exact comparisons against binary64 interval ends.
The 200-case matrix yields 168 unique, eight parallel, eight coincident and
16 degenerate results. Further checks cover endpoint permutations, reflection,
shear, translation, all constructed-point orientation permutations, near-parallel
lines, the 1/3 rounding counterexample and context/resource controls. This separate
arithmetic regression is not an organizationally independent qualification review.

Six further graph tests cover a 32-level chain and its refused extension, a
25-node shared diamond of depth 17, the 256/257-node boundary and combined roots,
constructed endpoint degeneracy, cancellation/context guards, 100 general nested
intersections against the separate rational oracle, and a 200-bit coordinate span
that must keep the unreduced exact path. These observations do not change the
production qualification state.

The complete candidate suite passes **29 tests in both debug and release**;
Clippy over all targets passes with warnings denied. The original 173-case
predicate corpus still yields 172 correct decisions and one explicit exponent
refusal. No runtime dependency or frozen qualification artifact was changed.

## Implementation provenance and verification

The mathematical source read on 2026-09-12 was Jonathan Richard Shewchuk,
*Adaptive Precision Floating-Point Arithmetic and Fast Robust Geometric
Predicates*, CMU-CS-96-140R (1997), especially §§2.3–2.5 and the exponent caveat
in §2.1. This follows the allowlisted `shewchuk-robust-predicates-paper` source
and ADR 0010's `original_from_paper` mode:

- [Author's publication page](https://www.cs.cmu.edu/~quake/robust.html)
- [Author-hosted paper](https://people.eecs.berkeley.edu/~jrs/papers/robustr.pdf)

No upstream implementation was opened, copied, translated, or compared. The
candidate uses the simpler repeated expansion insertion construction and its
own bounded orchestration. No source/license inventory or frozen qualification
artifact is modified by this candidate document.

Native unit tests check hand-derived cancellation signs, rational-versus-rounded
constants, admission and resource controls. A separate agent owns
`tests/independent_oracle.rs`; it consumes a fixture generated from the independent
BigRational test oracle. These candidate regressions do not alter frozen plans,
hash pins, clean-run counters, or qualification status.

Run `cargo test --offline --locked -p cad-predicates` from `crates/`.

## 3D line/plane construction recipes (candidate 4)

`intersect_line_plane3d` accepts authored or retained constructed endpoints and
three plane points. Exact homogeneous evaluation distinguishes a unique
intersection, parallel or contained line, and degenerate line or plane. The
result retains all five inputs, outward coordinate enclosures, line parameter
`t`, and plane parameters `u/v`. `orient3d_points` evaluates plane-side signs
without promoting rounded construction centers to authored inputs.

This uses the same bounded immutable recipe graph (depth 32, 256 unique nodes),
context validation and per-query resource limits as the 2D candidate. Three new
regression tests compare more than 200 rational fixtures and plane permutations
against separate affine rational arithmetic, check exact incidence and the
rounded-one-third counterexample, nested line and plane inputs through depth 32,
context mismatch and exact degeneracies. These are candidate regression checks,
not independent qualification. Infinite lines and planes are supported; segment
clipping, triangle/trim membership and solid topology are not certified here.
The API remains native-only and is not integrated into the geometry runtime.

Candidate 4 validation: **32 tests pass in debug and release**; Clippy with
`--all-targets -- -D warnings` passes. No runtime or WASM integration is claimed.

## Exact finite-domain location (candidate 5)

`classify_line_plane_domains3d` re-evaluates a retained unique intersection
against its original segment A/B and triangle C/D/E. It compares exact rational
parameter numerators with zero and their common denominator, without rounded
quotients or modelling tolerance. Segment locations distinguish before/start/
interior/end/after; triangle locations distinguish outside/interior, each vertex
and each opposite edge. `is_closed_domain_hit` includes exact boundaries.

The result carries the same context identity and charged work count; resource,
cancellation and precision failures remain explicit. This API only classifies
an already unique line/plane construction: coplanar segment overlap and
degenerate triangles are not handled, and no solid/trim certificate is issued.

Regression coverage includes 735 grid/domain/permutation combinations,
one-binary64-step differences across an edge, separate rational affine fixtures
in non-axis planes, and constructed plane/segment inputs through depth 32.

Candidate 5 validation: **34 tests pass in debug and release**;
Clippy `--all-targets -- -D warnings` passes.

## Coplanar segment/triangle overlap (candidate 6)

`intersect_coplanar_segment_triangle3d` clips an authored or constructed segment
against a coplanar triangle. Exact barycentric half-space inequalities determine
the lower and upper segment parameters; rational cross-products compare them
without division. Results distinguish empty, a singleton contact, and a
nonzero overlap. The overlap retains its five inputs and context, exposes
outward parameter enclosures, and can supply the original inputs for repeat
evaluation. Enclosure endpoints are not new authored or constructed points.

Noncoplanar inputs, zero-length segments and degenerate triangles have distinct
results. Context, graph limits, cancellation and arithmetic limits apply. This
query does not create trimmed topology or a solid certificate. Endpoint
construction for the returned overlap and runtime integration remain open.

A separate affine oracle enumerates boundary intersections and contained
endpoints before sorting them, rather than reproducing incremental clipping.
It checks 1,344 segment/placement/winding cases, including edge overlaps and
vertex contacts, under exact shears, reflections and translations. Retained
depth-32 constructed segment/triangle inputs are also exercised.

Candidate 6 validation: **36 tests pass in debug and release**;
Clippy `--all-targets -- -D warnings` passes.

## Retained overlap endpoint constructions (candidate 7)

`construct_overlap_endpoint3d` turns either bound of a coplanar overlap into a
`ConstructedPoint3`. Its immutable recipe retains all five source inputs and
the lower/upper selection; evaluation repeats exact clipping and homogeneous
interpolation. No enclosure center becomes a source coordinate. `kind()`
distinguishes line/plane and overlap endpoint recipes. The resulting points
can be used by orientation, finite-domain classification, line/plane
intersection, and further coplanar queries. Singleton bounds are exactly equal.

The existing depth-32, 256-node, context, work, cancellation and maximum entity
error policies apply to both recipe kinds. Coordinate and line/plane parameter
enclosures share a publication path. Depth 33 refuses without invalidating
previous inputs. Regression checks cover both endpoint coordinates across the
transformed clipping oracle, exact plane incidence, downstream domain checks,
a retained 4/3 coordinate through depth 32, singleton degeneracy and failures.

This implements the bounded endpoint construction gap recorded above for
candidate 6. General trim arrangement, topology mutation and runtime integration
remain separate unfinished requirements.

Candidate 7 validation: **38 tests pass in debug and release**;
Clippy `--all-targets -- -D warnings` passes.

## Unified closed segment/triangle query (candidate 8)

`intersect_segment_triangle3d` combines transverse finite-domain classification
and coplanar clipping. Results distinguish empty, a single retained point and
a nonzero segment with two retained endpoints in increasing input-segment
parameter order. Every returned point carries its exact segment/triangle
feature location. Degenerate primitives and arithmetic/resource failures are
explicit, and no partial overlap is returned if endpoint construction fails.

The whole query shares a work/cancellation context. Both output endpoint graphs
are checked together against the unique-node cap before publication. Admission
validates all source references before resource shortcuts. Regression checks
cover transverse/coplanar hits and misses, endpoint and vertex contacts, reverse
segment order, degenerate inputs, foreign inputs under cancellation, and budgets
expiring throughout the combined operation.

This is a native query API for later topology integration; it does not mutate
B-rep entities or close the runtime, trimming or qualification requirements.

Candidate 8 validation: **40 tests pass in debug and release**;
Clippy `--all-targets -- -D warnings` passes.

### Exact 3D coordinate ordering

`compare_points3d` compares authored and retained constructed points
lexicographically with exact homogeneous cross-products. Exact equality
does not depend on recipe identity or rounded coordinate centers. This supplies
a numerical primitive for future intersection deduplication, without assigning
persistent topology identity or tolerance-based coincidence. Admission, graph
limits and cancellation apply even when comparing the same handle. Regression
checks distinguish exact 1/3 from binary64 rounding, independent equivalent
recipes, later-coordinate ordering and context/cancellation precedence.

Coordinate-ordering validation: **41 tests pass in debug and release**;
Clippy `--all-targets -- -D warnings` passes. Runtime integration remains open.

### Noncoplanar triangle pair intersection

`intersect_triangles3d` assembles the six exact edge/triangle queries, removes
algebraically equal endpoints and returns empty, a point, or the common segment
in exact lexicographic order. Returned endpoints retain their constructions.
Coplanar triangles and degenerate inputs are explicit separate results;
coplanar polygon overlap is not yet assembled. Query work is shared and the
combined output graph is checked before publication.

Regression checks cover transverse overlap, vertex contact, separated planes,
coplanarity, degeneracy, triangle exchange and winding reversal. The native
suite passes **42 tests in debug and release**, and Clippy with all targets and
`-D warnings` passes. This bounded query does not close general trim topology,
curved intersections, runtime integration or full B-rep qualification.

### Candidate 9: coplanar triangle intersection contours

`intersect_triangles3d` now assembles coplanar area intersections into an
implicitly closed convex contour of three to six retained vertices. Exact
projected homogeneous orientation builds the hull and removes redundant
collinear points. The first nonsingular coordinate pair defines positive
(CCW) winding; the first vertex is the exact lexicographic minimum. The result
reports the projection axes and does not repeat the closing vertex.

Point and edge contacts still reduce to point/segment results. Checks cover
a six-vertex overlap in XY, XZ, YZ and a sheared reflected plane, operand swaps,
winding reversal, exact vertex coordinates/incidence and strict convexity.
This replaces the earlier explicit coplanar deferral for triangle pairs.
Arbitrary curved trims, persistent topology and runtime integration remain open.

Candidate 9 validation: **44 tests pass in debug and release**;
Clippy `--all-targets -- -D warnings` passes. Full B-rep completion remains open.

### Separate rational polygon oracle

A fixed-seed matrix now compares 172 nondegenerate triangle pairs (180 attempts,
eight degenerate pairs excluded) against separate rational sequential polygon
clipping. This oracle does not use the production edge-enumeration/convex-hull
path. Checks cover output cardinality and every exact rational vertex enclosure,
including more than 100 fractional coordinates, over 80 area overlaps and
multiple empty intersections. It is regression evidence, not independent
qualification or proof of complete B-rep behavior.

The full candidate suite passes **45 tests in debug and release**; Clippy
`--all-targets -- -D warnings` passes. No runtime behavior changed in this step.

### Exact point location on an arbitrary triangle

`classify_point_triangle3d` accepts authored or retained constructed points and
an arbitrary oriented triangle, with exact vertex/edge/interior/outside
classification. Off-plane results retain their exact side sign; degenerate
triangles and resource failures stay explicit. Modelling tolerance does not
snap a nearby point to the plane. The original recipe-domain classifier shares
the same feature-sign classification helper.

Checks cover all triangle features and positive/negative 1e-12 plane offsets.
Every output vertex in the 172-pair rational polygon matrix is additionally
classified on both original triangles. **46 tests pass in debug and release**;
Clippy `--all-targets -- -D warnings` passes. B-rep topology and runtime
integration remain unfinished.

### Portable 3D recipe records

`export_point3d` emits a bounded dependency-first graph with source label and
revision, authored leaf indices and exact bits/rationals, construction kinds,
root index, implementation identifier and tolerance specification. Shared
recipe dependencies appear once. No pointer identity, cached expansion or
rounded construction center is exported as source data.

These are portable typed records, not trusted admission tokens or a complete
file format/importer. A future importer must match the authored source and
replay validated operations. Checks cover shared dependencies after handle
drop, rational source preservation, ordering, repeatability and context/cancel
failures. **47 predicate tests pass in debug/release**; all-target Clippy passes.
Safe graph reconstruction and project persistence remain open.

### Source-checked 3D graph replay

`replay_point3d` checks schema/implementation, source label/revision, exact
source leaf values, tolerance bits, dependency ordering, reachability and graph
limits before rebuilding every operation. It binds the result to the supplied
already admitted source/context; it never creates an authored arena from the
record's embedded values. Line/plane and coplanar endpoint recipes are replayed
through existing checked constructors, with shared dependencies preserved.

Tests cover exact 1/3 and 4/3, rebinding to a separately admitted matching source,
shared DAG replay, modified values/revision/tolerance, forward references,
unreachable nodes, invalid operation semantics and cancellation. This is typed
record replay, not an authenticated file importer; callers still own source
admission. Byte encoding, full project integration and general construction
coverage remain open.

Replay validation: **49 tests pass in debug/release**; all-target Clippy passes.

### Bounded binary 3D recipe records

`encode_recipe_graph3` / `decode_recipe_graph3` implement BRG3 version 1:
little-endian integers, raw binary64 bits, signed rational numerators and
unsigned denominators, metadata and dependency-first operation records. Input
size is limited to 256 KiB, strings to 4096 bytes and nodes to 256 before their
allocations. Dependencies must precede their node; unknown tags/version,
truncation and trailing bytes are rejected. Decoding still produces untrusted
records requiring source-checked replay. No runtime dependency was added.

A roundtrip test preserves negative rational inputs and exact -1/9 replay,
checks byte stability, every truncated prefix, extra bytes, version corruption,
length overflow and oversize input. **50 predicate tests pass in debug/release**;
all-target Clippy passes. Project-level source admission and integration remain
open; this codec is not a complete B-rep file format or authenticity proof.

### Candidate 10: bounded one-pass exact replay

A new depth-32 binary roundtrip regression exposed ResourceLimit during replay:
the earlier implementation repeatedly reevaluated ancestor recipes through the
public constructors. Replay now evaluates each node once in dependency order,
retaining exact homogeneous results in a per-query cache capped at 65,536
expansion terms. Copy/arithmetic work remains charged; source admission and
coordinate/parameter publication checks remain in place.

The depth-32 overlap chain now restores under default limits with its exact
4/3 coordinate and shared recipe structure preserved. **50 tests pass in debug
and release**; all-target Clippy passes. The implementation identity advances
to candidate-10; replay continues to reject mismatching implementation records
explicitly. General project integration and full B-rep certification remain open.

### Atomic bounded vertex batch replay

`replay_point3d_batch` now decodes and restores an ordered set of binary vertex
recipes against one admitted source/context and one cumulative work budget.
Limits are 256 roots, 256 KiB per record and 8 MiB total encoded input; excessive
root count is rejected before scanning records. Decoding bytes consume work.
Malformed records, foreign source metadata and any indeterminate replay discard
the whole result; consumed work is not rolled back. This is atomic result
publication, not a topology/solid certificate or hard real-time cancellation.

The indexed six-vertex contour roundtrip now uses this batch path. Regressions
cover exact rational recovery, cumulative budget exhaustion after a successful
first vertex, corruption and foreign source in later records, count/per-record/
total byte limits, empty batches and cancellation. All 60 predicate/topology
tests pass in debug and release; module-only all-target Clippy passes with
warnings denied. Production project persistence and general curved B-rep
operations remain incomplete.

### Bounded batch export and publication checks

`export_point3d_batch` completes the save side of the vertex archive path:
ordered roots share one predicate work budget and the existing 256-root/8-MiB
batch limits. Individual recipes retain exact authored values and construction
DAGs. Failed exports return no partial archive and retain consumed work. Both
export and replay check cooperative cancellation/deadline immediately before
returning a successful batch; this does not prevent cancellation after return.
The six-vertex indexed topology integration now uses batch export and replay.

Regression evidence covers single-versus-double cumulative export cost,
second-root budget exhaustion, root count bounds, and cancellation/deadline on
empty batches. 61 predicate/topology tests pass in debug and release;
module-only all-target Clippy with denied warnings passes. This remains native
candidate infrastructure; UI project persistence and general curved B-rep
certification are still open.
