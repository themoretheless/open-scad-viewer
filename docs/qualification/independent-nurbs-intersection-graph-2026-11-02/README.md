# Independent NURBS boundary-intersection graphs — 2026-11-02

Status: a native checked COMPLETE boundary-intersection graph for the admitted
proper transverse class, including actual trims and stitching across ORIGINAL
seams of DIFFERENT carriers. This does NOT finish general Boolean: retained
TWO-dimensional face regions, inside-solid labels and ordinary Model authoring
are still separate, unimplemented admission obligations. Standard Boolean is
not authorized by the new graph alone.

## Rust API

`brep_core::minkowski3d::curved::{certify_nurbs_intersection_graph,
verify_nurbs_intersection_graph}` produce a private
`QualifiedNurbsIntersectionGraph`. Its public certificate binds both complete
input Models, source atlases, limits, EVERY original face pair, all boundary
events and trim-restricted arc pieces, original shared-root keys, graph arcs,
cycle census and work counters. `vertex_enclosure` returns a certified ORIGINAL
root box, never a rounded/snapped authored vertex. Verification reconstructs
the proof from original source coefficients and compares the complete receipt.
A rejection-only structural preflight does not replace that reconstruction.

## Proof composition

1. Both Models require complete original identities, uniquely owned CLOSED
   shells/faces, whole authored boundary-use agreement, and source atlases.
   All `face_count_A * face_count_B` pairs are visited (current cap: 16 faces
   per operand). No candidate-pair list can omit difficult geometry.
2. A whole filled-trim image enclosure can prove a pair disjoint. Positive UV
   control hulls enclose a bounded filled outer region; interval evaluation
   encloses its complete carrier image. Source-atlas validity supplies trim
   closure/domain prerequisites. Positive carrier hulls provide an independent
   bound. Periodic/lifted trims use the FULL carrier hull, preventing unsafe
   clipping of a lifted UV box to one natural cell.
3. Otherwise the TYPED native original-product fiber atlas must prove COMPLETE
   coverage and a complete root-component census. No numerical continuation or
   legacy JSON completeness flag is used. Whole-support injectivity and the
   existing physical-arc constructor must additionally prove proper embedded
   arcs with a common free coordinate on each component. Either carrier order
   can supply that sufficient proof; all domains/axes are mapped back to the
   original operand order. Refused physical assembly reserves its entire offered
   endpoint allowance before another order is tried.
4. EVERY actual outer/hole coedge on BOTH faces must have a source-exact
   3D-edge/pcurve/carrier identity. Fresh original edge/surface root queries
   require full product coverage, strict curve-parameter root ranking, and
   determinate actual-trim membership. Tangent, vertex and simultaneous
   trim-boundary/seam events refuse. Even zero-root queries need exact lift
   identity: a nearby auxiliary 3D curve cannot prove absence of trim events.
5. Each original boundary root is enclosed in the original four-parameter SS
   system. Complete fiber coverage plus exact root-box exclusion identifies ONE
   component, including cases where the enclosure crosses artificial fiber
   chart boundaries. Matching uses no root midpoint or nearest world position.
6. Boundary event stations must have proved strict order, or be EXACTLY at an
   original branch endpoint. They partition the complete original component.
   A station between successive isolating bands is independently root-certified;
   its WHOLE UV root box is classified against both actual trim domains.
   Membership is constant on the EXACT connected open arc between those events:
   a change would require another original trim-boundary root, contradicting (4).
   This includes the parts inside endpoint enclosure bands; no band is discarded.
7. Active pieces lie inside both FACE trims and have original boundary-root
   endpoints. Across original seams they glue only by `(operand, original edge,
   other original face, complete curve-root rank)`. Both incident carriers have
   exact lift identities to that SAME curve/root. Every active node must own
   exactly two actual physical arcs. Source embeddings and proper pair arcs
   exclude unowned crossings; explicit degree/cycle checks reconstruct every
   closed component. This is actual arc/root connectivity, not the nerve of
   overlapping parameter charts.

The `inside` flags classify FACE trim domains, NOT material inside the other
solid. Signed cycle traversal is NOT a material-left/outward face orientation.
Native branch metadata belongs to the selected carrier order; global graph
vertex IDs are the separate `vertices`/`arcs` census.

## Validated independent-carrier case

