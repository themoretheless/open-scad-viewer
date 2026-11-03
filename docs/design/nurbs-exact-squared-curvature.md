# Exact squared-curvature fields

`editing::exact_derivative::squared_curvature(source,max_work)` constructs a
scalar-field carrier `[kappa²,0]` or `[kappa²,0,0]` over every original source
span, with original absolute clocks and independent one-sided knot traces.
This is geometric curvature squared, not acceleration squared or a spatial path.
It is invariant under parameter reversal/stretch and spatial rigid transforms;
uniform spatial scale k changes the value by 1/k².

For original homogeneous velocity V/W² and acceleration A/W³, the target is
`norm(V cross A)² W² / norm(V)^6`. In 2D the cross product is the determinant.
No intermediate rounded derivative curves or square root is used. Joint content
reduction and exact binary64 storage precede fresh full-polynomial identity
verification against the ORIGINAL uncancelled target.

Before reduction, every coefficient of the original squared-velocity numerator
in its full-span Bernstein basis must be strictly positive. This sufficient
condition proves original-source regularity, including endpoints. Stationary
points and unresolved mixed-sign coefficients refuse the entire result.
Cancellation must never turn an undefined source curvature into a valid field.
A regular curve with mixed-sign Bernstein coefficients can still be refused:
this is not a complete regularity or singularity classification algorithm.

The regularity polynomial conversion is capped at degree25, as is the final
stored field. This prevents large-degree binomial arithmetic from leaving the
existing conversion ceiling; it is a conservative admission restriction.
Shared1m-work/4096-bit, geometry and weight limits are unchanged. Every output
piece must be representable exactly; a late refusal discards the full cover.
No topology, global Frenet frame, source certificate, JSON/WASM/Rush/editor/STEP
or whole-application authority is supplied. Native qualification is in progress.
