# Curvex position-mesh comparison

This standalone benchmark compares the native render adapters with the pinned
Lyon versions from Curvex's original lockfile. The Lyon dependencies belong to
this comparison executable; the planar library does not depend on them.

Run from the repository root:

```sh
cargo run --release --locked --offline \
  --manifest-path integrations/curvex/render-benchmark/Cargo.toml
```

The JSON output reports median and 95th percentile times from 21 measured runs
after three warmups. Both sides reuse their tessellator and receive identical
document coordinates, fill rule and 0.05-unit tolerance. Stroke width is 2 units,
except for the real Curvex sampled-gradient-stroke fixture, which uses 7 units.
Path construction is outside the measured section; vertex allocation,
tessellation and position conversion are included. Vertex counts, triangle
counts and covered triangle area accompany each timing.

Cases include a rectangle, a compound with 16 holes, cubic fills and strokes,
overlapping contours, a self-intersecting bowtie, the exact f32 polyline from the
headless Curvex gradient-stroke fixture, a 1200-point closed stroke and a
4096-point fill. The benchmark uses each library's own curve flattening, so
curved areas and vertex counts can differ within approximation tolerance.

Run without concurrent builds for useful timings. These measure uncached mesh
construction, not frame time: Curvex caches meshes and does additional gradient
sampling, transforms and draw submission outside this executable.
