# Instance history performance

Baseline: 1,000 linked instances plus source. Node commit/undo/redo each take approximately 0.85 seconds. Chromium baseline redo p50 is 526.93 ms, p95 549.85 ms and maximum RAF gap 250 ms. This browser run uses explicit GC between actions for retained memory diagnostics; it is not a normal-interaction performance comparison. Raw samples and CPU profile are retained.

Commit now reads Blender project identity from the compact snapshot when async restoration has not materialized geometry. It avoids resolving all old linked bodies solely for this metadata. Thirteen history tests and TypeScript passed, including preservation of identity and proof that old geometry remains unmaterialized during validation. No measured speedup or complete scene responsiveness claim is made yet. General scene performance remains open.
