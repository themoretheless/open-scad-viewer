# Original five timeout closure — final validation pending

The finalnorm current-main WASM a848ef44f3b5a12d3bccea4fb80c32621312ba6349671db6b7d5e01316ac9dd7 passed the periodic family: plain solid9.484s, plain hollow26.600s, affine solid8.040s, affine hollow26.658s. The original open varying affine RMF hollow passed20.127s. The original closed corrected arc reached30.606s under host load above80. Thus4/5originalcases are confirmed on this artifact, and all-five closure is not claimed. The entire31run was25passed/6timeouts during the load spike.

Rust geometry changes are integrated. Fresh NURBS1436/0, affected BRep face-contact6/0 and volume-validity10/0 passed; the shared-boundary baseline full BRep980/0/3existingignored also passed, but is not claimed as a fresh combined full run. Exact norm focused27/0 passed. The original strict isolated package passed8,079,440<=8,080,000. All original30second test/hook limits, ABI/build/Solid limits, tolerances and geometric work budgets are unchanged.

This task has not replaced main published WASM. A later source refresh was rejected because peer edits to material_chord.rs, fiber.rs and fiber_coverage/world_trace.rs occurred during compilation; those edits remain intact.

Pending: original-five-plus-periodic-baseline6test check waits for CPUload<=10stable30s (session74471), then current-source refresh waits for a60squiet native-source window (session95541). A resource-coordination question is pending in chat. Only a successful current-source build and final runtime evidence may authorize local publication; no full-library continuousBound/STEP/UI completion is claimed. See CURRENT-RUNS.json and source-bound receipts.
