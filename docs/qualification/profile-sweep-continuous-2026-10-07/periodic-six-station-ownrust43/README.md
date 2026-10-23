# Independent finite STEP development observation

This fixture uses the exact archived own-Rust v43 binary. OCCT checks geometry and first/second derivatives, topology and volume both before and after its independent STEP re-export. It does not qualify all loft/sweep modes or certify native global Solid guarantees. No G0/G1 qualification unit is imported.

Six closing stations produce twenty faces and one valid Solid. The retained loft volume, rather than the ideal ring volume, is compared across independent STEP re-export.
