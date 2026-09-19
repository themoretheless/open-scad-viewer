# Isolated B-rep semantic diagnostics

This internal lane executes actual source lowering, SemanticProgram evaluation
and the Rust/WASM B-rep scene adapter in a disposable worker. It does not activate
the permanent `openscad-viewer/brep-1` provider or qualify its production route.
The browser gallery's ModelGraph B-rep examples remain separately executable.

After `npm run build:geometry`, run the included OpenSCAD syntax example:

```sh
node --import tsx scripts/check-brep-semantic.mjs examples/brep/semantic-enclosure.brep.scad 6
```

The JSON report contains the actual packed-kernel identity, source/program and
display hashes, native snapshot revisions, output identities and derived mesh
metrics. Mesh volume and area depend on display sampling. The report never
asserts a deviation certificate, geometry qualification or production availability.

The Node API is `createBrepDiagnosticSupervisor()` from
`src/mcp/brepDiagnosticSupervisor.ts`. It exposes
`evaluate(source, {quality, segments}, {signal, deadlineMs})`. Each successful
receipt retains the normalized program JSON and native B-rep snapshots in its
scene. Callers must handle typed refusals; there is no geometry fallback. The
former browser worker lane was removed after confirming it had no importers.

Source must explicitly declare `// @language openscad-viewer/brep-1`. Capability
declarations using `@requires` are refused because this lane cannot attest a
production capability manifest. Only the implemented primitive/profile/Boolean/
straight-extrusion subset is supported. XY-preserving `multmatrix` is admitted
for profiles; circular contours require a similarity transform and arbitrary
elliptical trims remain unsupported. Three-coordinate profile curves are never
implicitly projected into 2D.

One job owns one disposable realm. Host cancellation and deadlines terminate
the worker even during synchronous WASM execution. Node waits for termination
before publishing or admitting another job; a failed join quarantines the lane.
Browser termination uses the platform's synchronous `Worker.terminate()` API.
Cancellation during asynchronous Node teardown also prevents publication.

The limits are 250,000 source characters, 256 outputs, 20,000 display triangles,
4 MiB of retained native snapshot/document characters, 16 MiB of accounted
transport data, 10 seconds startup, 30 seconds total deadline and 1 second Node
join. Configuration can only lower the time limits. Node V8 heap/stack limits
do not bound WASM linear memory or process RSS. These are finite diagnostic
guards, not OOM or security qualification.

The parent checks correlation, the current packed artifact, normalized program
attestation, output identities and scene/native data consistency. This does not
independently reproduce source lowering or certify that arbitrary returned
geometry is the mathematical result of the program. Qualification still needs
the complete independent oracle, provenance, fault and release matrix.
