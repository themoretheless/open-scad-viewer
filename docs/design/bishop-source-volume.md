# Original Bishop volume jets and normalized arc-length correspondence

This extends the native ideal-volume checkpoint `0807d77d4`. Public WASM, CI and semantic G1 delivery credit remain separate. All geometry, normalization, ODE differentiation, inverse phase, budgets and decisions are implemented in Rust.

## Original frame derivatives

For normalized original path parameter t, let v=P', a=P'', j=P'''. The owned original normal cover encloses the solution of

```
omega = (v cross a) / |v|²
omega' = (v cross j) / |v|² - 2 (v dot a) omega / |v|²
T' = omega cross T
T'' = omega' cross T + omega cross T'
N' = omega cross N
N'' = omega' cross N + omega cross N'
B = T cross N
```

Using the skew identities for T' and T'' preserves the cancellation of longitudinal acceleration. Differentiating a sampled reflection or a fitted normal is not used. The input normal enclosure belongs to the same immutable original path and parameter interval, and its transport is constructed once for the entire volume. Every normal-cover query, original derivative restriction and twist restriction spends the same declared ledger. Both first and second jets have normalized original parameter units, independent of the path's stored knot domain.

The semantic ODE reference is [Wang, Jüttler, Zheng and Liu, Computation of Rotation Minimizing Frames](https://www.microsoft.com/en-us/research/wp-content/uploads/2016/12/Computation-of-rotation-minimizing-frames.pdf). The production interval proof is independently implemented; numerical comparisons are validation only.

The raw open-transport jet API refuses incomplete covers, closed holonomy correction and insufficient query budgets. Those cases cannot inherit open-frame derivatives merely because their value reports exist.

## Certified inverse normalized arc length

The original positive-speed grid covers every closed parameter cell. Failed speed cells are subdivided under the same aggregate budget. Lengths start with whole-cell speed enclosures. Within a proved single original smooth span, a trapezoid bound with the outward speed-second-derivative remainder tightens the length; no C2 continuity across a knot is inferred from separate jets.

Original endpoint derivative restrictions, integrals, prefixes, suffixes and total length stay interval-valued. At a node the normalized phase p/(p+q) is bounded using its monotonic dependence on prefix p and suffix q. Inverse queries use monotonic node bounds and the positive interval mean speed to restrict all possible original parameters. This is not a fitted piecewise-linear inverse.

For exact total length L and normalized original speed w=|P_t|:

```
t_s  = L / w
t_ss = -L² w_t / w³
P_s  = L T
P_ss = L² T_t / w
```

The position identities are retained as correlated original-source facts. Multiplying independent boxes for v and L/|v| would unnecessarily lose cancellation. These position derivatives are never substituted for a relative vector or frame derivative.

Each guide has its own original inverse map. Exact source coefficient homothety may establish shared normalized phase; only then may an exactly subtracted original G-P curve preserve correlation. Different rational weights/domains otherwise retain separate maps. Contact fitting uses the owned original anchor and the original phased distance derivatives. Uniform scale, authored axes, center and twist remain functions of the declared normalized station coordinate.

## Whole-volume proof and subdivision

The existing single global projection and complete symmetric-Jacobian lower bound remain the admission condition. Source carriers that are literally identical share their original enclosure; `(1-s) C+s C` and `C-C` cannot invent a moving cap.

A native subdivision lookahead compares complete child bounds and caches the chosen pair. All attempted children, including discarded axes, spend the original budget. Both varying carriers retain the cheaper balanced strategy. Unsupported modes, unprocessed cells and resource exhaustion never supply partial embedding or topology credit.

## Current validation scope

- Independent cubic polynomial/rational Bernstein quotient jets and 8192-step RK4 compare original normal, tangent and binormal values, first derivatives and second derivatives at seven restrictions. Original path and twist domains differ.
- Independent exact length formulas for a nonuniformly parameterized line and a parabola check inverse first/second derivatives and correlated position jets, including endpoint and one-short work handling.
- The source-volume matrix covers rational weights, original-domain remapping, affine laws, parameter/arc stations, fixed/authored/fixed-normal/Frenet/planar/spatial RMF, shared-phase contact and independent curved guide phases. All successful cases retain one-short budget refusal and immutable source definitions.
- A constructor-owned six-station spatial body with non-dyadic station knots retains the exact existing model while gaining the original-source volume certificate. The public Rust JSON protocol verifies successful admission and refusal without emitting a model.

These are native qualifications of the listed modes under the sufficient global projection condition. They do not prove arbitrary closed source-volume embedding, arbitrary source/retained G1/G2 correspondence, or current public WASM delivery.

## Remaining full-goal work

1. Closed source frames need the corrected Bishop angular law, original arc-phase correction derivatives and exact periodic endpoint-law ownership. Raw open ODE jets must not be reused for holonomy-corrected normals.
2. Closed volumes and strongly turning open volumes need a finite local-chart cover plus exhaustive global inter-chart separation. Local determinant signs or independent per-cell projections alone cannot prove global injectivity. Wrapped seam charts require an exact quotient correspondence and one common projection on each compared local union.
3. Corrected-Frenet source phase/jet correspondence, general cap atlases, trimmed/multispan reversals and pole G1/G2 need their remaining native proofs. Sharp miter/cap joins remain C0.
4. Rebuild and qualify WASM, append successor immutable delivery records, and verify fresh CI/semantic G1/independent STEP/browser gates only after source geometry completion.
