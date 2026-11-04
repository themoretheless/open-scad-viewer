# Complete exact scalar candidate roots on a closed interval

`QualifiedSsAlgebraicRootCensus::isolate` returns every distinct real root of a
nonzero rational polynomial on a supplied CLOSED rational interval. It retains
the original normalized polynomial and returns ordinary exact clock owners in
root order. Repeated roots remain in that original polynomial but are counted
once as scalar root locations. A nonzero constant has an empty census; the zero
polynomial is refused because it does not define a finite root set.

Endpoint roots are evaluated exactly. Interior root counts use exact one-sided
Sturm variations: the first nonzero Taylor derivative determines each chain
polynomial's sign at x+ or x-. No numerical epsilon is used. Breadth-first exact
bisection partitions the open interval, separately retaining exact midpoint
roots and checking that child counts plus the midpoint root equal the parent
count. Nonpoint clock isolators have nonroot endpoints and exactly one root;
exact point clocks are emitted only after polynomial-zero proof. Global count
agreement prevents an incomplete list from becoming a certificate.

There is one explicit rational-operation/bit/degree allowance for the whole
census, plus cell and root-count limits. Defaults are 1,000,000 retained
operations, 16,384 processed cells and 64 roots. Exhaustion returns an error,
never partial roots. Clocks share an immutable exact Sturm chain. Reported
census work is global; a clock's stored historical operation count is the work
spent when it was emitted. Replay reconstructs the normalized proof and root
isolators; redundant input trailing zeros can legitimately change historical
input-work counts without changing the mathematical proof.

`QualifiedSsThirdSurfaceCandidates::isolate_candidate_clocks` connects this to
third-surface elimination. It proves the complete scalar candidate list on the
requested interval. It still does not prove real bounded third-UV membership,
finite trimmed-face membership, retained material or physical branch completeness.

Qualification includes repeated endpoint/interior roots, multiplicity 64 at an
endpoint, irrational repeated roots together with exact midpoint roots, two
clocks closer than a binary64 ULP, constant/empty/point domains, invalid inputs,
insufficient cells/roots/work and normalized replay. Actual third-surface
candidate constraints produce both expected roots or a proved empty list.

Next work remains finite third-UV/face membership and cross-source junctions,
complete geometric arrangements and winding, full trim/material/Boolean/healing,
lineage/history, bridge/WASM execution and the wider quad/offset/fillet/G2 scope.
