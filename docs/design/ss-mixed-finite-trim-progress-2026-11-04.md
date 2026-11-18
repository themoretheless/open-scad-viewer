# Mixed original boundaries and implicit finite-face cuts

Base: `e30eb4581`. The full general SS / trim / closed body / Boolean objective
remains active. This checkpoint is native candidate construction and evidence;
no candidate here is a fully qualified material Boolean result.

## Native implementation

- `mixed_root_model::MixedBrep` stores original rational world/pcurves together
  with exact implicit SS edges, root vertices and source UV projections.
  Rational edges remain their original definitions. Root edges cannot be
  substituted with rational pcurves or approximate endpoint vertices. World
  geometry is three-dimensional and pcurves have two UV coordinates.
- Conversion of a qualified original solid preserves rational edge definitions,
  curve trims, face surfaces, source loops, shell uses and bodies.
- `split_closed_contour_candidate` inserts a closed original SS contour into a
  real finite face. Every complete source arc must be proved inside the retained
  face domain. Shared placement work covers all edges and adaptive subintervals;
  outside, boundary contact and exhaustion cannot publish a partial split.
- The existing outer rim is preserved. New inside and outside face pieces share
  literal implicit world edges in opposite loop directions. The source face's
  shell normal reversal is retained for both pieces.
- Existing holes are assigned to the appropriate piece. Whole contour placement
  proves disjointness from original rims; an original rational hole endpoint is
  enclosed conservatively and classified by winding over that ENTIRE query box.
  A rounded endpoint is not used as nesting authority. Every old loop remains
  owned exactly once after the split, including transferred holes.
- Original SS winding now supports closed UV query boxes. Convex arc homotopies
  exclude the whole box, so every point in it has the returned winding. Midpoint
  witnesses are clamped inside their boxes, including subnormal singleton data.
- Candidate construction is atomic with respect to the original solid. Both
  `material_classified()` and `geometry_admitted()` remain false. No conversion
  to legacy rational-Curve geometry or `QualifiedClosedSolid` is supplied.

## Remaining obligations

1. Prove original contour Jordan embedding/simplicity, its physical ownership
   and the complete arrangement of ALL relevant SS branches. A unit winding
   at a seed is only an orientation witness, not a Jordan/material proof.
2. Cut paths that actually meet natural rational rims: exact curve/SS endpoint
   binding, edge splitting and mixed loops. Periodic homology and collapsed-pole
   UV bridges remain required. This interior cut API rejects unresolved phase
   and pole/node joins.
3. Extend native retained-domain, face-contact and body admission to mixed root
   geometry; source storage/topological closure do not establish volume validity.
4. Propagate independently qualified material through the complete arrangement,
   choose retained sectors, sew complete bodies and qualify general Boolean
   outputs and their volume. Both sectors are currently preserved.
5. Complete singular real-root stratification and physical branch census, then
   bridge/WASM delivery and actual runtime qualification.

## Evidence

The mixed-cut suite exercises preservation of a finite cuboid rim, exact shared
SS edges, transfer of an enclosed hole while another hole remains outside,
rejection at a concave outer rim and an intersecting hole, and atomic work
exhaustion. Root-model and winding/periodic-path regressions are rerun. The
qualification directory records final logs, architecture audit and source hashes.
Finite fixtures validate the generic mechanisms, not universal completion.

## Whole-cell material follow-up

See [cut region material](ss-cut-region-material-progress-2026-11-04.md) for
whole-UV-box membership and material, trim-aware hole separation, and actual
intersecting-body tests. Whole-candidate geometry/material admission is open.
