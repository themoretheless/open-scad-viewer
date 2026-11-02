# Exact Full Round history scene edit API

`editExactFullRoundSceneMesh` restores native history, binds its proved current artifact to the existing proved snapshot, executes the native history operation, then builds replacement display buffers, BVH, semantic edges and an exact snapshot containing the full history document. The original source program remains attribution, not attestation of the edited body. The function publishes only after every step succeeds; input buffers are not mutated.

Two installed-WASM tests pass. They exercise actual source geometry edits, stale selection refusal, scene serialization, undo/redo, display-detail independence, mismatched valid-history rejection and transactional failure. TypeScript check passed. Existing snapshot geometry encoding remains unchanged; history is stored in an explicit document envelope. Unsupported multiple triangle-dependent provenance runs refuse rather than fabricate mapping.

This is a scene library API. It does not yet wire editor commands/stores, update aggregate scene metrics or implement a history-edit Worker request. Synchronous proof calls still require worker isolation for hard cancellation. It does not certify the attributed old program as the source of edited geometry. All broader original parity requirements remain open.
