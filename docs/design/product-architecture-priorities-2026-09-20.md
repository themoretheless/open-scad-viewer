# Product and architecture priorities

Source review: main f68ce0a6, 2026-09-20. This is a scoped comparison, not a
competitive benchmark, feature certification or completed implementation plan.

## Verified comparison

OpenSCAD Playground documents Monaco, syntax highlighting, import and symbol
completion across transitive imports, bundled libraries and installable offline
PWA behavior. Its README also lists remaining preview and mobile-editor work;
do not infer production completeness from the feature list.
[Primary source](https://github.com/openscad/openscad-playground)

Our `App.vue` still owns textarea inputs. `editorDiagnostics.ts` already maps
build failures to source ranges and reveals them through selection. The gap is
an integrated editing experience, not an absence of diagnostics. Customizer
controls and named parameter presets already exist in `CustomizerPanel.vue`,
`scadCustomizer.ts` and `parameterPresets.ts`.

CadQuery documents selectors over geometric objects, including composable
selection operations. This is a useful reference for queryable semantic
selection rather than treating every picked triangle as a durable CAD face.
[Primary source](https://cadquery.readthedocs.io/en/latest/selectors.html)

Replicad's Blueprint represents reusable planar curves that can be placed on
planes or faces and exported as SVG. It is an architectural reference for
separating a drawing's definition from its placement, not evidence that our
sketch features are wholly absent.
[Primary source](https://replicad.xyz/docs/api/classes/Blueprint/)

Our `modelGraphAssembly.ts` already supports acyclic anchor mates, placements,
slider/revolute positions and bounds. A general constraint solver would be an
extension, not the first assembly implementation. Face/edge query parity with
CadQuery and drawing parity with Replicad have not been established here.

The original audit's universal claims about features nobody else has, and
WebGL being available everywhere else, are not established by this review.
Retain them only as hypotheses, not product positioning facts. Competitor
performance numbers cannot substitute for a shared workload and machine.

## Recommended implementation order

1. Finish runtime artifact identity and qualification publication. Admission
   metadata must identify the bytes actually executed. Existing nine failing
   evidence tests remain blockers to a green qualification claim.
2. Introduce an editor adapter around the existing source/revision state and
   diagnostic model; then integrate a proven editor. Preserve undo, IME,
   customizer edits, source-to-scene navigation and stale-build suppression.
   Acceptance: interaction tests, desktop/mobile screenshots, accessibility,
   large-source typing latency and before/after bundle/startup measurements.
3. Integrate measured Rust grouping work with one producer-owned computation.
   PR #20 currently groups in both `export_buffers` and `render_buffers`.
   Acceptance: complete publication benchmark, sparse/dense fixtures, cache
   reuse and exact selection IDs; kernel-only timing is insufficient.
4. Unify semantic execution behind a backend interface before broadening CAD
   operations. Full-AST roundtripping is neither a fast nor fully compatible
   replacement: current measurements and transport-depth tests disprove that.
   Acceptance: both language profiles, diagnostics, cancellation, geometry,
   provenance and ownership/leak checks across mesh and B-rep paths.

This ordering is engineering judgment based on the current code and measured
boundaries, not a competitor ranking. New editor dependencies have not been
selected or installed by this review.

## Greenfield choices that still apply

- Keep source semantics and evaluated IR in one runtime. Return handles,
  bounded diagnostics and typed bulk arrays rather than entire object trees.
- Separate language evaluation from CAD operations through an explicit backend
  contract. Preserve differing geometry capabilities; do not hide unsupported
  operations behind silent mesh fallback.
- Give scene revisions one authoritative owner. Renderers consume published
  snapshots/deltas; history and editor state do not mutate renderer semantics.
- Keep cold initialization, request readiness and cancellation separate. Shared
  warmup survives one caller's cancellation; disposable workers handle hard
  execution deadlines. These mechanisms already exist and should be retained.
- Share abstractions only where behavior is genuinely shared. Immutable
  historical manifests intentionally duplicate values; deduplicating them
  against mutable current evidence previously changed historical identity.

No wholesale rewrite is justified by this document. Each transition must keep
the current product usable and produce measurable acceptance evidence.
