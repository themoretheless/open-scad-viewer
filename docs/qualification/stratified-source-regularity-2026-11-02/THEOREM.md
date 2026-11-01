# Whole original face rank with physical C2 pole neighborhoods

`source_volume::regularity::stratified::qualify` receives an immutable native
`C2PoleAtlas`, not a body and an independently supplied report. The atlas retains
its `Boundary`, which retains the admitted original `source_volume::Body`.
Consequently all material regions, embedded source geometry, complete original
incidence and physical pole neighborhoods refer to one privately admitted body.

Each original face is visited. An ordinary face requires a fresh
`surface_regularity::inspect` proof of a nonzero one-sided normal on the closure
of every original knot rectangle. This covers its complete chart and therefore
every point of its original material trim region. Internal knot smoothness is
not inferred from these spanwise rank proofs.

A polar face must have exactly one declared collapsed side. The original
linear UV restriction must cover the complete natural side, including both
original parameter endpoints. The declared vertex, point and incident face must
belong to the supplied C2 atlas. An exact parameter exchange makes the collapsed
axis U; it permutes original binary inputs and performs no fitting.

The fresh full-face polar predicate writes `S-P=u D/W` and constructs
`F=f.(S-P)` and `G=(g.D)/(h.D)`. Its bounds prove positive denominators and
`F_u >= a > 0`, `G_v >= b > 0`, `|F_v| <= e`, `|G_u| <= c`, with
`a*b-e*c > 0` throughout the original rectangle. For every `u>0`, G is also
`g.(S-P)/h.(S-P)`, a smooth function of the original world point since h.D and
u are positive. Thus the chain rule gives
`rank D(F,G) <= rank DS`; the positive determinant establishes rank two of DS
on the entire punctured face, including its noncollapsed boundaries. Reflection
of an end-1 collar and affine raw-parameter normalization preserve rank.

This argument intentionally does not apply at `u=0`, where the original
parameters collapse. There the same body's private C2 pole atlas supplies a
regular physical graph disk with positive projected radius, complete owned fan
coverage, exact meridian G2 identities and common uniform gradient/Hessian limits.
It does not claim regularity of the collapsed original parameters.

Every face must succeed before a `StratifiedBody` is constructed. All face rank
queries share one cell allowance; all visited atlas memberships, declarations
and face records share one incidence allowance. Work cannot restart per face.
An unresolved predicate, unsupported layout, exhausted allowance or incomplete
pole membership yields no token. Two collapsed sides on one face require a
different complete chart cover and are refused here.

The resulting guarantee is an embedded material boundary with complete
piecewise face rank and physical C2 pole neighborhoods. Ordinary seams retain
their declared C0/G1/G2 classes. This is not whole-boundary C1/C2 at ordinary
seams, ordinary vertices or internal knots, nor an ideal loft/sweep law-to-shell
identity. The general-frame, all-cap and closed-seam objective remains open
until these additional original-source obligations are connected and qualified.

Native validation passed on frozen sources at `b9e6c68df`: one targeted
parabolic closed-body test and the complete `brep-core --lib` suite, with
2064 passed, 0 failed and 5 ignored. The 2878 recorded source/build-input files
remained unchanged. Ignored tests provide no evidence. Full command, counts and
source hashes are in `result.json`, `source-before.json`, `source-after.json`;
complete output is in `brep-full.log.gz`. Focused/full coverage overlaps.
No WASM, STEP, browser, remote CI or full-objective completion is inferred.
