# Preserve resolution obligations and inspect queued chart points

At a floating-point threshold contour, retain the original unresolved rectangle and probe midpoint enclosures of already queued rectangles. All probes use original rational interval bounds and charge actual cells/spans. A strict upper distance below the threshold refuses full-chart clearance. Exhaustion or failure to find such points preserves the resolution obligation and produces no certificate.

All 32 prior resolution pairs were independently rerun: 5 now have `source-face-gap-chart-close` refusals, 27 remain resolution limited. Aggregate cells: 26048 previously, 27761 with point probes. No pair is promoted to a wall certificate. 126 source unit tests and the source integration test pass. Whole-body coverage retention is being checked separately. This source change is not included in packaged WASM yet.
