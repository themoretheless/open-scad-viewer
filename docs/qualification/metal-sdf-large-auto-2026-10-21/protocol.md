<!-- noninterference-2026-10-21 -->
Current runners observe competing jobs and never suspend them. A busy or contaminated window cannot qualify. Earlier measurements retain their recorded runner hashes; their raw results have not been rewritten.

# Protocol fixed before measurements

Apple M4 Max / Metal, one frozen release test binary. Baseline disables only the SDF small-input arena; candidate retains the shipping arena. Both use the same shaders, queue, synchronization, readback and polygon extraction.

Seven cases: GPU sphere 64³; Auto sphere 64³ (CPU); Auto tetrahedron 48³ (CPU) / 49³ (GPU); Auto closed bipyramids with 84 / 86 triangles at 32³; sequence 84→86→84 at 32³. Aligned input sizes are 4048 / 4120 bytes. Transition samples clear cached buffers before timing, retain pipelines, then time all three public calls including initial allocation, growth and output drops. Steady samples retain capacities.

Before timing, compare every output coordinate bit and index, require nonempty meshes, check Auto selection, GPU sampler use, arena selection and retained fallback after shrink. CPU Auto must match explicit CPU and leave GPU buffers unallocated.

Each case: 300 ms warm-up, 24 alternating ABBA/BAAB paired blocks; three independent windows. Three AA-only windows for EVERY case must pass before any AB. Full windows contain AB followed by AA. AA gate: median A/B within 0.97–1.03 and 95% bootstrap interval relative width at most 1.06. Retain all raw attempts; retry environment-contaminated windows only. Four retries maximum. Pause build/test/explicit native benchmark competitors reversibly, journal identity before STOP, monitor every 0.5 s, require 3 s quiet, restore with finally and an independent 8-minute watchdog. Ordinary desktop activity is not stopped.

Only claim a new speed gain when all three full windows have stable AA and lower 95% paired-bootstrap bounds above one. CPU and >4 KiB cases are unchanged-path controls, so a speedup is not expected. Do not infer a regression from an interval crossing one. Previously qualified small-workload optimization can remain without a new large-grid speedup; any confirmed large-workload regression must be investigated before retaining it. No changes to Auto threshold or synchronization without separate evidence.
