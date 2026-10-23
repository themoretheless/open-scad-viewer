# Variable radius under rigid cuboid placement 

Native implementation recovers a local orthonormal frame from the selected straight edge and its two incident cuboid edges. The full source must then pass cuboid recognition and the global solid audit in that frame; the output is transformed back and audited again. Endpoint radii keep their ordering on the selected source edge. Existing axis-aligned vertical behavior is retained.

The native regression passed all 12 cuboid edges, two endpoint-radius orderings, and both axis-aligned and rotated placements: 48 authored results. Checks include global audit, naming completeness, closed boundary topology and analytic volume using the integral of the squared linear radius. All 12 edges of a sheared fixture are refused and source immutability is checked.

This is a linear cross-section radius law with a conical side surface. It does not qualify a general rolling-ball envelope, nonlinear laws, NURBS edges, cylinder–cylinder blends or corner networks.

Final WASM build and 35 product/real-worker tests passed. Capability scope was updated through a separate qualified plan. Current browser acceptance passed for one rotated edge using both mouse and keyboard, covering preview cancellation, apply, material/body identity, analytic volume, Undo/Redo and reload. Final Vite build, type checking and distribution verification passed. Main commit 351b82fd predates this change; this evidence accompanies the implementation and new WASM. Broader native analytic regression passed 41 tests. A product/WASM test now covers all 12 rotated edges with both endpoint-radius orderings, material/body identity, Undo/Redo and document roundtrip; it passed on the final packed WASM. Type checking passed before the UI help-text update.

Endpoint A/B order is independently checked by evaluating both ends of each rational conical patch and measuring their perpendicular distance to the original selected edge. This passed all 48 native results and prevents a symmetric-volume check from hiding reversed radii. Final type checking including the added worker test passed.
