# Continuously bounded surface differential-field storage

`editing::bounded_surface_differential::surface(source,field,max_error,max_work)`
constructs first partials, mixed partials of total order1..3, oriented non-unit
area normals or metric carriers `[E,F,G]` over every original fundamental UV cell.
Output patches retain absolute original U/V clocks and independent one-sided
traces; periodic input does not imply a stitched periodic output.

The exact surface APIs keep their original exact-storage-or-refusal contract.
This API first tries exact storage, then proposes rounded poles and ONE common
weight scale over the complete tensor net. Per-row or per-column gauges are
not allowed. Original targets, constant recognition and axis-content reduction
reuse the exact owner, followed by fresh full-polynomial cross checks against
the original uncancelled source target. General bivariate factor cancellation
and minimum-degree representations are not claimed.

Each actual stored field is reconstructed from its binary64 poles and weights.
For target `N/D` and stored `n/d`, tensor Bernstein residual hulls bound each
`N_i*d-n_i*D`. Strictly positive denominator coefficient minima give a closed
lower bound for `D*d`; the sum of squared component-error bounds proves a
Euclidean error enclosure on the ENTIRE cell, including its boundaries. The
shared bounded-curve outward error rounding avoids subnormal square-root loss.

The norm applies to the three numeric carrier coordinates. `[E,F,G]` are metric
coefficients with authored clock units, not spatial points or an intrinsic
coordinate-independent tensor norm. No sign, positive-definiteness, unit-normal,
regularity, knot smoothness, topology or evaluator-arithmetic certificate is
inferred from an approximate field. Near zeros, only the error enclosure is
promised. There is no adaptive fitting or degree reduction.

Output degrees25, geometry/weight/conditioning limits and shared1m-work/4096-bit
arithmetic limits are unchanged. Residual degrees are at most50 per axis. Invalid
inputs, mixed orders, tolerances and work budgets are errors. Any late refusal
discards ALL earlier patches and ALL earlier bounds. A conservative refusal
does not rule out alternate constructions. Native qualification is in progress;
JSON/WASM/Rush/editor/STEP and whole-application delivery remain separate.
