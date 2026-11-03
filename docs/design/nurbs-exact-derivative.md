# Exact original-clock derivative curves

`nurbs_core::editing::exact_derivative::curve` builds dC/du as a complete
sequence of rational curves on the source knot spans. It supports validated
2D/3D, multispan, unclamped and periodic input. Each output piece is clamped
and nonperiodic, retains its original absolute parameter interval and owns
its one-sided endpoint traces. At a source knot the two traces can differ;
no continuous derivative or periodic seam smoothness is fabricated.

Exact polynomial de Boor extraction uses original stored coordinates, weights
and knots. It does not treat floating trimming or a rounded homogeneous lift as
source history. On each span, u=a+(b-a)t, the numerator is
(A_t W - A W_t)/(b-a) and the denominator W². Exact common factors are cancelled
before testing the actual output degree. Thus polynomial and constant fields
can fit even for degree25 source curves; some genuinely higher-degree rational
fields must refuse the unchanged output degree25 limit.

One bounded rational work ledger covers extraction, products, factor reduction,
storage and the final full polynomial cross-product proof of actual stored
binary64 output. Poles and weights must be exactly representable and satisfy
existing geometry bounds. The existing 4096-bit and 1,000,000-work guards remain.
A valid request that exhausts work, cannot store exact output, cannot establish
positive output weights or exceeds degree/range bounds returns no pieces at all.
Invalid source definitions and invalid work budgets are input errors.

This constructs vector-field geometry. It does not replace the pointwise
curve differential API, transfer old source/B-rep/solid certificates or promise
bitwise numerical evaluator identity. JSON/WASM, Rush, editor and exchange
integration are separate. The existing `editing` root owns the implementation.
