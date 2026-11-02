# Exact affine cylindrical and conical sides

`nurbs_core::primitives::exact_frusta::side(center, axes, [r0,r1], max_work)`
returns the complete side of an affine cylinder, conical frustum or cone:

`center + axis0*r(v)*cos(u) + axis1*r(v)*sin(u) + axis2*v`,
where `r(v)=(1-v)*r0+v*r1` and `v` is the original linear height parameter.
These are one qualified construction family, not separately counted wrappers.
The older cylinder/cone/frustum constructors remain available unchanged.

The three finite authored axes must be exactly independent. The two finite
radii must be nonnegative and not both zero. Equal positive radii give a
cylinder; one zero end gives a cone apex. Oblique and reflected frames are
supported. The surface uses a degree2×1, 9×2 positive-weight tensor, domain
`[0,1]²`, clamped nonperiodic storage and an exactly repeated longitude seam.
The angular clock is rational and variable, not the old trigonometric clock.

Every proposed affine pole sum must round-trip exactly through binary64.
Nonrepresentable sums, overflow, dependent axes or proof exhaustion return
an explicit refusal with `surface=None`. No rounded or partial fallback is
returned. One-ULP differences between representable end radii are retained.

The actual basis, dimensions, positive weights and every homogeneous pole are
verified again. Exact source-frame Cramer projections bind the whole tensor
to one global gauge of the freshly proved circle times the linear radial and
height profiles. Thus all four tensor cells satisfy
`X²+Y²=[r0*D+(r1-r0)*Z]²`, and the original full UV parameterization is preserved.
Quadric membership alone is insufficient: changing the weights of one complete
end column retains the geometric quadric but changes the height clock. Both
native proof and independent oracle reject that change.

Because the interior radius is strictly positive, height recovers `v` uniquely
and the circle direction recovers longitude modulo its one seam. This proves
full injective side coverage. At a zero-radius boundary every longitude maps
exactly to the same endpoint, recorded by `Report.collapsed_end=Some(Start/End)`.
On refusals this metadata is `None`. A cone apex is intrinsically nonsmooth;
it must not inherit smooth pole/G2 support from the ellipsoid constructor.
No cap, B-rep vertex fan, oriented shell, solid, volume or numeric normal-error
bound is supplied. Tiny/near-dependent frames can still make floating-point
normal or derivative evaluation ill-conditioned.
The guarantees describe the freshly returned literal surface. Editing or
reweighting it requires new qualification; the endpoint metadata is not a
transferable source-bound certificate.

One shared ledger covers fresh circle proof, construction and literal output
proof. The existing `1..=1_000_000` work range and 4096-bit rational guard remain.
Shared bounded dot/cross arithmetic now returns the core arithmetic error type,
without a dependency on conic-construction refusal types; arithmetic bodies and
charges remain unchanged. This ownership improvement adds no capability count.

Qualification: [receipt](../qualification/nurbs-exact-frusta-2026-11-03/receipt.json).
Native Rust only; JSON/WASM/Rush/editor/STEP and current-source distribution
remain separate qualification work.
