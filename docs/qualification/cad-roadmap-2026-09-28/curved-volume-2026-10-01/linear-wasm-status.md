# Packaged linear face proofs

The WASM built from the linear-projection native stage is 9,540,153 bytes, SHA-256 1954cf90254300053ba40a73ce95336001155550b6a0327f1732ec9a3fdc017e. Eight host and actual-WASM tests passed, including cylinder allFacesInjective=true with 102 work units and unresolved distinct-face pairs. Input geometry is unchanged. TypeScript and 137-artifact dist verification passed. The existing Chromium self-intersection scenario passed invalid-input/retry, cancel/restart, selection/model change, stale replies after import and JSON preservation.

The later exact curved-cap and coordinate-plane shared-edge native improvements are not included in this binary. Full cylinder volume validity and distance remain to be packaged and checked separately. General spheres and curved-volume support remain open.
