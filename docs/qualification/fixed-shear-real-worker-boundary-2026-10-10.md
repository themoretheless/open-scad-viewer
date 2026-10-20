# Fixed-shear real worker boundary regression

The complete `tests/mainSolidWorkerRealBoundary.test.ts` suite passed:
34 tests, zero failures, 12.04 seconds test time, 13.07 seconds total.
Log: `/tmp/fixed-shear-real-worker-boundary-all-20261010.log`.
Published WASM remained SHA256
`ffb7c3ba14d85c5e5872f2b5bebe17dba49da81bef48c27b26ec27ab69983eaf`
after completion. The retained-cache optimizer was still processing its
separate temporary input and had not published a replacement.

Includes the actual cancelled positive profile-coverage reply delivered
during and after a replacement zero-budget result. Production controller
keeps the replacement and does not admit the late proof. These are real
worker-thread boundary tests, not full browser command acceptance.
