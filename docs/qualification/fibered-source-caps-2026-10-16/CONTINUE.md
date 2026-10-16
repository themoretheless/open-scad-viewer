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

Live jobs at handoff (poll the same sessions; never restart on observation timeout):

- Full native qualifier session **95348**; external status log
  `/private/tmp/osv-source-fibered-full-native-20261016.log`.
  Internal logs and incremental `summary.json` are in this directory.
  At latest observation, domain/owner audits, predicates **64**, and NURBS
  **1608** passed; original-volume matrices are active. New BRep body tests
  **6** observed passed in the active full suite. Full BRep is still active.
- Private release WASM build session **9607**; log
  `/private/tmp/osv-source-fibered-private-wasm-build-20261016.log`.
  Target `/private/tmp/osv-affine-caps-target-20261016` (new cache in this run).
  Command: pinned nightly locked geometry-wasm release wasm32 build.
  Build completed exit0. Preserved binary:
  `/private/tmp/osv-fibered-source-artifact-20261016/geometry_wasm.wasm`,
  **18,190,172 bytes**, SHA256
  `614ae302e233bdb23004c7b0bfc2efde43db0ff5dc40b451b017dd4e4f74c0bb`.
  Runtime session **57007** terminal0: all four new original source/physical-cap
  G2 fixtures passed construct/audit/STEP/refusal; real G1-only seam and explicit
  C0 passed. The same new binary also passed four original sphere and six
  rational annular-cap runtime cases, including true sharp C0: **14 total**. `wasm-build.json` and `wasm-runtime-report.json` are written.
  Run `finish.py` ONLY after all native gates finish: it checks source/log hashes
  and byte-for-byte final fresh fixture correspondence before final runtime credit.
- Earlier exploratory closed-owner regression session **85034**, terminal0: **32 passed**; log
  `/private/tmp/osv-source-fibered-closed-regression-20261016.log`.
  All shared exact-affine arithmetic was already final when this binary built;
  the later edits affected only the new fibered source owner/binder/transport.
  This exploratory run is not substituted for the final full BRep gate.

Completed exploratory checks: native JSON **3**, STEP tests **4** (four positive
fixtures plus adversarial re-audit/budget cases). Independent OCCT **4/4**:
valid positive-volume bodies, exact expected topology, every edge incident to
two faces, 130/130/130/370 original boundary samples. Maximum numerical volume
error **7.105427357601002e-14** mm3. Artifacts:
`/private/tmp/osv-source-fibered-step-inspect-20261016/` and
`/private/tmp/osv-source-fibered-occt-report-20261016.json`.
The final qualifier regenerates the same four fixtures and reruns OCCT freshly.
Keep earlier failed STEP attempts visible as development evidence, not passed gates.

## Runtime and preservation

The private harness tests construction, supplied-model audit, STEP re-audit,
short smoothness-budget refusal, wrong retained weights, true G1-only seam and
explicit C0. It consumes `<fixtures>/<case>/source.json`.
Initially it may use the completed exploratory fixture directory, but the final
fresh native inputs must match byte-for-byte before qualification is reported.
Do not overwrite previously preserved sphere or cap WASM artifacts.
No public WASM, fingerprint, browser/Rush/UI, semantic G1 or CI credit follows.
Force-add ignored evidence JSON/logs/STEP after terminal checks and guards pass,
commit receipts, then preserve the local ref in primary using local-only fetch.

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
