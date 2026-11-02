# Exact actual carrier-volume refusal reproduced in the Rust core

Captured from `9aaaf275a` with the temporary logging patch archived here. The
patch was removed from production sources. Every coefficient is represented as
16 hexadecimal binary64 bits, preserving signed zero and every original value.
No sampled, rounded, simplified, refitted or reconstructed geometry replaces
actual BRep arguments. The fixture includes both degrees/knots/poles/weights,
periodic flags, complete rational motion, origin, projection and exact limits.

The captured map has one degree-one pose piece: identity to a quarter-turn,
with Z translation 0 to 2 and unit motion weights. Limits are exactly 99800
volume cells and 746681 remaining exact work from the constructor call.

The new unignored native regression requires a COMPLETE global certificate.
It currently FAILS, as does the originating BRep test. The direct public-core
call reproduces the same refusal: 99800 cells, 235 exact work, 15640 boxes,
ideal-volume-work-limit and all three global flags false. Runtime: 11.25 seconds.
125 original world point-jet signs are positive; these are rejection diagnostics
only and provide no global regularity/injectivity/topology credit.

Test-only metric logging identifies a promising global metric whose complete
source cover stops at the unchanged 1024 visit ceiling, with unresolved boxes
near a carrier edge. Local metric trials then exhaust the aggregate budget.
The direct traced reproduction also FAILS (10.48 seconds). No limit was raised.

Commands: locked Cargo `-p nurbs-core --lib actual_hollow_quarter_turn_complete_field
-- --nocapture`, optionally OSV_TRACE_ACTUAL_METRIC=1. Toolchain
nightly-2026-09-16, test opt-level=1, debug info disabled, two build/test threads.

This packet is a verified failing regression, NOT qualification or a green
publication gate. The last prior full-core green result is the separate
paired-weight-cancellation packet. The full suite was not rerun after adding
this failing test. No WASM/STEP/browser/CI or global G1/G2 completion is claimed.
