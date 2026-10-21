# Rush automatic annular construction and part acceptance

Ordinary `brep_progressive_sweep` Rush calls now request native automatic annular caps. Rust owns the geometry decision, physical cap groups, exact-work accounting, original-source replay and material admission. Exact legacy caps retain their geometry; closed sweeps have no terminal caps. The authored example `examples/rush/automatic-annular-rmf-body.r` is included in the browser body-boundary inventory.

## Current runtime

Primary runtime: `4da889d03ce9cb217ad7fb893e9fc86c6718a6f436694f9d12ab90bed892e2f1`.

- Four actual-WASM Rush regressions pass: synchronous/asynchronous automatic construction, closed endpoints and exact legacy layout.
- Fourteen transport/presentation regressions pass. Navigation and exact fillet/history pass three focused UI tests, retaining the original 30-second interaction limit. Typecheck passes.
- The actual Rush → worker → Solid browser scenario passes 15 checks with Metal/WebGPU, including retained source/history and lifecycle cancellation. This does not measure interruption latency inside a running native call.
- Bracket, flange and enclosure each pass 20 edits, 20 Undo, 20 Redo, exact document reload and current STEP download in the mouse browser scenario. OpenCascade independently confirms all three downloaded STEP solids, dimensions and volumes. Maximum bounds error is 1.00000009e-7 mm; maximum volume error is 2.89928722e-7 mm³.
- All three native mixed-operation history tests pass with their original 60-second limits. Kernel warmup uses the original separate 30-second hook limit. Enclosure completes in 18.43 seconds; the earlier 107.267-second frozen-stage timing failure remains historical evidence.
- Keyboard browser acceptance also passes all three parts: 20 edits, full Undo/Redo, exact reload and current STEP export. Independent OpenCascade measurements pass all three keyboard-exported STEP files. Both browser scenarios report no page errors.

A genuine navigation regression was fixed: starting middle/right-button navigation on an edge now reaches the viewport. Two pre-fix failures become passing regressions, preserving primary edge selection and geometry.

## Finite native qualification

The frozen automatic-cap native snapshot passed raw and optimized ABI checks for three original annular cases and negative ownership/budget/replay controls. Fresh material covers all 5,232 face pairs. Independent rational-boundary volume and OpenCascade STEP checks pass all three raw cases. Its optimized runtime is `2176e05a8f3156141180096fd94532a64898c0c28abac3d8a286dc4549096783`. This snapshot does not qualify concurrent native source changes in primary.

This is a finite qualified domain, not general G2: exact G2 is confirmed for one of six terminal cap groups in the three original annular cases. Other groups remain unconfirmed; cap-to-wall joins are C0 and full-boundary smoothness is false.

## Release size gate remains open

Current distribution: **8,260,027 bytes**, unchanged limit **8,080,000**. Geometry chunk: **3,987,306 bytes**, unchanged limit **3,970,000**. Packed/raw identity and unique-payload checks pass. The frozen size-profile trial also exceeded its total limit by 6,583 bytes; byte-identical compression trials did not improve it. The trial build-profile overrides were reverted. The newer primary runtime was preserved.

Detailed local evidence is in `output/qualification/rush-auto-annular-2026-10-14/` (ignored generated output). No claim of full P0, general NURBS fillets or a passing release-size gate is made.
