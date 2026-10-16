# Coefficient-owned filled volume and strict smooth closed Solid

Status: new native code and focused checks; full frozen qualification pending.
The authoritative full-run result will be `summary.json`, with its source guard
in `native-inputs.json`. No lower-stage test results are reused.

See `docs/design/owned-affine-volume-solid.md` for the proof chain and limits.
The driver includes eight fresh gates: NURBS, original volume matrices, original
regularity/G2, BRep, serial AAG, JSON bridge, JSON/STEP integrations (including the
new supplied-model audit), and no-codec libraries.

Focused native evidence includes 6 affine-kernel cases, 11 strict Solid cases,
4 public JSON/STEP cases, and the affine candidate's literal double-cover
negative. The full suite must independently rerun these tests against frozen
inputs before this checkpoint is called qualified.

The original source has separate regularity/embedding/topology certification;
its full G1/G2 boundary guarantee remains open. Physical cap G1/G2, public WASM,
external STEP matrix, semantic G1 matrix, browser and fresh CI remain separate,
unqualified gates. This checkpoint does not complete the full user goal.
