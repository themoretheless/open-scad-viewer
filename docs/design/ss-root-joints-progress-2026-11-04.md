# Original SS local joint admission

`OriginalSsRootEdge::qualify_successor` searches bounded dyadic subarc
restrictions near a literal shared selector. Both complete subarcs must belong
to one sealed affine original-root graph. The shared point must lie strictly
inside its graph domain, and the two outer selectors must have strictly opposite
exact clock ordering about it. Per-query native limits and at most 2*(depth+1)
receiver attempts bound the search; max depth is 24. Exhaustion is unresolved.

The certificate owns both original edges and the successful witness choices;
re-verification repeats edge admission, whole-subarc enclosures, graph membership
and ordering. Phase shifts and nonlinear blowdown require separate admission.
`OriginalSsRootWire::qualify_joints` requires this at every joint, atomically.
This is local no-retracing evidence, not a global Jordan or material certificate.

Remaining: bind both independently admitted injective parent faces and retained
interior placement; justify global component/covering ownership; use original
unit winding to exclude repeated covering. One-sided knot chart unions,
singular/coincident strata, seams/poles, full finite arrangements and admitted
mixed-body Boolean/volume remain part of the original objective.
