# Typed curved B-rep source identity, 2026-11-02

Base: `4ad06539c9b6cef2032204363b9ac659d589e796`.

The previous no-default-features build failed because ordinary and seam-normalized Boolean operations called `minkowski3d::curved`, while that entire computation module was gated by `codec`. The repair compiles curved computations in both modes. Optional serialization implementations and the JSON-valued topology descriptor retain their codec gate. No geometric Boolean route is suppressed and codec is not made an unconditional dependency.

Three computational identity/key paths no longer build optional transport Value trees: complete Source, individual Surface, and ordered Curve wires. The private typed writer emits the existing `MGV1` byte protocol directly. Existing hash domain prefixes, certificate versions, object-key sorting, integer sign tags, IEEE-754 bits, nonfinite-to-Null encoding and optional-field rules are retained. Geometry, topology IDs, lineage, change sets, local tolerances, sewing markers and the complete portable tolerance specification remain part of Model identity. A new field added to any source transport schema must also be reflected in this private identity schema and its compatibility tests.

The writer is not a general replacement transport or a new file format. It only encodes the supported typed source definitions for existing hashes and wire keys. Its 128-level nesting and 32 MiB output bounds match the old binary encoder; it checks append size before allocation/copy and emits no partial result on failure. No geometry, degree, control-net, weight, proof-work, decompression or delivery limit is raised.

Five existing report/certificate transport sections move into codec modules. Native source/deviation/topology computations and Boolean verification remain available. The atlas legacy size pin is lowered from 861 to 801 lines; there are no new or raised architecture allowances.

Tests compare actual typed output byte-for-byte with the old Value/binary encoder, including rational/periodic geometry, complete naming records, Unicode, trimmed/reversed uses, integer lifts, signed zero and the original size/depth refusals. Typed smoke contracts exercise source-atlas replay and source mutation refusal, complete saved-corner/material proof, and seam-normalized Boolean replay/refusals. Running those contracts in both feature modes additionally checks the actual emitted source hash.

Results and initial failures are retained in the final receipt. No complete workspace, featureless unit-test suite, frontend runtime, packaged WASM, exchange or global geometry qualification is inferred from the focused checks. The catalogue remains 231/1000; this repair restores existing functionality and does not inflate the feature count.
