# Exact oriented area-normal surface fields

`nurbs_core::editing::exact_surface_derivative::area_normal(source,max_work)`
constructs the vector field `S_u × S_v` as rational surfaces over EVERY original
nonempty fundamental UV cell. It shares the exact source extraction, global
axis-content reduction, storage gauge and fresh stored-bit proof engine used by
the partial-field constructor. Original absolute U/V clocks, full cell coverage
and distinct one-sided traces remain intact for multispan, nonuniform, repeated,
unclamped and periodic source encodings.

This is an oriented non-unit area vector. Its direction follows the authored
parameter order; swapping U and V reverses the sign. Its magnitude is the local
area density, not1. Unit normalization is not performed. A zero vector field is
valid output for a degenerate source and does not certify source regularity,
embedding or a body-outward direction.

The homogeneous partial numerators are formed independently from actual
original control/weight/knot bits. Their signed cross product has denominator
W⁴. Constant fields and common U-only/V-only factors reduce before output
conversion. Each entire output net has one global storage gauge. The actual
stored output is proved against the complete ORIGINAL uncancelled cross-product
target, without transferring previous partial-field or topology certificates.

The existing1m-work/4096-bit ledger spans extraction, both partials, products,
cancellation, storage and proof across all cells. The existing degree25, weight,
conditioning and coordinate limits apply. Invalid input is an error; any work,
exact-storage or output-bound refusal returns no partial result, even if earlier
cells succeeded. General mixed-variable GCD/alternate basis search remains
separate. This native construction adds no JSON/WASM/Rush/editor/STEP or whole
application qualification and changes no installed assets or delivery profiles.