The source is independently deformed by `y += x²/16`; the tool top is
`z = x² - x + 1/8`, with its own other boundary surfaces. Four source vertical
carriers are extended while their original trims retain the actual body.
Their raw intersection branches pass OUTSIDE actual trims between seam events.

The full matrix has 36 face pairs, 8 original shared-root vertices, 8 physical
arcs and 2 closed four-arc cycles. Four vertices have irrational parameters
`x = 1/2 +- sqrt(1/8)`. Analytic roots independently check their certified world
enclosures. Both operand orders qualify. No common quadratic deformation,
carrier merge, sampled pcurve fit or polygonal intersection substitutes for the
original NURBS. An empty translated case needs no new root/lift work. Missing
face pairs, modified root ranks/cycles, forged inactive-piece trim labels, exhausted budgets and coincident geometry
are rejected. Standard Boolean still refuses this fixture because graph coverage
alone does not certify retained face regions or a result Model.

## Limits / unfinished work

- Proper transverse arc graphs only. Tangencies, coincident sheets, singular
  strata, natural/source vertices, simultaneous seam/seam events, mixed-clock
  physical components that neither carrier order admits, and closed components
  wholly inside one carrier pair can remain unproved.
- Global support injectivity is a sufficient gate, including unused carrier
  portions; valid trimmed nongraph cases can still refuse.
- Event interval order and exact original lift identities are mandatory.
  Approximate authored edge/carrier agreement does not authorize stitching.
- Default new-stage budgets: 1M query/trim cells, 1M CHARGED SS rectangles,
  1M reported exact queried-edge lift work, 512 curve/carrier roots. Source
  atlas/boundary and injectivity gates retain their own bounded native budgets.
- Most importantly: no TWO-dimensional retained-region arrangement, inside-solid
  material certificate, exact algebraic-edge ordinary Model extension or checked
  approximate Model author is implemented by this graph API. No new general
  standard-Boolean result path or WASM/dist/browser/STEP qualification is claimed.

## Qualification

Final checks use a copied Rust workspace with literal external compile-time
fixtures, toolchain file and lockfile; no source from the changing shared checkout
is read by those builds. `source-snapshot.tar.gz` retains that workspace and the
full source manifest. Dependency source/toolchain downloads are not vendored.
Compiled executables are copied and hashed before testing.

```sh
CARGO_TARGET_DIR=/private/tmp/osv-trim-graph-frozen-target-20261102 \
CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 \
cargo test --locked \
  --manifest-path /private/tmp/osv-trim-graph-source-20261102/crates/Cargo.toml \
  --config 'profile.test.package.brep-core.opt-level=0' \
  -p brep-core --lib --test nurbs_intersection_graph --test face_crossings \
  --test seam_boolean --test graph_box_boolean --no-run
```

Core opt-level 0 / dependencies 1, with debug assertions: this is functional
qualification, not a release performance benchmark. Only the test groups listed
in the final results are qualified, not every feature in the shared source tree.

The `*-diagnostic.log` files retain intermediate failures (an initial private
helper export, an anchor enclosure subsequently padded/requalified, a concurrent
unrelated contact export, and an invalid periodic TEST fixture subsequently
corrected). They are development diagnostics, not passing evidence. The periodic
regression uses a valid uniform degree-two periodic B-spline with the first two
poles repeated and a lifted UV interval spanning a full period.

New graph/test files pass scoped Rust 2024 formatting. The parent `curved.rs`
has pre-existing formatting differences around neighboring exports, retained
without rewriting unrelated work; see `format.log` versus `new-files-format.log`.
The API catalog check reports 242 public modules and 200 catalog entries.

## Final results

113 native tests passed, 0 failed, using the copied executables above.

| Group | Passed |
| --- | ---: |
| graph | 4 |
| face-crossings | 7 |
| seam-boolean | 5 |
| graph-box | 5 |
| periodic-bounds | 1 |
| operations | 25 |
| nurbs-ss | 46 |
| boundary | 14 |
| nonintersecting | 6 |

All 3,798 frozen input files match their SHA-256 manifest after qualification;
there are no unmanifested files in that snapshot. All 11 scoped workspace files
still match that source. The archive is 9,896,438 bytes; its hash and full
manifest are retained alongside the logs. No full-suite or WASM/UI claim.
