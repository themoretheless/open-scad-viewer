# Local SVG compatibility patches

`usvg` and `resvg` are vendored from the published 0.47.0 crates, upstream
Linebender/resvg commit `3a0fdba53ccf2d346b54cc53ba7adf0ee60d0707`.
Their original MIT and Apache-2.0 licenses and Cargo manifests are retained.
The workspace `[patch.crates-io]` entries ensure both libraries use the same
local `usvg` types; no package-registry cache is modified.

Local changes implement CSS geometry and non-scaling strokes through the
existing SVG parser and rendering pipeline. Non-scaling outlines are
constructed in the outermost SVG viewport, including dashes and caps/joins,
then returned to the element's coordinates for paint servers and clipping.
Stroke-width markers retain their host-space size. Flattened text retains
its absolute transform. The normalizer writes ordinary filled stroke outlines
at the document's intrinsic size, making exported geometry independent of a
consumer's viewport and DPI. `resvg` uses the same outline routine.

SVG source, font, image, reference expansion and stroke-work admission remain
in `geometry-bridge`. The local parser's CSS geometry helpers are shared with
that admission path; styles are resolved on the original source so attribute
selectors and `use` shadow contexts retain their meaning.

Acceptance tests live in `geometry-bridge` and are run by
`scripts/record-svg-evidence.mjs`. When updating the upstream versions, review
the local patches against the new parser, tree, writer and renderer together,
then re-run the native and browser/worker roundtrip qualification.
