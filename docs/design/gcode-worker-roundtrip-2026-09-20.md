# G-code worker roundtrip baseline

Run `node --import tsx benchmarks/gcode-worker-roundtrip.mts` from the repository root.
This measures the actual GcodePreviewWorker client and worker entrypoint through
the existing Node worker_threads harness, against executeGcodePreviewAsync in
the calling process. It is not a browser rendering or cold-start benchmark.

Baseline: production source c8e2a65e, Node v22.23.2, macOS arm64.
Geometry WASM SHA-256:
`909b94a4b895db447a184bcc6cfb544a226b51589b94b124424da6670eb0b8ca`.
No builds or tests ran alongside either campaign.

Each fresh-process campaign uses one reused worker, 10 warmup pairs and 31
alternating direct/worker measurement pairs per fixture. Full initial documents
must match; every timed result has its move count, estimated time and final move
checked outside the timer. Public and generated artifact hashes must match the
generated identity. Input and result hashes are emitted with all samples.

| Extruding moves | Direct A/B p50, ms | Worker A/B p50, ms |
| --- | --- | --- |
| 100 | 0.0778 / 0.0788 | 0.1688 / 0.1688 |
| 10,000 | 4.3073 / 4.2953 | 8.7467 / 8.7500 |
| 80,000 | 36.0862 / 36.6541 | 76.2486 / 75.9940 |

Fixtures also contain one initial positioning move. Input and complete result
hashes matched across both campaigns. The largest result's JSON representation
is 9,058,898 bytes; this is not a measurement of structured-clone wire size.

The roughly 39-40 ms large-case difference motivates inspecting worker transport
after the packed WASM boundary improvement. It includes client validation,
message delivery, structured cloning and separate-isolate execution effects;
it does not isolate cloning time or prove that removing the worker improves UI
responsiveness. The worker still protects the main thread from native parsing.

Next experiment: transfer owned numeric move buffers while retaining the public
document API, request identity, bounded validation, cancellation and worker reuse.
Keep that change only after paired measurements and real browser verification.

Implemented and measured in [transferable worker results](gcode-worker-transfer-2026-09-20.md).
