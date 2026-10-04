# GPU loss and restoration of retained profile scenes

Six browser scenarios pass in Chrome Canary 157.0.8081.0: union, subtraction and intersection, with mouse and keyboard. The qualifier now binds the destroyed device to GPUCanvasContext.configure for the canvas whose class is gpu-layer. Its previous last-requested-device variable could select a device belonging to another renderer, so those timeouts did not establish a Solid recovery failure.

Each scenario deliberately destroys the actual Solid device, waits for the visible Retry WebGPU control, checks the GPU layer is disabled, requires visible CPU body polygons, saves the complete unchanged document, activates Retry through mouse or Tab/Enter, waits for recovery, checks a new canvas device configuration and active WebGPU, compares the complete saved document again and verifies selected body remains selected. The three CPU fallback meshes contain 491, 632 and 642 polygons. No page/console errors occur.

The same scenarios continue through current STEP export, Undo/Repeat and real page reload. All six STEP exports match byte-for-byte the previously independently qualified OpenCascade files in profile-regions-native-2026-10-02/final. Those oracle reports remain the independent geometry evidence; OpenCascade was not rerun here. Native device/source/artifact hashes are in the reports. The recovered plate-with-hole screenshot was inspected.

This resolves the prior unqualified GPU-loss path for these six scenes. Hardware removal, driver reset, all devices/browsers, larger scenes and full P0–P3 coverage remain unproven. Product source is unchanged; the device targeting and recovery assertions are qualification fixes.
