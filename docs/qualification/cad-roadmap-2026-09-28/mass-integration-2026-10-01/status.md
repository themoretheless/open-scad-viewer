# Mass integration and palette qualification

Selective per-face directional quadrature keeps both directional error estimates, the original knot/weight conditioning, tolerances and shared evaluation budget. Constant-weight patches use five-point quadrature; varying positive weights use eight points. Mass results remain numerical estimates with solidGeometryStatus=not_certified.

## Verified

- Full native BRep run passed: library 684 passed / three existing ignored, all integration tests and doctests passed. Includes gear, mass conditioning and serialized drilled-sphere regression.
- Fresh optimized WASM: all 872 CAD tests / 74 files passed, including three tolerant Boolean volume identities and original input preservation.
- Typecheck, Vite and dist checks passed. The total asset budget was increased based on measured +2,359 bytes from baseline; other budgets unchanged.
- Chromium checked both injected native error codes, source names, localized recovery, disabled Apply, available Retry, Escape and unchanged exported document. Injection verifies recovery only.
- Chromium visited all 95 palette rows with mouse and keyboard, verified reasons and refused Enter for 45 disabled entries. Stationary-pointer scroll regression reproduced with pointerenter; pointermove passes. Final palette screenshot inspected.
- Independent circular-section Simpson volume oracle retained; sampled agreement is not a certified bound.

## Remaining

This does not prove execution of every command, every-error localization, general BRep validity, arbitrary Boolean dimension preservation or the full P0-P3 roadmap. Command inventory and interaction coverage remain distinct.
