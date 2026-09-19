# Exact Solid worker: 2026-09-19

Follow-up: [cooperative receiving validation](solid-receive-2026-09-19.md) reduces
the spinner's observed main-thread gap while preserving the same checks. The
measurements below describe the original worker-transfer change.

## Change

`App.vue::buildSolidGroup` no longer loads the compiler or constructs NURBS bodies
on the main thread. `exactSolidClient` starts a disposable instance of the existing
geometry worker entry. That instance lazily loads `exactSolidRuntime`, evaluates
through the engine facade and builds the exact graph in the same realm.

The result is a versioned JSON Solid document, not kernel handles or a partially
executed graph. The receiving realm warms its geometry kernel and uses the existing
`parseDirectDocument` checks, including B-rep validity and mesh/B-rep bounds. The
display-build protocol is unchanged. Sharing the worker entry avoids another
separately bundled compiler. Exact construction is lazy so display workers do not
load its NURBS builder until requested.

Limits follow the existing Solid document/group contracts: 100,000 source
characters, 200 bodies, 64 MiB document characters. The host enforces a 120-second
deadline. An AbortSignal, worker crash, message error, transport error, malformed
result, success or failure closes the disposable realm. No warm realm or native
handles survive the operation. This trades repeated worker initialization for
simple ownership and hard cancellation of synchronous WASM.

The editor exposes cancellation, cancels on unmount, does not navigate away on
cancellation, and preserves edits made to a group during an earlier snapshot's
build. A newly opened group editor cancels the previous operation.

## Bundle

Normal Vite output before: **6,333,440 bytes**. After: **5,932,485 bytes**.
Reduction: **400,955 bytes (6.33%)**. No size budgets were raised. Packed kernel
identity and individual chunk checks pass; the total remains **332,485 bytes over
the 5,600,000-byte budget**. This is a delivery-byte measurement, not an estimate
from source-map attribution or a claim about compressed HTTP transfer size.

## Browser evidence

```sh
node_modules/.bin/vite build
CHROMIUM_EXECUTABLE='/Applications/Google Chrome Dev.app/Contents/MacOS/Google Chrome Dev' \
  node benchmarks/exact-solid-browser.mjs
```

The executable override is optional when Playwright's own Chromium is installed.
The script uses the repository's isolated Playwright package, serves the actual
dist over loopback, and closes browser and server on completion. It records results
under `tmp/performance/exact-solid-browser/`.

One local headless run, not a statistical before/after throughput benchmark:

| Source | Bodies | Wall time | Main-thread 16 ms timer callbacks | Largest callback gap |
| --- | ---: | ---: | ---: | ---: |
| Box | 1 | 251.6 ms | 12 | 65.7 ms |
| Two translated boxes | 2 | 140.0 ms | 8 | 18.3 ms |
| Planetary spinner | 20 | 9,937.4 ms | 540 | 1,295.3 ms |

Each request starts a new worker. The first also warms the receiving realm.
The test records exact geometry hashes after omitting random body ids. It verifies
named hull refusal, cancellation at approximately 102 ms, and a successful build
after cancellation. The full App route creates bodies, replaces a group's two
bodies instead of appending duplicates, and preserves scene and editor on cancel.
Desktop/mobile screenshots were inspected; the existing narrow-mobile workspace
clipping is not fixed by this change.

## Remaining hot path

This is **not hitch-free**. The spinner's final receive/validation stage still
blocks the main thread for about 1.30 seconds. Structured data transport, JSON
parsing/cloning, topology validation and subsequent history publication require
separate phase profiling. Do not remove validation merely to improve the timer.
The next design step is bounded, incrementally publishable body data and a single
owner for validated immutable geometry, rather than repeated whole-document
serialization. End-to-end wall-time improvement over the old route has not been
measured; moving CPU work off-thread is not equivalent to reducing that work.

Unit tests compare exact geometry for both source languages and cover refusal,
source admission, cancellation, timeout, crash, malformed document and transport
failure. Browser evidence is separate from those tests and from release qualification.

Full Vitest run: 3182 passed, 14 failed in 329 files. The 14 failure titles match
the preceding mesh-inspection run exactly (qualification fingerprints, obsolete
refusal expectations and MCP contracts). After the final malformed-request
admission guard, 62 targeted worker/protocol/coordinator tests passed. App and MCP
typechecks and `git diff --check` passed. The full suite is not claimed green.
