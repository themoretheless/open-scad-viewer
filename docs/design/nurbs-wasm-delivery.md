# Current-source NURBS WASM delivery

The native catalogue remains 240/1000. The five exact primitive JSON routes do
not add capabilities and do not establish editor, Worker, STEP or production
package qualification.

Source `0640ebd846dcfa1112cf50f60ac33337f8e4e6b3` was built with the existing
release recipe in a private target. Stock Binaryen 116 produces 16,794,038 bytes:
16,822 above the decoded-module limit and 794,038 above the streaming limit.
The module is not admitted, installed or assigned a delivery profile.

Actual raw-module validation passes 102 surface-intersection binary ABI cases.
The exact constructors pass 11 native-output comparisons, 11 atomic budget
refusals and 11 forged-authority rejections. Independent Fraction-based oracles
check all 11 returned geometries and eight altered-record negative controls.
These are bounded correctness checks, not broad performance qualification.

`check-exact-primitives-wasm.mts` takes a WASM path, output directory, declared
source commit and the native contract log. Its receipt binds the binary and log
hashes; source provenance must still be established by the build receipt. It
preserves original request number tokens when writing oracle records because
JavaScript's decimal printing of exact large binary64 integers can differ from
the native integer spelling. Geometry responses always come from the actual ABI.

Two size experiments were rejected: JSON insertion outlining increased size by
24 bytes and was reverted; additional Binaryen GUFA/folding/convergence produced
16,779,786 bytes, still outside both limits. Neither experiment removed geometry
features or changed caps. JSON ordering, temporary destruction, duplicates,
number categories and binary round trips have dedicated regression coverage.

Evidence is in `docs/qualification/nurbs-wasm-delivery-2026-11-03/`. Subsequent
candidates need their own source/recipe hashes, ABI/oracle checks and runtime
measurements before production admission. Existing exact-hash delivery profiles,
asset budgets and historical receipts remain unchanged.

A private WASM-only `cad-predicates` opt-level `z` override produces 16,641,867
bytes (152,171 smaller), with 135,349 bytes decoded headroom but 641,867 bytes
over the streaming cap. Its 102 surface ABI cases, 33 exact-primitive checks
and independent geometry oracles pass. This is an experimental command-line
override, not an adopted production recipe or a native profile change.

Five alternating fresh-instance runs of each variant retain identical results
for all 102 ABI cases. The 26 timed cases total a median 477.48 ms for stock and
471.71 ms for the candidate. This small difference is a bounded timing smoke
check, not a general speed improvement or whole-application benchmark.

Private packing succeeds within the existing compression/decompression bounds;
the encoded `bytes.ts` is 5,887,451 bytes. The verifier decodes it byte-for-byte
to the candidate hash. This is source payload size, not a measured Vite chunk.
No production artifact, exact-hash profile or delivery budget was changed.
