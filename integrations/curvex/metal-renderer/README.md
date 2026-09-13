# Curvex retained Metal renderer

The retained 2D renderer is implemented and qualified on Apple M5 / Metal,
including the real native application at Retina scale. The deliverable is a
reproducible **isolated Curvex migration**; the original checkout is unchanged.
The [final report](evidence/production-summary.json),
[migration patch](evidence/production-migration.patch),
[source hashes](evidence/production-sources.json) and
[binary hashes](evidence/production-binaries.json) record the tested result.
This is platform/workload qualification, not a guarantee for every GPU or input.

## Architecture and behavior

The dependency-free planar kernel owns geometry and tessellation. The Curvex
adapter owns immutable CPU mesh snapshots. `cache.rs` owns bounded GPU geometry
and ordered batch identities; `lib.rs` owns buffers, camera uniforms and pipeline;
`egui_integration.rs` owns painter integration, frame admission and fallback.
Qualification/readback code is separate from the production library.
See also the [2D architecture report](../architecture-qualification/README.md).

Solid meshes, sampled gradients, gradient strokes and shadow meshes retain
vertices in document coordinates. Pan/zoom updates camera uniforms instead of
transforming and uploading every vertex each frame. Adjacent compatible meshes
share one batch without changing drawing order. Visible painter operations and
clip changes break a batch. A monotonic scan frontier avoids quadratic scanning
of preceding empty painter paths. Solid and gradient paths share snapshot/cache
ownership instead of maintaining independent implementations.

Immutable mesh identities change on edits/recoloring. Weak source ownership
prevents pointer-reuse cache hits; frame leases keep evicted buffers alive until
their draws finish. Reinstalling the renderer rebuilds device resources. The
ordinary egui mesh path handles unsupported formats/inputs and admission failure.
Set `CURVEX_DISABLE_GPU_CACHE=1` before launch to disable retained rendering.

| Payload policy | Bound |
| --- | ---: |
| CPU geometry cache in the GPU migration | 128 MiB / 8192 entries |
| CPU GPU-upload snapshots | 32 MiB / 8192 entries |
| Retained GPU geometry | 64 MiB / 8192 entries |
| Geometry admitted to one frame | 64 MiB |

These are payload limits, not a total process/driver RSS promise. The spatial
5000-shape workload retains 133,163,074 bytes of CPU geometry and 24,516,468 bytes
of GPU geometry. The former fits the 128 MiB budget; the previous 64 MiB geometry
cache thrashed on this working set. Warm camera motion causes no geometry uploads.

## Verification

All final runtime runs completed successfully with empty error logs.

- **1494 application tests pass**, zero failures; one existing ignored timing
  test also passes when explicitly run. Two renderer contract tests pass.
  Application `check --all-targets`, doctests (zero examples) and release builds
  pass. The renderer library passes Clippy with `-D warnings`.
- **288 exact GPU pixel comparisons**: 12 actual Curvex fixtures, three zooms,
  dithering on/off, RGBA/BGRA unorm, at DPI 1 and 2. Every case executes the
  retained callback. Fixtures cover holes, self-crossings, nonlinear/repeated
  gradients, strokes and shadows. [DPI 1](evidence/production-parity.json),
  [DPI 2](evidence/production-dpi.json).
- **10 lifecycle phases** match baseline pixels exactly: initial/warm, pan,
  fractional zoom, geometry edit, recolor, document restore, renderer reinstall,
  a newly created Metal device and disabled fallback.
  [Lifecycle evidence](evidence/production-lifecycle.json).
- The [cache probe](evidence/production-cache.json) checks 100 hits without upload,
  changed pixels after an edit, eviction with an active frame lease, reupload
  after clearing, invalid indices and insufficient budget.
- **66 full-canvas image comparisons** across 100/1000/5000 spatially distributed
  shapes. The maximum channel difference is 0 for 100/1000 and 1/255 for 5000.
  Final texture captures: [100](evidence/canvas-100.png),
  [1000](evidence/canvas-1000.png), [5000](evidence/canvas-5000.png).
  The 5000-shape capture was also visually inspected.
- The real application factory/window runs at DPI 2 with Bgra8Unorm:
  [enabled](evidence/production-native-enabled.json) reports one upload and
  29 cache hits; [disabled](evidence/production-native-disabled.json) reports
  no retained uploads. This smoke uses isolated persistence and a foreground
  qualification shape; it is not a complete manual editing session.

