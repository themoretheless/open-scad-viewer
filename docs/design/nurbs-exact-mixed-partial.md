# Exact original-clock mixed partial fields

`nurbs_core::editing::exact_surface_derivative::mixed_partial(source,[a,b],max_work)`
constructs `d^(a+b)S / du^a dv^b` as an ordered collection of rational vector-field
patches over every original nonempty fundamental UV cell. Total order must be
1, 2 or 3, matching the existing rational jet ceiling. This single capability
covers first derivatives, the Hessian fields `S_uu,S_uv,S_vv`, and third partials;
individual orders and directions are not separate catalogue entries.

The native algorithm extracts the original homogeneous tensor polynomials and
applies `d(N/W^k) = ((dN)W-kN(dW))/W^(k+1)`, using physical original span widths
at every differentiation. No intermediate rounded derivative surface or finite
difference is used. For rational sources, a derivative can remain nonzero above
the polynomial source degree. Transposing U/V swaps the requested orders, without
permuting the three spatial vector components.

Outputs are clamped and nonperiodic, in U-major/V-minor cell order, including
nonuniform, repeated, unclamped and periodic source storage. Absolute original
UV clocks and each cell's one-sided traces are preserved. This does not assert
global differentiability across repeated knots or periodic seams.

All operations share the existing exact-field work ledger and unchanged limits:
1,000,000 work units, 4096 rational bits, output degree25, coordinate/weight and
conditioning bounds. Constant and globally shared axis-content reduction may
make an output admissible, with one whole-net storage gauge. Actual stored bits
are checked against the original uncancelled target over the whole cell. A late
cell failure discards the entire result. Invalid source, order or budget is an
error; exhausted resources, bounds or unrepresentable storage produce an atomic
refusal. Admission does not exhaust all equivalent representations or general
mixed-variable GCDs.

The independent checker reconstructs the original source by exact homogeneous
de Boor interpolation, then uses generic numerator/denominator quotient rules
that square the current denominator at each step, independently of the native
power-of-original-W recurrence. It proves full polynomial identities against
actual serialized output. Pointwise comparisons supplement these identities.

This is native Rust construction. JSON/WASM/Rush/editor/STEP, regularity,
topology and current-source application delivery remain separate qualifications.
Existing pointwise higher-jet and first-partial APIs remain available.
