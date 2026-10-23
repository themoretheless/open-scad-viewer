# Groups, workspace state and source deletion

Source deletion in an edit now checks remaining linked instances before committing. Russian and English errors identify the source, count dependencies and explain how to detach or delete them together. Both object deletion and group deletion preserve the whole document on refusal.

## Evidence

- The existing 281 UI tests passed. Two new tests initially had an invalid group fixture (missing source text); after correction both locales passed. They qualify source refusal, group refusal, detachment, successful group deletion and full-document Undo/Redo.
- A separate actual Vue UI test with 1000 linked instances passed: refused source/group deletion preserves serialized history, moving the source into the instance group and deleting the entire group allows full Undo/Redo. This is a host UI test, not a browser rendering or latency benchmark.
- Actual Chromium mouse and keyboard runs passed creation/move/delete history, source/locked-group errors, hidden/locked/isolated/active-group persistence through reload, detachment and final document reload. All downloaded documents are retained with SHA256 hashes. Keyboard used 1655 Tab presses.
- Keyboard screenshots of the source error and workspace state were inspected. The source remains selected; the message identifies it and gives the next action.
- Typecheck, Vite and dist gates passed: 7,196,581 asset bytes, DirectModeler 380,622 bytes. The chunk budget increases to 381,400 for the measured dependency guard; total budget stays 7,197,000. No performance improvement is claimed.

## Open issue

Later mouse reruns, including an isolated run, completed document assertions but failed the strict rendering-error gate: the main WebGPURenderer reported Invalid Texture/CreateView. Diagnostic uncaptured-error data is retained; the Solid canvas was 611x415 at notification. Keyboard and an earlier mouse run passed. This rendering issue remains unresolved; browser reliability and full P0 are not closed.

The qualification does not close arbitrary geometry, all command execution, physical FPS/memory, large-scene browser interaction or the full P0–P3 plan.
