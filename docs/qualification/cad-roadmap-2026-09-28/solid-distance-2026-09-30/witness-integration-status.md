# Filled-volume witness integration, 2026-10-01

The native separation witness now reaches WASM, a checked worker response and the measurement panel. The panel shows original face indices and coordinates; the scene draws two markers and their connecting segment. Confirmed contact keeps its single marker. Close and input changes clear the markers and cancel the request.

Qualification: 272 DirectModeler UI tests, 9 volume-distance protocol/WASM/panel tests, TypeScript and dist verification passed. Chromium 151.0.7922.34 passed containment, invalid orientation, certified contact, 3 mm separated cuboids and a 1 mm gap inside a cavity. Both mouse and keyboard runs preserve exported JSON and terminate the held obsolete worker. Keyboard run used 2451 Tab presses. Reports and screenshots are alongside this file. The witness protocol rejects missing witnesses for non-null separated distances, bad face/UV indices, non-finite coordinates, invalid enclosures and coordinates inconsistent with the distance interval.

The packaged geometry WASM is 9,537,703 bytes, SHA-256 f4e8fbf05a398edaf652f52da6c34fecf8d9c19435560df8a5b0d40ebd47106b. A pair supplies an upper distance bound; it is within the requested minimum-distance tolerance only when convergence is proven. The UI therefore labels these points by their faces rather than claiming an exact optimizer.

General curved volumes, wider cavity/STEP cases, complete original-geometry self-intersection checks and full P0–P3 acceptance remain open.
