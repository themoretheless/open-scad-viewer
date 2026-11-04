# Candidate clocks from a third original NURBS surface

`qualify_ss_third_surface_candidates` constructs six exact original rational
surface equations for S1=S2=S3, using the existing SS ideal and a supplied third
surface Cox span. The third surface is polynomialized from its original poles,
weights and knots. All three positive-weight denominators are localized with an
auxiliary inverse variable before seven-variable rational Groebner elimination.
This removes denominator-zero complex artifacts without losing real points in
the supplied closed span patches.

The output owns the original SS ideal, third surface, third span, requested source
axis, exact univariate consequence and query policy. Replay regenerates the
source equations and elimination proof. `OriginalSsRootCurve::third_surface_candidates`
provides this for original deflated/pole span graphs and their native free axis.
Candidate clocks are exact selected roots of that consequence polynomial.

These constraints are necessary, not sufficient. Eliminated complex parameters
or real third parameters outside the finite patch can produce candidate roots.
They do not authorize a trim vertex, third-UV membership or retained material.
A nonzero constant consequence does prove disjointness on the original spans;
absence of a univariate consequence, including a coincident surface, is refused
rather than labeled disjoint. The SS branch's actual parameter domain and the
third face's finite trim still need their own checks.

Source polynomialization and elimination each have the supplied algebra budget;
the reported work sums those stages. Pair/basis bounds and source-variable order
come from `ExactSsGroebnerLimits`; third parameters and the inverse variable are
eliminated before the requested coordinate. Candidate degree is capped at 64,
consistent with the existing exact scalar-clock owner. This is bounded exact
algebra, not complete global triple-surface root isolation.

Qualification includes an actual circle SS pair cut by a third patch, x/y
candidate constraints and exact clock selection, a third patch with nonconstant
positive rational weights, replay, a constant disjointness proof, coincident
surface refusal, invalid spans/weights/axes and exhausted work. A patch entirely
at y>=2 deliberately yields the same eliminated roots as the near patch: the
regression preserves the boundary between candidate algebra and finite geometry.

Next required work is complete candidate root isolation and actual third-UV/
finite-face membership, followed by cross-source junctions, complete arrangements,
trim/material selection and intersecting Boolean/healing. Persistent lineage/
history, actual bridge/WASM calls and wider quad/offset/fillet/G2 requirements
remain in the full goal.
