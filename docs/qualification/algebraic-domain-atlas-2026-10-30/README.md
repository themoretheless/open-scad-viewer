# Algebraic-domain 3D atlas — 2026-10-30

A new native atlas representation retains the exact rational 3D parent maps and
represents their child parameter regions by an exact algebraic cell complex.
It does not force algebraic restriction boundaries into rational control nets.
This is a full-cover refinement of a body accepted by the existing base source
atlas verifier, not a replacement for that verifier on previously unproved
B-reps. No universal acceptance of arbitrary input geometry is claimed.

## Public Rust API

Under `minkowski3d::curved`:

* `certify_algebraic_domain_atlas(model, depth, cuts)`
* `verify_algebraic_domain_atlas(model, depth, cuts, certificate)`
* `check_algebraic_domain_atlas(...) -> VerifiedDomainAtlas`
* `VerifiedDomainAtlas::evaluate(parent, exact_u, exact_v)`

One cut set may be broadcast to all checked parent charts, or one set supplied
per chart. A cut is a canonical exact bivariate power polynomial in the parent
parameters, represented as coefficients `[v_power][u_power]`. Integer-string
ratios are never converted to binary64. An empty polynomial is identically zero.
Each local open region has a source-polynomial sign vector. The complete cell
complex also retains its one- and zero-dimensional strata symbolically.

## Local proof

The joint arrangement includes ALL cut polynomials and the three parameter
boundary factors v, 1-v and 1-u-v. One complete exact cylindrical decomposition
therefore sees intersections, tangencies, vertical components and closed loops
of the whole family; independently computed curves are not guessed together.
All root sections and critical fibers are represented by selected algebraic
roots and ranks. The square-domain decomposition's diagonal boundary tags select
the source triangle; nodes on the diagonal are retained, not deleted.

For each open projection slab, the ordered sections partition [0,1-u] into
connected open bands and root sections. At a critical parameter, ordered fiber
nodes and intervals cover the same segment. One-sided incidences attach every
cell boundary exactly. Cell chains are closed and oriented; their total boundary
must cancel to precisely the original three parameter sides, and V-E+F=1 is
checked. These are structural checks IN ADDITION to the complete algebraic root
census and incidence proof, not an area-only or Euler-only no-overlap heuristic.
Two-arc lens cells are legitimate. Zero and constant cuts are handled explicitly.
Rational samples only label already proved sign-invariant open cells; they do
not decide curve connectivity, coverage, or geometry.

## 3D assembly and nongraph geometry

The base source verifier reconstructs and proves the original maps, orientations,
parent incidence, vertex stars, global separation and source binding. The new
atlas retains those same maps, including nonplanar and removable-pole charts.
No graph representation is required by the domain layer. Parent boundaries are
paired using exact pointwise 3D identities; previously proved straight-image
clock relations are also retained as exact equality relations between the two
original edge maps. Child domains are glued by restriction of these FULL parent
transitions. Differently subdivided child boundaries do not need matching mesh
vertices: their exact overlap relation is inherited from the checked parent
maps. Each local partition covers its entire parent triangle, so this quotient
atlas has exactly the base body's geometry and topology.

This is an implicit-domain atlas, not a newly generated conforming global PL
mesh. Its `base` field retains the existing nominal mesh certificate. Removing
selected regions or creating a different solid would require a new closure and
embedding proof; that operation is not implemented here. Likewise this layer
does not bypass a failed base source proof for a difficult original trim.
Legacy v4 and its receipts are unchanged.

## Checked queries

A checked handle is constructible only after verification and exposes immutable
certificate data. Rational point location uses exact signs at selected roots,
not interval midpoints. It locates vertices, critical/section edges and open
regions, then evaluates the original 3D map by exact de Casteljau arithmetic.
Zero-weight source corners use the limits already proved by the base atlas.
The handle is an immutable snapshot; model edits require new verification.

## Bounds and qualification scope

At most four source cuts per parent, source cut degree <=8 in either variable,
joint degree <=16, total cell count <=16384, and the existing source, arithmetic
and algebraic-isolation budgets remain. Queries have separate finite parameter
and coordinate budgets. Unsupported or exhausted cases refuse. No general real
algebraic control-field arithmetic, arbitrary new trimmed-solid construction,
WASM/dist rebuild, browser run, or standalone checker-process replay is claimed.

Tests cover closed curves, intersecting cuts, coincident cuts, elliptic algebraic
boundaries, cube assembly, nonplanar conical maps, exact region/edge/vertex
queries, source limits at pole corners, and forgeries of maps, local regions,
oriented attachments, cut requests and parent transitions.

## Validation

**107 native Minkowski tests passed, 0 failed**. The four domain-focused tests
also passed. The final full run includes exact point queries, critical/boundary
strata and zero-weight corner limit checks, together with all previous source
atlas, nonplanar graph, cone, sphere and torus regressions. Test wall time 58.18 s
is observational, not a performance benchmark. Formatting and scoped whitespace
checks pass.

```sh
CARGO_TARGET_DIR=/private/tmp/osv-atlas-20261028-target \
CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 \
cargo test --manifest-path crates/Cargo.toml -p brep-core --lib minkowski3d:: -- --nocapture
```

`native-tests.log` is final full-run evidence; `focused-domain-tests.log` and
`checked-query-tests.log` are intermediate focused runs. `result.json` records
scope and unresolved work. `source-files.json` binds eight scoped files, not a
complete isolated workspace snapshot. Other concurrent work was preserved.
No commit, push, runtime rebuild or deployment was performed by this milestone.
