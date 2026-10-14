# Native station scale and combined-mode qualification

Station transverse normal-scale proposals now belong to Rust
`nurbs-core::continuity::propose_station_normal_scale`, exposed by JSON
`surface_station_normal_scale` and called from the TypeScript adapter.
Clamped Bezier spans with constant longitudinal weights use stored endpoint
control differences in normalized parameters. Other supported spans use native
rational derivatives. These proposals never promote a certificate: exact
projective strip identities and independent seam regularity remain mandatory.
Native tests cover unequal degree-5 spans, source preservation, degenerate
derivatives, invalid boundaries, varying weights and the JSON boundary.

Exact arithmetic cancels identical expansion representations before repeating
their subtraction. The comparison is charged and still observes cancellation
and work limits; nearby unequal values are not cancelled. Predicate tests and
the complete current native suites passed. The default shared profile/station
work budget is now 3,000,000, with each native predicate still bounded to
1,000,000. The moving-frame hollow reconstruction's complete G2 audit uses
approximately 2.45 million operations. An explicit 2,000,000 budget withholds
station G1/G2, and this refusal remains a regression.

The actual Rush guide + authored-frame + affine hollow reconstruction and the
moving-frame version retain full continuous-boundary certificates and certified
Solid geometry. Applicable profile/station G2 reaches viewport evidence. Original
sharp polyline vertices remain C0. Two runnable examples are supplied in
`examples/rush/progressive-miter-reconstructed-guide-frame-affine-hollow.r` and
`examples/rush/progressive-miter-reconstructed-moving-frame-guide-affine-hollow.r`.

The installed geometry WASM is 10,710,250 bytes, SHA-256
`3eb03b81aa774efbfa8d98a49994b4d0131e428be9e4da41a0ba8eb2acdd913e`.
The unchanged language WASM remains
`aac56d2056cc6b24dba96cb1667882797c8ed1a8cbfed080494faf739eb78a82`.
The complete build graph is retained at
`/private/tmp/open-scad-viewer-sweep-proof-budget-2026-10-04`.
It uses the qualified native graph with the new exact arithmetic and station
proposal code; the primary checkout has since reorganized its native modules.
Both the new native source in the current checkout and the installed WASM were
tested. This does not assert byte identity with a fresh build of all concurrent
primary-checkout changes.

Validation:

- Current native checkout: 693 B-rep, 21 predicates and 795 NURBS tests passed;
  two B-rep measurement tests remain ignored. Native checkout `cargo check`
  also passed.
- Installed WASM through TypeScript/Rush: 35 tests across eight related suites
  passed; Vue and MCP typechecks passed; frozen production Vite build passed.
- Independent STEP/OCCT: all 33 existing cases and all six reconstruction cases
  passed geometry/control-basis preservation, topology, coedge ownership,
  material orientation and volume checks on this installed WASM.
- The two new combined-mode volumes use an independent exact-fraction integral
  of polynomial centre/transverse generators: pi times integral det(C',A,B).
  Reversed hole generators contribute negative volume. This reference rejects
  noncanonical poles/weights and has three analytic/refusal tests. Its scope is
  canonical circle weights rounded to binary64; it makes no general arbitrary
  rational-profile volume claim. OCCT relative volume discrepancies for the
  guide and moving-frame bodies are below 3e-16.

UI qualification passed 64 scenarios with 726 assertions, recorded
in `ui/matrix.json`. It covers finite named fixtures, both viewport widths, successful or
explicitly refused Solid, held build/Solid cancellation, source changes and
recovery. Held dispatch verifies lifecycle cancellation; it does not measure
mid-kernel interruption latency. Headless CPU rendering does not qualify GPU.

This is local qualification, not a new CI result or publication. General
unsupported frames, weights, singular geometry and exhausted proofs still
refuse; selected finite fixtures do not establish universal all-input geometric
guarantees. The broader goal remains active.
