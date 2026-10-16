# Original closed source G1/G2 successor

Status: full frozen native qualification passed: **3178 passed, 4 ignored**.
All nine gates ran fresh, with zero lower-check reuse. Qualified native revision:
`182f91c694210c50dd4552ba0d5ea4b828c8353e`. Native guard:
`d6b5289088813ec90acdf49a473c43330e6b63082b17a959e3b42f74db628cb7`.

The previous coefficient-owned affine-volume checkpoint is a separate frozen
checkout. Its eventual success does not qualify these changed native inputs.
This successor adds full cad-predicates tests to the eight previous fresh gates.
No lower results are reused. `summary.json` and `native-inputs.json` hold
the authoritative terminal run and unchanged source guard.

The full suite reran: 2 exact geometric-curve predicate tests; 3 original differential
tests covering six frame/clock combinations and all source short budgets; 7
owned-G2 tests including original/retained certificates; 5 native JSON/STEP
tests, including supplied-model audit, strict source-budget refusals, genuine
G1-only source curvature and fresh imported-model source G2.

Additional native STEP diagnostic matrix in `probes/` passes RMF, authored,
guided, nonplanar and forty-interval cases against this exact code. Hole and
reversed-hole imported-model audits refuse: this checkpoint's coefficient owner
requires native face order and parameter gauge. Initial malformed authored/
guided fixture requests and their corrected successor are both retained in the
attempt evidence. These refusals are being fixed in a separate successor; its
changed native code receives no credit from this qualification.

WASM compilation passed (`wasm-compile.json`). Public package rebuilding and
WASM runtime qualification remain false.

See `docs/design/original-closed-boundary-gk.md` for the theorem, native ownership
and limits. Physical caps, public WASM, external STEP, semantic G1, browser and
fresh CI remain separate gates. Full goal remains active.
