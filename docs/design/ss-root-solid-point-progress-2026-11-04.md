# Original-surface point membership for paired root solids

`QualifiedPairedRootSolid::classify_point` uses complete original-surface ray
intersection coverage and the actual selected root-loop domains. It shares the
legacy ray traversal rather than fitting or tessellating boundary curves.
Only complete positive crossings inside the trim contribute to parity.
Unisolated roots, overlapping ray intervals, boundary origins, unresolved trim
and exhausted work return an unresolved result. Point queries permit up to 4,096 winding segments per query; the previous
uniform subdivision exhausted both 1,024 and 4,096 segment allowances on the
near-edge regression. Native winding now adaptively splits only arcs whose
complete convex enclosures overlap the query. Accepted chords remain connected
through shared original endpoint enclosures, and every arc examination and
original enclosure query consumes the existing shared allowances. This does
not relax the required separation proof.
Retry directions share bounded
ray-cell, domain-query and winding-query allowances.

A complete enclosure of every original boundary arc permits root-free domain
classification. When a conservative enclosure overlaps a ray-root UV box,
whole-box native winding can prove its material signature directly. A sample
at the center alone cannot authorize that fallback. Previously certified boxes
can authorize contained boxes on the same original support.

Regression coverage includes genus-zero and genus-one quartic lenses, points
inside and outside, the central hole, a near-edge point within a conservative
boundary band, actual faces and a shared edge. Missing boundary work and an
insufficient shared ray allowance remain unresolved. Existing ray-parity tests
check compatibility of the extracted traversal.

This is a prerequisite for shell nesting and material selection, not proof of
arbitrary cavity assembly or a general Boolean. General all-face-pair coverage,
open-rim splitting, arbitrary singular/coincident branches, periodic seams and
poles, healing, full topology lineage, and actual bridge/WASM delivery remain
required by the original goal. Point membership alone cannot prove disjoint
shell boundaries or authorize their nesting.
