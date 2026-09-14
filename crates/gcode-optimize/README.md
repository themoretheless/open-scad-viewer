# gcode-optimize

Reorder, simplify, and comb model-space toolpaths, then serialize them with
`gcode-core`. Preview emit (`emit_optimized`) stays on the print-preview dialect:
travel is `G0`, never a negative E. Pass a `JobProfile` to
`emit_optimized_job` / `emit_optimized_gcode_3mf_job` for heat + retract job
artifacts (and optional mesh body in the thick `.gcode.3mf`).

Passes, in order: Douglas–Peucker simplify, closed-path seam rotation,
nearest-neighbour order plus bounded 2-opt (open paths may reverse), combing
when avoid contours are supplied, then a retract *policy* that only counts
long travels for the optimize report. Job emit performs real absolute-E retract
from `JobProfile`; preview emit does not.

Combing needs per-layer closed contours (`OptimizeInput::with_avoid`). Without
them every inter-path hop is marked `uncombed_travel` and stays a straight
`G0`. Comb waypoints are single-point paths so emit can insert extra `G0`
moves without extruding.

```sh
cargo test --locked --manifest-path crates/Cargo.toml -p gcode-optimize
```
