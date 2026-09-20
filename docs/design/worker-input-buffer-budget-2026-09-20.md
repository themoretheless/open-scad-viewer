# Worker Input Buffer Budget

G-code mesh admission previously counted only the visible vertex/index views,
omitting the transform and the remainder of each backing buffer. A tiny view
could therefore enqueue a clone much larger than the advertised 16 MiB limit.
SharedArrayBuffer views also violated the promised scene snapshot semantics.

The regression failed for all three mesh fields before the fix. Admission now
uses `fitsClonedBufferBudget`, shared with nominal lattice graph admission:

- Count whole distinct backing buffers, including the transform.
- Accept ordinary ArrayBuffer only; reject shared memory.
- Count aliased views once and accept the exact limit.
- Refuse before worker creation, posting, cancellation of active work or kernel
  warming. Retain the existing triangle and geometry validation boundaries.

An actual structuredClone test verifies that two tiny views on a 1 MiB buffer
produce one distinct cloned 1 MiB buffer. This is bounded-copy enforcement,
not a measured solver throughput improvement. Inputs above the limit are
refused rather than compacted with another main-thread allocation.

Validation: 27 focused tests passed, vue-tsc passed, Vite build and verify-dist
passed (95 artifacts, 15,743,458 total bytes). No Rust/WASM change. The full suite
was not rerun for this change; historical qualification identity failures are
not resolved by this input-admission fix.
