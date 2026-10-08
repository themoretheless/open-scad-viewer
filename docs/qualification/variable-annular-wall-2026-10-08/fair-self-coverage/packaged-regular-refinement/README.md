# Packaged regular self-wall refinement

Package SHA-256: `bc23656581d2e743a07a63f6029fca6b7cbad4fd12a2197e9a7d846130c29e92`.
Native source at `64683ece`; this package excludes the later chart-close gap refusal.

The real WASM worker test checks ten request variants and retains previously proven pair obligations. At 5.99 mm and maximum sine squared 1e-6, all 27 within-face obligations are certified. 160 distinct-face pairs remain unresolved, so whole-wall qualification stays false and its lower bound stays absent.

The UI now requests the separately bounded regular refinement. Production build and artifact verification pass; budgets retain the previous byte margins and use measured artifact sizes. The package audit confirms exact packing round trip and unchanged exported names/kinds. This is not a complete ABI type check or independent geometry verification.

Browser rendering, pointer interaction and crash recovery remain unverified. Worker cancellation/Retry tests use controlled ports and do not prove rendered UI behavior.
