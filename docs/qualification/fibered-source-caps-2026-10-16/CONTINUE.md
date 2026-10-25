# Original affine fibers with physical G2 end caps: active qualification

Frozen code: `a24cc8934793256d4d6f028fadc7b2ab45a9f3ad`.
Branch: `codex/source-fibered-caps`.
Checkout: `/private/tmp/osv-source-fibered-caps-20261016`.
Already preserved as a local branch in the primary repository, without network
or touching its dirty checkout. Do not modify frozen source during qualification.
The preceding qualified sphere/cap checkpoint `72f79a3c3` remains intact.

Native source guard: 3796 inputs,
`ac7fac07d2b0d1f227f50fb96f59e312498cd7b1491305af03f899ed6c9a29ea`.
The guard includes all crates, the new original-volume contract, native/OCCT
qualification code, the private WASM runtime harness and its transport codec.
`validate.py` runs 15 fresh gates; no preceding lower-source gate is reused.
Do not infer qualification completion until summary and logs are terminal.

## Completed checkpoint

All jobs are terminal; **do not restart the completed qualifier**.
`summary.json`: **3361 passed, 4 ignored, 0 failed**, all **15** fresh gates.
Native source/log guards passed. No lower-source gate was reused.
Full BRep: 1120 passed/3 ignored; separate AAG:44; NURBS:1608;
predicates:64; original-volume matrices:21; regularity/G2:11; CAD STEP:106;
bridge:340/1 ignored; native JSON/STEP:47.

Independent OCCT: **14** specified cases: four new original-fibered disk bodies,
four original implicit spheres and six original rational annular-cap bodies.
The new disk bodies are valid, have positive volume and exactly expected
incidence/topology. Original boundary-point counts:130/130/130/370; maximum
numerical volume discrepancy: **7.105427357601002e-14** mm3. This is supplementary
numerical evidence, not the native continuous theorem.

Private release WASM was freshly built and separately preserved:
`/private/tmp/osv-fibered-source-artifact-20261016/geometry_wasm.wasm`,
**18,190,172 bytes**, SHA256
`614ae302e233bdb23004c7b0bfc2efde43db0ff5dc40b451b017dd4e4f74c0bb`.
The same binary passed **14** runtime cases (new fibers:4, spheres:4, rational
annular caps:6), including construct, unchanged supplied-model audit, STEP
re-audit, short-budget refusal and wrong-model/source refusal. Real G1-only
seam, explicit C0 and true sharp cap C0 remain distinct. Node26 private ABI
qualification is not public package, Node20-floor, browser or UI qualification.

`finish.py` checked final native source/log hashes, original independent OCCT
reports, binary/build/runtime hashes and exact fresh-native/runtime fixture
byte correspondence. `wasm-build.json`, `wasm-runtime-inputs.json` and runtime
reports bind these results. The frozen production source remains `a24cc8934`;
subsequent commits only preserve receipts. Earlier failed STEP development
attempts remain under `exploratory/` with zero qualification credit.

External completed session/log references (historical, not active):
- Native qualifier95348: `/private/tmp/osv-source-fibered-full-native-20261016.log`.
- Private release build9607: `/private/tmp/osv-source-fibered-private-wasm-build-20261016.log`.
- New runtime57007; sphere runtime42590; rational-cap runtime64581: all terminal0.
- Exploratory closed-owner regression85034:32 passed,1119.64seconds; retained
  for development evidence, not substituted for the fresh full BRep gate.

No public WASM/fingerprint/browser/Rush/UI/semantic-G1/main/CI credit follows.
Preserve final receipts in the branch and the primary local ref. Do not change
the user's dirty primary checkout or archive any of the frozen worktrees.

## Remaining full goal

The theorem is a sufficient class: original rational affine fibers in parallel
world height planes, globally regular planar source carrier and complete
filled planar physical end caps. It includes flat end parameter singularities
without claiming closed-cube Jacobian regularity. Every actual face remains
independently regular; exact oriented cap/wall covers distinguish a cusp from G1.
Arbitrary tilting original frames, nonplanar physical end-cap atlases,
unrepresentable exact retained products and arbitrary pole fans remain open.

Closed spatial Bishop arc seam work must handle BOTH original owners:
`original_smoothness.rs` currently retains a Cartesian path-jet gate even when
exact geometric arc direction jets succeed; `rmf_transport.rs` separately
requires Cartesian C1 at all original knots and closure before ODE integration.
Relaxing only the first gate cannot qualify a source/body. Any successor needs
piecewise positive-speed original transport, exact geometric direction jets,
consistent original arc phase and holonomy correction rates, full knot/seam
coverage and actual/source boundary correspondence. Preserve parameter-mode
speed requirements. Source frame smoothness alone does not prove retained Gk.
Corrected-Frenet planar arc jets also stop outside the proved forward-line case
in `ideal_volume/arc_frames.rs`.

`fullGoalComplete` remains false. Publication remains blocked by the earlier
automatic approval review rejection: fresh human authorization must name the
exact GitHub destination and accumulated private source/evidence payload.
Local preservation is authorized. Do not retry via another upload route.
