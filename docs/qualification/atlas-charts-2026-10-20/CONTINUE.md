# Atlas charts continuation

Scope: user asked for curved trims, parameter poles and nonlinear seam clocks.
Rust owns all geometry and certificates. Preserve other shared edits. No agents,
push, cleanup or shared-runtime promotion was requested in this turn.

Implemented sufficient classes:
- embedded three-edge rational UV trims, without holes, inside one tensor knot
  cell; exact triangular composition and global source-atlas checks;
- exact polynomial blowdown of collapsed tensor sides (all4 rotations);
- fractional-linear edge correspondences, including stationary endpoint jets;
  simultaneous positive projective scales over the complex, exact cycle/root
  checks and full final seam identities.

Files: curved/atlas.rs and atlas/{bernstein,transport,trim}.rs in brep-core's
minkowski3d module. The existing standalone atlas API/schema is unchanged.
There is no general non-projective seam solver, general holed/multi-edge trim
triangulation, unresolved trim-vs-knot partition, or universal pole replacement.
The Minkowski nominal-endpoint binding and source-seam preflight are retained;
standalone source admission must not imply full sum topology.

An existing shared event_nodes.rs optimization had two regressions: destructuring
Cell data instead of its verified node tuple, and hardcoding version1 while the
public intersection verifier accepts1/2/3. Those two local corrections preserve
other authors' edits. Its named replay/forgery regression is run separately.

Evidence source snapshot /private/tmp/osv-atlas-charts-source-20261020;
isolated target /private/tmp/osv-atlas-charts-target-20261020;
checker /private/tmp/osv-atlas-charts-checker-20261020.
Source ledger, receipts and final logs are in this dossier. Read manifest.json
and README.md for final qualification status and binary SHA, not temporary logs.

The sphere fixture is a deliberate exact rational input: radius3 with rational
circular trim clocks [1,1,2] and matching 3D equator clocks. It is not a tolerance
repair of the default sqrt(1/2)-weighted constructor. Other fixtures include a
closed tetrahedron with collapsed sides and a freeform box with a nonlinear
surface clock and inverse face-local pcurves.

Commands from FROZEN cwd (never just staging-root from the live checkout):
- cargo test --locked --manifest-path crates/Cargo.toml -p brep-core --lib
  minkowski3d --target-dir=/private/tmp/osv-atlas-charts-target-20261020
  --config profile.dev.opt-level=1 --config profile.dev.debug=0 -- --nocapture
- same target/profile for --lib
  algebraic_node_replay_rejects_rebound_geometry_parameters_and_census
- same target/profile for cargo build --example verify_profile_certificate

Native replay from main cwd:
python3 scripts/check-atlas-charts-native.py CHECKER DOSSIER/fixtures
DOSSIER/native-replay.json

No WASM compilation or browser/package release gate was performed. Shared
public/generated/dist and existing immutable runtime evidence were not modified
by this task. A future private WASM build must freeze the complete coherent
source tree and requalify the actual byte artifact before promotion.
