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

## Native Measurement

Reproduce with a release build of the `bench_preview_writer` example, then run
`crates/target/release/examples/bench_preview_writer` independently of builds
and tests. It compares emit-String-then-FNV-hash with direct emit-to-FNV-writer.
Five warmup pairs precede 31 measured pairs with alternating order. Exact byte
equality is checked outside timing; each measured result verifies length/hash.

Two independent process runs on the development machine:

| Points | Output bytes | String median ms A/B | Writer median ms A/B | String allocated bytes | Writer allocated bytes |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 100 | 4,505 | 0.03225 / 0.03004 | 0.02208 / 0.02392 | 18,039 | 2,376 |
| 10,000 | 451,847 | 2.77417 / 2.73850 | 2.29454 / 2.25917 | 1,247,511 | 239,976 |
| 60,000 | 2,756,780 | 16.65813 / 16.66142 | 13.89092 / 13.80571 | 9,500,823 | 1,439,976 |

Allocation totals count requested alloc/realloc bytes, including replacement
capacities, not peak live memory or RSS. Timing includes an atomic allocation
counter and is specific to this instrumented native emit-and-hash workload.
The largest fixture saves about 85% of requested allocation bytes and 17% of
elapsed time here. No browser, filesystem, printer I/O or end-to-end speedup
is established. The existing String API remains available and unchanged.

### Counter-Free Timing Control

The benchmark now uses a compile-time allocation counter switch. Build with
`env -u OSV_WRITER_COUNT_ALLOCATIONS cargo build --release --locked --manifest-path crates/Cargo.toml -p gcode-core --example bench_preview_writer`
for timing, or set `OSV_WRITER_COUNT_ALLOCATIONS=1` on that build command for
allocation accounting. Run the resulting executable separately after each
build; changing the variable only when running it does not change instrumentation.
JSON declares its mode and reports allocation fields as null when disabled.

Two counter-free runs, same paired protocol:

| Points | String median ms A/B | Writer median ms A/B |
| ---: | ---: | ---: |
| 100 | 0.06479 / 0.03013 | 0.05442 / 0.02392 |
| 10,000 | 2.75171 / 2.72454 | 2.25121 / 2.20521 |
| 60,000 | 16.50596 / 16.62267 | 13.79079 / 13.64679 |

The large fixture retains a 16-18% elapsed-time reduction without allocation
counter increments. The 100-point absolute timings vary substantially; do not
treat its percentage as stable. A separate instrumented rebuild reproduced
the allocation byte totals above. These measurements do not establish browser
performance or filesystem throughput.

## Machine Job Follow-up

`emit_job_to` now exposes the same bounded destination path for machine jobs.
The String-returning `emit_job` and direct writer use one job emission body;
preview and job writers share budget/error handling. No firmware command,
rounding, heating, retraction or shutdown semantics are changed.

Native tests compare exact output for Marlin, Klipper and RepRapFirmware and
parse each result through the job parser. Additional tests cover invalid job
settings before output, writer failure after partial progress, and the exact
4 MiB append boundary. gcode-core/gcode-optimize tests pass. Browser WASM and
3MF callers remain on their existing API; preview benchmark percentages do
not establish performance of this job API.
