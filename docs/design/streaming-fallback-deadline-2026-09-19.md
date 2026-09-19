# Bounded optional streaming compilation

`compileStreamingWasm` previously caught rejected fetch/compilation but could
wait forever for a pending response or body. Geometry/language warmup and the
photogrammetry shared module then never reached their embedded fallback.
The geometry provider's separate 250 ms readiness deadline did not cancel that
work; subsequent readiness queries could remain unavailable indefinitely.

The optional streaming attempt now races a 2,000 ms deadline. Success returns
the compiled module; failure or timeout returns `null`, preserving the existing
caller-owned fallback. Cleanup clears the timer and aborts the fetch/body, also
on an early compile rejection. The race observes late rejection of its losing
promise. Calls do not share timeout state.

Two seconds is an explicit bounded wait policy, not a measured optimal network
threshold. On slower transfers this trades streamed compilation for the
embedded payload. It does not change the 250 ms provider deadline or guarantee
an overall warmup duration: fallback decompression/compilation still takes time.
Abort releases network activity, but cannot forcibly stop browser compilation
already in progress; late streaming completion cannot replace the caller's
fallback result. JavaScript timers also depend on event-loop scheduling.

## Verification

Six focused Vitest tests cover successful completion/timer cleanup, timeout
with an uncancellable compiler and late rejection, fetch abort and independent
retry, immediate error, non-browser bypass, and actual SCAD compilation with
the embedded language kernel after the timeout. No replacement fake kernel is
used in the last test. The initial combined run with geometry readiness and
photogrammetry tests passed 29 tests; the added real-fallback case then passed
with the other five streaming tests.

Typecheck, production Vite build and the packed-payload/dist gate are checked
after the change. These are local checks, not proof of remote CI or deployment.
