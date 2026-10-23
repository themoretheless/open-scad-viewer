# Combined native sweep and CI correction binding

Bind the rebuilt CAD, Laser CAM and sweep release artifact and current sources. Preserve G0 v46, G1 v63 and own-Rust v37 byte-for-byte. The new candidate starts with zero completed clean work, no imported results and no G0/G1 closure claim. The recorded binary is the tracked published WASM; ignored per-host rebuilds remain separately observed runtime artifacts. This prevents host-specific rebuild bytes from rewriting the release fingerprint while still rejecting any unrecorded mutation of the pinned binary.
