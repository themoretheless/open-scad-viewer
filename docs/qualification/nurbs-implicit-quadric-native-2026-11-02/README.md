# Original implicit quadric public contract

This qualifies existing native `analysis::implicit_quadric` as one catalogue capability. It does not count each example equation as a separate feature. The exact authored homogeneous equation and continuous nonzero-gradient cover are distinct from sampled primitive fitting and from parameter-surface/B-rep validity.

Both default and no-codec runs passed 101 public tests across fourteen targets, including six new quadric contracts. Source hashes bind the implementation, native predicate, knot-removal correction and tests. The current default full library was separately checked at that same runtime source in the knot-removal receipt: 2044 passed, zero failures, one existing ignored microbenchmark. New integration contracts do not alter the library implementation.

Coverage includes nonuniform rational weights, cylinders/ellipsoids, graph quadrics, literal original radius-product retention, independent analytic point equations, exact/cell budget boundaries, one-ULP source mutations, cone singularity, collapsed parameter maps, unsupported multispan storage and invalid inputs. A collapsed patch can have a regular ambient supporting plane; that is deliberately not proof of rank, coverage, injectivity, pole quotient, shell incidence or solid validity.

Existing degree, coordinate, predicate-work and interval-cell limits remain unchanged. No JSON, WASM, Rush, editor, STEP or main delivery qualification is claimed. See `docs/design/nurbs-implicit-quadrics.md` for the full admission and refusal contract.
