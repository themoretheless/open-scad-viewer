# Reuse of complete metric proof domains

Base: `2809101d2`.

A complete regularity/injectivity proof for a single convex source hull H also
proves every contained convex box. Reuse is confined to one immutable source
and projection inside one native atlas invocation. No serialized report escapes,
no union of different metrics is formed, and failed proposals never authorize
geometry. Every attempted containment comparison spends the same cell ledger,
including unsuccessful comparisons and cache-only visits. Exact-key cache
semantics remain intact; a prior failed proposal cannot override a larger
complete positive proof. Multi-piece wrapped domains are not reused this way.

The regression obtains a real native interval proof for F=(u-u^2,v,t) on the
left quarter. A contained box reuses it without new source queries, charging
one unit; missing that unit refuses. The surrounding cube includes a genuine
fold and cannot borrow the local proof. Thus domain containment is essential.

Native evidence (nightly-2026-09-16, locked sources, test opt-level=1, test/dev
debug info disabled, two build/test threads):

- Cargo `-p nurbs-core --lib complete_metric_domain_reuse -- --nocapture`: **1 passed**.
- Same freshly built NURBS unit executable, no filter, `--test-threads=2`:
  **2227 passed / 0 failed / 1 ignored**, 70.47 seconds.
- Cargo `-p brep-core --lib rational_hollow_cap_quarter_turn_frame -- --nocapture`:
  **0 passed / 1 failed**, 157.79 seconds. Actual boundary equality passes at
  241560 work; actual filled volume still exhausts 99800 source cells inside
  the unchanged 100000 aggregate budget. No Solid is admitted.

This packet qualifies proof-domain reuse, not the still OPEN hard actual
rational-cap volume or the full original loft/sweep/G1/G2 objective. WASM,
browser, STEP and CI were not run. No TypeScript geometry changed.
Next investigate the paired-volume point-coordinate inverse proposal: it
currently absorbs source derivative lengths as well as their directions.
