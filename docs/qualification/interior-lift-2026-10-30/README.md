# Compact C2 interior material corrections

The point-constrained central graph experiments did not preserve the original
material. This follow-up changes stored poles locally rather than refitting
all six boundary Hermite fields. It is candidate construction, not a solid
admission path, and is not connected to document commands or WASM/UI.

`region_replacement::propose_interior_lift` checks every requested pole's entire
closed tensor B-spline support rectangle against the actual trimmed face,
including holes. Only a fresh `Inside` classification authorizes a correction.
The support must also lie strictly inside the natural parameter domain. All
internal knots must retain at least C2 continuity. A midpoint or a previous
surface receipt cannot authorize an edit.

Positive weights and coordinatewise nonnegative stored control changes imply
coordinatewise nondecrease of the mathematical rational surface. The compact
basis correction and its first two derivatives vanish outside its support.
Since the certified support misses every trimmed boundary, boundary jets are
unchanged in exact spline algebra. Floating-point evaluation, whole seams,
upper material bounds, injectivity and whole-body geometry still require fresh
checks; this function grants no such receipt.

The source model is immutable. At most 64 changes, 4096 poles, degree 16 and a
shared one-million-cell UV budget are supported. Periodic supports, insufficient
continuity, boundary-touching basis supports, negative changes, overflow and
unresolved membership refuse without returning a partial surface.

The offline trihedral binary fixture has a shared loader for its regression
and diagnostic export. Its rounded XY reconstruction is a new candidate and
inherits no G2 or material certificate. `OSV_CENTRAL_GRAPH_FIXTURE_EXPORT`
exports it from `central_graph_diagnostics` for reproducible inspection without
the removed exploratory JSON artifacts. `interior_lift_diagnostics` checks
active basis supports at an explicit witness and saves a proposal only if a
complete support passes. Its output always remains unadmitted.

## Current evidence

Five focused Rust tests pass: compact nonnegative correction with unchanged
boundary values and available first/second derivatives; atomic refusals;
whole-support hole rejection despite an Inside midpoint; non-C2 rejection;
and independently calculated rational-weight lift magnitude. Both diagnostic
examples compile. This is not a full kernel, WASM, worker or UI qualification.

The shared binary fixture was exported, then freshly grafted. Native output
again has 13 faces, 33 edges and 22 vertices, WithinTolerance closure and all
trim regions valid. Native material containment remains Outside. An Inside
witness at UV (0.9430759112081446, 0.4836167378919158) has a world Y enclosure
approximately [-1.0395565321418e-5, -1.0395565320925e-5] mm, outside the unchanged
-1e-7 mm allowance.

All 100 active pole supports were audited. None was certified Inside: 81
classifications report JoinBand and 19 BoundaryBand, rather than WorkLimit.
Those are unresolved classifications, not proofs that all rectangles lie
outside. No local correction was saved or applied.

A smaller closed UV rectangle around the same point *is* freshly certified
Inside in 16 cells: U [0.9420759112081446, 0.9440759112081446],
V [0.4826167378919158, 0.4846167378919158]. A degree-9 basis with support
contained in this particular box needs at least 11 additional knots on each
axis; the resulting whole tensor chart would have at least 69x69=4761 poles,
above this construction path's 4096-pole budget. This is a bound for that
chosen box, not a proof that every other refinement is impossible. A local
subpatch/atlas representation is the next construction path; simply raising
one of the existing global poles or accepting a midpoint would change the
boundary or bypass ownership checks.

[summary.json](summary.json) binds the source/fixture hashes, native witness,
current proof scope and support audit. No material or closed-solid admission
is claimed; no live document has been replaced.
