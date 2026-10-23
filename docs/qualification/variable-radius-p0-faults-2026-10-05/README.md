# Variable-radius command: worker faults — 2026-10-05

The current browser command scenario injects one transport failure, verifies Apply is disabled and the document unchanged, retries and obtains a ready preview, then cancels without changing the document. A second scenario captures a real successful worker reply, cancels the command and explicitly delivers the old reply; the document stays unchanged. Normal apply, Undo/Redo and reload then pass.

Mouse and keyboard scenarios passed. These checks qualify this command and do not establish the full P0 execution matrix.
