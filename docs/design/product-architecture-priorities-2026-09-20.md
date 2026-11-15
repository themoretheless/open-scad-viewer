# Product and architecture priorities

Updated 2026-11-13: removed 0 resolved items; none of the four implementation
steps is complete. File names below follow the current `src/App.svt` and
`src/components/*.svt` layout.

Source review: main f68ce0a6, 2026-09-20. This is a scoped comparison, not a
competitive benchmark, feature certification or completed implementation plan.

## Verified comparison

OpenSCAD Playground documents Monaco, syntax highlighting, import and symbol
completion across transitive imports, bundled libraries and installable offline
PWA behavior. Its README also lists remaining preview and mobile-editor work;
do not infer production completeness from the feature list.
[Primary source](https://github.com/openscad/openscad-playground)

Our `App.svt` still owns textarea inputs. `editorDiagnostics.ts` already maps
build failures to source ranges and reveals them through selection. The gap is
an integrated editing experience, not an absence of diagnostics. Customizer
controls and named parameter presets already exist in `CustomizerPanel.svt`,
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

Our `rushGraphAssembly.ts` already supports acyclic anchor mates, placements,
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
   `crates/geometry-bridge/src/mesh.rs` still calls `face_ids` from both
   `export_buffers` and `render_buffers`.
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

## Step 2 status: editor adapter and baseline (2026-11-14)

Done without new dependencies:

- `src/editor/EditorAdapter.ts` defines `EditorAdapter`: text get/set and
  range replacement as single programmatic edits, selection with direction,
  focus, scroll and line height, engine-native undo/redo, highlight ranges for
  source↔scene navigation, and IME composition state/events.
  `createTextareaEditorAdapter` wraps the current `<textarea>` overlay.
  `App.svt`, `useCodeEditorViewport`, `useEditorFind`, `useEditorFormatting`,
  `useParameterPresets`, `useSourceProvenanceCaret` and `revealDiagnostic` now
  reach the editor only through the adapter; the raw element ref is kept only
  for the template binding.
- One deliberate behaviour fix: Svet's `v-model` checked a `composing` flag that
  nothing set, so intermediate IME text reached `code` (and render scheduling)
  on every compositionupdate. The adapter now sets the flag and commits the
  composed text once on compositionend; `handleEditorInput` ignores input while
  composing.
- `src/editor/editHistory.ts` states the undo-granularity contract a
  replacement engine must keep (typing coalesced within 500 ms on one run,
  newline splits, every programmatic edit one step, one step per composition).
  The textarea engine keeps the browser's native undo stack as before; the
  programmatic `setRangeText` edits are not on that native stack today, which
  is unchanged and is the main undo gap a real editor engine closes.
- Tests: `tests/editorAdapter.test.ts` (adapter, synthetic
  compositionstart/update/end through the real `vModelText`, find/replace,
  Tab indent, caret→scene highlight and scene pick→editor selection, undo
  contract) and `tests/editorInputLatency.test.ts` (baseline below).

### Baseline the editor decision is compared against

Node 22.23, Apple M4 Max, `npx vitest run --project node
tests/editorInputLatency.test.ts`, median of 15 keystrokes in the middle of a
generated OpenSCAD source. "Keystroke→model" is adapter edit, `code` update and
caret↔scene highlight sync. "Highlight refresh" is the synchronous recompute
of `highlightCode`, the block/guide structure and the visible gutter window
that the next render reads. Browser layout and paint are not included.

| Source | Characters | Keystroke→model | Highlight refresh |
|---|---|---|---|
| 5,000 lines | 278 k | 0.06 ms | 47 ms |
| 20,000 lines | 1.13 M | 0.05 ms | 295 ms |

The whole-document regex highlighter dominates: a 5k-line file already exceeds
a 16 ms frame per keystroke, 20k lines is ~0.3 s. Both CodeMirror 6 and Monaco
highlight incrementally and only render the viewport, so this is the number
the integration must beat.

Bundle (`npx vite build`): the editor is not a separate chunk; it lives in the
entry chunk `index-*.js` at 240.3 kB minified / 73–74 kB gzip. The editor
modules themselves (adapter, viewport, find, formatting, provenance caret,
diagnostics, highlighter, block model; esbuild `--minify`, `svet` external)
are 16.0 kB minified / 6.8 kB gzip.

### CodeMirror 6 vs Monaco (decision for the user)

| | CodeMirror 6 | Monaco |
|---|---|---|
| Bundle | Modular ESM; a minimal setup (state, view, commands, history, search, language support) is roughly 120–200 kB minified, ~50–70 kB gzip, tree-shakeable | Monolithic editor core ~2–3 MB minified (~0.7–1 MB gzip) before languages; tree-shaking is limited |
| Workers | None required; parsing runs on the main thread incrementally (Lezer), can be moved to a worker manually | Needs `MonacoEnvironment.getWorker` and bundled worker entries (editor worker at minimum); Vite needs worker plumbing |
| IME | Uses `contenteditable` with explicit composition handling; good CJK and Android support, mobile is a design target | Hidden-textarea input; desktop IME is solid, mobile/touch support is officially limited |
| Accessibility | Native DOM text in a `contenteditable`, works with screen readers; ARIA hooks configurable | Strong screen-reader mode (VS Code heritage), but an "accessibility mode" toggle and hidden textarea model |
| Theming | CSS + `EditorView.theme` / `HighlightStyle`; maps directly to the app's CSS variables and both themes | JSON themes via `defineTheme`; global per page, harder to tie to CSS variables |
| Undo | `@codemirror/commands` history: transaction-based, `isolateHistory` and `userEvent` annotations map 1:1 to the contract in `editHistory.ts` | Model undo stack with `pushStackElement`; also maps, less granular control |
| OpenSCAD language | Write a Lezer grammar or a `StreamLanguage` from the existing highlighter | Monarch tokenizer from the existing regexes |

Recommendation (engineering judgment, not a measurement): CodeMirror 6. It
fits the bundle budget of the entry chunk, needs no worker setup, supports
mobile IME, themes through CSS variables and exposes the transaction
annotations the undo contract needs. Monaco is worth it only if VS Code-level
features (multi-cursor UX parity, rich IntelliSense UI, diff editor) are
wanted and a ~1 MB gzip lazily loaded chunk is acceptable. The bundle figures
in the table are approximate public sizes, not measured in this repository;
measure them with `vite build` after installing the chosen package.

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
