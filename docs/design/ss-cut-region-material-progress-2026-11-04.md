# Whole-region cells and original NURBS Boolean material

Source base: `28d4ceb97`, whose tree equals main `acfe7c27a`.
The complete general SS / trim / material body / Boolean objective remains open.
This checkpoint supplies proof for entire finite UV cells, not complete output
assembly or global material/embedding admission of a subdivision candidate.

Follow-up checkpoints: `ss-interior-jordan-progress-2026-11-04.md`,
`ss-interior-census-progress-2026-11-04.md`, and
`ss-census-arrangement-progress-2026-11-04.md` add admitted regular interior
contours, exhaustive pair-census ownership and combined closed-contour topology.
The general requirements below remain open, including positive multi-component
qualification and global mixed-solid admission.

## Implemented

- `ClosedCutCandidate::classify_rectangle` classifies the whole UV rectangle
  against both finite pieces. It combines the original retained face domain
  with interval-homotopy winding of the exact SS contour. Inherited holes are
  outside both pieces. Root/rim boundary bands and missing proof stay unresolved.
- Reports retain their original body, source carrier, contour, rectangle and
  budgets; replay reconstructs source geometry, orientation and membership.
  Failed winding does not report a measured zero cost: segment count is None.
- `qualify_surface_material_box` encloses every point of an original rational
  NURBS image over a UV box. The complete enclosure is connected to a binary
  parity witness through a boundary-free Cartesian ball. A witness midpoint
  alone never establishes uniform material.
- Exact original periodic seam identity binds periodic source enclosures;
  the original flags and knot clocks are retained. Source span work is bounded.
- The common separation kernel first uses original surface distance bounds.
  When an untrimmed support meets the witness ball, a complete trim-aware
  exclusion forest visits original knot patches. A leaf is discharged only by
  certified Cartesian ball separation or by proved finite-domain Outside.
  Unknown cells, unvisited roots and work exhaustion cannot authorize material.
  Geometry and trim-domain work are shared across every relevant body face.
- `qualify_boolean_face_box` derives union/intersection/difference/XOR selection
  and material-normal reversal for every source point in an admitted finite
  face box. Both operand roles are explicit.
- `ClosedCutCandidate::qualify_boolean_rectangle` binds that material proof to
  one proved finite inside/outside piece and its literal mixed-BRep face index.
  This does not classify an entire connected piece from one chosen cell.
- Point-seed classification uses the shared enclosure-separation kernel while
  retaining its prior untrimmed sufficient path and existing public budgets.

## Evidence

Final native tests cover whole-cell membership, inherited holes, reversed
contours, unresolved crossing boxes and work limits; rational source boxes;
material void through an actual finite hole; all four Boolean operations and
subtraction orientation; and full source-solid/healing/volume regressions.

The integration example uses TWO independently continuously admitted bodies:
a planar box and a box transformed by an exact dyadic quadratic graph warp.
The warped body's original curved face is checked equal to the actual second
SS support. Its exact edge lifts, simple trims, all face contacts, embedding,
nesting and orientation are admitted before material classification. Additional
per-pair work is explicitly allocated for its overlapping control hulls.
Selections cover both cut pieces, both paired UV carriers and the second-operand
subtraction normal. No sampled body, fitted SS curve or fixture recognizer is
substituted for these native proofs.

## Still required

1. Jordan/physical embedding and complete arrangements of ALL original branches,
   including arbitrary singular nodes, irrational strata and coincident regions.
2. Open cut paths meeting rational rims, exact endpoint/edge splitting, periodic
   homology and collapsed-pole UV bridges.
3. Extend these per-cell decisions to every retained connected region using a
   complete arrangement/coverage proof; sew the whole selected mixed BRep.
4. Admit mixed implicit geometry as a complete closed material body, with
   independent volume checks and full topology lineage for Boolean outputs.
5. Native bridge/WASM and actual host/Worker delivery of the completed pipeline.

`material_classified()` and `geometry_admitted()` remain false for the WHOLE
candidate. The isolated verified cells do not change those global verdicts.
