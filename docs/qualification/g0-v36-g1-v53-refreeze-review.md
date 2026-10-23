# G0 v36 / G1 v53 byte refresh

This successor binds the loft kernel after a clean Cargo rebuild, current sources and the worker bootstrap that avoids an inherited second TS loader. It preserves v35/v52 and all earlier bytes and reuses hosted-runner environment freeze v34 unchanged. Own Rust CAD v28 adds the explicit loft implementation paths to the source bundle.

The candidate begins with zero completed clean runs and work units. This refresh executes no qualification row and approves no G0, G1 or production cutover.

macOS ARM64 and Linux ARM64 clean builds differ before optimization; Node 22 and 26 optimize the macOS input identically. Linux x64 independently reports the same digest on Node 20 and 22 in CI. The successor enumerates these observed artifact hashes. Qualification freezing refuses unknown hashes; runtime metadata reports the actual packaged digest without claiming qualification; this is no cross-host byte-reproducibility claim. Windows fixture-mutation tests normalize CRLF before appending STEP entities.
