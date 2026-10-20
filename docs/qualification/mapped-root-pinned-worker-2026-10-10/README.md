# Mapped root chain through pinned worker and STEP

The selected real-worker tests passed (two tests, 5.00 s): restore the existing
unit tetrahedron with a split root edge, preserve original definitions and
shared endpoint identity, prepare STEP with five vertices/seven edges/four
faces, then Undo/Redo and serialize/reload the document and restore the same
source definition and edge vertex IDs. Log:
`/tmp/mapped-root-pinned-worker-20261010.log`.

The worker kernel SHA is pinned to
`ec4828e66d4bfb072d5153d85fb12d9a7974ddb84ffb0712add3a4d648bc015b`.
The source-body test harness now verifies each worker's imported identity
before dispatch and exports the initial pinned SHA rather than rereading the
public file at report time. Four matching/mismatching worker startup controls
pass (`/tmp/source-worker-artifact-pin-controls-20261010.log`).

`worker.json` preserves the native result; `mapped-chain.step` is the exported
solid. Independent OCCT verification is recorded in `occt.json` when complete.

Scope: one existing mapped root chain fixture through actual WASM/Node worker,
STEP and document history. This is not the three-part CAD acceptance, browser
interaction/reload, general nonlinear roots or the latest local/global
oblique-plane extensions. The current optimizing WASM build is not this binary.

Independent OCCT acceptance passed: one valid solid, volume
0.16666666666666663 mm³ versus 1/6 mm³ (error 2.78e-17 mm³), axis-bound error
1.01e-7 mm or less including OCCT's boundary tolerance. This confirms the
specified exported tetrahedron; it does not qualify general fillets.
