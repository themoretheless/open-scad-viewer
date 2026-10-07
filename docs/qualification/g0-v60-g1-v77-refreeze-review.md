# G0 v60 / G1 v77 re-freeze review

This successor binds the generalized native periodic profile seams, the independent complete-surface regularity/embedding diagnostics and the tighter open spatial Bishop-frame envelope. The branch also incorporates main commit 7330a433b7f28ef5b35e654cc379cb2af051d96c; source fingerprints must describe that combined source tree rather than the former branch-only App.vue.

## Native claim boundary

The new native tests cover exact periodic knot/control identities, radial and height projection proofs with both orientations, a regular two-cover refusal, exhausted work and one-ulp knot tampering. Closing seams at additional station counts are built through native periodic collocation; the quadratic three-interval case has G1 rather than G2. The spatial frame bound applies only to open Bishop transport, and excludes closing correction. All interval arithmetic includes outward rounding.

The geometry diagnostic proves only one complete untrimmed surface. It does not certify a Solid, inter-face contacts, cap interiors, shell containment or orientation. The matched-parameter deviation certificate remains a separate predicate. No diagnostic may silently promote a surface to a globally certified shell.

Local nurbs-core validation after the exact-trace change: 694 unit tests, two integration tests and one doctest passed. Rebuilt WASM and host-boundary checks are recorded separately after materialization; this review does not infer their success from native tests.

Native B-rep curve reversal now reverses the knot vector together with control points and rational weights. Two regressions cover nonuniform rational parameter reversal (including the derivative sign) and a capped retained-wall loft with a nonuniform multi-span knot vector. All 29 analytic B-rep tests passed. The subsequent complete B-rep unit suite passed 931 tests, with three existing ignored tests (two manual performance measurements and one curved-boundary embedding roadmap gate); none failed. All 101 integration tests passed in that earlier full-suite execution. The later exact-trace change passed 23 curve/surface identity tests, 79 B-rep boundary tests, 11 volume/orientation tests (one existing roadmap gate ignored), and a joint rational multi-span loft regression. The regression proves exact boundary identity, embedding, nesting and orientation, preserves the source model and refuses exhausted exact-work budget. This correction is native; JavaScript does not repair or approximate the geometry.

Exact natural multi-span NURBS boundary identity compares the complete original basis, Euclidean controls and rational weights before producing a source-bound equality decision. It is limited to supported clamped normalized natural traces; periodic, nonmatching and rounded-complement representations remain unresolved. It does not use approximate knot insertion. The independent finite STEP observations archived under profile-sweep-continuous-2026-10-07 use v43 and verify six/20 faces, one valid Solid, jets and retained volume through OCCT re-export. Final successor-binary observations are recorded separately.

## Immutable module delivery candidate

The qualification host verifies and compiles the released geometry bytes once. Every job still creates a fresh Worker, a separate WASM instance, its own memory and its own Rust handle arena. A private qualification-only channel transfers the immutable compiled module. Bootstrap failure is fatal; the qualification Worker cannot fall back to recompiling the byte payload. The normal product retains verified compilation and rejects this bootstrap API. A private Vite resolver applies the same module-only Worker dependency in both built and development qualification servers. Both qualification producers now use the bounded built bundle.

Local tests verify distinct memories and isolated Rust CAD handles and reject bootstrap in the normal application context. The direct Chromium producer passed 24 browser boundary checks. These are development observations, not a Linux-contained complete qualification or proof that the WebKit memory failure is fixed. The memory and job-count budgets remain unchanged.

An additional Linux ARM64/Node 24 development observation using the module-only dev Worker still failed: slope 339066.0176842105 bytes/job and endpoint drift 157405184 bytes. It is zero-unit diagnostic evidence on a dirty source snapshot, not a frozen hosted environment. A bounded separate RSS observer saw growth in WPENetworkProcess; this alone does not identify its cause.

The successor memory lane serves the same inspected, bounded and content-hashed qualification build as the actual browser lane, including a dedicated memory HTML entry. The previous lane used Vite development modules. The cube result checks, exact-loopback HTTP/WebSocket isolation, blocked service workers, fresh Workers, separate native instances, post-termination sampling point, all budgets and 50/500 job counts are preserved. The memory record includes the delivered artifact inventory. No browser GC or cache policy is overridden to obtain a pass; full fresh hosted qualification is still required.

One additional development observation with that built delivery passed unchanged budgets: slope 30339.225984962406 bytes/job, endpoint drift 24133632 bytes. The seven-artifact build was 4503921 bytes. Both failed and passed diagnostic records are archived in profile-sweep-continuous-2026-10-07/g1-memory-delivery-diagnostic. This remains Linux ARM64/Node 24 dirty-source evidence, not a frozen hosted qualification result, and uses the preceding own-Rust v42 binary before the B-rep reversal rebuild.

The same local built WebKit delivery passed all 24 actual-browser checks through the Linux supervisor, including 3 native default Workers and 11 fault Workers. All 37 browser memory/actual/supervisor regressions passed. These observations remain zero imported qualification units; source and runtime identity must be frozen before the new complete matrix.

## Qualification boundary

G1 v76 run 37641272884 completed with 129 successful working fragments and three failed WebKit memory fragments. Their endpoint RSS drifts were 175525888, 155308032 and 200757248 bytes against 67108864; slopes were 516624.9691428571, 381042.0720601504 and 399779.33184962405 bytes/job against 131072. These are failures, not partial approval.

This re-freeze imports no pass from that run. The original 50 warmup / 500 measured jobs, sampling every 25 jobs, memory budgets, three fresh runs, fresh Worker realms and prohibition on forced browser GC remain unchanged. A new exact-source full matrix is required after addressing the memory failure. No G0/G1 approval, production cutover or independent human signoff is claimed.

## Observational CI diagnostic

The optional memory_diagnostic_only workflow input skips normal matrix preparation and aggregation. It runs the unchanged frozen WebKit memory supervisor once and records an additional bounded, read-only process RSS trace. The trace inspects only descendants of its own supervisor owner, omits command arguments and is capped at 240 reads. This job contributes zero qualification units even if its single observed budget passes. The default complete matrix remains unchanged.


Final development artifact own-Rust v44 is 12,181,014 bytes, SHA-256 8a50b676698a403b00fe8bbb601f4674f6e004f8a5a0b21c77a74dc9e0b04ed3. Fifteen native-module/worker/sweep protocol tests, 37 browser harness/supervisor regressions and Vue type checking passed. Product build and distribution verification passed unchanged limits: 152 artifacts, 8,336,190 asset bytes. Both finite STEP fixtures were repeated successfully against this binary; the six-face nonuniform rational-wall native joint audit now reports complete boundary, face-pair, nesting and orientation evidence. These results remain finite development observations.

A fresh local built WebKit observation against v44 passed unchanged 50/500-job memory budgets: slope 14164.522345864663 bytes/job and endpoint drift 13246464 bytes. Actual browser checks passed for local Chromium and Linux-contained WebKit. The local Linux environment remains ARM64/Node 24 with sourceSha null and zero imported qualification units. Full fresh hosted G1 matrix evidence is still required.
