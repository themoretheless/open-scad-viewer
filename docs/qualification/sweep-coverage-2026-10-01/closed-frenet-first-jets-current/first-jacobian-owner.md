# First-Jacobian ownership for linear chart injectivity

The linear-monotonicity owner uses only the projected first derivatives Su and Sv, their interval intersection, and the positivity of the symmetric projected Jacobian on every charged rectangle. Its theorem has no curvature or third-derivative premise. Projection, exact station splitting, query domains, subdivision, work charges and matrix inequalities are unchanged.

The restricted and stable homogeneous routines visit derivatives in increasing total order. For total order zero or one, their control differences, Bernstein evaluation, convex-hull intersections, origin translation, weight bounds and directed interval arithmetic depend only on orders zero or one. The Cartesian quotient recurrence for order a+b references only lower total orders. Stopping after order one therefore leaves all returned first-order interval bits unchanged wherever the original full routine succeeds.

The new internal FirstJets type exposes only value/du/dv. Only surface_linear_monotonicity requests it. Existing full-order routines continue through order three for other owners, including retained smoothness. First-order numeric failures still refuse this owner; numeric failures exclusive to discarded higher derivatives do not become a G1/G2 claim.

Native regression checks 294 restricted/stable configurations across rational tensor degrees zero through six, translated controls, nonconstant positive weights, complete/interior/ULP-wide query domains. Every lower and upper first-order interval bit equals the full-order routine. Full NURBS1385 passed. Full BRep, bridge, exact original global report parity, paired CPU timing, final WASM/UI and source-bound delivery remain separate gates.
