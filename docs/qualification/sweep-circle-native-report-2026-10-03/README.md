# Native bounded circle-section repair — 2026-10-03

Added nurbs-core section_circle_repair::repair and JSON operation curve_repair_circle_section, plus a typed TypeScript wrapper. The native operation reconstructs nine-pole positive-weight sections through a common dyadic center and two generators. Exact integer sums stay within binary64 mantissa limits; non-power-of-two quantum, unsupported input, work exhaustion, displacement overflow and excess correction tolerance refuse without publishing partial curves. Original knots and weights are preserved. Outward interval squared distances bound every control displacement; positive unchanged weights extend that bound over the represented rational curve.

Two native tests passed: shared tangent identities with tolerance/work/mantissa refusal, and actual JSON dispatch candidate/refusal round-trip. This is a correction candidate, not a regularity, B-rep ownership or Solid certificate.

The frozen geometry WASM build at /private/tmp/open-scad-viewer-sweep-circle-native-2026-10-03 completed Rust release compilation and is still optimizing/packaging. Runtime verification of the new operation remains pending. Integration must repair owned edges/caps, compose correction displacement into continuousBound and independently revalidate Solid. The full sweep/miter goal remains active and unpublished.
