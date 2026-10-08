# Scalar sweep STEP qualification

The existing Rust constructors produce four swept square edges, retained side
surfaces, isocurves and capped B-rep topology. The existing native STEP V5
exporter and importer execute the first round-trip. The orchestration script
checks sampled wall positions against an independent rational formula.

OpenCascade 8.0.1 then imports, checks B-rep validity, re-exports and imports the
file again. Both stages require six faces and one solid, matching wall positions
and positive volume. The weighted scaled sweep uses independent path and scale
weights and nonunit source domains. Its volume oracle integrates
`4 * ((3-v)/(3-2*v))^2 * 10/(1+v)^2`, with a Simpson convergence check. The
straight RMF profile sweep has analytical volume `140/3` cubic millimeters.
The volume comparison uses relative tolerance `1e-7`; sampled positions use
`1e-8` mm. This is a finite pair of fixtures, not a global embedding, smoothness
or continuous RMF error proof.

Run with Python 3.12 and `cadquery-ocp==8.0.1.0.0` installed:

```
SWEEP_OCCT_PYTHON=/path/to/python node scripts/run-sweep-qualification.mjs scalar-step /tmp/scalar-sweep-step
```

The `Scalar sweep independent STEP` workflow repeats the native and independent
round-trips and uploads all fixtures and reports. The archived manifest pins the
actual runtime binary. Earlier sweep qualification records are left unchanged.
