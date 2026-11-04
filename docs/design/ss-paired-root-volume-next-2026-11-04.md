# Independent volume and outward material: design and implementation

Implemented for paired root-only boundaries; see
`ss-paired-root-volume-progress-2026-11-04.md` and its frozen native receipt.
The complete general Boolean goal and its other missing strata remain open.

1. Integrate original `surface_flux` on the two selected face domains, with
   actual new shell-use orientation. Do not reuse input-body volume or outward
   flags. Exact polynomial quadrature is optional; rational interval bounds must
   remain valid. Integrate the original source parameter clocks.
2. A root-only face is a bounded Jordan disk minus immediate child disks. A
   complete original enclosure of its outer curve bounds that entire UV disk.
   Its axis-aligned enclosure can restrict integration; no unvisited retained
   area may be silently omitted.
3. Cache conservative original UV enclosures of ALL root arcs, subdividing their
   actual owned parameter restrictions with bounded query count. The cache is
   complete or it cannot authorize root-free cells. No fitted polygon supplies
   separation authority. Cache both supports from the original UV4 images.
4. A closed UV cell strictly disjoint from the complete root-arc enclosure band
   is root-free. Native original winding at a binary witness qualifies a seed
   signature. Winding is constant on a connected root-free union, so signatures
   can propagate only across proved intersecting/touching root-free rectangles.
   Record that proof relation; midpoint signs alone are not cell authority.
   Full root winding per every small cell would be unnecessarily expensive.
5. Inside cells use whole-cell original flux integral; outside cells use zero.
   For unresolved cells use the pointwise flux DENSITY interval multiplied by
   parameter area and [0,1] mask. Multiplying a full signed integral by [0,1]
   would be wrong when density changes sign. The entire parent bound must stay
   in the total if child evaluation fails or a work budget is exhausted.
6. Use bounded priority subdivision and an interval summation tree. Explicit
   cells/spans/band queries/winding work determine a report, never an incomplete
   partial sum labeled volume. Fixed bands impose an accuracy floor: increase
   their explicit refinement allowance or report unresolved, not convergence.
7. A signed interval excluding zero establishes global orientation of this
   embedded connected boundary. A negative interval requires flipping ALL shell
   face uses. A separate immutable material owner must bind the fresh boundary,
   full retained-domain volume proof and requested accuracy. It cannot accept
   a caller scalar, or convert implicit roots to fitted legacy curves.
8. Test genus-zero inner lens and genus-one annular lens, comparing independently
   authored quartic reference volumes. Reversing original U must preserve volume
   and outward material. Zero work/insufficient accuracy must not grant ownership.
   Through-hole/rim cases still need the general cross-source stitching gate.

The paired-shell certificate is a sufficient step, not complete Boolean output
assembly. All-face-pair coverage, open-rim endpoints/splits, arbitrary singular
and coincident strata, seams/poles, nested shell material/cavities, healing,
lineage and bridge/WASM delivery remain required by the original objective.
