# Rush station reconstruction integration — 2026-10-03

New explicit `.brep_smooth_miter_stations(wall_tolerance: ..., quantum: ...,
max_work: ..., max_deviation: ...)` follows a progressive miter B-rep. Native
lowering and graph normalization, generated JSON schema, TypeScript evaluator,
async progressive construction, viewport evidence and Solid use this operation.
Dimension errors refuse before geometry; exhausted construction and complete
boundary budgets refuse without retaining final certificate presentation.

Constructors privately retain final corrected sections and original polyline
station indices. Reconstruction does not consume mutable public approximation
samples and requires intact model/bound ownership. Original polyline vertices
retain independent one-sided jets, including the closed seam. Rebuilt reports
carry new model-specific profile/station, chart and material proofs, complete
error composition and source authoring flags; original wall proof objects are
not transferred. Unsupported inputs or material geometry remain refused.

The curved Rush example has verified station G2 and Solid, complete boundary
upper 1.1500000000000279 within its explicit 2 mm budget; a 1 mm budget refuses.
The separate right-angle Rush example keeps its original station [1] C0, with
G1/G2 withheld, and independently passes Solid. A finer 1/32 mm reconstruction
lattice keeps this sharp example's wall displacement within its 0.1 mm limit.
These are finite qualifications, not all-mode guarantees.

65 native language tests passed (55 runtime, 10 lowering) with an isolated Cargo
target. 34 frontend tests across eight suites passed on the installed artifacts.
Vue/MCP type checks, production build and scoped whitespace checks passed.
New installed language WASM is
`aac56d2056cc6b24dba96cb1667882797c8ed1a8cbfed080494faf739eb78a82`,
2,278,043 bytes. Geometry remains the qualified
`c2b0c324a53fb352df49386e8768fb608baa47712cb6a959ef593b261e96689d`.

The G2 Rush example passed two wide/narrow scenarios and 28 UI assertions,
including explicit work/bound refusals, Solid success, cancelled build/Solid,
source replacement and recovery. The sharp C0 example passed two scenarios
and 24 assertions. Held worker dispatch tests lifecycle cancellation rather
than interruption latency inside a running kernel; headless rendering does
not qualify a production GPU.

Independent STEP/OCCT matrix contains four rebuilt solids: direct straight,
direct curved, actual Rush G2 and actual Rush C0. All passed geometry, topology,
retained basis/control-net, edge, orientation and analytic-volume checks.
Relative volume errors are at most 2.2617277734851674e-16. Existing direct STEP
round-trip also independently re-proves station G2 and Solid.

Final frozen snapshot:
`/private/tmp/open-scad-viewer-sweep-station-graph-qualified-2026-10-03`.
Source hashes and concrete logs are included. Scope is this frozen graph and
listed primary checks, not all concurrent source migration edits.

Remaining: qualify moving-frame/guide/affine, hollow and closed reconstruction
combinations; extend shared-edge separation beyond the current sufficient
Cartesian synchronized-coordinate criterion where needed; rerun complete
STEP/UI matrices; finish scoped publication and new CI. Overall goal remains
active and unpublished.
