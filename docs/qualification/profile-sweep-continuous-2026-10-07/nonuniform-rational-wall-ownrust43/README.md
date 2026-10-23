# Independent finite STEP development observation

This fixture uses the exact archived own-Rust v43 binary. OCCT checks geometry and first/second derivatives, topology and volume both before and after its independent STEP re-export. It does not qualify all loft/sweep modes or certify native global Solid guarantees. No G0/G1 qualification unit is imported.

The rational nonuniform wall case reproduces the old retained-edge reversal refusal. Native v43 constructs six faces, and OCCT confirms one valid Solid with volume 20. Its native joint audit is still unproven: the later original-basis exact boundary proof was not in v43. Adaptive OCCT volume integration uses relative target 1e-10. The checker uses authored parameter domains and affine chart transformations, including the derivative chain rule; it retains point 1e-8 and jet 1e-7 thresholds.
