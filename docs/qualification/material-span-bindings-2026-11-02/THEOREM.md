# Closed original volume and material boundary ownership

The opaque Rust OriginalClassifiedClosedMaterial owner combines a freshly
certified original filled source embedding with a valid, complete material UV
region, including every hole, and exact oriented boundary correspondence.
Each ordered binding now retains both original curve addresses: global source
profile index / nonempty knot span, and material ring / trim curve / nonempty
knot span. These records are minted only after the corresponding native proof
succeeds and become public only through the successfully admitted owner.

NormalizedMap records prove equality after each span's positive affine clock
normalization. OrientedLineImage and OrientedProjectiveImage records prove only
the same oriented image; they do not confer equal clocks or derivative scales.
The retained/error-bound preparation path continues to demand NormalizedMap.
The typed classified volume separately proves the requested profile G1/G2 jets
and keeps declared C0 joins explicit. UV span counts need not equal original
curve counts, so addresses are captured during exact decomposition, not inferred
later from aggregate bound_spans or class indices.

The complete valid material region lies in the source carrier domain. Restricting
the certified filled embedding to this region preserves its material topology;
orientation and hole ownership come from the fresh native trim audit and the
ordered, same-orientation boundary identities. This does not turn a raw volume
certificate or a copied binding record into authority for another source.

Scope: closed source motion with one immutable reference material carrier. This
change does not admit independent start/end trim clocks, general open capped
volumes, unsupported pole charts, or globally smooth sharp C0 corners. It adds
no TypeScript geometry and makes no new retained Solid, WASM, STEP or CI claim.
