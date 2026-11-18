# Exact original-clock metric coefficient fields

`nurbs_core::editing::exact_surface_derivative::metric(source,max_work)` builds
the first fundamental coefficients as carrier coordinates `[E,F,G]`, with
`E=S_u·S_u`, `F=S_u·S_v`, `G=S_v·S_v`. These are three scalar fields, not spatial
positions or normalized tangent vectors. F is the dot product, not2F; the line
element is `E du² + 2F du dv + G dv²`.

Every original nonempty fundamental UV cell is represented, including general
multispan/nonuniform/repeated/unclamped and periodic source encodings. Output
patches are clamped/nonperiodic, retain absolute original UV clocks and own
one-sided traces. Transposing parameters swaps E/G and preserves F; reversing
one parameter reverses the cross coefficient. Singular and zero metrics are
valid fields, not positive-definite/regularity or topology certificates.

The shared Rust engine extracts actual original homogeneous source bits,
constructs both partial numerators without intermediate binary64 surfaces,
forms the three Gram numerators over W⁴ and applies constant/global axis-content
reduction. Actual stored output is proved against the ORIGINAL uncancelled
Gram targets. One storage gauge spans each whole output net. The same1m-work,
4096-bit, degree25 and geometry/weight/conditioning limits apply to every cell;
a late work/storage/bound refusal discards the entire result. Invalid input is
an error. General mixed-variable GCD/alternate basis admission remains separate.

Independent qualification additionally checks `EG-F²=|S_u×S_v|²` as a full
polynomial identity from the original source. Numerical dot products alone do
not establish that identity or regularity. Existing pointwise differential APIs
remain supported. This native construction adds no JSON/WASM/Rush/editor/STEP,
source/topology authority transfer or current-source application qualification.
