# Exact original-clock unnormalized binormal fields

`editing::exact_derivative::binormal(source,max_work)` constructs `C′ × C″`
on every original nonempty knot span of a validated spatial (3D) curve.
This is a vector-field carrier, not a spatial path or unit Frenet frame.
The construction uses original homogeneous source polynomials: velocity has
W² denominator and acceleration W³, so the original cross-product target has
W⁵ denominator. No rounded intermediate derivative curve is used.

Joint polynomial reduction, one piece-wide storage gauge and fresh polynomial
identity proof against the original uncancelled target use actual stored
binary64 bits. Physical original clocks and separate one-sided knot traces
are retained. Reversing the clock reverses the field; stretching by k scales
it by 1/k³. Zero fields, stationary points and inflections are valid outputs.
No regularity, unit orientation, curvature, topology or old certificate transfer.

The existing shared work, rational-bit, output-degree, geometry and weight
limits remain unchanged. Invalid source, non-3D input and invalid budgets are
errors. Any exact-storage, bound or work refusal discards the entire result.
Native qualification passed; complete regression evidence is recorded in `docs/qualification/nurbs-exact-differential-regression-2026-11-03/receipt.json`. Application delivery remains separate.
