# Exact finite-chart projective transformations

`nurbs_core::editing::exact_projective::{curve,surface}` applies a row-major
4×4 homogeneous matrix to validated 3D rational geometry. This adds perspective
and general projective transformations to the existing affine editing tools.
The original degree, knots, periodic flags and complete parameter clock remain
unchanged, including nonuniform rational weights and multispan/tensor bases.

The matrix must have exact rank four. Every transformed homogeneous control
weight must have the same strict sign. Since B-spline basis functions are
nonnegative and form a partition on the admitted domain, this proves the
entire denominator stays away from zero. A mixed-sign control net is refused,
even if a more advanced denominator analysis might establish finiteness.
There is no claim that every finite projective image is admitted.

All arithmetic uses the shared bounded rational ledger: at most 1,000,000 work
units, 4096-bit rationals and the existing geometry bounds. A single rational
storage gauge covers the entire control net; separate row gauges would change
the tensor surface. Stored binary64 poles and weights are re-lifted and proved
proportional to the exact transformed homogeneous net with one global factor.
Linearity in the unchanged basis proves equality on every span, without sample
certification. Inexact storage, invalid output bounds, singular maps, unproved
denominators and exhausted work return no geometry. Invalid inputs are errors.

This is literal geometry construction. It transfers no B-rep incidence,
orientation, trimming, solid, external proof or source-epoch authority. It does
not promise bitwise equality of numerical evaluators. JSON/WASM/Rush/editor and
STEP exposure remain separate. The owner is the existing `editing` domain.
