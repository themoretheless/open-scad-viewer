# Physical cap G1/G2: remaining native proof obligations

This is a design frontier, not a completed certificate. The closed material
constructor has no physical end caps. Its filled reference section cannot be
counted as a qualified cap/wall join.

## Why the strict product-volume theorem is insufficient

At an end of a source product volume `F(u,v,t)`, a filled cap has tangent plane
`span(F_u,F_v)`. If its UV boundary is `(u(s),v(s))`, the wall tangents are

```
F_s = F_u u'(s) + F_v v'(s),  F_t.
```

At a regular G1 cap/wall join, `F_t` also belongs to the cap tangent plane.
Consequently `det(F_u,F_v,F_t)=0` at that join. The current original-volume
theorem requires nonsingular one-sided 3D charts on the whole closed product
domain, so it cannot admit this smooth join. This is a limitation of that
parameterization/theorem; it does not establish that the ideal physical body
itself is singular or intersecting.

Allowing zero determinant at the edge without a replacement proof would lose
regularity and global injection guarantees. A sample-normal match, denser
sampling, larger tolerance or retained valid BRep does not replace that proof.

## Next native implementation boundary

1. Own the original wall and physical-cap charts, complete trim correspondence,
   source seam orientation and all source laws in one native boundary atlas.
2. Construct an exact or outward-enclosed collar along each actual UV trim.
   Use shared boundary coordinates to compare wall and cap first/second jets.
   Natural strip joins are a first case; arbitrary curved trims need original
   curve/surface composition and derivative correspondence, not a fitted strip.
3. Prove local regularity and injection in the joined boundary chart, including
   cap/wall and vertex neighborhoods. Pole identifications need their own
   quotient chart; rectangular Jacobian checks alone cannot qualify them.
4. Cover all distinct boundary-parameter pairs, retaining seam equivalence only
   after exact ownership. Nonadjacent separation and local joined-chart
   injection must exhaust the whole boundary under one explicit work ledger.
5. Bind the closed oriented embedded boundary to material/shell topology and
   the ideal volume. The original strict 3D interior atlas may remain a useful
   obligation away from cap collars, but grants no omitted-collar credit.
6. For an actual constructed Solid, independently bind coefficients and domains,
   inspect every edge/vertex, and qualify topology, volume and native STEP
   roundtrip. Source smoothness and actual smoothness remain separate proofs.

Sharp cap/miter joins stay C0. A strict G1/G2 request must refuse them. Mixed
smooth/sharp claims need native provenance of the sharp set and complete
classification; a caller-selected ignored-edge list is insufficient.
