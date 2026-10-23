# Vertex and curvature measurement UI, 2026-10-02

## Behavior

Vertex indices and the edge parameter are validated before worker dispatch. Invalid fields show a localized message, a red outline and an accessible association with the alert; previous results disappear. Transport failures provide localized recovery text and separate retry buttons. Closing measurements cancels pending work and ignores its late reply. Edge selection handles Arrow keys, Home and End through the existing selection state, including the empty option.

## Verification

Actual Chrome Canary 157.0.8081.0 / native Apple Metal browser scenarios passed with the GPU scene active, strict page/console error gates, mouse controls and sequential Tab/keyboard controls. The keyboard scenario used 1125 Tab presses. No programmatic selectOption or fill was used in keyboard mode.

- A held vertex request is canceled with Escape, its worker is terminated, and the complete downloaded document stays equal to the baseline.
- The cylinder fixture gives vertex distance 14.142135623730951 mm and circular edge radius 10 mm at parameter 0.25.
- Invalid vertex 9999 and parameter 2 clear their previous results and associate errors with the fields. Both keyboard screenshots were visually inspected: the red outline and nearby correction text are visible.
- Injected worker failures hide private transport details. Retrying both calculations restores the results without document edits.
- Undo removes the cylinder creation, proving measurement and retries did not add history entries. Redo and reload reproduce the exact complete baseline document.
- Full DirectModeler UI suite: 288 tests passed. English and Russian unit cases exercise invalid input and both worker retries. Existing tests cover stale vertex and curvature replies.
- Typecheck, Vite build and dist verification passed. DirectModeler is 383,412 bytes; all assets total 7,199,431 bytes. This is measured feature growth, not a performance improvement.

## Limits

This qualifies the measurements command on one controlled cylinder. It does not certify minimum distance for general B-rep bodies, all possible curvature cases, other diagnostic commands, or the complete P0 command matrix. P1–P3 remains open. Publication and RAG upload remain pending the existing destination approval.
