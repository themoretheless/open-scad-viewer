# Display preparation cancellation — 2026-10-01

The shared curve/profile/surface/sketch preparation queue previously collected serialized geometry keys for every item synchronously before starting its first worker job. Cache availability checks at the end also scanned the full list without yielding.

Both loops now yield to the browser after an 8 ms slice and check the generation immediately afterward. Cancellation during key collection prevents worker dispatch. Exact geometry keys, response mutation checks and bounded cache behavior remain enforced.

Three focused tests passed, including cancellation during collection of 1000 items before any worker request. Vue typecheck passed. This is cancellation evidence, not an FPS or latency benchmark. One expensive key serialization can still exceed the slice budget, and rendering cache lookups remain synchronous. P0 responsiveness remains open.

Four tests now pass, including cancellation during final cache scan. Production build passed. The DirectModeler chunk is 377,171 bytes, requiring a measured 1,000-byte budget increment while retaining approximately the prior 847-byte headroom.

Production CPU browser surface regression passed: terminated held worker on Esc, display retry, surface picking, refinement, unchanged authored surface, Undo/Redo, reload and preview cancellation. Screenshot inspected. This run uses pointer controls and some keyboard actions; it does not qualify the full keyboard-only matrix.

A reproducible Node benchmark (scripts/bench-solid-display-collection.mjs) transpiles the actual queue source and prepares 1000 distinct 100-control-point curves with immediate mock worker results. Archived sample elapsed time 14.54 ms, maximum timer heartbeat gap 14.65 ms, 1000 jobs, no errors and 7,032,000 retained bytes. This is a single Node sample, not a browser rendering, memory or transport benchmark.
