# Exact original carrier rank-loss witnesses

For normalized source coordinates, let H(u,v,t)=(1-t)C0(u,v)+t C1(u,v).
The original sweep pose is affine in the source point: F=P(t)+A(t)(H-P0).
Thus F_u=A H_u and F_v=A H_v. If all three exact minors of [H_u,H_v]
vanish, the volume parameter map cannot have rank three, for any supported pose.
This is a negative parameter-map witness; it does not assert that the geometric
image cannot admit another regular parameterization. A filled-carrier witness
also does not establish a collision inside a particular material trim region.

The implementation uses unchanged binary64 source data lifted to bounded
rationals, homogeneous one-sided de Boor jets, the Cartesian quotient rule,
and each cap's own raw-to-normalized parameter widths before blending.
An already proved exact identity of normalized cap maps permits reusing one
source representative, while retaining the separate identity-proof ledger.
At original internal knots, all incident Cartesian position and first jets
must agree exactly. Ambiguous jets decline the witness.

A floating filter only proposes exact queries at the UV midpoint and traversal
midpoint/endpoints. A miss is not evidence of regularity or injectivity: the
existing complete positive atlas remains responsible for all positive flags.
The optional query uses the same aggregate cell/exact-work ledger; exhaustion
never grants a certificate. Periodic carriers are currently declined here.

Native regressions cover unequal raw domains, a false cancellation in raw
parameter units, collinear nonzero columns, a one-ULP nonzero minor, unequal
rational multispan weights, ambiguous one-sided jets, unchanged inputs, and
exact/one-less/zero resource replay. Qualification results are recorded only
after terminal execution; WASM, STEP and CI are not qualified by these tests.
