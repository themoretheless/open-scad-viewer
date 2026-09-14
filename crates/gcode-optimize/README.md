# gcode-optimize

Reorder, simplify, and comb model-space toolpaths, then serialize them with
`gcode-core::emit`. This crate does not section meshes, talk to a printer, or
change the print-preview dialect.

Passes, in order: Douglas–Peucker simplify, closed-path seam rotation,
nearest-neighbour order plus bounded 2-opt (open paths may reverse), combing
when avoid contours are supplied, then a retract *policy* that only counts
long travels. Preview emit still writes `G0` travel and never decreases E.

Combing needs per-layer closed contours (`OptimizeInput::with_avoid`). Without
them every inter-path hop is marked `uncombed_travel` and stays a straight
`G0`. Comb waypoints are single-point paths so emit can insert extra `G0`
moves without extruding.

```sh
cargo test --locked --manifest-path crates/Cargo.toml -p gcode-optimize
```
