# G0 v60 / G1 v77 re-freeze review

This successor binds the generalized native periodic profile seams, the independent complete-surface regularity/embedding diagnostics and the tighter open spatial Bishop-frame envelope. The branch also incorporates main commit 7330a433b7f28ef5b35e654cc379cb2af051d96c; source fingerprints must describe that combined source tree rather than the former branch-only App.vue.

## Native claim boundary

The new native tests cover exact periodic knot/control identities, radial and height projection proofs with both orientations, a regular two-cover refusal, exhausted work and one-ulp knot tampering. Closing seams at additional station counts are built through native periodic collocation; the quadratic three-interval case has G1 rather than G2. The spatial frame bound applies only to open Bishop transport, and excludes closing correction. All interval arithmetic includes outward rounding.

The geometry diagnostic proves only one complete untrimmed surface. It does not certify a Solid, inter-face contacts, cap interiors, shell containment or orientation. The matched-parameter deviation certificate remains a separate predicate. No diagnostic may silently promote a surface to a globally certified shell.

Local nurbs-core validation: 692 unit tests, two integration tests and one doctest passed. Rebuilt WASM and host-boundary checks are recorded separately after materialization; this review does not infer their success from native tests.

## Qualification boundary

G1 v76 run 37641272884 completed with 129 successful working fragments and three failed WebKit memory fragments. Their endpoint RSS drifts were 175525888, 155308032 and 200757248 bytes against 67108864; slopes were 516624.9691428571, 381042.0720601504 and 399779.33184962405 bytes/job against 131072. These are failures, not partial approval.

This re-freeze imports no pass from that run. The original 50 warmup / 500 measured jobs, sampling every 25 jobs, memory budgets, three fresh runs, fresh Worker realms and prohibition on forced browser GC remain unchanged. A new exact-source full matrix is required after addressing the memory failure. No G0/G1 approval, production cutover or independent human signoff is claimed.

## Observational CI diagnostic

The optional memory_diagnostic_only workflow input skips normal matrix preparation and aggregation. It runs the unchanged frozen WebKit memory supervisor once and records an additional bounded, read-only process RSS trace. The trace inspects only descendants of its own supervisor owner, omits command arguments and is capped at 240 reads. This job contributes zero qualification units even if its single observed budget passes. The default complete matrix remains unchanged.
