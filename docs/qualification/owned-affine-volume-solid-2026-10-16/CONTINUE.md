# Coefficient-owned filled volume and strict smooth closed Solid

Status: full frozen native qualification passed: **3116 passed, 4 ignored**.
All eight gates ran fresh. Authoritative results are `summary.json` and
`native-inputs.json`; their unchanged native source guard is
`869d8daa4ba1a510fa25e86ecdb6961da976f9f9ab1dc8572720de621b3902bc`.
Qualified code revision: `56e5a79d01d6a60d8f3ea732dd14c15d84bba54a`.

See `docs/design/owned-affine-volume-solid.md` for the proof chain and limits.
The driver includes eight fresh gates: NURBS, original volume matrices, original
regularity/G2, BRep, serial AAG, JSON bridge, JSON/STEP integrations (including the
new supplied-model audit), and no-codec libraries.

The full suite independently reran 6 affine-kernel cases, 11 strict Solid cases,
4 public JSON/STEP cases, and the affine candidate's literal double-cover
negative against the frozen native inputs. It also reran original volume and
surface regularity/G2 matrices, all BRep and bridge cases, serial AAG and
no-codec libraries. No result from a lower checkpoint was reused.

The original source has separate regularity/embedding/topology certification;
its full G1/G2 boundary guarantee remains open. Physical cap G1/G2, public WASM,
external STEP matrix, semantic G1 matrix, browser and fresh CI remain separate,
unqualified gates. This checkpoint does not complete the full user goal.
