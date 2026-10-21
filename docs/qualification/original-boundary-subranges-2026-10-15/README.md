# Corresponding portions of original B-rep boundaries

## Native API

`watertight::inspect_boundary_subranges(models, addresses, nodes, max_cells)`
checks two independent free face boundaries over explicit partial ranges.
Each node retains four normalized fractions:

1. Source 3D edge, in the source coedge traversal.
2. Source face-local pcurve.
3. Target 3D edge, in the target coedge traversal.
4. Target face-local pcurve.

The source coordinates increase. Both target coordinates follow the opposite
shell traversal. The ranges need not cover the complete original boundaries.
No extraction, knot insertion, rounding of trimmed poles or reverse-knot copy
is used as proof of the original curves.

Three continuous checks share one work ledger: source edge/source lift, source
edge/target lift, target edge/target lift. The third check prevents a mismatching
target 3D edge from passing merely because its endpoints and pcurve agree.
Different edge/pcurve parameterizations retain different coordinates.

Admission requires all three checks. The returned outward distance bound is
between the original 3D edges along the supplied relation. This is tolerance
qualification; it is not exact equality. The original full-boundary API retains
its endpoint/complete-coverage requirement.

## Remaining work

This API checks an explicit relation. Automatic root isolation, selection among
multiple branches, uniqueness of roots, topological splitting, corner authoring,
sewing, embedding, thickness and solid admission remain separate requirements.
The report grants no topology mutation authority. WASM and UI admission are not
established by this change.

## Native results, 2026-10-15

All eight `watertight_correspondence` integration tests passed (five new and
three previous full-boundary tests). New cases retain fractional ends including
0.1, reject a bowed original target edge with unchanged endpoints, accept
independent nonlinear edge parameterization with a verified piecewise-affine
relation, retain original reversed curves on nonunit domains, and reject invalid
maps or exhausted work budgets. Inputs remain unchanged. This establishes the
native relation audit only; it does not complete the remaining work above.
