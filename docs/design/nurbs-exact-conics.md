# Exact rational-weight affine ellipses

`nurbs_core::primitives::exact_conics::ellipse(center, axis_u, axis_v, max_work)`
constructs the whole locus `center + axis_u*cos(theta) + axis_v*sin(theta)`.
The two finite 3D axes may be oblique but must be exactly independent.
This extends the older trigonometric primitives with checked binary-rational
storage and a full exact geometric proof; the older APIs remain available.

The output has degree2, nine poles, four quadratic pieces, domain `[0,1]`,
clamped closed nonperiodic storage, and weights `[1,1,2,1,1,1,2,1,1]`.
Each quadrant has its own rational clock. Angular speed is variable; the API
does not preserve the parameter clock of the older trigonometric constructors.
It does not promise bitwise identity of floating-point evaluator results.

Every pole sum is computed as an exact rational from the original binary64
inputs and must round-trip exactly through binary64. A rounded approximation
is never silently substituted. In particular ordinary decimal translations
and axes can produce `NotRepresentable` even when an approximate conic would
be visually indistinguishable. Callers needing approximate storage should
choose an approximate constructor and keep its accuracy contract separate.

The returned literal curve is proved again: actual basis and dimensions,
positive weights, source-plane identity, exact Gram projection into the
authored two-axis frame, and coefficientwise `A²+B²=D²` on every piece.
Positive angular derivative numerator and canonical quadrant controls prove
the complete oriented locus, rather than isolated points. The Gram determinant
is computed exactly, so nearly parallel independent axes need no epsilon test.

`Report.curve` is `None` on every refusal; `work_used` includes construction
and proof on one ledger. Invalid finite-input/budget contracts are errors;
dependent axes, nonrepresentable storage, resource exhaustion or failed proof
are explicit refusals. Existing `max_work` range `1..=1_000_000` and the
4096-bit arithmetic guard are unchanged. Underflow/overflow is not a proof.

Bounded rational arithmetic now lives in `core::bounded_rational`; editing
and construction use the same implementation without a construction-to-editing
dependency. Moving this owner is not counted as another capability.

Qualification: [receipt](../qualification/nurbs-exact-conics-2026-11-03/receipt.json).
Native Rust only. JSON/WASM/Rush/editor/STEP and distribution qualification
remain separate; no old topology or artifact certificate transfers to this API.
