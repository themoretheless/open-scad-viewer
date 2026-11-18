# Algebraic clocks in the common SS/BRep selector

`OriginalSsRootPoint` now owns either its exact binary selector or an
`OriginalSsAlgebraicPoint` with an explicit clock refinement policy. The latter
retains the literal original graph and qualified clock owners. `binary_parameter`
returns `None` for algebraic selectors; no midpoint is exposed as a definition.
The former `parameter() -> f64` API was removed and its BRep callers migrated to
`validate_selector`. This is an intentional source API change.

Common endpoint ports use exact rational clock bounds for same-graph binding
and original UV4/world enclosures for foreign affine-frame graph binding.
Existing binary node and collapsed-pole aliases remain unchanged; algebraic
node/pole aliases require a further exact endpoint equality proof. Reverification
now replays the owned point/clock as well as the target graph and membership.
Clock arithmetic has its own explicit allowance; point enclosure adds its
counted operations to reported projection work with checked overflow.

Common edges and wires therefore retain algebraic selectors without introducing
a separate BRep edge type. RootBrep and MixedBrep vertex validation checks the
owned selector domain rather than assuming every vertex has a finite f64 clock.
Storage closure does not prove Jordan embedding, material choice or solidity.

Qualification includes splitting a real original SS circle edge at sqrt(1/8),
closing the resulting nine-edge common wire, creating the paired BRep, replaying
it and querying both original pcurves. Independent clock owners cannot weld a
vertex. A separate native regression binds an irrational common selector across
mixed parameter axes, checks clock work accounting and refuses wrong branches,
invalid phases and exhausted clock policies. Existing binary and periodic
regressions are retained.

Still required: generated third-face constraint polynomials with original-source
provenance, complete cross-source face-local junctions, algebraic node/pole
aliases, finite trim/winding across singular and periodic arrangements, material
selection, general intersecting Boolean/healing, full lineage/history and actual
bridge/WASM execution. The wider quad/offset/fillet/G2 requirements remain open.

Follow-up: exact rational endpoint equality now supports algebraic node/pole
aliases; see `ss-algebraic-node-pole-progress-2026-11-04.md`. This does not
complete arbitrary singular or periodic arrangements.
