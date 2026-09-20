# Bounded Preview Writer

Selectively adapts the `emit_to` capability from gcode-core-fdm without adopting
its old emitter, machine profile, dialect or interpreter. The public native
API now accepts a `std::fmt::Write` destination. The existing `emit` String API
and the new writer share one emission body, including plan validation and
quantized extrusion checks. Job and 3MF APIs remain unchanged.

The writer counts bytes successfully written per call and refuses any append
that would exceed 4 MiB. Destination errors return GCODE_WRITE, independently
of GCODE_OUTPUT_LIMIT. Errors can leave a prefix in the destination: callers
must discard it, not use it as a completed G-code document. The API does not
flush or provide transactional rollback, printer I/O or print readiness.

Tests compare exact bytes against the existing String API, including append
semantics, prove invalid input leaves a destination untouched, distinguish
writer failure and assert the output limit on a 90,000-point path. All native
gcode-core/gcode-optimize tests pass.

This is a native API port, not a browser integration or measured speedup.
It avoids requiring a full output String when the native caller uses a
non-buffering writer; formatting still has small per-segment allocations.
The checked-in browser WASM was not rebuilt and does not expose this API.
The broader branch integration remains open.
