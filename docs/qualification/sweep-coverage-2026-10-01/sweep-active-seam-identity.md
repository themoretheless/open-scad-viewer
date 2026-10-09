# Exact active seam identity without Bernstein materialization

For a validated degree-p rational curve with n poles, let a=U[p], b=U[n],
s=n-p and T=b-a. The native predicate accepts only when:

- neighboring knots strictly straddle both endpoints;
- T and each checked knot translation are exact binary64 operations;
- U[j]+T=U[j+s] for every j in 0..2p;
- P[j]=P[j+s] and w[j]=w[j+s] for every j in 0..p-1.

At a, only basis functions 0..p-1 can contribute. At b, only functions
s..s+p-1 can contribute. The strict neighboring knots exclude ambiguous
repeated-knot endpoint conventions and make the omitted endpoint basis zero.
Each contributing basis has the same complete knot stencil after translation
by T. The Cox-de Boor recurrence therefore gives N[j,p](a)=N[j+s,p](b)
in exact real arithmetic. Matching original poles and positive weights make
both rational numerators and denominators identical. The active endpoint
positions coincide even if that common position is not representable as a
binary64 Bernstein pole.

The predicate proves only this equality. It does not require or trust the
periodic flag and does not prove smoothness, simplicity, nesting, cap material
coverage, wall injectivity or global embedding. It is used only for a
single-curve loop; joins between distinct curves retain their separate exact
endpoint audit. Exhausted or unsuccessful alternatives remain refused.

Implementation: `crates/nurbs-core/src/numerics/exact_curve_segments.rs`,
`translated_active_seam_closed`; ownership consumer:
`crates/nurbs-core/src/sweeps/progressive_miter/level_certificate.rs`.
Native regressions include a nonrepresentable common midpoint, degrees 1–6,
unequal positive rational weights, altered seam poles and weights, and a
1-ULP exterior knot mutation. Numerical endpoint evaluations are falsification
checks; the basis-stencil identity is the proof.
