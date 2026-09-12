# Actual Curvex Boolean benchmark

Measured on the same machine using identical release examples and actual public
Curvex application operations. The original source revision is
`bf6967e4df618815e05eefc987947ca6b1c22461`; both builds use isolated copies.
The replacement binary uses the final migration staging `final-01` and the
corrected conservative intersection pruning. The final replay runs
original/replacement/replacement/original with seven calibrated samples per
workload per run (14 samples per engine). Reported values are median
microseconds per operation, including result destruction; fixture construction
and output summary extraction are excluded. Other team builds and benchmarks
were idle during this replay.

| Workload | Original µs | Replacement µs | Ratio |
| --- | ---: | ---: | ---: |
| `ellipse_union_2` | 51.50 | 63.00 | 1.22× |
| `ellipse_union_12` | 1329.67 | 744.36 | 0.56× |
| `rounded_rectangle_difference` | 49.89 | 37.48 | 0.75× |
| `rectangle_minus_50_ellipses` | 17092.26 | 8357.68 | 0.49× |
| `three_circle_arrangement_divide` | 746.37 | 940.63 | 1.26× |
| `compound_evenodd_normalization` | 49.08 | 84.03 | 1.71× |
| `cubic_self_intersection_normalization` | 15.94 | 18.49 | 1.16× |

The 50 ellipse cutters produce the same 51 contours, 200 cubic segments and
204 total segments in both engines. Other outputs can have different segment
counts because the solvers choose different split points; their complete
counts are retained. Timing and output counts do not themselves prove geometry
parity. The independent curve-preserving Boolean suite covers all four
operations, source-control retention, compound fill rules, self-crossings,
tangencies, nearly coincident and partially shared arcs, stationary cubics,
translated geometry at 1e7, and 5000-vertex input. The full native run passed
142 unit tests and 18 Curvex parity tests with no failures. The full legacy
application suite in migrated-02 passed 1490 tests with one existing ignored
test. Final application qualification with all newly added gradient integration
tests is recorded separately.

The initial implementation was materially slower (about 137 ms for the 50
cutters). Profiling exposed unnecessary chord generation for quarter-ellipse
pairs whose bounds meet although their interiors cannot intersect. Analytic
bounds and monotonicity now prune these pairs before flattening. Prepared
analytic winding queries also avoid repeated extrema discovery and curve
allocation. Intersection refinement and exact source subcurves remain in the
output. Remaining measured regressions are visible above: compound
normalization is 1.71×, while its absolute cost is 84 µs in this fixture.

An earlier optimized build failed Curvex's four-rectangle Divide test: a small
positive coordinate overlap was incorrectly treated as endpoint-only contact.
That build's measurements are retained as historical evidence, not as a
qualified result. The corrected predicate prunes only nonpositive overlap.
Regression tests now verify complete area and disjoint point-grid coverage
for all subset cells in four cyclic input orders, plus nearly vertical and
horizontal crossing edges with only 1e-9 coordinate motion. Both the new core
regressions and the original, unchanged Curvex test pass. The final table above
comes from a rebuilt binary containing this correction.

Evidence:

- [Final raw measurements and build/source hashes](boolean-final.json).
- [Compact comparison and evidence hashes](boolean-summary.json).
- [Historical initial measurements](boolean-before.json). Its recorded source
  stability flag is false, so it is diagnostic history, not frozen qualification.
- [Historical measurements before the crossing correction](boolean-before-crossing-fix.json).
- [Shared benchmark source](../boolean_benchmark.rs) and
  [build/replay runner](../run_boolean_benchmark.py).

The retained binaries and logs are under
`/private/tmp/osv-curvex-qualification/boolean-benchmark-fixed-build`; the final
idle replay is under
`/private/tmp/osv-curvex-qualification/boolean-benchmark-fixed-idle`.
The replay verifies binary and fixture hashes and preserves the original build
source hashes. At artifact creation, all recorded kernel hashes still match
the source tree. Hashes and complete raw samples are retained so these timings
cannot be mistaken for a build of subsequently changed code.
