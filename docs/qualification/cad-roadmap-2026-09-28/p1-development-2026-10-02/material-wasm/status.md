# Material diagnostics in packaged WASM and UI — 2026-10-03

## Packaged kernel

- SHA-256: `b43a354faf74ee89269e3d675e071709cd725ed35ca3f220a78c9937aee5145a`.
- Optimized geometry WASM: 9,811,574 bytes.
- All seven native contract fixtures passed through the actual packaged kernel.
- All seven passed through `createMainSolidWorkerHandler`, with the request-bound
  protocol validator. This includes cuboid/cavity and curved annular cases,
  strict interior segments, original-root chords and exhausted geometry work.
- Original coefficients, source models and stage budgets are preserved.
- The annular wall length interval is
  `[15.3171730398, 15.3171730529]` mm in the browser output.

## Product interface

Measurements now includes “Material along a line”. Users choose between a
strict interior segment and material between two boundary crossings, enter
start/displacement XYZ in mm, and run with the button or Enter. Changing inputs
clears the result and cancels its worker. Esc closes and cancels the check.

The scene shows the authored line and approximate locations of certified
crossings or unresolved regions. Green indicates a proven result, red indicates
a refused line/contact, amber indicates unresolved regions. Labels identify
original face numbers. The panel lists contact/unresolved counts and gives a
localized next action for each native refusal reason. At most 32 scene markers
and 16 listed issues are shown; counts still include the full report.
Vertex distance overlays and controls are hidden while this check is active.

Both final production Chrome Canary runs passed:

- Mouse and keyboard input.
- Annular wall chord with two visible endpoints and the length interval.
- Line through the cavity: four crossings and explicit refusal.
- Strict interior segment success.
- Zero displacement: localized error and cleared scene overlay.
- Held request cancellation: worker termination on Esc.
- Injected transport failure: localized error followed by successful Retry.
- Diagnostic checks leave the exported document unchanged.
- Reload restores the same exported document.
- No browser errors.

Final mouse screenshots were inspected for the result and cavity indication.
The held-request test proves worker lifecycle cancellation; it does not measure
cooperative interruption inside a Rust calculation. Protocol tests separately
check late responses for both job types.

## Verification

`vue-tsc --noEmit`, production Vite build and distribution verification passed.
The distribution has 142 artifacts: 7,319,292 asset bytes plus 11,867,694 raw
WASM bytes, 19,186,986 bytes total. Size budgets were adjusted from measured
artifact sizes while keeping bounded headroom and all identity/shared-runtime
checks.

## Remaining scope

The result qualifies material along an authored line. Normal alignment,
automatic opposing-region selection and global minimum wall thickness remain
unfinished. This evidence does not close general curved fillets, the full P0
command matrix, general B-rep diagnostics or the P1–P3 roadmap.
