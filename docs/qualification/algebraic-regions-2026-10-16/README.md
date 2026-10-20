# Algebraic parameters, multi-span coincidence and global offset regions

Date: 2026-10-16. Geometry and proof validation remain in Rust; TypeScript is
transport. This report covers the three requested extensions, not all dirty
parallel changes in the checkout.

## Delivered behavior

- Non-dyadic rational and irrational isolated intersections are represented by
  exact polynomial factors, rational isolating intervals and independently
  checked parameter maps. A separating projection handles repeated first-curve
  parameters. Exact root counts reject omitted events and false classifications.
  Production Boolean fallback and high-degree parent normalization use this
  census; rounded trim parameters retain numerical status.
- Multi-span image agreement covers unequal knot partitions without requiring
  rational knot preimages in the other curve. Each checked overlap advances an
  exact image cursor. Reversed and closed piecewise paths are admitted; missing
  tails or changed relations refuse.
- Global positive-winding cleanup splits crossings and collinear overlaps,
  removes internal borders/negative artifacts, merges overlapping components
  and assigns holes. The certificate binds source topology, each joined offset
  wire and the exact rational output boundary. Output rounding is checked for
  displacement, new contacts, orientation and ownership.

Validation also preserves full multi-span NURBS definitions while checking
their active retained spans. Refined/rescaled circles keep their analytic
carrier route; elevated circles use the general profile route. Explicit 2D
admission rejects 3D source poles without silently dropping coordinates.

The new WASM operations and TypeScript service functions expose all three
certificate producers and verifiers. Separate native checker modes validate
WASM-produced JSON without using the geometry bridge.

## Scope and limits

These are executable exact-arithmetic Rust certificates over stored input bits,
with trusted compiler/runtime/arithmetic libraries. They are not a
proof-assistant verification of the Rust implementation. The algebraic event
scope currently admits one active retained span per curve, degrees 1–8, positive
weights and a finite resultant/factor/root budget. Seven projection candidates
are tried; nonisolated fibers and exhausted budgets explicitly refuse.
Injective self certificates prove absence of nontrivial self crossings and do
not claim a complete stationary-jet census. Root interval width is checked in
the actual source domains; outward binary64 conversion may add an endpoint ULP.

Multi-span proofs require a covering sequence of common local monotone
coordinates within six subdivision levels, 128 cells per source and 256 steps.
Singular/unpartitioned cases can remain unproved. The producer does not refit
source poles or replace irrational preimages by exact floating-point witnesses.

Offset cleanup proves positive-fill regularization of the specified bounded
joined wires. It does not prove universal Minkowski erosion/dilation or
Hausdorff proximity to discarded internal wire segments. The earlier exact
line/circle filled-region API is preserved. Singular normals, exhausted
arrangement budgets, unresolved hole ownership and topology-changing rounding
refuse. Separate components are not relabelled a manifold compound solid.
Constructed profiles and solid meshes retain their separate numerical checks.

## Runtime evidence

The runtime fixtures include irrational quartic crossings, a multiple tangency
at parameter 1/3, two events sharing one first-curve parameter, an irrational
quartic self loop, two irrational stationary diagonal roots and unequal knot
partitions with knot preimage sqrt(1/2). Offset fixtures cover merged separated
source boxes, inward corner artifact removal and a regular quadratic source.

Five actual extrusions (two normalized self-loop components and three cleaned
offset regions) have closed meshes, no degenerate triangles, no nonmanifold
edges, no orientation conflicts and volume discrepancy below 0.3%. These mesh
checks are fixture qualification, not a formal tessellation or mesh
self-intersection proof.

Performance is recorded per API call in `wasm-proofs.json`: the curved source
uses 261 points across its checked wire parts and 281 output boundary vertices.
At deviation 0.05 mm the final WASM producer took about 13.86 seconds and a
separate verifier call about 5.54 seconds. An earlier staged run took 45.9 /
13.8 seconds under different compilation/load conditions; these runs are not
a controlled speed comparison. Simpler box-region calls were below three
seconds. General curved certificate construction is currently expensive; this
delivery does not claim interactive latency.

Final native/host/standalone counts, browser evidence and artifact hashes are
recorded below and in `manifest.json` after packaging.

## Final validation and package

- 98 selected native Rust tests passed: 50 certificate tests, 41 Bézier modeling
  tests and seven bridge tests. Filtered-out tests are not counted.
- 69 service/host tests passed across 11 files, including the corrected refined,
  rescaled and elevated circle admission plus analytic elliptical volume and
  centroid regressions. Vue and MCP type checks passed.
- Nine final-WASM certificates were accepted by the independently built native
  checker; nine forged counterparts were refused.
- Five extrusion meshes passed the stated numerical gates. The merged offset
  fixture was imported and visually checked in the final built app with zero
  recorded page warnings/errors (`browser-merged-offset.png`).
- Public, generated and dist WASM SHA256:
  `6762b2a0db6aa5864d7fd704bdef49b02ec8e80e4c33536b82b680913b8b0bbb`,
  11,470,062 bytes each. The packed fallback and raw artifacts passed dist checks.
- Vite build and verification passed for 173 artifacts: 8,176,813 asset bytes +
  14,498,343 raw WASM bytes = 22,675,156 total bytes. Asset growth is 44,415 bytes
  from the previous delivery. The aggregate allowance is explicitly 8,190,000;
  the geometry chunk allowance remains 3,970,000.

The temporary browser tab and preview server were closed. No commit or push was
performed for this request; parallel dirty work was preserved.
