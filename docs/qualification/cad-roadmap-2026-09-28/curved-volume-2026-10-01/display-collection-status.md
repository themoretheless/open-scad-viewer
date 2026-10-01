# Display preparation cancellation — 2026-10-01

The shared curve/profile/surface/sketch preparation queue previously collected serialized geometry keys for every item synchronously before starting its first worker job. Cache availability checks at the end also scanned the full list without yielding.

Both loops now yield to the browser after an 8 ms slice and check the generation immediately afterward. Cancellation during key collection prevents worker dispatch. Exact geometry keys, response mutation checks and bounded cache behavior remain enforced.

Three focused tests passed, including cancellation during collection of 1000 items before any worker request. Vue typecheck passed. This is cancellation evidence, not an FPS or latency benchmark. One expensive key serialization can still exceed the slice budget, and rendering cache lookups remain synchronous. P0 responsiveness remains open.

Four tests now pass, including cancellation during final cache scan. Production build passed. The DirectModeler chunk is 377,171 bytes, requiring a measured 1,000-byte budget increment while retaining approximately the prior 847-byte headroom.
