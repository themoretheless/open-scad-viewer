# Active Solid cancellation and source replacement — 2026-10-04

Actual browser UI at localhost:5202, Rust/WASM computation, no mocked browser
workers or artificial delays. Initial scene: original `model` group, one B-rep
body, no selection. Test source was the compact affine + authored frame + guide
hollow quadratic profile used in `sweep-group-error-ui.md`.

| Scenario | Wide window | 720 x 900 |
| --- | --- | --- |
| Explicit active Solid cancellation | Pass | Pass |
| Source replacement during active Solid build | Pass | Pass |
| Subsequent successful Solid build | Pass, initial setup | Pass |
| Restore original scene through Undo | Pass | Pass |

For each cancellation/source replacement, the build button was observed as the
disabled `…` immediately before the action, rather than assuming a worker was
still running from a previous screenshot. Explicit Cancel closed the group
editor; source replacement re-enabled Build and preserved the edited source.
No test body appeared afterwards. Wide source replacement used invalid scale
[0,0]; narrow replacement used the valid source plus a new comment. The group
source was not rebuilt after those changes until the next explicit action.

Successful narrow construction was observed after the group editor closed;
hiding source showed the selected hollow B-rep in the actual Solid canvas.
Undo removed it. Resetting viewport showed scene count 1 and only original
`model`, with no selection. Temporary browser tab closed; main source untouched.

Screenshots:

- `sweep-group-active-cancel-wide.jpg`
- `sweep-group-active-cancel-narrow.jpg`
- `sweep-group-source-change-wide.jpg`
- `sweep-group-source-change-narrow.jpg`
- `sweep-group-solid-success-narrow.jpg`
- `sweep-group-restored-narrow.jpg`

Worker regression: `npx vitest run --maxWorkers=1 tests/exactSolidWorker.test.ts`
passed 12/12 in 1.76 seconds (`sweep-solid-cancel-worker.log`). It covers
termination, ignored late replies, cancellation during receiving-realm B-rep
validation, timeout, malformed responses, and retained swept cap holes.

An initial attempted cancellation setup completed normally before the next
tool call; it was undone and is not counted as cancellation. A later click on
the Solid scene's add-group control while the source drawer covered it did not
open an editor; the drawer was hidden before retrying. These failed setup
attempts are not treated as passes.

Remaining UI scope: cancellation/source replacement in the main progressive
viewport build path, main-source To Solid route, draft recovery after reload,
and geometric refusal classes beyond the scalar-scale example. These group
workflow results do not prove all modes or global geometric correctness.