## Measured performance

Release build, actual `draw_canvas`, same document for baseline and retained
renderers, camera movement, two warmups and twenty alternating samples per path.
Values below are medians from [raw samples](evidence/production-spatial.json).

| Shapes | CPU preparation, baseline → retained | Submit-to-render-completion, baseline → retained |
| ---: | ---: | ---: |
| 100 | 0.546 → 0.244 ms | 1.537 → 1.538 ms |
| 1000 | 6.914 → 1.906 ms | 3.079 → 3.061 ms |
| 5000 | 40.560 → 10.402 ms | 6.117 → 3.076 ms |

CPU preparation includes encoding and harness target allocation. Completion
latency includes queue submission and waiting; texture readback is a separate
submission. **Neither column is GPU-exclusive time or whole-editor FPS.** The
adapter advertises timestamps but returned zero/reversed counter pairs; invalid
samples are excluded and GPU-exclusive medians are null. Cold geometry creation
is not covered by the warm performance claim, and other machine activity was not
controlled. Each retained spatial scene uses one batch and uploads no additional
geometry during measured camera movement.

## Reproduce

Run from the open-scad-viewer root on macOS with Metal and Rust 1.92 or newer
(tested compiler: stable 1.98.1; the minimum version itself was not executed).
Use a destination that does not already exist:

```sh
python3 integrations/curvex/prepare.py /path/to/curvex /tmp/curvex-metal-new
python3 integrations/curvex/metal-renderer/prepare_gpu.py /tmp/curvex-metal-new
cargo +stable test --manifest-path /tmp/curvex-metal-new/Cargo.toml --tests
cargo +stable build --release --manifest-path /tmp/curvex-metal-new/Cargo.toml
cargo +stable run --release --manifest-path /tmp/curvex-metal-new/Cargo.toml --bin curvex
```

Copy `curvex_headless.rs`, `curvex_dpi.rs`, `curvex_lifecycle.rs`,
`curvex_spatial.rs` and `curvex_app_smoke.rs` from this directory into the isolated
application's `examples/`, naming them `osv_metal_headless.rs`, etc. Run each with
`cargo +stable run --release --manifest-path /tmp/curvex-metal-new/Cargo.toml
--example osv_metal_headless` (substitute the example name). Headless, DPI,
lifecycle and spatial probes print JSON; spatial accepts an optional capture
output directory. Native smoke requires a JSON output filename argument. Run it
once normally and once with `CURVEX_DISABLE_GPU_CACHE=1`.
The standalone cache probe is:

```sh
cargo +stable run --release --manifest-path integrations/curvex/metal-renderer/Cargo.toml --features qualification
cargo +stable test --manifest-path integrations/curvex/metal-renderer/Cargo.toml --features egui-integration --lib
cargo +stable clippy --manifest-path integrations/curvex/metal-renderer/Cargo.toml --features egui-integration --lib -- -D warnings
```

The saved full migration patch reconstructs 21 changed/new application files,
including its lockfile, byte-for-byte from the reference checkout. Its local
path dependencies describe this workspace; regenerate with the scripts when
using another filesystem location. Qualification examples are separate artifacts.
[Dependency comparison](evidence/production-dependencies.json) adds no external
package versions for the native macOS target. The application still uses its
existing wgpu/egui and GUI/SVG dependencies; only the planar kernel is independent
of external runtime geometry libraries.

## Integration limits

The verified host uses MSAA 1, RGBA/BGRA non-sRGB attachments and the configured
egui dithering/blending. Unsupported formats use fallback. The adapter's painter
contract is append-only: do not rewrite an earlier shape via `Painter::set`
after submitting later retained paints. Curvex's tested canvas obeys this.
On renderer/device replacement the host must restore its own fonts/image
textures, including egui's WHITE_UV atlas; this library owns geometry resources.
Automatic recovery from a real OS-level device fault was not injected.

Complex intersection/wide-stroke construction still has cold CPU costs.
Geometry, arrangement and adaptive-gradient precision/work limits remain;
arbitrary pathological documents and other hardware require their own testing.
Historical checkpoint/diagnostic JSON files are retained for provenance;
`production-*` files supersede their interim performance and readiness claims.
