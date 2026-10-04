# Main-source Solid positive/refusal matrix — 2026-10-04

Actual browser at localhost:5202, existing Rust/WASM build. Test fixture:
compact equivalent of `miter-unsegmented-frame-guide-affine-hollow.r`, tessellation
4. Initial source and original one-body `model` scene were preserved.

| Main-source scenario | Wide | 720 x 900 |
| --- | --- | --- |
| To Solid refuses scalar scale [0,0] visibly | Pass | Pass |
| Previous scene remains on refusal, Undo disabled | Pass | Pass |
| Retry scale [1,1] successfully replaces `model` | Pass | Pass |
| Source drawer closes, body selected in group `model` | Pass | Pass |
| Undo restores original body | Pass | Pass |

Narrow viewport source replacement was also exercised: Build fixture, wait for
the currently visible Cancel build control, restore original main source during
the job. Original 2560 triangles, 1.35 volume and 42.34 area remained; original
boundary certificate returned with the original source. Superseded fixture did
not replace the viewport.

Restored Auto checked, hidden source, reloaded while still 720 x 900. Textarea
source exactly matched the previously captured original. The active group menu
contained `model`; after resetting viewport the restored original Body 1 and
scene were visible with no selection and Undo disabled. Temporary tab closed.
An initial wait for the Body 1 scene button at narrow width timed out because
the scene panel is absent at that width; it was checked after resetting width,
not counted as evidence of missing/restored geometry by itself.

Evidence screenshots:

- `sweep-main-solid-refusal-wide.jpg`
- `sweep-main-solid-success-wide.jpg`
- `sweep-main-solid-refusal-narrow.jpg`
- `sweep-main-solid-success-narrow.jpg`
- `sweep-main-reload-restored-narrow.jpg`

Together with the group and main cancellation reports this covers the requested
workflow classes for this selected combined mode. It does not prove every
geometric refusal, every declared mode, crash recovery, or arbitrary race timing.
The common all-mode UI matrix is still incomplete.
