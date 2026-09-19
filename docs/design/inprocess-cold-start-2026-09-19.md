# In-process provider initialization

The in-process `HeadlessGeometryService` previously put its first kernel warmup
inside the 250 ms readiness check. A controlled injected provider taking 350 ms
reproduced `GeometryEngineUnavailableError`, matching the CI contention evidence.

The service now invokes `GeometryBuildEngine.initializeSource` before building
when no host runtime is injected. This explicit initialization uses a 5,000 ms
deadline and the selected registered provider, not a hard-coded global kernel.
Injected disposable-worker runtimes retain ownership of their startup lifecycle.
Normal `capabilities`, `manifest` and `buildSource` readiness remains 250 ms.

Admission checks are shared by initialization/build through `admitSource`.
Revocations, missing capabilities and provider absence are checked before warming;
admission is checked again after waiting. Initialization remains within the
existing pending-job limit, execution-error attachment and total duration timing.

Warmup is single-flight per provider. Initialization may await the original
attempt after a shorter capabilities probe timed out, without starting another
warmup. Once initialization itself times out, later requests fail immediately
until the original attempt settles. A concurrent short probe cannot overwrite
that longer-timeout state and reopen the deadline.

Abort cancels only the caller's wait; it removes its event listener and timer,
does not build that caller's source, and leaves shared warmup available to other
callers. It does not interrupt synchronous JS/WASM or stop native compilation.
These are asynchronous waiting bounds, not a hard in-process execution limit.

## Verification

Six controlled tests cover slow startup, reuse after short-probe timeout,
concurrent cancellation, hung initialization/late rejection, revoked providers,
and the race between initialization and short-probe deadlines. Both the original
350 ms failure and the timeout-state race were reproduced before their fixes.

The broad control run passed 3284 tests and failed the same nine evidence gates
(344 files, 112.11 seconds). This snapshot preceded the final timeout-state guard;
after that guard, all 59 focused service/engine/isolation/worker tests passed.
UI and MCP type checks, Vite build and dist verification passed. Dist contains
5,782,496 asset bytes plus 9,697,456 raw WASM bytes across 89 artifacts.

No qualification status, manifest fingerprint or global readiness limit was
relaxed. Standalone capability probes can still report transient unavailability
while cold initialization is in flight. A fresh CI run is still required.
