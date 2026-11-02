# Combined 218-capability macOS WASM build identity

The actual combined recovery sources, including the reconstruction fit correction, compile to the same optimized geometry WASM bytes as the existing v8 public and archived artifact: SHA-256 442d2703d243fe29ed8434dcbce04e7cf06c2be36be149af07b51b00e45160f6, 15,900,166 bytes. All three payloads were compared directly. The candidate was not promoted over a different public binary.

The build recipe, checksummed Binaryen116 tool identity, exact compiler version, raw/optimized hashes, 4310 Rust-source/manifest hashes and successful build log are preserved. Inputs were verified unchanged across the build and checked again here.

This is byte identity for the macOS compiler build, not a Linux/Windows build qualification, integration of all 218 native APIs into WASM, new source-bundle/G0/G1 qualification, hosted CI or main delivery. The full native library and selected public tests have their separate source-bound receipt. Native-only additions do not acquire application qualification from unchanged WASM bytes.
