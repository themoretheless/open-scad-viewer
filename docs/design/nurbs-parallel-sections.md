# Parallel plane sections in Rust

`nurbs_core::parallel_sections::intersect` sections an original rational surface
by the equations `normal dot S(u,v) = level`. The shared normal is not
renormalized: level values use those exact supplied coefficients. The API
accepts 1..1024 levels, positive spatial tolerance and 1..1000000 shared work
units. Coordinates and normal components are bounded by 1e6; levels by 1e12.
The natural surface chart is used; authored trims are not applied.

Each level returns one of three outcomes:

- `Empty`: an outward enclosure of the complete original surface excludes the
  plane. No seed search or sampled mesh proves absence.
- `Approximate`: every root in the complete natural chart belongs to one
  continuously covered, piecewise regular parameter graph. The actual stored
  degree-one NURBS curves have a continuous spatial error bound within the
  requested tolerance. The report gives the solved axis and error bound.
- `Unresolved`: the complete original parameter domain remains an explicit
  obligation. No incomplete set of segments is presented as a complete section.

Admission proves source C0 continuity across internal knots, one-sided surface
regularity and a consistently nonzero plane-residual derivative on every
original knot rectangle. Strict opposite signs at the two solved-axis boundaries
prove existence of exactly one interior root for every free parameter. Either
parameter direction may provide the graph; negative derivative orientation and
nonunit normals are supported. This proves the complete graph in parameter
space, not ownership of a trimmed B-rep edge or global surface injectivity.

Interval mean-value/interval-Newton contraction encloses the true root bands.
Original rational spatial enclosures then bound the distance between every
true section point and the retained chord at its corresponding free parameter.
The calculation includes arithmetic rounding and actual stored chord poles.
Endpoint values are shared through a cache; retained curves have at most 256
controls each and share exact endpoints between chunks. There is no claim that
a future floating-point evaluation routine is exact.

Work counts original spatial-enclosure and jet queries, including discarded
parents, root contractions and endpoint construction. Source preparation is
reported separately and reused between levels. On exhaustion, no partial fit
is admitted; later levels may still be proved empty by the already computed
whole-image enclosure. Source surfaces are unchanged.

Tangencies, coincident patches, nonmonotone sections, charts whose full-image
regularity bounds are inconclusive, partial graphs entering another domain
boundary, periodic seam ownership and arbitrary multi-component topology are
not admitted by this graph constructor. They remain explicitly unresolved;
this API does not complete the general surface/surface intersection contract.
WASM, editor and STEP integration are separate qualifications.
