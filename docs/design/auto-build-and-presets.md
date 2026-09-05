# Adaptive builds and parameter presets

## Automatic builds

`AutoBuildScheduler` owns one timer and keeps build scheduling separate from
Worker admission, cancellation and source-revision publication checks.

- Text editing uses a trailing debounce: initially 120 ms, then the smoothed
  Worker preview duration clamped to 60–450 ms. Observations weight the previous
  estimate by 0.6 and the new duration by 0.4. A full build seeds the estimate
  only before the first observation. Host queue/transport time is excluded.
- Range dragging uses a non-postponing preview cadence of 1.5 times that
  estimate, clamped to 100–750 ms. While another job is running, further drag
  edits are coalesced; they do not repeatedly discard a warm Worker. When the
  job settles, the latest edit becomes eligible for the next preview.
- Release/cancellation/window blur flushes the final dirty value. A reduced
  current preview promotes to full after the gesture ends. Keyboard range
  changes also flush their committed value. Equivalent previews still need
  only one build, using the existing promotion contract.
- Manual rendering clears pending automatic work. Turning Auto off or
  disposing the workspace clears the timer and deferred promotion. Existing
  coordinator cancellation and watchdog boundaries remain intact.

The timing limits are policy bounds, not guaranteed frame-rate targets.
Fake-clock tests cover continuous edits, heavy in-flight jobs, final flushing,
promotion, coalescing and disposal. In a live WebGPU smoke test, a sphere drag
with eight source updates produced three builds and no forced restart. A warm
comment-only cube edit reached frame submission in about 69 ms; the preceding
fixed-delay implementation had measured about 188 ms on that small-model
scenario. These are observations, not a controlled cross-device benchmark.

## Parameter presets

The Parameters dock supports named save, apply, deletion and one explicit
Undo Apply while the resulting source is still current. Saving/deleting a set
changes workspace metadata only and does not trigger geometry compilation.
Applying a set replaces all captured values in one source update.

Workspace schema 2 adds `parameterPresets`; schema 1 migrates to an empty list.
The existing IndexedDB CAS and recovery journal carry the list with the source.
Preset-only changes advance persistence mutation, not geometry revision, and
are included in snapshot equality and cross-tab conflict detection.

Limits: 20 sets, 128 uniquely named parameters per set, 80-character set names,
2,048-character string values and 64 KiB total serialized preset data. The
reader validates finite numbers, unique IDs/names and typed entries and returns
detached normalized objects. Unsupported/malformed schema-2 snapshots fail
validation rather than silently dropping their presets.

Each set includes a SHA-256 of the source with captured literal values replaced
by typed placeholders. Changing those values preserves compatibility; changing
other source bytes, including comments or whitespace, requires restoring that
template before applying the set. This deliberately conservative first version
does not guess how an old parameter maps to structurally changed code.

Presets live in the local workspace. SCAD downloads and shared links continue
to contain only the current source, as stated in the dock. Portable preset
bundles and cross-version rebinding are separate future work.
