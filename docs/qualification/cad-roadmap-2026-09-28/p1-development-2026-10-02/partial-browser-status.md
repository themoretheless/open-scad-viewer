# Partial annular preview browser acceptance

2026-10-02. Native Chrome Canary 157, production bundle, actual WASM worker.

Both `partial-browser-keyboard-verified/result.json` and
`partial-browser-mouse-verified/result.json` pass on the annular fixture
(outer radius 20 mm, inner radius 5 mm, height 6 mm, selected source edge 2).

- Actual preview appears after entering radius 1.25 mm.
- Apply stays disabled; Enter in the workspace leaves the saved document unchanged.
- Esc terminates the actual worker while its completed response is held.
- Injected late success and failure responses leave the document unchanged and remove no cancellation state.
- No browser errors were captured.

The browser check exposed a missing `partialAnnularPreview` request in the
worker handler allowlist. The handler now accepts it. A regression test runs
the complete handler with the actual WASM kernel and validates its 7714-triangle
response. All three protocol tests and TypeScript checking pass.

Screenshots use the CPU SVG fallback. These checks establish input, worker,
preview and cancellation behavior on this fixture; they do not establish GPU
performance, general edge coverage, transition continuity or absence of
geometric intersections. This remains a preview-only operation.
