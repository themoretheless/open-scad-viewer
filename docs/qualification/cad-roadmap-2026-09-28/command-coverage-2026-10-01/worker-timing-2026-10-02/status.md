# Worker round-trip observation, 2026-10-02

Qualification-only browser instrumentation records real Worker postMessage synchronous duration, main-thread dispatch-to-message-event elapsed time, first request/worker age, and terminated pending jobs. It does not modify application runtime, protocol or kernel. Measurements are captured before reload resets page state; injected errors and held jobs do not fabricate native timing replies.

The existing full measurement scenario passes in native Chrome Canary 157 / Apple Metal, with strict console/page error gates, exact document, Undo/Redo and reload checks. This run dispatched native primitive/topology/snaps/display/edge and measurement work.

Observed first responses: primitive 104.7 ms, topology 108.4 ms, bodySnaps 112.9 ms, displayMesh 131.9 ms, vertex measurement 89.7 ms, bodyEdges 96.9 ms, curvature 93.6 ms. Later vertex measurement 14.7 ms; curvature 1.2 and 13.4 ms. Synchronous postMessage spans were 0..0.2 ms at the browser clock resolution.

These are one-run observations with small fixtures and concurrent startup work. They include worker bootstrap, execution and response delivery; they exclude client decode/validation, reactive publication, actual UI paint and isolated Rust-call durations. They are not stable percentiles or performance acceptance evidence. Further repetitions and end-to-end UI/Rust phase measurements remain required; no speed improvement is claimed. Full P0–P3 remains open.
