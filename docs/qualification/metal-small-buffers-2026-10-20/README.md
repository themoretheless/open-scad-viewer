# Small Metal updates, 2026-10-20

Retained: `ComputeRuntime::write_small_parts(array, element_offset, parts)` joins
contiguous typed slices into **one immediate queue write**, up to 4096 bytes total.
Packing uses a fixed stack array, with no temporary heap Vec or persistent pool.
A single nonempty part borrows the original data through `write` instead of
packing. All ownership, length and range checks precede queue mutation. There is
no submission, map, device poll or GPU wait in this method.

This is an explicit batching API. Existing individual writes retain their ordering
and visibility. No current production caller was silently converted to deferred
writes: the audited update paths already issue single writes, and no verified
sequence of adjacent writes to the same destination was found there. Applications
must choose a batch before their next submission. The benchmark exercises the
actual public API; results describe that workload, not a renderer/frame speedup.

## Measurements

Apple M4 Max, Metal, release workspace profile; same device, retained 4 KiB target,
same values and update ranges for both paths. Baseline: four `runtime.write`
calls for adjacent quarters; candidate: one `runtime.write_small_parts` with the
same quarters. 16/64/256/1024/4096 byte updates. Each case verifies a real copied
GPU readback before warm-up. Both paths use normal queue staging and keep the
same submission boundaries.

Three serial process runs. Each case/phase warms for 80 ms then records 24
counterbalanced ABBA/BAAB blocks. CPU samples average 128 updates with queue
submission/drain **outside** the timing; full samples average eight updates,
submitting/waiting after each complete update on both paths. No timed output or
verification. Thread QoS user-initiated for both paths. No own build/test ran
during these timings. Other projects were compiling/testing and desktop activity
continued; there was no exclusive GPU access or frequency/thermal control.
`environment.json` records the start conditions locally. AA controls expose the
remaining noise; no samples/runs were discarded or trimmed.

Median paired CPU speedup, ranges across the three process runs:

| Total update | Four writes -> one batch |
| --- | --- |
| 16 B | 4.36–4.45x |
| 64 B | 4.25–4.89x |
| 256 B | 4.73–5.05x |
| 1024 B | 3.28–3.31x |
| 4096 B | 2.69–3.22x |

All 15 per-run CPU bootstrap 95% intervals exclude 1. The full path is
**inconclusive**: median ratios 0.919–1.289, with intervals crossing 1 in several
cases, and AA full controls 0.955–1.267. This is a CPU preparation/staging-request
reduction, not a demonstrated GPU latency or frame-time improvement. `summary.csv`
contains every interval and absolute median. `analyze.py` computes log paired
median ratios and 4000 fixed-seed bootstrap resamples without filtering.

Rejected candidate: encode generated words into a 4 KiB stack array before
`queue.write_buffer`, replacing direct generation into `write_buffer_with`.
Across the final runs CPU median ratios were 0.812–1.115 and full 0.871–1.083;
there is no consistent win. Production `write_buffer_words` is unchanged. The
candidate remains only in the benchmark so the rejection is reproducible.

`exploratory-2/3.csv` used a simpler raw-byte batching model, not the new API;
excluded from final evidence. `exploratory-1.log` records a preflight failure:
the initial benchmark incorrectly tried to map a COPY_SRC target directly. It
was corrected to use the same snapshot readback as production before any final
runs. Final source was rustfmt-formatted after the measured build; formatting
only, no semantic edits. The measured binary/source hashes are in environment.json.

## Reuse and synchronization audit

- `ScratchPool::reserve` already reuses the same allocation and generation while
  capacity suffices. New tests prove same raw buffer/4096-byte retained budget.
- Prepared `ComputeProgram` retains its bindings and small uniform buffers across
  submissions; replay does not recreate them. No new global pool was introduced.
- Existing `write` and `write_generated` do not wait. Replacing them with CPU
  mapping could force waiting for in-flight readers and was not implemented.
- Readback snapshot storage stays independent while pending. The reuse regression
  test queues A, starts a snapshot, reuses the target for B without an intervening
  wait, consumes B first and verifies A is still intact. No staging slot is reused
  while a ticket owns it.
- A batch describes contiguous data only: no gaps filled with zero, no merging
  overlapping updates, and no movement across dispatch/submission boundaries.

Example: retain an allocation through ScratchPool/prepared bindings, then use
`runtime.write_small_parts(&array, 0, &[header, values])?` instead of two adjacent
writes before submission. For a single slice use the existing `write`.

## Reproduce

```sh
cargo build --manifest-path crates/Cargo.toml -p compute-core --example bench_small_buffers --release
# Run serially three times, directing stdout to separate CSV files:
crates/target/release/examples/bench_small_buffers > paired-N.csv
python3 docs/qualification/metal-small-buffers-2026-10-20/analyze.py
COMPUTE_REQUIRE_GPU=1 cargo test --manifest-path crates/Cargo.toml -p compute-core --test small_updates --test shared_upload -- --test-threads=1
```

The new tests cover the complete 4 KiB boundary, offsets, empty parts, foreign
ownership, oversized/out-of-bounds rejection before mutation, u32/f32 payload
bits, aliases, buffer reuse and outstanding snapshots. See focused-tests.log.

Final verification: 179 GPU/compute tests passed across 35 executables, zero failures; compute-core wasm32 check passed. Logs: native-tests.log and wasm-check.log.
