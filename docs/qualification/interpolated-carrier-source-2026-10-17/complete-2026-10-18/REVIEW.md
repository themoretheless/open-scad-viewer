# Completed frozen interpolated-carrier checkpoint

Production Rust revision 312bb2613fe034a9cd3d7ffcebf4f46a1eac15f4 remains
unchanged against all 4473 guarded inputs. The complete selected native
kernel/JSON/STEP/no-codec gate passes 3408 tests, zero failures and four
existing ignored diagnostics. This is the named kernel gate, not the entire
repository workspace or the semantic G1 matrix.

Nineteen fresh private WASM cases pass on artifact 4099324d...3c75428f.
Every freshly generated Rust request is byte-identical to the request already
executed in that WASM; the correspondence ledger and verifier hashes are
recorded. No earlier WASM execution result is transferred to another binary.
Nineteen independent OCCT cases now pass on the current native outputs:
four fibered, six rational filled-cap, four sphere/pole, Bishop, general
frame, affine frame, guided frame, and distinct interpolated-carrier cases.
Independent checks cover STEP validity, rational bases/weights, incidence,
orientation and volume. Native original-source embedding and Gk remain
separate proofs; numerical quadrature does not establish them.

The new six-face different-carrier case explicitly retains C0 physical cap
joins. Its positive original embedding/regularity/topology certificate is
not a cap G1/G2 certificate. The theorem remains conditional on sufficient
proved hypotheses and shared work budgets; unsupported/exhausted inputs
return no certified model. No universal acceptance is claimed.

Public package rebuild, browser/Rush/viewport integration, full semantic G1,
new hosted CI and successor publication remain separate delivery gates.
