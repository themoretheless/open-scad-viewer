# cad-step

STEP and IGES interchange for `brep-core` models, split out of the kernel so
that `brep-core` carries no file-format code and `cad-step` reaches it only
through its public API (there is no dependency back).

| module | capability |
| --- | --- |
| `step_interchange` | constructor-corpus STEP (`step-interchange/1`, `/2`) |
| `step_interchange_v3` | AP242 topology STEP v3–v10, product occurrences, regularity evidence |
| `nurbs_step_interchange`, `nurbs_step_trimmed`, `nurbs_step_solid` | freeform bicubic faces, trimmed faces and solids |
| `iges_interchange_v1` | the frozen `iges-interchange/1` walking slice |
| `iges_interchange_v2` | IGES v2 (`iges-interchange/2`) |
| `close_topology_interchange` | STEP/IGES envelopes for audited topology complexes |
| `source_exchange_step` | AP242 candidates for original source bodies |

`geometry-bridge` routes the `brep_*step*` / `brep_*iges*` operations here.
Qualification fixtures stay under the repository `tests/fixtures/`; the JSON
model and definition fixtures under `tests/fixtures/cad-step/` are produced by
`examples/dump_step_model.rs` and the two env-var hooks named in the tests
that read them.
