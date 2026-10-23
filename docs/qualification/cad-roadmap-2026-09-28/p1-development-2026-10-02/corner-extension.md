# Eight-corner extension

The equal-radius axis-aligned cuboid blend now supports all eight corners. Reflection preserves radius and accounts for shell orientation; the final solid passes audit and naming checks. Duplicate edges and excessive radii refuse atomically.

Validation: 39 Rust operation tests; 34 service, STEP and WASM packing tests; TypeScript; production build and dist budgets. Eight corners x three radii (0.25, 1, 2.5 mm) x two STEP cycles produce 48 valid single solids in independent OpenCascade. Maximum bounds error: 1.01e-7 mm. Maximum volume error: 4.64e-7 mm3.

Default OCCT integration missed the acceptance tolerance for the largest radius. Adaptive integration with explicit Eps=1e-10 resolves this without changing acceptance tolerances. Both reports are retained. This is numerical independent qualification of these cases, not a continuous proof for arbitrary geometry.

WASM was rebuilt. The earlier README and baseline checks describe the initial audit. General NURBS fillets, rotated cuboids, unequal radii, mixed edges, multi-corner networks and complete mechanical-part acceptance remain open. All eight corners have not yet been tested through browser interactions.
